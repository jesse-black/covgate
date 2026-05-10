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
    pub verbose: bool,
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
    verbose: Option<bool>,
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
            verbose,
            base: _,
            diff_file: _,
            fail_under_regions: _,
            fail_under_lines: _,
            fail_under_branches: _,
            fail_under_functions: _,
            fail_under_named_functions: _,
            fail_uncovered_regions: _,
            fail_uncovered_lines: _,
            fail_uncovered_branches: _,
            fail_uncovered_functions: _,
            fail_uncovered_named_functions: _,
        } = &args;

        let dir = env::current_dir()
            .context("failed to determine current directory for covgate config discovery")?;
        let repo_root = git::resolve_repo_root().ok().flatten();
        let file_config = load_file_config_from_with_repo_root(&dir, repo_root.as_deref())?;
        let match_root = repo_root.unwrap_or(dir);
        let diff_source = resolve_diff_source(&args, file_config.as_ref())?;
        let gates = resolve_gates(&args, file_config.as_ref(), &match_root)?;
        let markdown_output = markdown_output.clone().or_else(|| {
            file_config
                .as_ref()
                .and_then(|config| config.markdown_output.clone())
        });
        let verbose = *verbose
            || file_config
                .as_ref()
                .and_then(|config| config.verbose)
                .unwrap_or(false);

        Ok(Self {
            coverage_report: coverage_report.clone(),
            diff_source,
            gates,
            markdown_output,
            verbose,
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
    args: &Args,
    file_config: Option<&FileConfig>,
    match_root: &Path,
) -> Result<Vec<ConfiguredGate>> {
    let mut configured = Vec::new();
    let mut has_fallback = false;
    let repo_ignores = Arc::new(build_repo_ignores(match_root)?);

    if let Some(config) = file_config {
        for (index, gate) in config.gates.iter().enumerate() {
            let is_fallback = gate.include.is_empty();
            if is_fallback {
                has_fallback = true;
            }

            let rules = resolve_gate_rules(
                args,
                if is_fallback { Some(&gate.rules) } else { None },
                Some(&gate.rules),
                is_fallback,
            );

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

    if !has_fallback && has_cli_rules(args) {
        configured.push(ConfiguredGate {
            label: None,
            rules: resolve_gate_rules(args, None, None, true),
            matcher: None,
        });
    }

    if configured.is_empty() {
        bail!(
            "at least one rule (e.g., --fail-under-regions or --fail-uncovered-regions) is required unless {} defines a supported [[gates]] entry",
            CONFIG_FILE_NAME
        )
    }

    let configured = configured;
    Ok(configured)
}

fn resolve_gate_rules(
    args: &Args,
    fallback_config: Option<&GateRuleConfig>,
    gate_config: Option<&GateRuleConfig>,
    allow_cli_overrides: bool,
) -> Vec<GateRule> {
    let Args {
        fail_under_regions,
        fail_under_lines,
        fail_under_branches,
        fail_under_functions,
        fail_under_named_functions,
        fail_uncovered_regions,
        fail_uncovered_lines,
        fail_uncovered_branches,
        fail_uncovered_functions,
        fail_uncovered_named_functions,
        coverage_report: _,
        base: _,
        diff_file: _,
        markdown_output: _,
        verbose: _,
    } = args;

    let mut configured = Vec::new();
    let cli_fail_under_regions = allow_cli_overrides.then_some(*fail_under_regions).flatten();
    let cli_fail_under_lines = allow_cli_overrides.then_some(*fail_under_lines).flatten();
    let cli_fail_under_branches = allow_cli_overrides
        .then_some(*fail_under_branches)
        .flatten();
    let cli_fail_under_functions = allow_cli_overrides
        .then_some(*fail_under_functions)
        .flatten();
    let cli_fail_under_named_functions = allow_cli_overrides
        .then_some(*fail_under_named_functions)
        .flatten();
    let cli_fail_uncovered_regions = allow_cli_overrides
        .then_some(*fail_uncovered_regions)
        .flatten();
    let cli_fail_uncovered_lines = allow_cli_overrides
        .then_some(*fail_uncovered_lines)
        .flatten();
    let cli_fail_uncovered_branches = allow_cli_overrides
        .then_some(*fail_uncovered_branches)
        .flatten();
    let cli_fail_uncovered_functions = allow_cli_overrides
        .then_some(*fail_uncovered_functions)
        .flatten();
    let cli_fail_uncovered_named_functions = allow_cli_overrides
        .then_some(*fail_uncovered_named_functions)
        .flatten();

    let (
        f_under_regions,
        f_under_lines,
        f_under_branches,
        f_under_functions,
        f_under_named_functions,
        f_uncovered_regions,
        f_uncovered_lines,
        f_uncovered_branches,
        f_uncovered_functions,
        f_uncovered_named_functions,
    ) = match fallback_config {
        Some(c) => {
            let GateRuleConfig {
                fail_under_regions,
                fail_under_lines,
                fail_under_branches,
                fail_under_functions,
                fail_under_named_functions,
                fail_uncovered_regions,
                fail_uncovered_lines,
                fail_uncovered_branches,
                fail_uncovered_functions,
                fail_uncovered_named_functions,
            } = c;
            (
                *fail_under_regions,
                *fail_under_lines,
                *fail_under_branches,
                *fail_under_functions,
                *fail_under_named_functions,
                *fail_uncovered_regions,
                *fail_uncovered_lines,
                *fail_uncovered_branches,
                *fail_uncovered_functions,
                *fail_uncovered_named_functions,
            )
        }
        None => (None, None, None, None, None, None, None, None, None, None),
    };

    let (
        g_under_regions,
        g_under_lines,
        g_under_branches,
        g_under_functions,
        g_under_named_functions,
        g_uncovered_regions,
        g_uncovered_lines,
        g_uncovered_branches,
        g_uncovered_functions,
        g_uncovered_named_functions,
    ) = match gate_config {
        Some(c) => {
            let GateRuleConfig {
                fail_under_regions,
                fail_under_lines,
                fail_under_branches,
                fail_under_functions,
                fail_under_named_functions,
                fail_uncovered_regions,
                fail_uncovered_lines,
                fail_uncovered_branches,
                fail_uncovered_functions,
                fail_uncovered_named_functions,
            } = c;
            (
                *fail_under_regions,
                *fail_under_lines,
                *fail_under_branches,
                *fail_under_functions,
                *fail_under_named_functions,
                *fail_uncovered_regions,
                *fail_uncovered_lines,
                *fail_uncovered_branches,
                *fail_uncovered_functions,
                *fail_uncovered_named_functions,
            )
        }
        None => (None, None, None, None, None, None, None, None, None, None),
    };

    push_percent_rule(
        &mut configured,
        MetricKind::Region,
        f_under_regions,
        g_under_regions,
        cli_fail_under_regions,
    );
    push_percent_rule(
        &mut configured,
        MetricKind::Line,
        f_under_lines,
        g_under_lines,
        cli_fail_under_lines,
    );
    push_percent_rule(
        &mut configured,
        MetricKind::Branch,
        f_under_branches,
        g_under_branches,
        cli_fail_under_branches,
    );
    push_percent_rule(
        &mut configured,
        MetricKind::Function,
        f_under_functions,
        g_under_functions,
        cli_fail_under_functions,
    );
    push_percent_rule(
        &mut configured,
        MetricKind::NamedFunction,
        f_under_named_functions,
        g_under_named_functions,
        cli_fail_under_named_functions,
    );
    push_uncovered_rule(
        &mut configured,
        MetricKind::Region,
        f_uncovered_regions,
        g_uncovered_regions,
        cli_fail_uncovered_regions,
    );
    push_uncovered_rule(
        &mut configured,
        MetricKind::Line,
        f_uncovered_lines,
        g_uncovered_lines,
        cli_fail_uncovered_lines,
    );
    push_uncovered_rule(
        &mut configured,
        MetricKind::Branch,
        f_uncovered_branches,
        g_uncovered_branches,
        cli_fail_uncovered_branches,
    );
    push_uncovered_rule(
        &mut configured,
        MetricKind::Function,
        f_uncovered_functions,
        g_uncovered_functions,
        cli_fail_uncovered_functions,
    );
    push_uncovered_rule(
        &mut configured,
        MetricKind::NamedFunction,
        f_uncovered_named_functions,
        g_uncovered_named_functions,
        cli_fail_uncovered_named_functions,
    );

    configured
}

fn push_percent_rule(
    configured: &mut Vec<GateRule>,
    metric: MetricKind,
    fallback_value: Option<f64>,
    config_value: Option<f64>,
    cli_value: Option<f64>,
) {
    if let Some(minimum_percent) = cli_value.or(config_value).or(fallback_value) {
        configured.push(GateRule::Percent {
            metric,
            minimum_percent,
        });
    }
}

fn push_uncovered_rule(
    configured: &mut Vec<GateRule>,
    metric: MetricKind,
    fallback_value: Option<usize>,
    config_value: Option<usize>,
    cli_value: Option<usize>,
) {
    if let Some(maximum_count) = cli_value.or(config_value).or(fallback_value) {
        configured.push(GateRule::UncoveredCount {
            metric,
            maximum_count,
        });
    }
}

fn has_cli_rules(args: &Args) -> bool {
    [
        args.fail_under_regions.is_some(),
        args.fail_under_lines.is_some(),
        args.fail_under_branches.is_some(),
        args.fail_under_functions.is_some(),
        args.fail_under_named_functions.is_some(),
        args.fail_uncovered_regions.is_some(),
        args.fail_uncovered_lines.is_some(),
        args.fail_uncovered_branches.is_some(),
        args.fail_uncovered_functions.is_some(),
        args.fail_uncovered_named_functions.is_some(),
    ]
    .into_iter()
    .any(std::convert::identity)
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
    fn parses_region_cli_rules() {
        let gates = resolve_gates(
            &Args {
                coverage_report: "coverage.json".into(),
                base: None,
                diff_file: None,
                fail_under_regions: Some(90.0),
                fail_under_lines: None,
                fail_under_branches: None,
                fail_under_functions: None,
                fail_under_named_functions: None,
                fail_uncovered_regions: Some(1),
                fail_uncovered_lines: None,
                fail_uncovered_branches: None,
                fail_uncovered_functions: None,
                fail_uncovered_named_functions: None,
                markdown_output: None,
                verbose: false,
            },
            None,
            std::path::Path::new("."),
        )
        .expect("gates should parse");
        let rules = gate_rules(&gates);

        assert_eq!(rules.len(), 2);
        assert!(rules.contains(&GateRule::Percent {
            metric: MetricKind::Region,
            minimum_percent: 90.0
        }));
        assert!(rules.contains(&GateRule::UncoveredCount {
            metric: MetricKind::Region,
            maximum_count: 1
        }));
    }

    #[test]
    fn prefers_cli_over_config_defaults() {
        let file_config = parse_file_config(
            "base = \"main\"\n[[gates]]\nfail-under-regions = 40\nfail-uncovered-regions = 5\n",
        )
        .expect("config should parse");

        let args = Args {
            coverage_report: "coverage.json".into(),
            base: Some("release".to_string()),
            diff_file: None,
            fail_under_regions: Some(90.0),
            fail_under_lines: None,
            fail_under_branches: None,
            fail_under_functions: None,
            fail_under_named_functions: None,
            fail_uncovered_regions: None, // Will fallback to TOML
            fail_uncovered_lines: None,
            fail_uncovered_branches: None,
            fail_uncovered_functions: None,
            fail_uncovered_named_functions: None,
            markdown_output: None,
            verbose: false,
        };

        let diff_source =
            resolve_diff_source(&args, Some(&file_config)).expect("diff source should resolve");
        let rules = gate_rules(
            &resolve_gates(&args, Some(&file_config), std::path::Path::new("."))
                .expect("gates should resolve"),
        );

        match diff_source {
            DiffSource::GitBase(base) => assert_eq!(base, "release"),
            DiffSource::DiffFile(_) => panic!("expected git base"),
        }
        assert_eq!(rules.len(), 2);
        assert!(rules.contains(&GateRule::Percent {
            metric: MetricKind::Region,
            minimum_percent: 90.0
        }));
        assert!(rules.contains(&GateRule::UncoveredCount {
            metric: MetricKind::Region,
            maximum_count: 5
        }));
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
            fail_under_regions: None,
            fail_under_lines: None,
            fail_under_branches: None,
            fail_under_functions: None,
            fail_under_named_functions: None,
            fail_uncovered_regions: None,
            fail_uncovered_lines: None,
            fail_uncovered_branches: None,
            fail_uncovered_functions: None,
            fail_uncovered_named_functions: None,
            markdown_output: None,
            verbose: false,
        };

        let diff_source =
            resolve_diff_source(&args, Some(&file_config)).expect("diff source should resolve");
        let rules = gate_rules(
            &resolve_gates(&args, Some(&file_config), std::path::Path::new("."))
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

        let args = Args {
            coverage_report: "coverage.json".into(),
            base: None,
            diff_file: None,
            fail_under_regions: None,
            fail_under_lines: None,
            fail_under_branches: None,
            fail_under_functions: None,
            fail_under_named_functions: None,
            fail_uncovered_regions: None,
            fail_uncovered_lines: None,
            fail_uncovered_branches: None,
            fail_uncovered_functions: None,
            fail_uncovered_named_functions: None,
            markdown_output: None,
            verbose: false,
        };

        let rules = gate_rules(
            &resolve_gates(&args, Some(&file_config), std::path::Path::new("."))
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
    fn cli_function_rules_override_repo_config_defaults() {
        let file_config = parse_file_config(
            "base = \"main\"\n[[gates]]\nfail-under-functions = 100\nfail-uncovered-functions = 0\n",
        )
        .expect("config should parse");

        let args = Args {
            coverage_report: "coverage.json".into(),
            base: None,
            diff_file: Some("scenario.diff".into()),
            fail_under_regions: None,
            fail_under_lines: None,
            fail_under_branches: None,
            fail_under_functions: Some(80.0),
            fail_under_named_functions: None,
            fail_uncovered_regions: None,
            fail_uncovered_lines: None,
            fail_uncovered_branches: None,
            fail_uncovered_functions: Some(2),
            fail_uncovered_named_functions: None,
            markdown_output: None,
            verbose: false,
        };

        let rules = gate_rules(
            &resolve_gates(&args, Some(&file_config), std::path::Path::new("."))
                .expect("gates should resolve"),
        );

        assert!(rules.contains(&GateRule::Percent {
            metric: MetricKind::Function,
            minimum_percent: 80.0
        }));
        assert!(rules.contains(&GateRule::UncoveredCount {
            metric: MetricKind::Function,
            maximum_count: 2
        }));
        assert!(!rules.contains(&GateRule::Percent {
            metric: MetricKind::Function,
            minimum_percent: 100.0
        }));
        assert!(!rules.contains(&GateRule::UncoveredCount {
            metric: MetricKind::Function,
            maximum_count: 0
        }));
    }

    #[test]
    fn cli_thresholds_override_only_the_fallback_gate() {
        let file_config = parse_file_config(
            "[[gates]]\nname = \"js-ui\"\ninclude = [\"**/*.tsx\"]\nfail-under-lines = 70\n\n[[gates]]\nfail-under-lines = 10\n",
        )
        .expect("config should parse");

        let args = Args {
            coverage_report: "coverage.json".into(),
            base: None,
            diff_file: Some("scenario.diff".into()),
            fail_under_regions: None,
            fail_under_lines: Some(40.0),
            fail_under_branches: None,
            fail_under_functions: None,
            fail_under_named_functions: None,
            fail_uncovered_regions: None,
            fail_uncovered_lines: None,
            fail_uncovered_branches: None,
            fail_uncovered_functions: None,
            fail_uncovered_named_functions: None,
            markdown_output: None,
            verbose: false,
        };

        let gates = resolve_gates(&args, Some(&file_config), std::path::Path::new("."))
            .expect("gates should resolve");
        let scoped_gate = gates
            .iter()
            .find(|gate| gate.label.as_deref() == Some("js-ui"))
            .expect("scoped gate should exist");
        let fallback_gate = gates
            .iter()
            .find(|gate| gate.is_fallback())
            .expect("fallback gate should exist");

        assert!(scoped_gate.rules.contains(&GateRule::Percent {
            metric: MetricKind::Line,
            minimum_percent: 70.0,
        }));
        assert!(!scoped_gate.rules.contains(&GateRule::Percent {
            metric: MetricKind::Line,
            minimum_percent: 40.0,
        }));
        assert!(fallback_gate.rules.contains(&GateRule::Percent {
            metric: MetricKind::Line,
            minimum_percent: 40.0,
        }));
    }

    #[test]
    fn cli_thresholds_synthesize_a_fallback_gate_when_config_is_scoped_only() {
        let file_config = parse_file_config(
            "[[gates]]\nname = \"js-ui\"\ninclude = [\"**/*.tsx\"]\nfail-under-lines = 70\n",
        )
        .expect("config should parse");

        let args = Args {
            coverage_report: "coverage.json".into(),
            base: None,
            diff_file: Some("scenario.diff".into()),
            fail_under_regions: None,
            fail_under_lines: Some(40.0),
            fail_under_branches: None,
            fail_under_functions: None,
            fail_under_named_functions: None,
            fail_uncovered_regions: None,
            fail_uncovered_lines: None,
            fail_uncovered_branches: None,
            fail_uncovered_functions: None,
            fail_uncovered_named_functions: None,
            markdown_output: None,
            verbose: false,
        };

        let gates = resolve_gates(&args, Some(&file_config), std::path::Path::new("."))
            .expect("gates should resolve");
        let scoped_gate = gates
            .iter()
            .find(|gate| gate.label.as_deref() == Some("js-ui"))
            .expect("scoped gate should exist");
        let fallback_gate = gates
            .iter()
            .find(|gate| gate.is_fallback())
            .expect("fallback gate should be synthesized");

        assert_eq!(gates.len(), 2);
        assert!(scoped_gate.rules.contains(&GateRule::Percent {
            metric: MetricKind::Line,
            minimum_percent: 70.0,
        }));
        assert!(!scoped_gate.rules.contains(&GateRule::Percent {
            metric: MetricKind::Line,
            minimum_percent: 40.0,
        }));
        assert_eq!(fallback_gate.label, None);
        assert!(fallback_gate.rules.contains(&GateRule::Percent {
            metric: MetricKind::Line,
            minimum_percent: 40.0,
        }));
    }

    #[test]
    fn resolve_gates_rejects_named_gate_without_rules() {
        let file_config =
            parse_file_config("[[gates]]\nname = \"js-ui\"\ninclude = [\"**/*.tsx\"]\n")
                .expect("config should parse");

        let error = resolve_gates(
            &Args {
                coverage_report: "coverage.json".into(),
                base: None,
                diff_file: Some("scenario.diff".into()),
                fail_under_regions: None,
                fail_under_lines: None,
                fail_under_branches: None,
                fail_under_functions: None,
                fail_under_named_functions: None,
                fail_uncovered_regions: None,
                fail_uncovered_lines: None,
                fail_uncovered_branches: None,
                fail_uncovered_functions: None,
                fail_uncovered_named_functions: None,
                markdown_output: None,
                verbose: false,
            },
            Some(&file_config),
            std::path::Path::new("."),
        )
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
            "base = \"main\"\nmarkdown-output = \"summary.md\"\nverbose = true\n[[gates]]\nfail-under-regions = 80\n",
        )
        .expect("config should parse");

        assert_eq!(config.base.as_deref(), Some("main"));
        assert_eq!(config.markdown_output, Some(PathBuf::from("summary.md")));
        assert_eq!(config.verbose, Some(true));
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
        assert!(
            resolve_gates(
                &Args {
                    coverage_report: "coverage.json".into(),
                    base: None,
                    diff_file: None,
                    fail_under_regions: None,
                    fail_under_lines: None,
                    fail_under_branches: None,
                    fail_under_functions: None,
                    fail_under_named_functions: None,
                    fail_uncovered_regions: None,
                    fail_uncovered_lines: None,
                    fail_uncovered_branches: None,
                    fail_uncovered_functions: None,
                    fail_uncovered_named_functions: None,
                    markdown_output: None,
                    verbose: false,
                },
                Some(&config),
                std::path::Path::new("."),
            )
            .is_err()
        );
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
