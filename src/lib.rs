pub mod cli;
pub mod config;
pub mod coverage;
pub mod diff;
pub mod gate;
pub mod git;
pub mod metrics;
pub mod model;
pub mod render;

use anyhow::{Context, Result};
use std::{collections::BTreeSet, io::Write, path::PathBuf};

use crate::{
    config::{Config, ConfiguredGate, OutputSink},
    diff::DiffSource,
    model::{ChangedFile, CheckResult, ComputedMetric, MetricKind},
};

pub fn run(config: Config) -> Result<i32> {
    let Config {
        coverage_report,
        diff_source,
        gates,
        markdown_output,
        no_github_summary,
    } = config;
    let coverage_report = &coverage_report;
    let diff_source = &diff_source;
    let gates = &gates;
    let markdown_output = &markdown_output;

    let report = coverage::load_from_path(coverage_report)?;
    let coverage_files = supported_files(&report);
    let diff = load_changed_lines_with_warnings(diff_source, &coverage_files)?;

    let mut overall_metrics = Vec::new();
    for kind in [
        MetricKind::Region,
        MetricKind::Line,
        MetricKind::Branch,
        MetricKind::Function,
        MetricKind::NamedFunction,
    ] {
        if let Ok(metric) = metrics::compute_overall_metric(&report, kind) {
            overall_metrics.push(metric);
        }
    }
    let overall_metrics = overall_metrics;

    let gate_inputs = assign_changed_files(&report, &diff, gates)?;
    let mut gate_evaluations = Vec::new();

    for gate_input in &gate_inputs {
        let mut metrics = Vec::new();
        let mut requested_metrics = gate_input
            .gate
            .rules
            .iter()
            .map(|rule| rule.metric())
            .collect::<Vec<_>>();
        requested_metrics.sort();
        requested_metrics.dedup();

        for metric_kind in requested_metrics {
            let metric = match metrics::compute_changed_metric(
                &report,
                &gate_input.changed_files,
                &gate_input.paths,
                metric_kind,
            ) {
                Ok(metric) => metric,
                Err(_) if !gate_input.has_assigned_changes => empty_changed_metric(metric_kind),
                Err(error) => return Err(error),
            };
            metrics.push(metric);
        }

        let evaluation = gate::evaluate(
            gate_input.gate.label.clone(),
            metrics,
            &gate_input.gate.rules,
        )?;
        gate_evaluations.push(evaluation);
    }
    let gate_evaluations = gate_evaluations;
    let changed_metrics = compute_run_changed_metrics(&report, &diff)?;

    let check_result = CheckResult {
        passed: gate_evaluations.iter().all(|gate| gate.passed),
        gates: gate_evaluations,
        changed_metrics,
        overall_metrics,
    };

    let console = render::console::render(&check_result, &diff_source.describe());
    println!("{console}");

    let github_summary = (!no_github_summary)
        .then(|| std::env::var_os("GITHUB_STEP_SUMMARY"))
        .flatten();
    if markdown_output.is_some() || github_summary.is_some() {
        let markdown = render::markdown::render(&check_result, &diff_source.describe());
        match markdown_output {
            Some(OutputSink::File(path)) => std::fs::write(path, &markdown)
                .with_context(|| format!("failed to write markdown output: {}", path.display()))?,
            Some(OutputSink::Stdout) => print!("{markdown}"),
            None => {}
        }
        if let Some(path) = github_summary {
            let path = PathBuf::from(path);
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .with_context(|| {
                    format!("failed to open GitHub step summary: {}", path.display())
                })?;
            file.write_all(markdown.as_bytes()).with_context(|| {
                format!("failed to write GitHub step summary: {}", path.display())
            })?;
        }
    }

    Ok(if check_result.passed { 0 } else { 1 })
}

fn compute_run_changed_metrics(
    report: &crate::model::CoverageReport,
    diff: &[ChangedFile],
) -> Result<Vec<ComputedMetric>> {
    let included_paths = supported_files(report);
    let mut metrics = Vec::new();
    for metric_kind in report.totals_by_file.iter().filter_map(|(metric, totals)| {
        totals
            .values()
            .any(|file_totals| file_totals.total > 0)
            .then_some(*metric)
    }) {
        metrics.push(metrics::compute_changed_metric(
            report,
            diff,
            &included_paths,
            metric_kind,
        )?);
    }
    Ok(metrics)
}

fn empty_changed_metric(metric: MetricKind) -> ComputedMetric {
    ComputedMetric {
        metric,
        covered: 0,
        total: 0,
        percent: 100.0,
        uncovered_changed_opportunities: Vec::new(),
        changed_totals_by_file: std::collections::BTreeMap::new(),
        totals_by_file: std::collections::BTreeMap::new(),
    }
}

struct GateRunInput<'a> {
    gate: &'a ConfiguredGate,
    changed_files: Vec<ChangedFile>,
    paths: BTreeSet<std::path::PathBuf>,
    has_assigned_changes: bool,
}

fn assign_changed_files<'a>(
    report: &crate::model::CoverageReport,
    diff: &[ChangedFile],
    gates: &'a [ConfiguredGate],
) -> Result<Vec<GateRunInput<'a>>> {
    let supported_files = supported_files(report);
    let mut scoped_inputs = gates
        .iter()
        .filter(|gate| !gate.is_fallback())
        .map(|gate| GateRunInput {
            gate,
            changed_files: Vec::new(),
            paths: BTreeSet::new(),
            has_assigned_changes: false,
        })
        .collect::<Vec<_>>();
    let has_scoped_gates = !scoped_inputs.is_empty();
    let fallback_gate = gates.iter().find(|gate| gate.is_fallback());
    let mut fallback_changed_files = Vec::new();
    let mut fallback_paths = BTreeSet::new();

    for changed_file in diff {
        if !supported_files.contains(&changed_file.path) {
            continue;
        }

        let mut matching = Vec::new();
        for gate in &scoped_inputs {
            if !gate.gate.matches(&changed_file.path) {
                continue;
            }

            let label = match &gate.gate.label {
                Some(label) => label.clone(),
                None => "unnamed".to_string(),
            };
            matching.push(label);
        }

        if matching.len() > 1 {
            anyhow::bail!(
                "changed file `{}` matches multiple scoped gates: {}",
                changed_file.path.display(),
                matching.join(", ")
            );
        }

        if let Some(input) = scoped_inputs
            .iter_mut()
            .find(|gate| gate.gate.matches(&changed_file.path))
        {
            input.paths.insert(changed_file.path.clone());
            input.changed_files.push(changed_file.clone());
        } else if fallback_gate.is_some() {
            fallback_paths.insert(changed_file.path.clone());
            fallback_changed_files.push(changed_file.clone());
        } else {
            anyhow::bail!(
                "changed file `{}` has supported coverage opportunities but does not match any scoped gate and no fallback gate is configured",
                changed_file.path.display()
            );
        }
    }

    for input in &mut scoped_inputs {
        input.has_assigned_changes = !input.changed_files.is_empty();
    }

    let mut participating = scoped_inputs;
    let any_scoped_has_changes = participating.iter().any(|input| input.has_assigned_changes);

    if let Some(gate) = fallback_gate {
        let has_assigned_changes = !fallback_changed_files.is_empty();
        participating.push(GateRunInput {
            gate,
            paths: if !has_scoped_gates
                || (fallback_changed_files.is_empty() && !any_scoped_has_changes)
            {
                supported_files
            } else {
                fallback_paths
            },
            changed_files: fallback_changed_files,
            has_assigned_changes,
        });
    }

    Ok(participating)
}

fn supported_files(report: &crate::model::CoverageReport) -> BTreeSet<std::path::PathBuf> {
    report
        .totals_by_file
        .values()
        .flat_map(|totals| {
            totals
                .iter()
                .filter_map(|(path, totals)| (totals.total > 0).then_some(path.clone()))
        })
        .collect()
}

fn load_changed_lines_with_warnings(
    source: &DiffSource,
    coverage_files: &BTreeSet<std::path::PathBuf>,
) -> Result<Vec<ChangedFile>> {
    check_untracked_coverage_files(source, coverage_files)?;
    diff::load_changed_lines(source)
}

fn check_untracked_coverage_files(
    source: &DiffSource,
    coverage_files: &BTreeSet<std::path::PathBuf>,
) -> Result<()> {
    if !matches!(source, DiffSource::GitBase(_)) {
        return Ok(());
    }

    let untracked_files = crate::git::list_untracked_files()?;
    let relevant: Vec<String> = untracked_files
        .into_iter()
        .filter(|path| coverage_files.contains(std::path::Path::new(path)))
        .collect();

    if relevant.is_empty() {
        return Ok(());
    }

    let add_command = format_git_add_command(&relevant);
    Err(anyhow::anyhow!(
        "untracked files appear in the coverage report and are excluded from diff gating, which produces a false pass — add them with: `{add_command}`"
    ))
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
