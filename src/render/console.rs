use std::collections::BTreeMap;

use crate::model::{
    ComputedMetric, GateResult, GateScopeResult, MetricKind, RuleOutcome, SourceSpan, Verbosity,
};
use crate::render::title_case;

#[must_use]
pub fn render(result: &GateResult, diff_description: &str, verbosity: Verbosity) -> String {
    match verbosity {
        Verbosity::Verbose => render_verbose(result, diff_description),
        Verbosity::Normal => render_minimal(result, diff_description),
    }
}

fn render_verbose(result: &GateResult, diff_description: &str) -> String {
    let mut out = String::new();
    out.push_str("-------------\n");
    out.push_str(&format!(
        "Diff Coverage: {}\n",
        if result.passed { "PASS" } else { "FAIL" }
    ));
    out.push_str(&format!("Diff: {diff_description}\n"));
    out.push_str("-------------\n");

    let multiple_scopes = result.scopes.len() > 1;

    for scope in &result.scopes {
        if let Some(label) = scope_label(scope, multiple_scopes) {
            out.push_str(&format!("Gate: {label}\n"));
        }

        for metric in &scope.metrics {
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

        for metric in &scope.metrics {
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
                "{} Coverage: {:.2}%\n",
                title_case(metric.metric.as_str()),
                metric.percent
            ));
        }

        for outcome in &scope.rules {
            let status = if outcome.passed { "PASS" } else { "FAIL" };
            match &outcome.rule {
                crate::model::GateRule::Percent {
                    metric: _,
                    minimum_percent,
                } => {
                    let comparator = if outcome.passed { "≥" } else { "≱" };
                    out.push_str(&format!(
                        "Rule {}: {} ({:.2}% {} {:.2}%)\n",
                        outcome.rule.label(),
                        status,
                        outcome.observed_percent,
                        comparator,
                        minimum_percent
                    ));
                }
                crate::model::GateRule::UncoveredCount {
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

fn render_minimal(result: &GateResult, diff_description: &str) -> String {
    let mut out = String::new();

    if !result.passed {
        out.push_str(&format!("Diff: {diff_description}\n\n"));
        out.push_str(&render_failures(result));
    }

    let multiple_scopes = result.scopes.len() > 1;
    for scope in &result.scopes {
        let label = summary_label(scope, multiple_scopes);
        for metric in &scope.metrics {
            let outcome = scope
                .rules
                .iter()
                .find(|rule| rule.rule.metric() == metric.metric);
            if let Some(outcome) = outcome {
                out.push_str(&render_metric_summary(
                    metric,
                    Some(outcome),
                    label.as_deref(),
                ));
                out.push('\n');
            }
        }
    }

    out.trim_end().to_string()
}

fn render_failures(result: &GateResult) -> String {
    let mut out = String::new();
    let multiple_scopes = result.scopes.len() > 1;

    for scope in &result.scopes {
        let files_with_uncovered = group_uncovered_by_file(scope);
        if files_with_uncovered.is_empty() {
            continue;
        }

        if let Some(label) = summary_label(scope, multiple_scopes) {
            out.push_str(&format!("[{label}]\n"));
        }

        for (path, metrics) in files_with_uncovered {
            out.push_str(&render_file_failure_header(&path, scope));
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
    }

    out
}

fn render_metric_summary(
    metric: &ComputedMetric,
    rule_outcome: Option<&RuleOutcome>,
    scope_label: Option<&str>,
) -> String {
    let Some(outcome) = rule_outcome else {
        return String::new();
    };

    let status = if outcome.passed { "PASS" } else { "FAIL" };
    let metric_label = title_case(metric.metric.label());

    let rule_str = match &outcome.rule {
        crate::model::GateRule::Percent {
            metric: _,
            minimum_percent,
        } => {
            let comparator = if outcome.passed { "≥" } else { "≱" };
            format!("  {} {:.2}%", comparator, minimum_percent)
        }
        crate::model::GateRule::UncoveredCount {
            metric: _,
            maximum_count,
        } => {
            let comparator = if outcome.passed { "≤" } else { "≰" };
            format!("  {} {}", comparator, maximum_count)
        }
    };

    let summary = format!(
        "{}  {:<11} {:>7.2}% {:>13}{:<11}",
        status,
        format!("{}:", metric_label),
        metric.percent,
        format!("({}/{})", metric.covered, metric.total),
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
    scope: &GateScopeResult,
) -> BTreeMap<std::path::PathBuf, BTreeMap<MetricKind, Vec<SourceSpan>>> {
    let mut files_with_uncovered: BTreeMap<
        std::path::PathBuf,
        BTreeMap<MetricKind, Vec<SourceSpan>>,
    > = BTreeMap::new();
    for metric in &scope.metrics {
        if !scope
            .rules
            .iter()
            .any(|rule| rule.rule.metric() == metric.metric)
        {
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

fn render_file_failure_header(path: &std::path::Path, scope: &GateScopeResult) -> String {
    let mut header = path.display().to_string();
    let mut stats = Vec::new();

    for metric in &scope.metrics {
        if !scope
            .rules
            .iter()
            .any(|rule| rule.rule.metric() == metric.metric)
        {
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

fn summary_label(scope: &GateScopeResult, multiple_scopes: bool) -> Option<String> {
    if let Some(label) = &scope.label {
        return Some(label.clone());
    }

    if multiple_scopes {
        return Some("default".to_string());
    }

    None
}

fn scope_label(scope: &GateScopeResult, multiple_scopes: bool) -> Option<&str> {
    if let Some(label) = scope.label.as_deref() {
        return Some(label);
    }

    if multiple_scopes {
        return Some("default");
    }

    None
}

#[cfg(test)]
mod tests {
    use crate::model::MetricKind;
    use std::collections::BTreeMap;

    #[test]
    fn render_metric_summary_handles_none_outcome() {
        let metric = crate::model::ComputedMetric {
            metric: MetricKind::Region,
            covered: 1,
            total: 1,
            percent: 100.0,
            uncovered_changed_opportunities: Vec::new(),
            changed_totals_by_file: BTreeMap::new(),
            totals_by_file: BTreeMap::new(),
        };
        assert_eq!(super::render_metric_summary(&metric, None, None), "");
    }
}
