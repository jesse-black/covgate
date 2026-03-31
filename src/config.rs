use std::{
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

use crate::{
    cli::Args,
    diff::DiffSource,
    git,
    model::{GateRule, MetricKind},
};

const CONFIG_FILE_NAME: &str = "covgate.toml";

#[derive(Debug, Clone)]
pub struct Config {
    pub coverage_report: PathBuf,
    pub diff_source: DiffSource,
    pub rules: Vec<GateRule>,
    pub markdown_output: Option<PathBuf>,
}

#[derive(Debug, Default, Deserialize)]
struct FileConfig {
    base: Option<String>,
    markdown_output: Option<PathBuf>,
    #[serde(default)]
    gates: GateConfig,
}

#[derive(Debug, Default, Deserialize)]
struct GateConfig {
    fail_under_regions: Option<f64>,
    fail_under_lines: Option<f64>,
    fail_under_branches: Option<f64>,
    fail_under_functions: Option<f64>,
    fail_uncovered_regions: Option<usize>,
    fail_uncovered_lines: Option<usize>,
    fail_uncovered_branches: Option<usize>,
    fail_uncovered_functions: Option<usize>,
}

impl TryFrom<Args> for Config {
    type Error = anyhow::Error;

    fn try_from(args: Args) -> Result<Self> {
        let file_config = load_file_config()?;
        let diff_source = resolve_diff_source(&args, file_config.as_ref())?;
        let rules = resolve_rules(&args, file_config.as_ref())?;
        let markdown_output = args.markdown_output.or_else(|| {
            file_config
                .as_ref()
                .and_then(|config| config.markdown_output.clone())
        });
        Ok(Self {
            coverage_report: args.coverage_report,
            diff_source,
            rules,
            markdown_output,
        })
    }
}

fn load_file_config() -> Result<Option<FileConfig>> {
    let dir = env::current_dir()
        .context("failed to determine current directory for covgate config discovery")?;
    let repo_root = git::resolve_repo_root().ok().flatten();
    load_file_config_from_with_repo_root(&dir, repo_root.as_deref())
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
    toml::from_str::<FileConfig>(text).context("failed to parse covgate config text")
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

fn resolve_rules(args: &Args, file_config: Option<&FileConfig>) -> Result<Vec<GateRule>> {
    let mut configured = Vec::new();

    push_percent_rule(
        &mut configured,
        MetricKind::Region,
        args.fail_under_regions,
        file_config.and_then(|c| c.gates.fail_under_regions),
    );
    push_percent_rule(
        &mut configured,
        MetricKind::Line,
        args.fail_under_lines,
        file_config.and_then(|c| c.gates.fail_under_lines),
    );
    push_percent_rule(
        &mut configured,
        MetricKind::Branch,
        args.fail_under_branches,
        file_config.and_then(|c| c.gates.fail_under_branches),
    );
    push_uncovered_rule(
        &mut configured,
        MetricKind::Region,
        args.fail_uncovered_regions,
        file_config.and_then(|c| c.gates.fail_uncovered_regions),
    );
    push_uncovered_rule(
        &mut configured,
        MetricKind::Line,
        args.fail_uncovered_lines,
        file_config.and_then(|c| c.gates.fail_uncovered_lines),
    );
    push_percent_rule(
        &mut configured,
        MetricKind::Function,
        args.fail_under_functions,
        file_config.and_then(|c| c.gates.fail_under_functions),
    );
    push_uncovered_rule(
        &mut configured,
        MetricKind::Branch,
        args.fail_uncovered_branches,
        file_config.and_then(|c| c.gates.fail_uncovered_branches),
    );
    push_uncovered_rule(
        &mut configured,
        MetricKind::Function,
        args.fail_uncovered_functions,
        file_config.and_then(|c| c.gates.fail_uncovered_functions),
    );

    if configured.is_empty() {
        bail!(
            "at least one rule (e.g., --fail-under-regions or --fail-uncovered-regions) is required unless {} defines a supported [gates] default",
            CONFIG_FILE_NAME
        )
    }

    Ok(configured)
}

fn push_percent_rule(
    configured: &mut Vec<GateRule>,
    metric: MetricKind,
    cli_value: Option<f64>,
    config_value: Option<f64>,
) {
    if let Some(minimum_percent) = cli_value.or(config_value) {
        configured.push(GateRule::Percent {
            metric,
            minimum_percent,
        });
    }
}

fn push_uncovered_rule(
    configured: &mut Vec<GateRule>,
    metric: MetricKind,
    cli_value: Option<usize>,
    config_value: Option<usize>,
) {
    if let Some(maximum_count) = cli_value.or(config_value) {
        configured.push(GateRule::UncoveredCount {
            metric,
            maximum_count,
        });
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{
        FileConfig, config_candidate_paths, parse_file_config, resolve_diff_source, resolve_rules,
    };
    use crate::{
        cli::Args,
        diff::DiffSource,
        model::{GateRule, MetricKind},
    };

    #[test]
    fn parses_region_cli_rules() {
        let rules = resolve_rules(
            &Args {
                coverage_report: "coverage.json".into(),
                base: None,
                diff_file: None,
                fail_under_regions: Some(90.0),
                fail_under_lines: None,
                fail_under_branches: None,
                fail_under_functions: None,
                fail_uncovered_regions: Some(1),
                fail_uncovered_lines: None,
                fail_uncovered_branches: None,
                fail_uncovered_functions: None,
                markdown_output: None,
            },
            None,
        )
        .expect("rules should parse");

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
        let file_config: FileConfig = toml::from_str(
            "base = \"main\"\n[gates]\nfail_under_regions = 40\nfail_uncovered_regions = 5\n",
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
            fail_uncovered_regions: None, // Will fallback to TOML
            fail_uncovered_lines: None,
            fail_uncovered_branches: None,
            fail_uncovered_functions: None,
            markdown_output: None,
        };

        let diff_source =
            resolve_diff_source(&args, Some(&file_config)).expect("diff source should resolve");
        let rules = resolve_rules(&args, Some(&file_config)).expect("rules should resolve");

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
        let file_config: FileConfig = toml::from_str(
            "base = \"main\"\n[gates]\nfail_under_regions = 75\nfail_uncovered_lines = 2\n",
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
            fail_uncovered_regions: None,
            fail_uncovered_lines: None,
            fail_uncovered_branches: None,
            fail_uncovered_functions: None,
            markdown_output: None,
        };

        let diff_source =
            resolve_diff_source(&args, Some(&file_config)).expect("diff source should resolve");
        let rules = resolve_rules(&args, Some(&file_config)).expect("rules should resolve");

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
        let file_config: FileConfig = toml::from_str(
            "base = \"main\"\n[gates]\nfail_under_functions = 100\nfail_uncovered_functions = 0\n",
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
            fail_uncovered_regions: None,
            fail_uncovered_lines: None,
            fail_uncovered_branches: None,
            fail_uncovered_functions: None,
            markdown_output: None,
        };

        let rules = resolve_rules(&args, Some(&file_config)).expect("rules should resolve");

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
        let file_config: FileConfig = toml::from_str(
            "base = \"main\"\n[gates]\nfail_under_functions = 100\nfail_uncovered_functions = 0\n",
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
            fail_uncovered_regions: None,
            fail_uncovered_lines: None,
            fail_uncovered_branches: None,
            fail_uncovered_functions: Some(2),
            markdown_output: None,
        };

        let rules = resolve_rules(&args, Some(&file_config)).expect("rules should resolve");

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
            "base = \"main\"\nmarkdown_output = \"summary.md\"\n[gates]\nfail_under_regions = 80\n",
        )
        .expect("config should parse");

        assert_eq!(config.base.as_deref(), Some("main"));
        assert_eq!(config.markdown_output, Some(PathBuf::from("summary.md")));
        assert_eq!(config.gates.fail_under_regions, Some(80.0));
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
        let config: FileConfig = toml::from_str("").expect("empty config should parse");
        assert!(config.base.is_none());
        assert!(
            resolve_rules(
                &Args {
                    coverage_report: "coverage.json".into(),
                    base: None,
                    diff_file: None,
                    fail_under_regions: None,
                    fail_under_lines: None,
                    fail_under_branches: None,
                    fail_under_functions: None,
                    fail_uncovered_regions: None,
                    fail_uncovered_lines: None,
                    fail_uncovered_branches: None,
                    fail_uncovered_functions: None,
                    markdown_output: None,
                },
                Some(&config)
            )
            .is_err()
        );
    }
}
