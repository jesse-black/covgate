use std::{
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

use crate::{
    cli::Args,
    diff::DiffSource,
    model::{MetricKind, Threshold},
};

const CONFIG_FILE_NAME: &str = "covgate.toml";

#[derive(Debug, Clone)]
pub struct Config {
    pub coverage_json: PathBuf,
    pub diff_source: DiffSource,
    pub threshold: Threshold,
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
    combined: Option<f64>,
}

impl TryFrom<Args> for Config {
    type Error = anyhow::Error;

    fn try_from(args: Args) -> Result<Self> {
        let file_config = load_file_config()?;
        let diff_source = resolve_diff_source(&args, file_config.as_ref())?;
        let threshold = resolve_threshold(&args, file_config.as_ref())?;
        let markdown_output = args
            .markdown_output
            .or_else(|| file_config.and_then(|config| config.markdown_output));

        Ok(Self {
            coverage_json: args.coverage_json,
            diff_source,
            threshold,
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

fn resolve_threshold(args: &Args, file_config: Option<&FileConfig>) -> Result<Threshold> {
    if let Some(threshold) = cli_threshold(args)? {
        return Ok(threshold);
    }

    if let Some(config) = file_config
        && let Some(threshold) = config.gates.to_threshold()?
    {
        return Ok(threshold);
    }

    bail!(
        "one of --fail-under-regions, --fail-under-lines, or --fail-under-branches is required unless {} defines a supported [gates] default",
        CONFIG_FILE_NAME
    )
}

fn cli_threshold(args: &Args) -> Result<Option<Threshold>> {
    let mut configured = Vec::new();
    if let Some(minimum_percent) = args.fail_under_regions {
        configured.push(Threshold {
            metric: MetricKind::Region,
            minimum_percent,
        });
    }
    if let Some(minimum_percent) = args.fail_under_lines {
        configured.push(Threshold {
            metric: MetricKind::Line,
            minimum_percent,
        });
    }
    if let Some(minimum_percent) = args.fail_under_branches {
        configured.push(Threshold {
            metric: MetricKind::Branch,
            minimum_percent,
        });
    }
    exactly_one_threshold(configured, "CLI flags")
}

impl GateConfig {
    fn to_threshold(&self) -> Result<Option<Threshold>> {
        let mut configured = Vec::new();
        if let Some(percent) = self.fail_under_regions {
            configured.push(Threshold {
                metric: MetricKind::Region,
                minimum_percent: percent,
            });
        }
        if let Some(percent) = self.fail_under_lines {
            configured.push(Threshold {
                metric: MetricKind::Line,
                minimum_percent: percent,
            });
        }
        if let Some(percent) = self.fail_under_branches {
            configured.push(Threshold {
                metric: MetricKind::Branch,
                minimum_percent: percent,
            });
        }
        if let Some(percent) = self.combined {
            configured.push(Threshold {
                metric: MetricKind::Combined,
                minimum_percent: percent,
            });
        }

        exactly_one_threshold(configured, &format!("{CONFIG_FILE_NAME} [gates]"))
    }
}

fn exactly_one_threshold(configured: Vec<Threshold>, source: &str) -> Result<Option<Threshold>> {
    match configured.len() {
        0 => Ok(None),
        1 => Ok(configured.into_iter().next()),
        _ => bail!("{source} may set exactly one threshold in v1"),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::{
        CONFIG_FILE_NAME, FileConfig, GateConfig, cli_threshold, load_file_config_from,
        resolve_diff_source, resolve_threshold,
    };
    use crate::{cli::Args, diff::DiffSource, model::MetricKind};

    #[test]
    fn parses_region_cli_threshold() {
        let threshold = cli_threshold(&Args {
            coverage_json: "coverage.json".into(),
            base: None,
            diff_file: None,
            fail_under_regions: Some(90.0),
            fail_under_lines: None,
            fail_under_branches: None,
            markdown_output: None,
        })
        .expect("threshold should parse")
        .expect("threshold should exist");
        assert_eq!(threshold.metric, MetricKind::Region);
        assert_eq!(threshold.minimum_percent, 90.0);
    }

    #[test]
    fn rejects_multiple_cli_thresholds() {
        let error = cli_threshold(&Args {
            coverage_json: "coverage.json".into(),
            base: None,
            diff_file: None,
            fail_under_regions: Some(90.0),
            fail_under_lines: Some(80.0),
            fail_under_branches: None,
            markdown_output: None,
        })
        .expect_err("multiple thresholds should fail");

        assert!(
            error
                .to_string()
                .contains("CLI flags may set exactly one threshold in v1")
        );
    }

    #[test]
    fn config_threshold_rejects_multiple_metrics() {
        let thresholds = GateConfig {
            fail_under_regions: Some(90.0),
            fail_under_lines: Some(80.0),
            fail_under_branches: None,
            combined: None,
        };

        let error = thresholds
            .to_threshold()
            .expect_err("multiple thresholds should fail");
        assert!(
            error
                .to_string()
                .contains("covgate.toml [gates] may set exactly one threshold in v1")
        );
    }

    #[test]
    fn prefers_cli_over_config_defaults() {
        let file_config: FileConfig =
            toml::from_str("base = \"main\"\n[gates]\nfail_under_regions = 40\n")
                .expect("config should parse");

        let args = Args {
            coverage_json: "coverage.json".into(),
            base: Some("release".to_string()),
            diff_file: None,
            fail_under_regions: Some(90.0),
            fail_under_lines: None,
            fail_under_branches: None,
            markdown_output: None,
        };

        let diff_source =
            resolve_diff_source(&args, Some(&file_config)).expect("diff source should resolve");
        let threshold =
            resolve_threshold(&args, Some(&file_config)).expect("threshold should resolve");

        match diff_source {
            DiffSource::GitBase(base) => assert_eq!(base, "release"),
            DiffSource::DiffFile(_) => panic!("expected git base"),
        }
        assert_eq!(threshold.metric, MetricKind::Region);
        assert_eq!(threshold.minimum_percent, 90.0);
    }

    #[test]
    fn loads_defaults_from_repo_config() {
        let file_config: FileConfig =
            toml::from_str("base = \"main\"\n[gates]\nfail_under_regions = 75\n")
                .expect("config should parse");

        let args = Args {
            coverage_json: "coverage.json".into(),
            base: None,
            diff_file: None,
            fail_under_regions: None,
            fail_under_lines: None,
            fail_under_branches: None,
            markdown_output: None,
        };

        let diff_source =
            resolve_diff_source(&args, Some(&file_config)).expect("diff source should resolve");
        let threshold =
            resolve_threshold(&args, Some(&file_config)).expect("threshold should resolve");

        match diff_source {
            DiffSource::GitBase(base) => assert_eq!(base, "main"),
            DiffSource::DiffFile(_) => panic!("expected git base"),
        }
        assert_eq!(threshold.metric, MetricKind::Region);
        assert_eq!(threshold.minimum_percent, 75.0);
    }

    #[test]
    fn loads_repo_config_file_when_present() {
        let temp = tempdir().expect("tempdir");
        fs::write(
            temp.path().join(CONFIG_FILE_NAME),
            "base = \"main\"\nmarkdown_output = \"summary.md\"\n[gates]\nfail_under_regions = 80\n",
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
    }

    #[test]
    fn file_config_defaults_empty_gates() {
        let config: FileConfig = toml::from_str("").expect("empty config should parse");
        assert!(config.base.is_none());
        assert!(
            config
                .gates
                .to_threshold()
                .expect("threshold parse")
                .is_none()
        );
    }
}
