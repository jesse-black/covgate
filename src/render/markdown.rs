use crate::model::{GateResult, SpanKey};
use crate::render::title_case;

#[must_use]
pub fn render(result: &GateResult, _diff_description: &str) -> String {
    let mut out = String::new();
    out.push_str("## Covgate\n\n");
    out.push_str("### Diff Coverage\n\n");
    out.push_str("| Result | Rule | Observed | Configured |\n");
    out.push_str("| --- | --- | ---: | ---: |\n");
    for outcome in &result.rules {
        let status = if outcome.passed { "✅PASS" } else { "❌FAIL" };
        match &outcome.rule {
            crate::model::GateRule::Percent {
                metric: _,
                minimum_percent,
            } => {
                out.push_str(&format!(
                    "| {} | `{}` | {:.2}% | ≥ {:.2}% |\n",
                    status,
                    outcome.rule.label(),
                    outcome.observed_percent,
                    minimum_percent
                ));
            }
            crate::model::GateRule::UncoveredCount {
                metric: _,
                maximum_count,
            } => {
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
    out.push('\n');

    for metric in &result.metrics {
        let metric_label = title_case(metric.metric.label());
        out.push_str(&format!("#### {}\n\n", title_case(metric.metric.as_str())));
        out.push_str(&format!(
            "| File | Covered Changed {metric_label} | Changed {metric_label} | Coverage | Missed Changed Spans |\n"
        ));
        out.push_str("| --- | ---: | ---: | ---: | --- |\n");
        let mut missed_by_file =
            std::collections::BTreeMap::<String, std::collections::BTreeMap<SpanKey, usize>>::new();
        for opportunity in &metric.uncovered_changed_opportunities {
            missed_by_file
                .entry(opportunity.span.path.display().to_string())
                .or_default()
                .entry(opportunity.span.key())
                .and_modify(|count| *count += 1)
                .or_insert(1);
        }
        for (path, totals) in &metric.changed_totals_by_file {
            let percent = if totals.total == 0 {
                100.0
            } else {
                (totals.covered as f64 / totals.total as f64) * 100.0
            };
            let missed = missed_by_file
                .get(&path.display().to_string())
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
                .unwrap_or_default();
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
        out.push('\n');
    }

    out.push_str("### Overall Coverage\n\n");
    for metric in &result.metrics {
        let metric_label = title_case(metric.metric.label());
        out.push_str(&format!("#### {}\n\n", title_case(metric.metric.as_str())));
        out.push_str(&format!(
            "| File | Covered {metric_label} | {metric_label} | Missed {metric_label} | Coverage |\n"
        ));
        out.push_str("| --- | ---: | ---: | ---: | ---: |\n");
        for (path, totals) in &metric.totals_by_file {
            let percent = if totals.total == 0 {
                100.0
            } else {
                (totals.covered as f64 / totals.total as f64) * 100.0
            };
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
        let overall_percent = if overall_total == 0 {
            100.0
        } else {
            (overall_covered as f64 / overall_total as f64) * 100.0
        };
        let overall_missed = overall_total.saturating_sub(overall_covered);
        out.push_str(&format!(
            "| **Total** | **{}** | **{}** | **{}** | **{:.2}% {}** |\n",
            overall_covered,
            overall_total,
            overall_missed,
            overall_percent,
            coverage_circle(overall_percent)
        ));
        out.push('\n');
    }

    out
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
