pub mod cli;
pub mod config;
pub mod coverage;
pub mod diff;
pub mod gate;
pub mod git;
pub mod metrics;
pub mod model;
pub mod render;

use anyhow::Result;

use crate::{config::Config, diff::DiffSource, model::ChangedFile};

pub fn run(config: Config) -> Result<i32> {
    let report = coverage::load_from_path(&config.coverage_report)?;
    let diff = load_changed_lines_with_warnings(&config.diff_source)?;

    let mut metrics = Vec::new();
    let mut requested_metrics = config.rules.iter().map(|r| r.metric()).collect::<Vec<_>>();
    requested_metrics.sort();
    requested_metrics.dedup();

    let available_metrics = report
        .totals_by_file
        .iter()
        .filter(|(_, totals)| totals.values().any(|file_totals| file_totals.total > 0))
        .map(|(metric, _)| *metric)
        .filter(|metric| !requested_metrics.contains(metric))
        .collect::<Vec<_>>();

    requested_metrics.extend(available_metrics);

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

fn load_changed_lines_with_warnings(source: &DiffSource) -> Result<Vec<ChangedFile>> {
    emit_untracked_files_warning(source)?;
    diff::load_changed_lines(source)
}

fn emit_untracked_files_warning(source: &DiffSource) -> Result<()> {
    if !matches!(source, DiffSource::GitBase(_)) {
        return Ok(());
    }

    let untracked_files = list_untracked_files()?;
    if untracked_files.is_empty() {
        return Ok(());
    }

    let add_command = format_git_add_command(&untracked_files);
    eprintln!(
        "⚠️ Untracked-files warning: untracked files are not included in diff gating and can produce a false pass. Add them with: `{add_command}`."
    );
    Ok(())
}

fn list_untracked_files() -> Result<Vec<String>> {
    crate::git::list_untracked_files()
}

fn format_git_add_command(paths: &[String]) -> String {
    let mut command = String::from("git add -N");
    for path in paths {
        command.push(' ');
        command.push_str(&shell_escape_path(path));
    }
    command
}

fn shell_escape_path(path: &str) -> String {
    if path
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '/' | '.' | '_' | '-'))
    {
        return path.to_string();
    }

    format!("'{}'", path.replace('\'', "'\''"))
}
