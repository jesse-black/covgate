use std::collections::BTreeMap;

use crate::model::{
    CheckResult, ComputedMetric, GateEvaluation, GateRule, MetricKind, RuleOutcome, SourceSpan,
};
use crate::render::title_case;

#[must_use]
pub fn render(result: &CheckResult, diff_description: &str) -> String {
    let mut out = String::new();

    if !result.passed {
        out.push_str(&format!("Diff: {diff_description}\n\n"));
        out.push_str(&render_failures(result));
    }

    let multiple_gates = result.gates.len() > 1;
    for gate in &result.gates {
        let label = summary_label(gate, multiple_gates);
        for outcome in &gate.rules {
            out.push_str(&render_rule_summary(outcome, label.as_deref()));
            out.push('\n');
        }
    }

    out.trim_end().to_string()
}

fn render_failures(result: &CheckResult) -> String {
    let mut out = String::new();
    let failed_metrics = result
        .gates
        .iter()
        .flat_map(|gate| &gate.rules)
        .filter_map(|outcome| (!outcome.passed).then_some(outcome.rule.metric()))
        .collect::<Vec<_>>();
    render_failure_metrics(&mut out, &result.changed_metrics, &failed_metrics);
    out
}

fn render_failure_metrics(
    out: &mut String,
    metrics: &[ComputedMetric],
    failed_metrics: &[MetricKind],
) {
    let files_with_uncovered = group_uncovered_by_file(metrics, failed_metrics);
    for (path, file_metrics) in files_with_uncovered {
        out.push_str(&render_file_failure_header(&path, metrics, failed_metrics));
        out.push('\n');

        for (metric_kind, spans) in file_metrics {
            let grouped = group_file_spans(&spans);
            out.push_str(&format!(
                "  {}: {}\n",
                metric_kind.label(),
                grouped.join(", ")
            ));
        }
        out.push('\n');
    }
}

fn render_rule_summary(outcome: &RuleOutcome, scope_label: Option<&str>) -> String {
    let status = if outcome.passed { "PASS" } else { "FAIL" };
    let metric_label = title_case(outcome.rule.metric().label());

    let summary = match &outcome.rule {
        GateRule::Percent {
            metric: _,
            minimum_percent,
        } => {
            let comparator = if outcome.passed { "≥" } else { "≱" };
            let observed = format_percent(outcome.observed_percent, outcome.observed_total_count);
            let counts = format!(
                "({}/{})",
                outcome.observed_covered_count, outcome.observed_total_count
            );
            format!(
                "{status} {metric_label}: {observed} {counts} {comparator} {minimum_percent:.2}%"
            )
        }
        GateRule::UncoveredCount {
            metric: _,
            maximum_count,
        } => {
            let comparator = if outcome.passed { "≤" } else { "≰" };
            format!(
                "{status} {metric_label}: {} uncovered {comparator} {maximum_count}",
                outcome.observed_uncovered_count
            )
        }
    };

    if let Some(scope_label) = scope_label {
        format!("[{scope_label}] {summary}")
    } else {
        summary
    }
}

fn group_uncovered_by_file(
    metrics: &[ComputedMetric],
    failed_metrics: &[MetricKind],
) -> BTreeMap<std::path::PathBuf, BTreeMap<MetricKind, Vec<SourceSpan>>> {
    let mut files_with_uncovered: BTreeMap<
        std::path::PathBuf,
        BTreeMap<MetricKind, Vec<SourceSpan>>,
    > = BTreeMap::new();
    for metric in metrics {
        if !failed_metrics.contains(&metric.metric) {
            continue;
        }

        for opportunity in &metric.uncovered_changed_opportunities {
            files_with_uncovered
                .entry(opportunity.span.path.clone())
                .or_default()
                .entry(metric.metric)
                .or_default()
                .push(opportunity.span.clone());
        }
    }
    files_with_uncovered
}

fn render_file_failure_header(
    path: &std::path::Path,
    metrics: &[ComputedMetric],
    failed_metrics: &[MetricKind],
) -> String {
    let mut header = path.display().to_string();
    let mut stats = Vec::new();

    for metric in metrics {
        if !failed_metrics.contains(&metric.metric) {
            continue;
        }

        if let Some(file_totals) = metric.changed_totals_by_file.get(path) {
            let percent = if file_totals.total == 0 {
                100.0
            } else {
                (file_totals.covered as f64 / file_totals.total as f64) * 100.0
            };
            stats.push(format!("{percent:.2}% {}", metric.metric.as_str()));
        }
    }

    if !stats.is_empty() {
        header.push_str(&format!(" ({})", stats.join(", ")));
    }

    header
}

fn group_file_spans(spans: &[SourceSpan]) -> Vec<String> {
    SourceSpan::group_by_span(spans)
        .into_iter()
        .map(|(key, count)| {
            let label = key.format_span();
            if count > 1 {
                format!("{label}({count})")
            } else {
                label
            }
        })
        .collect()
}

fn summary_label(gate: &GateEvaluation, multiple_scopes: bool) -> Option<String> {
    if let Some(label) = &gate.label {
        return Some(label.clone());
    }

    if multiple_scopes {
        return Some("default".to_string());
    }

    None
}

fn format_percent(percent: f64, total: usize) -> String {
    if total == 0 {
        "N/A".to_string()
    } else {
        format!("{percent:.2}%")
    }
}
