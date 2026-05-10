use std::collections::BTreeMap;

use crate::model::{
    CheckResult, ComputedMetric, GateEvaluation, GateRule, MetricKind, RuleOutcome, SourceSpan,
    Verbosity,
};
use crate::render::title_case;

#[must_use]
pub fn render(result: &CheckResult, diff_description: &str, verbosity: Verbosity) -> String {
    match verbosity {
        Verbosity::Verbose => render_verbose(result, diff_description),
        Verbosity::Normal => render_minimal(result, diff_description),
    }
}

fn render_verbose(result: &CheckResult, diff_description: &str) -> String {
    let mut out = String::new();
    out.push_str("-------------\n");
    out.push_str(&format!(
        "Diff Coverage: {}\n",
        if result.passed { "PASS" } else { "FAIL" }
    ));
    out.push_str(&format!("Diff: {diff_description}\n"));
    out.push_str("-------------\n");

    for metric in &result.changed_metrics {
        let spans: Vec<&SourceSpan> = metric
            .uncovered_changed_opportunities
            .iter()
            .map(|opportunity| &opportunity.span)
            .collect();
        let grouped = group_spans(&spans);
        for (path, totals) in &metric.changed_totals_by_file {
            let path_display = path.display().to_string();
            let percent = if totals.total == 0 {
                100.0
            } else {
                (totals.covered as f64 / totals.total as f64) * 100.0
            };
            if let Some(spans) = grouped.get(&path_display) {
                out.push_str(&format!(
                    "{path_display} ({percent:.2}%): uncovered changed {} spans {}\n",
                    metric.metric.as_str(),
                    spans.join(", ")
                ));
            } else {
                out.push_str(&format!(
                    "{path_display} ({percent:.2}%) [{}]\n",
                    metric.metric.as_str()
                ));
            }
        }
    }

    out.push_str("-------------\n");

    for metric in &result.changed_metrics {
        out.push_str(&format!(
            "Changed {}: {}\n",
            metric.metric.label(),
            metric.total
        ));
        out.push_str(&format!(
            "Covered {}: {}\n",
            metric.metric.label(),
            metric.covered
        ));
        out.push_str(&format!(
            "{} Coverage: {}\n",
            title_case(metric.metric.as_str()),
            format_percent(metric.percent, metric.covered, metric.total)
        ));
    }

    let multiple_gates = result.gates.len() > 1;
    for gate in &result.gates {
        if let Some(label) = scope_label(gate, multiple_gates) {
            out.push_str(&format!("Gate: {label}\n"));
        }

        for outcome in &gate.rules {
            let status = if outcome.passed { "PASS" } else { "FAIL" };
            match &outcome.rule {
                GateRule::Percent {
                    metric: _,
                    minimum_percent,
                } => {
                    let comparator = if outcome.passed { "≥" } else { "≱" };
                    out.push_str(&format!(
                        "Rule {}: {} ({} {} {:.2}%)\n",
                        outcome.rule.label(),
                        status,
                        format_percent(
                            outcome.observed_percent,
                            outcome.observed_covered_count,
                            outcome.observed_total_count
                        ),
                        comparator,
                        minimum_percent
                    ));
                }
                GateRule::UncoveredCount {
                    metric: _,
                    maximum_count,
                } => {
                    let comparator = if outcome.passed { "≤" } else { "≰" };
                    out.push_str(&format!(
                        "Rule {}: {} ({} {} {})\n",
                        outcome.rule.label(),
                        status,
                        outcome.observed_uncovered_count,
                        comparator,
                        maximum_count
                    ));
                }
            }
        }

        out.push_str("-------------\n");
    }

    if !result.overall_metrics.is_empty() {
        out.push_str("Overall Coverage\n");
        out.push_str("-------------\n");
        for metric in &result.overall_metrics {
            out.push_str(&format!(
                "{:<15} {:>7.2}% ({}/{})\n",
                format!("{}:", title_case(metric.metric.as_str())),
                metric.percent,
                metric.covered,
                metric.total
            ));
        }
        out.push_str("-------------\n");
    }

    if out.ends_with("-------------\n") {
        out.truncate(out.len() - 1);
    }

    out
}

fn render_minimal(result: &CheckResult, diff_description: &str) -> String {
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
    let files_with_uncovered = group_uncovered_by_file(&result.changed_metrics, &failed_metrics);
    for (path, metrics) in files_with_uncovered {
        out.push_str(&render_file_failure_header(
            &path,
            &result.changed_metrics,
            &failed_metrics,
        ));
        out.push('\n');

        for (metric_kind, spans) in metrics {
            let grouped = group_file_spans(&spans);
            out.push_str(&format!(
                "  {}: {}\n",
                metric_kind.label(),
                grouped.join(", ")
            ));
        }
        out.push('\n');
    }

    out
}

fn render_rule_summary(outcome: &RuleOutcome, scope_label: Option<&str>) -> String {
    let status = if outcome.passed { "PASS" } else { "FAIL" };
    let metric_label = title_case(outcome.rule.metric().label());

    let rule_str = match &outcome.rule {
        GateRule::Percent {
            metric: _,
            minimum_percent,
        } => {
            let comparator = if outcome.passed { "≥" } else { "≱" };
            format!("  {} {:.2}%", comparator, minimum_percent)
        }
        GateRule::UncoveredCount {
            metric: _,
            maximum_count,
        } => {
            let comparator = if outcome.passed { "≤" } else { "≰" };
            format!("  {} {}", comparator, maximum_count)
        }
    };

    let observed = match &outcome.rule {
        GateRule::Percent {
            metric: _,
            minimum_percent: _,
        } => format_percent(
            outcome.observed_percent,
            outcome.observed_covered_count,
            outcome.observed_total_count,
        ),
        GateRule::UncoveredCount {
            metric: _,
            maximum_count: _,
        } => outcome.observed_uncovered_count.to_string(),
    };

    let counts = match &outcome.rule {
        GateRule::Percent {
            metric: _,
            minimum_percent: _,
        } => {
            format!(
                "({}/{})",
                outcome.observed_covered_count, outcome.observed_total_count
            )
        }
        GateRule::UncoveredCount {
            metric: _,
            maximum_count: _,
        } => String::new(),
    };

    let summary = format!(
        "{}  {:<11} {:>7} {:>13}{:<11}",
        status,
        format!("{}:", metric_label),
        observed,
        counts,
        rule_str
    )
    .trim_end()
    .to_string();

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

fn group_spans(spans: &[&SourceSpan]) -> BTreeMap<String, Vec<String>> {
    let mut grouped: BTreeMap<String, Vec<SourceSpan>> = BTreeMap::new();
    for span in spans {
        grouped
            .entry(span.path.display().to_string())
            .or_default()
            .push((*span).clone());
    }
    grouped
        .into_iter()
        .map(|(path, spans)| (path, group_file_spans(&spans)))
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

fn scope_label(gate: &GateEvaluation, multiple_scopes: bool) -> Option<&str> {
    if let Some(label) = gate.label.as_deref() {
        return Some(label);
    }

    if multiple_scopes {
        return Some("default");
    }

    None
}

fn format_percent(percent: f64, _covered: usize, total: usize) -> String {
    if total == 0 {
        "N/A".to_string()
    } else {
        format!("{percent:.2}%")
    }
}
