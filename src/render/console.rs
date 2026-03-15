use std::collections::BTreeMap;

use crate::model::{GateResult, SourceSpan};

pub fn render(result: &GateResult, diff_description: &str) -> String {
    let mut out = String::new();
    out.push_str("-------------\n");
    out.push_str(&format!(
        "Diff Coverage: {}\n",
        if result.passed { "PASS" } else { "FAIL" }
    ));
    out.push_str(&format!("Diff: {diff_description}\n"));
    out.push_str("-------------\n");

    for metric in &result.metrics {
        let grouped = group_spans(
            &metric
                .uncovered_changed_opportunities
                .iter()
                .map(|o| &o.span)
                .collect::<Vec<_>>(),
        );
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

fn title_case(value: &str) -> String {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

struct FileSummary {
    missed: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct SpanKey {
    start_line: u32,
    end_line: u32,
}

fn group_spans(spans: &[&SourceSpan]) -> BTreeMap<String, FileSummary> {
    let mut grouped: BTreeMap<String, BTreeMap<SpanKey, usize>> = BTreeMap::new();
    for span in spans {
        let key = SpanKey {
            start_line: span.start_line,
            end_line: span.end_line,
        };
        let entry = grouped.entry(span.path.display().to_string()).or_default();
        *entry.entry(key).or_default() += 1;
    }
    grouped
        .into_iter()
        .map(|(path, spans)| {
            let missed = spans
                .into_iter()
                .map(|(key, count)| {
                    let label = format!("{}-{}", key.start_line, key.end_line);
                    if count > 1 {
                        format!("{label}({count})")
                    } else {
                        label
                    }
                })
                .collect();
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
    fn renders_console_summary() {
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

        let rendered = render(&result, "origin/main...HEAD");
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
                        },
                        covered: false,
                    },
                    crate::model::CoverageOpportunity {
                        kind: crate::model::OpportunityKind::Region,
                        span: crate::model::SourceSpan {
                            path: PathBuf::from("src/lib.rs"),
                            start_line: 5,
                            end_line: 6,
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

        let rendered = render(&result, "origin/main...HEAD");
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
                        },
                        covered: false,
                    },
                    crate::model::CoverageOpportunity {
                        kind: crate::model::OpportunityKind::Region,
                        span: crate::model::SourceSpan {
                            path: PathBuf::from("src/lib.rs"),
                            start_line: 48,
                            end_line: 48,
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

        let rendered = render(&result, "origin/main...HEAD");
        let row = rendered
            .lines()
            .find(|line| line.starts_with("src/lib.rs "))
            .expect("file row should exist");
        assert!(row.find("48-48").expect("48-48") < row.find("102-102").expect("102-102"));
    }
}
