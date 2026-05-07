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
use std::collections::BTreeSet;

use crate::{
    config::{Config, ConfiguredGate},
    diff::DiffSource,
    model::{ChangedFile, GateResult, MetricKind},
};

pub fn run(config: Config) -> Result<i32> {
    let Config {
        coverage_report,
        diff_source,
        gates,
        markdown_output,
        verbose,
    } = config;
    let coverage_report = &coverage_report;
    let diff_source = &diff_source;
    let gates = &gates;
    let markdown_output = &markdown_output;

    let report = coverage::load_from_path(coverage_report)?;
    let diff = load_changed_lines_with_warnings(diff_source)?;

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
    let mut scopes = Vec::new();

    for gate_input in gate_inputs {
        let mut metrics = Vec::new();
        let mut requested_metrics = gate_input
            .gate
            .rules
            .iter()
            .map(|rule| rule.metric())
            .collect::<Vec<_>>();
        requested_metrics.sort();
        requested_metrics.dedup();

        let available_metrics = report
            .totals_by_file
            .iter()
            .filter(|(metric, totals)| {
                !requested_metrics.contains(metric)
                    && totals.iter().any(|(path, file_totals)| {
                        gate_input.paths.contains(path) && file_totals.total > 0
                    })
            })
            .map(|(metric, _)| *metric)
            .collect::<Vec<_>>();
        requested_metrics.extend(available_metrics);

        for metric_kind in requested_metrics {
            let metric = metrics::compute_changed_metric(
                &report,
                &gate_input.changed_files,
                &gate_input.paths,
                metric_kind,
            )?;
            metrics.push(metric);
        }

        let scope = gate::evaluate(
            gate_input.gate.label.clone(),
            metrics,
            &gate_input.gate.rules,
        )?;
        scopes.push(scope);
    }
    let scopes = scopes;

    let gate_result = GateResult {
        passed: scopes.iter().all(|scope| scope.passed),
        scopes,
        overall_metrics,
    };

    let verbosity = if verbose {
        crate::model::Verbosity::Verbose
    } else {
        crate::model::Verbosity::Normal
    };

    let console = render::console::render(&gate_result, &diff_source.describe(), verbosity);
    println!("{console}");

    if let Some(path) = markdown_output {
        let markdown = render::markdown::render(&gate_result, &diff_source.describe());
        std::fs::write(path, markdown)?;
    }

    Ok(if gate_result.passed { 0 } else { 1 })
}

struct GateRunInput<'a> {
    gate: &'a ConfiguredGate,
    changed_files: Vec<ChangedFile>,
    paths: BTreeSet<std::path::PathBuf>,
}

fn assign_changed_files<'a>(
    report: &crate::model::CoverageReport,
    diff: &[ChangedFile],
    gates: &'a [ConfiguredGate],
) -> Result<Vec<GateRunInput<'a>>> {
    let supported_files = report
        .totals_by_file
        .values()
        .flat_map(|totals| {
            totals
                .iter()
                .filter_map(|(path, totals)| (totals.total > 0).then_some(path.clone()))
        })
        .collect::<BTreeSet<_>>();
    let mut scoped_inputs = gates
        .iter()
        .filter(|gate| !gate.is_fallback())
        .map(|gate| GateRunInput {
            gate,
            changed_files: Vec::new(),
            paths: BTreeSet::new(),
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

    let mut participating = scoped_inputs
        .into_iter()
        .filter(|input| !input.changed_files.is_empty())
        .collect::<Vec<_>>();

    if let Some(gate) = fallback_gate
        && (!fallback_changed_files.is_empty() || participating.is_empty())
    {
        participating.push(GateRunInput {
            gate,
            paths: if !has_scoped_gates
                || (fallback_changed_files.is_empty() && participating.is_empty())
            {
                supported_files
            } else {
                fallback_paths
            },
            changed_files: fallback_changed_files,
        });
    }

    Ok(participating)
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
