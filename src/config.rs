use std::{
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

use crate::{
    cli::Args,
    diff::DiffSource,
    model::{GateRule, MetricKind},
};

const CONFIG_FILE_NAME: &str = "covgate.toml";

#[derive(Debug, Clone)]
pub struct Config {
    pub coverage_json: PathBuf,
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
        let markdown_output = args
            .markdown_output
            .or_else(|| file_config.and_then(|config| config.markdown_output));

        Ok(Self {
            coverage_json: args.coverage_json,
            diff_source,
            rules,
            markdown_output,
        })
    }
}

fn load_file_config() -> Result<Option<FileConfig>> {
    let dir = env::current_dir()
        .context("failed to determine current directory for covgate config discovery")?;
    load_file_config_from(&dir)
}

fn load_file_config_from(dir: &Path) -> Result<Option<FileConfig>> {
    let path = dir.join(CONFIG_FILE_NAME);
    if !path.exists() {
        return Ok(None);
    }

    let text = fs::read_to_string(&path)
        .with_context(|| format!("failed to read config file: {}", path.display()))?;
    let config = toml::from_str::<FileConfig>(&text)
        .with_context(|| format!("failed to parse config file: {}", path.display()))?;
    Ok(Some(config))
}

fn resolve_diff_source(args: &Args, file_config: Option<&FileConfig>) -> Result<DiffSource> {
    match (args.base.clone(), args.diff_file.clone()) {
        (Some(base), None) => Ok(DiffSource::GitBase(base)),
        (None, Some(path)) => Ok(DiffSource::DiffFile(path)),
        (Some(_), Some(_)) => bail!("--base and --diff-file are mutually exclusive"),
        (None, None) => {
            if let Some(base) = file_config.and_then(|config| config.base.clone()) {
                Ok(DiffSource::GitBase(base))
            } else {
                bail!(
                    "either --base, --diff-file, or {} with a base value is required",
                    CONFIG_FILE_NAME
                )
            }
        }
    }
}

fn resolve_rules(args: &Args, file_config: Option<&FileConfig>) -> Result<Vec<GateRule>> {
    let mut configured = Vec::new();

    // fail_under_regions
    if let Some(minimum_percent) = args.fail_under_regions {
        configured.push(GateRule::Percent {
            metric: MetricKind::Region,
            minimum_percent,
        });
    } else if let Some(minimum_percent) = file_config.and_then(|c| c.gates.fail_under_regions) {
        configured.push(GateRule::Percent {
            metric: MetricKind::Region,
            minimum_percent,
        });
    }

    // fail_under_lines
    if let Some(minimum_percent) = args.fail_under_lines {
        configured.push(GateRule::Percent {
            metric: MetricKind::Line,
            minimum_percent,
        });
    } else if let Some(minimum_percent) = file_config.and_then(|c| c.gates.fail_under_lines) {
        configured.push(GateRule::Percent {
            metric: MetricKind::Line,
            minimum_percent,
        });
    }

    // fail_under_branches
    if let Some(minimum_percent) = args.fail_under_branches {
        configured.push(GateRule::Percent {
            metric: MetricKind::Branch,
            minimum_percent,
        });
    } else if let Some(minimum_percent) = file_config.and_then(|c| c.gates.fail_under_branches) {
        configured.push(GateRule::Percent {
            metric: MetricKind::Branch,
            minimum_percent,
        });
    }

    // fail_uncovered_regions
    if let Some(maximum_count) = args.fail_uncovered_regions {
        configured.push(GateRule::UncoveredCount {
            metric: MetricKind::Region,
            maximum_count,
        });
    } else if let Some(maximum_count) = file_config.and_then(|c| c.gates.fail_uncovered_regions) {
        configured.push(GateRule::UncoveredCount {
            metric: MetricKind::Region,
            maximum_count,
        });
    }

    // fail_uncovered_lines
    if let Some(maximum_count) = args.fail_uncovered_lines {
        configured.push(GateRule::UncoveredCount {
            metric: MetricKind::Line,
            maximum_count,
        });
    } else if let Some(maximum_count) = file_config.and_then(|c| c.gates.fail_uncovered_lines) {
        configured.push(GateRule::UncoveredCount {
            metric: MetricKind::Line,
            maximum_count,
        });
    }

    // fail_under_functions
    if let Some(minimum_percent) = args.fail_under_functions {
        configured.push(GateRule::Percent {
            metric: MetricKind::Function,
            minimum_percent,
        });
    } else if let Some(minimum_percent) = file_config.and_then(|c| c.gates.fail_under_functions) {
        configured.push(GateRule::Percent {
            metric: MetricKind::Function,
            minimum_percent,
        });
    }

    // fail_uncovered_branches
    if let Some(maximum_count) = args.fail_uncovered_branches {
        configured.push(GateRule::UncoveredCount {
            metric: MetricKind::Branch,
            maximum_count,
        });
    } else if let Some(maximum_count) = file_config.and_then(|c| c.gates.fail_uncovered_branches) {
        configured.push(GateRule::UncoveredCount {
            metric: MetricKind::Branch,
            maximum_count,
        });
    }

    // fail_uncovered_functions
    if let Some(maximum_count) = args.fail_uncovered_functions {
        configured.push(GateRule::UncoveredCount {
            metric: MetricKind::Function,
            maximum_count,
        });
    } else if let Some(maximum_count) = file_config.and_then(|c| c.gates.fail_uncovered_functions) {
        configured.push(GateRule::UncoveredCount {
            metric: MetricKind::Function,
            maximum_count,
        });
    }

    if configured.is_empty() {
        bail!(
            "at least one rule (e.g., --fail-under-regions or --fail-uncovered-regions) is required unless {} defines a supported [gates] default",
            CONFIG_FILE_NAME
        )
    }

    Ok(configured)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::{
        CONFIG_FILE_NAME, FileConfig, load_file_config_from, resolve_diff_source, resolve_rules,
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
                coverage_json: "coverage.json".into(),
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
            coverage_json: "coverage.json".into(),
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
            coverage_json: "coverage.json".into(),
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
            coverage_json: "coverage.json".into(),
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
            coverage_json: "coverage.json".into(),
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
    fn loads_repo_config_file_when_present() {
        let temp = tempdir().expect("tempdir");
        fs::write(
            temp.path().join(CONFIG_FILE_NAME),
            "base = \"main\"\nmarkdown_output = \"summary.md\"\n[gates]\nfail_under_regions = 80\nfail_uncovered_regions = 1\n",
        )
        .expect("write config");

        let config = load_file_config_from(temp.path())
            .expect("config should load")
            .expect("config file");

        assert_eq!(config.base.as_deref(), Some("main"));
        assert_eq!(
            config.markdown_output.as_deref(),
            Some(std::path::Path::new("summary.md"))
        );
        assert_eq!(config.gates.fail_under_regions, Some(80.0));
        assert_eq!(config.gates.fail_uncovered_regions, Some(1));
    }

    #[test]
    fn file_config_defaults_empty_gates() {
        let config: FileConfig = toml::from_str("").expect("empty config should parse");
        assert!(config.base.is_none());
        assert!(
            resolve_rules(
                &Args {
                    coverage_json: "coverage.json".into(),
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
