pub mod cli;
pub mod config;
pub mod coverage;
pub mod diff;
pub mod gate;
pub mod metrics;
pub mod model;
pub mod render;

use anyhow::Result;

use crate::config::Config;

pub fn run(config: Config) -> Result<i32> {
    let report = coverage::parse_path(&config.coverage_json)?;
    let diff = diff::load_changed_lines(&config.diff_source)?;

    let mut metrics = Vec::new();
    let mut requested_metrics = config.rules.iter().map(|r| r.metric()).collect::<Vec<_>>();
    requested_metrics.sort();
    requested_metrics.dedup();

    for metric_kind in requested_metrics {
        let metric = metrics::compute_changed_metric(&report, &diff, metric_kind)?;
        metrics.push(metric);
    }

    let gate_result = gate::evaluate(metrics, &config.rules)?;

    let console = render::console::render(&gate_result, &config.diff_source.describe());
    println!("{console}");

    if let Some(path) = &config.markdown_output {
        let markdown = render::markdown::render(&gate_result, &config.diff_source.describe());
        std::fs::write(path, markdown)?;
    }

    Ok(if gate_result.passed { 0 } else { 1 })
}
