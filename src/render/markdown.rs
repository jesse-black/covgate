use crate::model::GateResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct SpanKey {
    start_line: u32,
    end_line: u32,
}

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
                minimum_percent, ..
            } => {
                out.push_str(&format!(
                    "| {} | `{}` | {:.2}% | ≥ {:.2}% |\n",
                    status,
                    outcome.rule.label(),
                    outcome.observed_percent,
                    minimum_percent
                ));
            }
            crate::model::GateRule::UncoveredCount { maximum_count, .. } => {
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
                .entry(SpanKey {
                    start_line: opportunity.span.start_line,
                    end_line: opportunity.span.end_line,
                })
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
                            let label = format!("{}-{}", key.start_line, key.end_line);
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
                "| `{}` | {} | {} | {:.2}% | {} |\n",
                path.display(),
                totals.covered,
                totals.total,
                percent,
                missed
            ));
        }
        out.push_str(&format!(
            "| **Total** | **{}** | **{}** | **{:.2}%** |  |\n",
            metric.covered, metric.total, metric.percent
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
                "| `{}` | {} | {} | {} | {:.2}% |\n",
                path.display(),
                totals.covered,
                totals.total,
                missed,
                percent
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
            "| **Total** | **{}** | **{}** | **{}** | **{:.2}%** |\n",
            overall_covered, overall_total, overall_missed, overall_percent
        ));
        out.push('\n');
    }

    out
}

fn title_case(value: &str) -> String {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, path::PathBuf};

    use crate::model::{FileTotals, GateResult, GateRule, MetricKind, RuleOutcome};

    use super::render;

    #[test]
    fn renders_markdown_tables() {
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
                totals_by_file: BTreeMap::from([(
                    PathBuf::from("src/lib.rs"),
                    FileTotals {
                        covered: 3,
                        total: 4,
                    },
                )]),
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
        assert!(rendered.contains("| Result | Rule | Observed | Configured |"));
        assert!(rendered.contains("| ❌FAIL | `fail-under-regions` | 50.00% | ≥ 90.00% |"));
        assert!(rendered.contains(
            "| File | Covered Changed Regions | Changed Regions | Coverage | Missed Changed Spans |"
        ));
        assert!(rendered.contains("| `src/lib.rs` | 1 | 2 | 50.00% |"));
        assert!(rendered.contains("| **Total** | **1** | **2** | **50.00%** |  |"));
        assert!(
            rendered.contains("| File | Covered Regions | Regions | Missed Regions | Coverage |")
        );
        assert!(rendered.contains("| `src/lib.rs` | 3 | 4 | 1 | 75.00% |"));
        assert!(rendered.contains("| **Total** | **3** | **4** | **1** | **75.00%** |"));
        assert!(rendered.contains("### Overall Coverage"));
        assert!(!rendered.contains("Informational only. Does not affect the gate result in v1."));
    }

    #[test]
    fn renders_all_nonzero_metrics_in_markdown_summary() {
        let result = GateResult {
            metrics: vec![
                crate::model::ComputedMetric {
                    metric: MetricKind::Region,
                    covered: 1,
                    total: 2,
                    percent: 50.0,
                    uncovered_changed_opportunities: Vec::new(),
                    changed_totals_by_file: BTreeMap::from([(
                        PathBuf::from("src/lib.rs"),
                        FileTotals {
                            covered: 1,
                            total: 2,
                        },
                    )]),
                    totals_by_file: BTreeMap::from([(
                        PathBuf::from("src/lib.rs"),
                        FileTotals {
                            covered: 3,
                            total: 4,
                        },
                    )]),
                },
                crate::model::ComputedMetric {
                    metric: MetricKind::Line,
                    covered: 2,
                    total: 2,
                    percent: 100.0,
                    uncovered_changed_opportunities: Vec::new(),
                    changed_totals_by_file: BTreeMap::from([(
                        PathBuf::from("src/lib.rs"),
                        FileTotals {
                            covered: 2,
                            total: 2,
                        },
                    )]),
                    totals_by_file: BTreeMap::from([(
                        PathBuf::from("src/lib.rs"),
                        FileTotals {
                            covered: 5,
                            total: 5,
                        },
                    )]),
                },
            ],
            rules: vec![RuleOutcome {
                rule: GateRule::Percent {
                    metric: MetricKind::Region,
                    minimum_percent: 90.0,
                },
                passed: false,
                observed_percent: 50.0,
                observed_uncovered_count: 0,
            }],
            passed: false,
        };

        let rendered = render(&result, "origin/main...HEAD");
        assert!(rendered.contains("#### Region"));
        assert!(rendered.contains("#### Line"));
        assert!(rendered.contains(
            "| File | Covered Changed Lines | Changed Lines | Coverage | Missed Changed Spans |"
        ));
        assert!(rendered.contains("| File | Covered Lines | Lines | Missed Lines | Coverage |"));
    }

    #[test]
    fn renders_rule_status_with_unicode_icons() {
        let result = GateResult {
            metrics: vec![crate::model::ComputedMetric {
                metric: MetricKind::Region,
                covered: 2,
                total: 2,
                percent: 100.0,
                uncovered_changed_opportunities: Vec::new(),
                changed_totals_by_file: BTreeMap::from([(
                    PathBuf::from("src/lib.rs"),
                    FileTotals {
                        covered: 2,
                        total: 2,
                    },
                )]),
                totals_by_file: BTreeMap::new(),
            }],
            rules: vec![
                RuleOutcome {
                    rule: GateRule::Percent {
                        metric: MetricKind::Region,
                        minimum_percent: 90.0,
                    },
                    passed: true,
                    observed_percent: 100.0,
                    observed_uncovered_count: 0,
                },
                RuleOutcome {
                    rule: GateRule::UncoveredCount {
                        metric: MetricKind::Region,
                        maximum_count: 0,
                    },
                    passed: false,
                    observed_percent: 100.0,
                    observed_uncovered_count: 1,
                },
            ],
            passed: false,
        };

        let rendered = render(&result, "origin/main...HEAD");
        assert!(rendered.contains("| ✅PASS | `fail-under-regions` | 100.00% | ≥ 90.00% |"));
        assert!(rendered.contains("| ❌FAIL | `fail-uncovered-regions` | 1 | ≤ 0 |"));
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
        assert!(rendered.contains("`5-6(2)`"));
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
            .find(|line| line.starts_with("| `src/lib.rs` |"))
            .expect("file row should exist");
        assert!(row.find("`48-48`").expect("48-48") < row.find("`102-102`").expect("102-102"));
    }
}
