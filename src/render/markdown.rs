use std::collections::{BTreeMap, BTreeSet};

use crate::model::{CheckResult, ComputedMetric, GateEvaluation, MetricKind, SpanKey};
use crate::render::title_case;

#[must_use]
pub fn render(result: &CheckResult, _diff_description: &str) -> String {
    let mut out = String::new();
    let show_gate_column = result.gates.len() > 1
        || result
            .gates
            .first()
            .is_some_and(|gate| gate.label.is_some());

    out.push_str("## Covgate\n\n");
    out.push_str("### Diff Coverage\n\n");
    if show_gate_column {
        out.push_str("| Gate | Result | Rule | Observed | Configured |\n");
        out.push_str("| --- | --- | --- | ---: | ---: |\n");
        for gate in &result.gates {
            let gate_label = scope_label(gate, show_gate_column);
            for outcome in &gate.rules {
                write_rule_row(&mut out, Some(gate_label.as_str()), outcome);
            }
        }
    } else if let Some(gate) = result.gates.first() {
        out.push_str("| Result | Rule | Observed | Configured |\n");
        out.push_str("| --- | --- | ---: | ---: |\n");
        for outcome in &gate.rules {
            write_rule_row(&mut out, None, outcome);
        }
    }
    out.push('\n');

    for metric_kind in metric_kinds(&result.changed_metrics) {
        out.push_str(&format!("#### {}\n\n", title_case(metric_kind.as_str())));
        if let Some(metric) = result
            .changed_metrics
            .iter()
            .find(|metric| metric.metric == metric_kind)
        {
            render_changed_metric_table_single_scope(&mut out, metric);
        }
        out.push('\n');
    }

    out.push_str("### Overall Coverage\n\n");
    for metric in &result.overall_metrics {
        out.push_str(&format!("#### {}\n\n", title_case(metric.metric.as_str())));
        render_overall_metric_table_single_scope(&mut out, metric);
        out.push('\n');
    }

    out
}

fn write_rule_row(out: &mut String, gate_label: Option<&str>, outcome: &crate::model::RuleOutcome) {
    let status = if outcome.passed { "✅PASS" } else { "❌FAIL" };
    match &outcome.rule {
        crate::model::GateRule::Percent {
            metric: _,
            minimum_percent,
        } => {
            if let Some(gate_label) = gate_label {
                out.push_str(&format!(
                    "| `{gate_label}` | {} | `{}` | {} | ≥ {:.2}% |\n",
                    status,
                    outcome.rule.label(),
                    format_observed_percent(outcome),
                    minimum_percent
                ));
            } else {
                out.push_str(&format!(
                    "| {} | `{}` | {} | ≥ {:.2}% |\n",
                    status,
                    outcome.rule.label(),
                    format_observed_percent(outcome),
                    minimum_percent
                ));
            }
        }
        crate::model::GateRule::UncoveredCount {
            metric: _,
            maximum_count,
        } => {
            if let Some(gate_label) = gate_label {
                out.push_str(&format!(
                    "| `{gate_label}` | {} | `{}` | {} | ≤ {} |\n",
                    status,
                    outcome.rule.label(),
                    outcome.observed_uncovered_count,
                    maximum_count
                ));
            } else {
                out.push_str(&format!(
                    "| {} | `{}` | {} | ≤ {} |\n",
                    status,
                    outcome.rule.label(),
                    outcome.observed_uncovered_count,
                    maximum_count
                ));
            }
        }
    }
}

fn render_changed_metric_table_single_scope(out: &mut String, metric: &ComputedMetric) {
    let metric_label = title_case(metric.metric.label());
    out.push_str(&format!(
        "| File | Covered Changed {metric_label} | Changed {metric_label} | Coverage | Missed Changed Spans |\n"
    ));
    out.push_str("| --- | ---: | ---: | ---: | --- |\n");
    let missed_by_file = missed_by_file(metric);
    for (path, totals) in &metric.changed_totals_by_file {
        let percent = percent(totals.covered, totals.total);
        let missed = format_missed_spans(missed_by_file.get(&path.display().to_string()));
        out.push_str(&format!(
            "| `{}` | {} | {} | {:.2}% {} | {} |\n",
            path.display(),
            totals.covered,
            totals.total,
            percent,
            coverage_circle(percent),
            missed
        ));
    }
    out.push_str(&format!(
        "| **Total** | **{}** | **{}** | **{:.2}% {}** |  |\n",
        metric.covered,
        metric.total,
        metric.percent,
        coverage_circle(metric.percent)
    ));
}

fn render_overall_metric_table_single_scope(out: &mut String, metric: &ComputedMetric) {
    let metric_label = title_case(metric.metric.label());
    out.push_str(&format!(
        "| File | Covered {metric_label} | {metric_label} | Missed {metric_label} | Coverage |\n"
    ));
    out.push_str("| --- | ---: | ---: | ---: | ---: |\n");
    for (path, totals) in &metric.totals_by_file {
        let percent = percent(totals.covered, totals.total);
        let missed = totals.total.saturating_sub(totals.covered);
        out.push_str(&format!(
            "| `{}` | {} | {} | {} | {:.2}% {} |\n",
            path.display(),
            totals.covered,
            totals.total,
            missed,
            percent,
            coverage_circle(percent)
        ));
    }
    let overall_covered: usize = metric
        .totals_by_file
        .values()
        .map(|totals| totals.covered)
        .sum();
    let overall_total: usize = metric
        .totals_by_file
        .values()
        .map(|totals| totals.total)
        .sum();
    let overall_percent = percent(overall_covered, overall_total);
    let overall_missed = overall_total.saturating_sub(overall_covered);
    out.push_str(&format!(
        "| **Total** | **{}** | **{}** | **{}** | **{:.2}% {}** |\n",
        overall_covered,
        overall_total,
        overall_missed,
        overall_percent,
        coverage_circle(overall_percent)
    ));
}

fn metric_kinds(metrics: &[ComputedMetric]) -> Vec<MetricKind> {
    let mut kinds = BTreeSet::new();
    for metric in metrics {
        kinds.insert(metric.metric);
    }
    kinds.into_iter().collect()
}

fn missed_by_file(metric: &ComputedMetric) -> BTreeMap<String, BTreeMap<SpanKey, usize>> {
    let mut missed_by_file = BTreeMap::<String, BTreeMap<SpanKey, usize>>::new();
    for opportunity in &metric.uncovered_changed_opportunities {
        missed_by_file
            .entry(opportunity.span.path.display().to_string())
            .or_default()
            .entry(opportunity.span.key())
            .and_modify(|count| *count += 1)
            .or_insert(1);
    }
    missed_by_file
}

fn format_missed_spans(spans: Option<&BTreeMap<SpanKey, usize>>) -> String {
    spans
        .map(|values| {
            values
                .iter()
                .map(|(key, count)| {
                    let label = key.format_span();
                    if *count > 1 {
                        format!("`{label}({count})`")
                    } else {
                        format!("`{label}`")
                    }
                })
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default()
}

fn percent(covered: usize, total: usize) -> f64 {
    if total == 0 {
        100.0
    } else {
        (covered as f64 / total as f64) * 100.0
    }
}

fn scope_label(gate: &GateEvaluation, multiple_scopes: bool) -> String {
    if let Some(label) = &gate.label {
        return label.clone();
    }

    if multiple_scopes {
        return "default".to_string();
    }

    String::new()
}

fn format_observed_percent(outcome: &crate::model::RuleOutcome) -> String {
    let counts = format!(
        "({}/{})",
        outcome.observed_covered_count, outcome.observed_total_count
    );
    if outcome.observed_total_count == 0 {
        format!("N/A {counts}")
    } else {
        format!("{:.2}% {counts}", outcome.observed_percent)
    }
}

fn coverage_circle(percent: f64) -> &'static str {
    if percent < 50.0 {
        "🔴"
    } else if percent < 80.0 {
        "🟡"
    } else {
        "🟢"
    }
}
