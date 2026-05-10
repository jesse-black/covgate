use std::{
    env, fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{Context, Result, bail};
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use serde::{Deserialize, Deserializer, de::Error as _};

use crate::{
    cli::Args,
    diff::DiffSource,
    git,
    model::{GateRule, MetricKind},
};

const CONFIG_FILE_NAME: &str = "covgate.toml";

#[derive(Debug)]
pub struct Config {
    pub coverage_report: PathBuf,
    pub diff_source: DiffSource,
    pub gates: Vec<ConfiguredGate>,
    pub markdown_output: Option<PathBuf>,
}

#[derive(Debug)]
pub struct ConfiguredGate {
    pub label: Option<String>,
    pub rules: Vec<GateRule>,
    matcher: Option<PathMatcher>,
}

impl ConfiguredGate {
    #[must_use]
    pub fn is_fallback(&self) -> bool {
        self.matcher.is_none()
    }

    #[must_use]
    pub fn matches(&self, path: &Path) -> bool {
        self.matcher
            .as_ref()
            .is_some_and(|matcher| matcher.matches(path))
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct FileConfig {
    base: Option<String>,
    markdown_output: Option<PathBuf>,
    #[serde(default)]
    gates: Vec<GateEntryConfig>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct GateEntryConfig {
    name: Option<String>,
    #[serde(default, deserialize_with = "deserialize_pattern_vec")]
    include: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_pattern_vec")]
    exclude: Vec<String>,
    #[serde(flatten)]
    rules: GateRuleConfig,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum PatternList {
    Single(String),
    Multiple(Vec<String>),
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct GateRuleConfig {
    fail_under_regions: Option<f64>,
    fail_under_lines: Option<f64>,
    fail_under_branches: Option<f64>,
    fail_under_functions: Option<f64>,
    fail_under_named_functions: Option<f64>,
    fail_uncovered_regions: Option<usize>,
    fail_uncovered_lines: Option<usize>,
    fail_uncovered_branches: Option<usize>,
    fail_uncovered_functions: Option<usize>,
    fail_uncovered_named_functions: Option<usize>,
}

impl TryFrom<Args> for Config {
    type Error = anyhow::Error;

    fn try_from(args: Args) -> Result<Self> {
        let Args {
            coverage_report,
            markdown_output,
            base: _,
            diff_file: _,
        } = &args;

        let dir = env::current_dir()
            .context("failed to determine current directory for covgate config discovery")?;
        let repo_root = git::resolve_repo_root().ok().flatten();
        let file_config = load_file_config_from_with_repo_root(&dir, repo_root.as_deref())?;
        let match_root = repo_root.unwrap_or(dir);
        let diff_source = resolve_diff_source(&args, file_config.as_ref())?;
        let gates = resolve_gates(file_config.as_ref(), &match_root)?;
        let markdown_output = markdown_output.clone().or_else(|| {
            file_config
                .as_ref()
                .and_then(|config| config.markdown_output.clone())
        });

        Ok(Self {
            coverage_report: coverage_report.clone(),
            diff_source,
            gates,
            markdown_output,
        })
    }
}

fn config_candidate_paths(dir: &Path, repo_root: Option<&Path>) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    for candidate_dir in dir.ancestors() {
        candidates.push(candidate_dir.join(CONFIG_FILE_NAME));
        if repo_root.is_some_and(|root| candidate_dir == root) {
            break;
        }
    }

    candidates
}

fn parse_file_config(text: &str) -> Result<FileConfig> {
    let parsed =
        toml::from_str::<toml::Value>(text).context("failed to parse covgate config text")?;
    if parsed
        .get("gates")
        .is_some_and(|value| matches!(value, toml::Value::Table(_)))
    {
        bail!("legacy [gates] table is no longer supported; use [[gates]] entries instead");
    }

    let config =
        toml::from_str::<FileConfig>(text).context("failed to parse covgate config text")?;
    validate_file_config(&config)?;
    Ok(config)
}

fn deserialize_pattern_vec<'de, D>(deserializer: D) -> std::result::Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let patterns = PatternList::deserialize(deserializer)
        .map_err(|_| D::Error::custom("expected string or list of strings"))?;
    match patterns {
        PatternList::Single(pattern) => Ok(vec![pattern]),
        PatternList::Multiple(patterns) => Ok(patterns),
    }
}

fn load_file_config_from_with_repo_root(
    dir: &Path,
    repo_root: Option<&Path>,
) -> Result<Option<FileConfig>> {
    for path in config_candidate_paths(dir, repo_root) {
        if !path.exists() {
            continue;
        }

        let text = fs::read_to_string(&path)
            .with_context(|| format!("failed to read config file: {}", path.display()))?;
        let config = parse_file_config(&text)
            .context("failed to parse covgate config text")
            .with_context(|| format!("failed to parse config file: {}", path.display()))?;
        return Ok(Some(config));
    }

    Ok(None)
}

fn resolve_diff_source(args: &Args, file_config: Option<&FileConfig>) -> Result<DiffSource> {
    match (args.base.clone(), args.diff_file.clone()) {
        (Some(base), None) => Ok(DiffSource::GitBase(base)),
        (None, Some(path)) => Ok(DiffSource::DiffFile(path)),
        (Some(_), Some(_)) => bail!("--base and --diff-file are mutually exclusive"),
        (None, None) => {
            if let Some(base) = file_config.and_then(|config| config.base.clone()) {
                Ok(DiffSource::GitBase(base))
            } else if let Some(base) = git::discover_base_ref()? {
                Ok(DiffSource::GitBase(base))
            } else {
                bail!(
                    "unable to determine a base ref automatically. Try one of: pass --base <REF>; run covgate record-base; create {} manually with `git update-ref {} HEAD`; or configure {} with a base value",
                    git::RECORDED_BASE_REF,
                    git::RECORDED_BASE_REF,
                    CONFIG_FILE_NAME,
                )
            }
        }
    }
}

fn validate_file_config(config: &FileConfig) -> Result<()> {
    let mut seen_names = std::collections::BTreeSet::new();
    let mut fallback_count = 0usize;

    for gate in &config.gates {
        if gate.include.is_empty() {
            fallback_count += 1;
            if !gate.exclude.is_empty() {
                bail!("gate `exclude` requires `include`");
            }
        }

        if let Some(name) = &gate.name
            && !seen_names.insert(name.clone())
        {
            bail!("duplicate gate name `{name}`");
        }
    }

    if fallback_count > 1 {
        bail!("only one fallback gate is allowed");
    }

    Ok(())
}

fn resolve_gates(
    file_config: Option<&FileConfig>,
    match_root: &Path,
) -> Result<Vec<ConfiguredGate>> {
    let mut configured = Vec::new();
    let repo_ignores = Arc::new(build_repo_ignores(match_root)?);

    if let Some(config) = file_config {
        for (index, gate) in config.gates.iter().enumerate() {
            let is_fallback = gate.include.is_empty();
            let rules = gate_rules_from_config(&gate.rules);

            if rules.is_empty() {
                let label = match &gate.name {
                    Some(label) => label.clone(),
                    None => format!("gate #{}", index + 1),
                };
                bail!("gate `{label}` has no rules configured");
            }

            let matcher = if is_fallback {
                None
            } else {
                Some(PathMatcher::new(
                    match_root,
                    &gate.include,
                    &gate.exclude,
                    repo_ignores.clone(),
                )?)
            };

            configured.push(ConfiguredGate {
                label: gate
                    .name
                    .clone()
                    .or_else(|| derive_scoped_gate_label(&gate.include, index + 1)),
                rules,
                matcher,
            });
        }
    }

    if configured.is_empty() {
        bail!("at least one rule is required; configure a [[gates]] entry in {CONFIG_FILE_NAME}")
    }

    let configured = configured;
    Ok(configured)
}

fn gate_rules_from_config(config: &GateRuleConfig) -> Vec<GateRule> {
    let mut configured = Vec::new();

    push_percent_rule(
        &mut configured,
        MetricKind::Region,
        config.fail_under_regions,
    );
    push_percent_rule(&mut configured, MetricKind::Line, config.fail_under_lines);
    push_percent_rule(
        &mut configured,
        MetricKind::Branch,
        config.fail_under_branches,
    );
    push_percent_rule(
        &mut configured,
        MetricKind::Function,
        config.fail_under_functions,
    );
    push_percent_rule(
        &mut configured,
        MetricKind::NamedFunction,
        config.fail_under_named_functions,
    );
    push_uncovered_rule(
        &mut configured,
        MetricKind::Region,
        config.fail_uncovered_regions,
    );
    push_uncovered_rule(
        &mut configured,
        MetricKind::Line,
        config.fail_uncovered_lines,
    );
    push_uncovered_rule(
        &mut configured,
        MetricKind::Branch,
        config.fail_uncovered_branches,
    );
    push_uncovered_rule(
        &mut configured,
        MetricKind::Function,
        config.fail_uncovered_functions,
    );
    push_uncovered_rule(
        &mut configured,
        MetricKind::NamedFunction,
        config.fail_uncovered_named_functions,
    );

    configured
}

fn push_percent_rule(configured: &mut Vec<GateRule>, metric: MetricKind, value: Option<f64>) {
    if let Some(minimum_percent) = value {
        configured.push(GateRule::Percent {
            metric,
            minimum_percent,
        });
    }
}

fn push_uncovered_rule(configured: &mut Vec<GateRule>, metric: MetricKind, value: Option<usize>) {
    if let Some(maximum_count) = value {
        configured.push(GateRule::UncoveredCount {
            metric,
            maximum_count,
        });
    }
}

fn derive_scoped_gate_label(include: &[String], index: usize) -> Option<String> {
    if include.is_empty() {
        return None;
    }

    let joined = include.join(", ");
    if joined.chars().count() <= 40 {
        Some(joined)
    } else {
        Some(format!("gate #{index}"))
    }
}

#[derive(Debug, Clone)]
struct PathMatcher {
    repo_ignores: Arc<Gitignore>,
    include: Gitignore,
    exclude: Gitignore,
}

impl PathMatcher {
    fn new(
        root: &Path,
        include: &[String],
        exclude: &[String],
        repo_ignores: Arc<Gitignore>,
    ) -> Result<Self> {
        let include = build_pattern_matcher(root, include)?;
        let exclude = build_pattern_matcher(root, exclude)?;
        Ok(Self {
            repo_ignores,
            include,
            exclude,
        })
    }

    fn matches(&self, path: &Path) -> bool {
        let include_match = self.include.matched(path, false);
        if !include_match.is_ignore() && !include_match.is_whitelist() {
            return false;
        }

        let exclude_match = self.exclude.matched(path, false);
        if exclude_match.is_ignore() {
            return false;
        }

        let repo_ignore_match = self.repo_ignores.matched(path, false);
        if repo_ignore_match.is_ignore() {
            return false;
        }

        true
    }
}

fn build_pattern_matcher(root: &Path, patterns: &[String]) -> Result<Gitignore> {
    let mut builder = GitignoreBuilder::new(root);
    for pattern in patterns {
        builder
            .add_line(None, pattern)
            .context(format!("invalid gate pattern `{pattern}`"))?;
    }
    builder.build().context("failed to build gate matcher")
}

fn build_repo_ignores(root: &Path) -> Result<Gitignore> {
    let mut builder = GitignoreBuilder::new(root);
    add_gitignore_files(root, &mut builder)?;

    let info_exclude = root.join(".git").join("info").join("exclude");
    if info_exclude.exists()
        && let Some(error) = builder.add(&info_exclude)
    {
        return Err(error).context("failed to load repository exclude file");
    }

    builder
        .build()
        .context("failed to build repository ignore matcher")
}

fn add_gitignore_files(dir: &Path, builder: &mut GitignoreBuilder) -> Result<()> {
    for entry in fs::read_dir(dir).context(format!(
        "failed to read directory while loading gitignore files: {}",
        dir.display()
    ))? {
        let entry = entry.context(format!(
            "failed to read directory entry while loading gitignore files: {}",
            dir.display()
        ))?;
        let path = entry.path();
        let file_name = entry.file_name();

        if file_name == ".git" {
            continue;
        }

        if path.is_dir() {
            add_gitignore_files(&path, builder)?;
            continue;
        }

        if file_name == ".gitignore"
            && let Some(error) = builder.add(&path)
        {
            return Err(error).context(format!(
                "failed to load gitignore file for gate matching: {}",
                path.display()
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use super::{
        PathMatcher, config_candidate_paths, derive_scoped_gate_label, parse_file_config,
        resolve_diff_source, resolve_gates,
    };
    use crate::{
        cli::Args,
        diff::DiffSource,
        model::{GateRule, MetricKind},
    };

    fn gate_rules(gates: &[super::ConfiguredGate]) -> Vec<GateRule> {
        gates
            .iter()
            .flat_map(|gate| gate.rules.iter().cloned())
            .collect()
    }

    #[test]
    fn loads_defaults_from_repo_config() {
        let file_config = parse_file_config(
            "base = \"main\"\n[[gates]]\nfail-under-regions = 75\nfail-uncovered-lines = 2\n",
        )
        .expect("config should parse");

        let args = Args {
            coverage_report: "coverage.json".into(),
            base: None,
            diff_file: None,
            markdown_output: None,
        };

        let diff_source =
            resolve_diff_source(&args, Some(&file_config)).expect("diff source should resolve");
        let rules = gate_rules(
            &resolve_gates(Some(&file_config), std::path::Path::new("."))
                .expect("gates should resolve"),
        );

        match diff_source {
            DiffSource::GitBase(base) => assert_eq!(base, "main"),
            DiffSource::DiffFile(_) => panic!("expected git base"),
        }
        assert_eq!(rules.len(), 2);
        assert!(rules.contains(&GateRule::Percent {
            metric: MetricKind::Region,
            minimum_percent: 75.0
        }));
        assert!(rules.contains(&GateRule::UncoveredCount {
            metric: MetricKind::Line,
            maximum_count: 2
        }));
    }

    #[test]
    fn loads_function_rules_from_repo_config() {
        let file_config = parse_file_config(
            "base = \"main\"\n[[gates]]\nfail-under-functions = 100\nfail-uncovered-functions = 0\n",
        )
        .expect("config should parse");

        let rules = gate_rules(
            &resolve_gates(Some(&file_config), std::path::Path::new("."))
                .expect("gates should resolve"),
        );

        assert_eq!(rules.len(), 2);
        assert!(rules.contains(&GateRule::Percent {
            metric: MetricKind::Function,
            minimum_percent: 100.0
        }));
        assert!(rules.contains(&GateRule::UncoveredCount {
            metric: MetricKind::Function,
            maximum_count: 0
        }));
    }

    #[test]
    fn resolve_gates_rejects_named_gate_without_rules() {
        let file_config =
            parse_file_config("[[gates]]\nname = \"js-ui\"\ninclude = [\"**/*.tsx\"]\n")
                .expect("config should parse");

        let error = resolve_gates(Some(&file_config), std::path::Path::new("."))
            .expect_err("gate without rules should fail");

        assert!(error.to_string().contains("js-ui"));
        assert!(error.to_string().contains("no rules"));
    }

    #[test]
    fn derive_scoped_gate_label_falls_back_for_long_globs() {
        let label = derive_scoped_gate_label(
            &[
                "**/*.really-long-typescript-file-pattern.ts".to_string(),
                "**/*.equally-long-component-pattern.tsx".to_string(),
            ],
            3,
        );

        assert_eq!(label.as_deref(), Some("gate #3"));
    }

    #[test]
    fn path_matcher_respects_repo_gitignore_files() {
        let temp = tempfile::tempdir().expect("tempdir should exist");
        fs::write(temp.path().join(".gitignore"), "ignored.ts\n").expect("gitignore should exist");

        let repo_ignores = std::sync::Arc::new(
            super::build_repo_ignores(temp.path()).expect("repo ignores should build"),
        );
        let matcher = PathMatcher::new(temp.path(), &["**/*.ts".to_string()], &[], repo_ignores)
            .expect("matcher should build");

        assert!(matcher.matches(std::path::Path::new("src/kept.ts")));
        assert!(!matcher.matches(std::path::Path::new("ignored.ts")));
    }

    #[test]
    fn config_candidate_paths_stop_at_repo_root() {
        let dir = std::path::Path::new("/workspace/repo/nested/deeper");
        let repo_root = std::path::Path::new("/workspace/repo");

        let candidates = config_candidate_paths(dir, Some(repo_root));

        assert_eq!(
            candidates,
            vec![
                PathBuf::from("/workspace/repo/nested/deeper/covgate.toml"),
                PathBuf::from("/workspace/repo/nested/covgate.toml"),
                PathBuf::from("/workspace/repo/covgate.toml"),
            ]
        );
    }

    #[test]
    fn config_candidate_paths_walk_to_filesystem_root_when_repo_root_is_unknown() {
        let dir = std::path::Path::new("/workspace/repo/nested");

        let candidates = config_candidate_paths(dir, None);

        assert_eq!(
            candidates,
            vec![
                PathBuf::from("/workspace/repo/nested/covgate.toml"),
                PathBuf::from("/workspace/repo/covgate.toml"),
                PathBuf::from("/workspace/covgate.toml"),
                PathBuf::from("/covgate.toml"),
            ]
        );
    }

    #[test]
    fn parses_file_config_from_toml_text() {
        let config = parse_file_config(
            "base = \"main\"\nmarkdown-output = \"summary.md\"\n[[gates]]\nfail-under-regions = 80\n",
        )
        .expect("config should parse");

        assert_eq!(config.base.as_deref(), Some("main"));
        assert_eq!(config.markdown_output, Some(PathBuf::from("summary.md")));
        assert_eq!(config.gates.len(), 1);
        assert_eq!(config.gates[0].rules.fail_under_regions, Some(80.0));
    }

    #[test]
    fn parse_file_config_reports_invalid_toml() {
        let error = parse_file_config("not = [valid toml").expect_err("config should fail");
        assert!(
            error
                .to_string()
                .contains("failed to parse covgate config text")
        );
    }

    #[test]
    fn file_config_defaults_empty_gates() {
        let config = parse_file_config("").expect("empty config should parse");
        assert!(config.base.is_none());
        assert!(resolve_gates(Some(&config), std::path::Path::new(".")).is_err());
    }

    #[test]
    fn parses_path_scoped_gates_from_toml_text() {
        let config = parse_file_config(
            "base = \"main\"\n[[gates]]\nname = \"js-logic\"\ninclude = [\"**/*.ts\"]\nfail-under-lines = 95\n\n[[gates]]\nfail-under-lines = 90\n",
        )
        .expect("config should parse");

        assert_eq!(config.base.as_deref(), Some("main"));
    }

    #[test]
    fn parse_file_config_rejects_duplicate_gate_names() {
        let error = parse_file_config(
            "[[gates]]\nname = \"dup\"\ninclude = [\"**/*.ts\"]\nfail-under-lines = 95\n\n[[gates]]\nname = \"dup\"\ninclude = [\"**/*.tsx\"]\nfail-under-lines = 80\n",
        )
        .expect_err("config should fail");
        let error_text = error.to_string();

        assert!(error_text.contains("duplicate"));
        assert!(error_text.contains("dup"));
    }

    #[test]
    fn parse_file_config_rejects_exclude_without_include() {
        let error =
            parse_file_config("[[gates]]\nexclude = [\"**/*.tsx\"]\nfail-under-lines = 80\n")
                .expect_err("config should fail");
        let error_text = error.to_string();

        assert!(error_text.contains("exclude"));
        assert!(error_text.contains("include"));
    }

    #[test]
    fn parses_single_string_include_and_exclude() {
        let config = parse_file_config(
            "[[gates]]\ninclude = \"**/*.ts\"\nexclude = \"**/*.test.ts\"\nfail-under-lines = 90\n",
        )
        .expect("config should parse with single string include/exclude");

        assert_eq!(config.gates[0].include, ["**/*.ts"]);
        assert_eq!(config.gates[0].exclude, ["**/*.test.ts"]);
    }

    #[test]
    fn parses_sequence_include_and_exclude() {
        let config = parse_file_config(
            "[[gates]]\ninclude = [\"**/*.ts\", \"**/*.js\"]\nexclude = [\"**/*.test.ts\", \"**/*.test.js\"]\nfail-under-lines = 90\n",
        )
        .expect("config should parse with sequence include/exclude");

        assert_eq!(config.gates[0].include, ["**/*.ts", "**/*.js"]);
        assert_eq!(config.gates[0].exclude, ["**/*.test.ts", "**/*.test.js"]);
    }

    #[test]
    fn parse_file_config_rejects_invalid_include_type() {
        let error = parse_file_config("[[gates]]\ninclude = 42\nfail-under-lines = 90\n")
            .expect_err("config should fail with invalid include type");
        assert!(format!("{error:#}").contains("include"));
    }

    #[test]
    fn parse_file_config_rejects_invalid_include_list_item_type() {
        let error =
            parse_file_config("[[gates]]\ninclude = [\"**/*.rs\", 42]\nfail-under-lines = 90\n")
                .expect_err("config should fail with invalid include item type");
        assert!(format!("{error:#}").contains("include"));
    }

    #[test]
    fn parse_file_config_rejects_multiple_fallback_gates() {
        let error = parse_file_config(
            "[[gates]]\nfail-under-lines = 90\n\n[[gates]]\nfail-under-branches = 80\n",
        )
        .expect_err("config should fail");
        let error_text = error.to_string();

        assert!(error_text.contains("fallback"));
        assert!(error_text.contains("one"));
    }

    #[test]
    fn parse_file_config_rejects_legacy_gate_table_with_actionable_error() {
        let error = parse_file_config("[gates]\nfail-under-lines = 90\n")
            .expect_err("legacy config should fail");
        let error_text = error.to_string();

        assert!(error_text.contains("[gates]"));
        assert!(error_text.contains("[[gates]]"));
    }
}
