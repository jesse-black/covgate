use std::collections::BTreeMap;

use crate::model::{ComputedMetric, GateResult, MetricKind, RuleOutcome, SourceSpan};
use crate::render::title_case;

pub fn render(result: &GateResult, diff_description: &str, verbose: bool) -> String {
    if verbose {
        render_verbose(result, diff_description)
    } else {
        render_minimal(result, diff_description)
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

    for metric in &result.metrics {
        let spans: Vec<&SourceSpan> = metric
            .uncovered_changed_opportunities
            .iter()
            .map(|o| &o.span)
            .collect();
        let grouped = group_spans(&spans);
        for (path, totals) in &metric.changed_totals_by_file {
            let path_display = path.display().to_string();
            let file_total = totals.total;
            let covered = totals.covered;
            let percent = if file_total == 0 {
                100.0
            } else {
                (covered as f64 / file_total as f64) * 100.0
            };
            if let Some(spans) = grouped.get(&path_display) {
                out.push_str(&format!(
                    "{path_display} ({percent:.2}%): uncovered changed {} spans {}\n",
                    metric.metric.as_str(),
                    spans.missed.join(", ")
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

    for metric in &result.metrics {
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

    for outcome in &result.rules {
        let status = if outcome.passed { "PASS" } else { "FAIL" };
        match &outcome.rule {
            crate::model::GateRule::Percent {
                minimum_percent, ..
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
            crate::model::GateRule::UncoveredCount { maximum_count, .. } => {
                if outcome.passed {
                    out.push_str(&format!(
                        "Rule {}: {} ({} <= {})\n",
                        outcome.rule.label(),
                        status,
                        outcome.observed_uncovered_count,
                        maximum_count
                    ));
                } else {
                    out.push_str(&format!(
                        "Rule {}: {} ({} > {})\n",
                        outcome.rule.label(),
                        status,
                        outcome.observed_uncovered_count,
                        maximum_count
                    ));
                }
            }
        }
    }

    out.push_str("-------------");
    out
}

fn render_minimal(result: &GateResult, diff_description: &str) -> String {
    let mut out = String::new();

    if !result.passed {
        out.push_str(&format!("Diff: {diff_description}\n\n"));
        out.push_str(&render_failures(result));
    }

    for metric in &result.metrics {
        let rule_outcome = result
            .rules
            .iter()
            .find(|r| r.rule.metric() == metric.metric);
        out.push_str(&render_metric_summary(metric, rule_outcome));
        out.push('\n');
    }

    out.trim_end().to_string()
}

fn render_failures(result: &GateResult) -> String {
    let mut out = String::new();
    let files_with_uncovered = group_uncovered_by_file(result);

    for (path, metrics) in files_with_uncovered {
        out.push_str(&render_file_failure_header(&path, result));
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

fn render_metric_summary(metric: &ComputedMetric, rule_outcome: Option<&RuleOutcome>) -> String {
    let status = if let Some(outcome) = rule_outcome {
        if outcome.passed { "PASS" } else { "FAIL" }
    } else {
        "PASS"
    };

    let label = title_case(metric.metric.label());

    let rule_str = if let Some(outcome) = rule_outcome {
        match &outcome.rule {
            crate::model::GateRule::Percent {
                minimum_percent, ..
            } => {
                let comparator = if outcome.passed { "≥" } else { "≱" };
                format!("  {} {:.2}%", comparator, minimum_percent)
            }
            crate::model::GateRule::UncoveredCount { maximum_count, .. } => {
                let comparator = if outcome.passed { "≤" } else { ">" };
                format!("  {} {}", comparator, maximum_count)
            }
        }
    } else {
        String::new()
    };

    format!(
        "{}  {:<11} {:>7.2}% ({}/{}){}",
        status,
        format!("{}:", label),
        metric.percent,
        metric.covered,
        metric.total,
        rule_str
    )
}

fn group_uncovered_by_file(
    result: &GateResult,
) -> BTreeMap<std::path::PathBuf, BTreeMap<MetricKind, Vec<SourceSpan>>> {
    let mut files_with_uncovered: BTreeMap<
        std::path::PathBuf,
        BTreeMap<MetricKind, Vec<SourceSpan>>,
    > = BTreeMap::new();
    for metric in &result.metrics {
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

fn render_file_failure_header(path: &std::path::Path, result: &GateResult) -> String {
    let mut header = format!("{}", path.display());
    let mut stats = Vec::new();
    for metric_kind in [
        MetricKind::Line,
        MetricKind::Branch,
        MetricKind::Function,
        MetricKind::Region,
    ] {
        if let Some(metric_data) = result.metrics.iter().find(|m| m.metric == metric_kind) {
            if let Some(file_totals) = metric_data.changed_totals_by_file.get(path) {
                let percent = if file_totals.total == 0 {
                    100.0
                } else {
                    (file_totals.covered as f64 / file_totals.total as f64) * 100.0
                };
                stats.push(format!("{:.2}% {}", percent, metric_kind.as_str()));
            }
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

struct FileSummary {
    missed: Vec<String>,
}

fn group_spans(spans: &[&SourceSpan]) -> BTreeMap<String, FileSummary> {
    let mut grouped: BTreeMap<String, Vec<SourceSpan>> = BTreeMap::new();
    for span in spans {
        grouped
            .entry(span.path.display().to_string())
            .or_default()
            .push((*span).clone());
    }
    grouped
        .into_iter()
        .map(|(path, spans)| {
            let missed = group_file_spans(&spans);
            (path, FileSummary { missed })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, path::PathBuf};

    use crate::model::{FileTotals, GateResult, GateRule, MetricKind, RuleOutcome};

    use super::render;

    #[test]
    fn renders_console_summary_minimal() {
        let result = GateResult {
            metrics: vec![crate::model::ComputedMetric {
                metric: MetricKind::Region,
                covered: 1,
                total: 2,
                percent: 50.0,
                uncovered_changed_opportunities: vec![crate::model::CoverageOpportunity {
                    kind: crate::model::OpportunityKind::Region,
                    span: crate::model::SourceSpan {
                        path: PathBuf::from("src/lib.rs"),
                        start_line: 5,
                        end_line: 6,
                        start_col: None,
                        end_col: None,
                    },
                    covered: false,
                }],
                changed_totals_by_file: BTreeMap::from([(
                    PathBuf::from("src/lib.rs"),
                    FileTotals {
                        covered: 1,
                        total: 2,
                    },
                )]),
                totals_by_file: BTreeMap::new(),
            }],
            rules: vec![RuleOutcome {
                rule: GateRule::Percent {
                    metric: MetricKind::Region,
                    minimum_percent: 90.0,
                },
                passed: false,
                observed_percent: 50.0,
                observed_uncovered_count: 1,
            }],
            passed: false,
        };

        let rendered = render(&result, "origin/main...HEAD", false);
        assert!(!rendered.contains("Diff Coverage: FAIL"));
        assert!(rendered.contains("src/lib.rs (50.00% region)"));
        assert!(rendered.contains("FAIL  Regions:      50.00% (1/2)  ≱ 90.00%"));
    }

    #[test]
    fn renders_console_summary_verbose() {
        let result = GateResult {
            metrics: vec![crate::model::ComputedMetric {
                metric: MetricKind::Region,
                covered: 1,
                total: 2,
                percent: 50.0,
                uncovered_changed_opportunities: vec![crate::model::CoverageOpportunity {
                    kind: crate::model::OpportunityKind::Region,
                    span: crate::model::SourceSpan {
                        path: PathBuf::from("src/lib.rs"),
                        start_line: 5,
                        end_line: 6,
                        start_col: None,
                        end_col: None,
                    },
                    covered: false,
                }],
                changed_totals_by_file: BTreeMap::from([(
                    PathBuf::from("src/lib.rs"),
                    FileTotals {
                        covered: 1,
                        total: 2,
                    },
                )]),
                totals_by_file: BTreeMap::new(),
            }],
            rules: vec![RuleOutcome {
                rule: GateRule::Percent {
                    metric: MetricKind::Region,
                    minimum_percent: 90.0,
                },
                passed: false,
                observed_percent: 50.0,
                observed_uncovered_count: 1,
            }],
            passed: false,
        };

        let rendered = render(&result, "origin/main...HEAD", true);
        assert!(rendered.contains("Diff Coverage: FAIL"));
        assert!(rendered.contains("src/lib.rs (50.00%)"));
        assert!(rendered.contains("Rule fail-under-regions: FAIL (50.00% ≱ 90.00%)"));
    }

    #[test]
    fn groups_duplicate_spans_with_counts() {
        let result = GateResult {
            metrics: vec![crate::model::ComputedMetric {
                metric: MetricKind::Region,
                covered: 1,
                total: 3,
                percent: 33.33,
                uncovered_changed_opportunities: vec![
                    crate::model::CoverageOpportunity {
                        kind: crate::model::OpportunityKind::Region,
                        span: crate::model::SourceSpan {
                            path: PathBuf::from("src/lib.rs"),
                            start_line: 5,
                            end_line: 6,
                            start_col: None,
                            end_col: None,
                        },
                        covered: false,
                    },
                    crate::model::CoverageOpportunity {
                        kind: crate::model::OpportunityKind::Region,
                        span: crate::model::SourceSpan {
                            path: PathBuf::from("src/lib.rs"),
                            start_line: 5,
                            end_line: 6,
                            start_col: None,
                            end_col: None,
                        },
                        covered: false,
                    },
                ],
                changed_totals_by_file: BTreeMap::from([(
                    PathBuf::from("src/lib.rs"),
                    FileTotals {
                        covered: 1,
                        total: 3,
                    },
                )]),
                totals_by_file: BTreeMap::new(),
            }],
            rules: vec![RuleOutcome {
                rule: GateRule::Percent {
                    metric: MetricKind::Region,
                    minimum_percent: 90.0,
                },
                passed: false,
                observed_percent: 33.33,
                observed_uncovered_count: 2,
            }],
            passed: false,
        };

        let rendered = render(&result, "origin/main...HEAD", false);
        assert!(rendered.contains("5-6(2)"));
    }

    #[test]
    fn sorts_spans_numerically() {
        let result = GateResult {
            metrics: vec![crate::model::ComputedMetric {
                metric: MetricKind::Region,
                covered: 1,
                total: 3,
                percent: 33.33,
                uncovered_changed_opportunities: vec![
                    crate::model::CoverageOpportunity {
                        kind: crate::model::OpportunityKind::Region,
                        span: crate::model::SourceSpan {
                            path: PathBuf::from("src/lib.rs"),
                            start_line: 102,
                            end_line: 102,
                            start_col: None,
                            end_col: None,
                        },
                        covered: false,
                    },
                    crate::model::CoverageOpportunity {
                        kind: crate::model::OpportunityKind::Region,
                        span: crate::model::SourceSpan {
                            path: PathBuf::from("src/lib.rs"),
                            start_line: 48,
                            end_line: 48,
                            start_col: None,
                            end_col: None,
                        },
                        covered: false,
                    },
                ],
                changed_totals_by_file: BTreeMap::from([(
                    PathBuf::from("src/lib.rs"),
                    FileTotals {
                        covered: 1,
                        total: 3,
                    },
                )]),
                totals_by_file: BTreeMap::new(),
            }],
            rules: vec![RuleOutcome {
                rule: GateRule::Percent {
                    metric: MetricKind::Region,
                    minimum_percent: 90.0,
                },
                passed: false,
                observed_percent: 33.33,
                observed_uncovered_count: 2,
            }],
            passed: false,
        };

        let rendered = render(&result, "origin/main...HEAD", false);
        let _row = rendered
            .lines()
            .find(|line| line.contains("src/lib.rs"))
            .expect("file row should exist");
        let spans_row = rendered
            .lines()
            .find(|line| line.contains("regions:"))
            .expect("spans row should exist");
        assert!(spans_row.find("48").expect("48") < spans_row.find("102").expect("102"));
    }
}
