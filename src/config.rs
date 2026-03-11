use std::path::PathBuf;

use anyhow::{Context, Result, bail};

use crate::{
    cli::Args,
    diff::DiffSource,
    model::{MetricKind, Threshold},
};

#[derive(Debug, Clone)]
pub struct Config {
    pub coverage_json: PathBuf,
    pub diff_source: DiffSource,
    pub threshold: Threshold,
    pub markdown_output: Option<PathBuf>,
}

impl TryFrom<Args> for Config {
    type Error = anyhow::Error;

    fn try_from(args: Args) -> Result<Self> {
        let diff_source = match (args.base, args.diff_file) {
            (Some(base), None) => DiffSource::GitBase(base),
            (None, Some(path)) => DiffSource::DiffFile(path),
            (None, None) => bail!("either --base or --diff-file is required"),
            (Some(_), Some(_)) => bail!("--base and --diff-file are mutually exclusive"),
        };

        let threshold = parse_threshold(&args.fail_under)
            .with_context(|| format!("invalid --fail-under value: {}", args.fail_under))?;

        Ok(Self {
            coverage_json: args.coverage_json,
            diff_source,
            threshold,
            markdown_output: args.markdown_output,
        })
    }
}

fn parse_threshold(value: &str) -> Result<Threshold> {
    let (metric, percent) = value
        .split_once('=')
        .context("expected METRIC=PERCENT, e.g. region=90")?;
    let metric = MetricKind::parse(metric)?;
    let minimum_percent: f64 = percent
        .parse()
        .context("threshold percent must be a number")?;
    Ok(Threshold {
        metric,
        minimum_percent,
    })
}

#[cfg(test)]
mod tests {
    use super::parse_threshold;
    use crate::model::MetricKind;

    #[test]
    fn parses_region_threshold() {
        let threshold = parse_threshold("region=90").expect("threshold should parse");
        assert_eq!(threshold.metric, MetricKind::Region);
        assert_eq!(threshold.minimum_percent, 90.0);
    }

    #[test]
    fn rejects_invalid_threshold() {
        assert!(parse_threshold("nope").is_err());
    }
}
