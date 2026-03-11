use crate::model::GateResult;

pub fn render(result: &GateResult, _diff_description: &str) -> String {
    let mut out = String::new();
    out.push_str("## Covgate\n\n");
    out.push_str("### Diff Coverage\n\n");
    out.push_str("| Result | Metric | Changed Coverage | Threshold |\n");
    out.push_str("| --- | --- | ---: | ---: |\n");
    out.push_str(&format!(
        "| {} | {} | {:.2}% | {:.2}% |\n\n",
        if result.passed { "PASS" } else { "FAIL" },
        result.metric.as_str(),
        result.percent,
        result.threshold.minimum_percent
    ));
    out.push_str("| File | Covered Changed Opportunities | Total Changed Opportunities | Cover | Missed Changed Spans |\n");
    out.push_str("| --- | ---: | ---: | ---: | --- |\n");
    let mut missed_by_file =
        std::collections::BTreeMap::<String, std::collections::BTreeMap<String, usize>>::new();
    for opportunity in &result.uncovered_changed_opportunities {
        missed_by_file
            .entry(opportunity.span.path.display().to_string())
            .or_default()
            .entry(format!(
                "{}-{}",
                opportunity.span.start_line, opportunity.span.end_line
            ))
            .and_modify(|count| *count += 1)
            .or_insert(1);
    }
    for (path, totals) in &result.changed_totals_by_file {
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
                    .map(|(label, count)| {
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
    out.push_str("\n### Overall Coverage\n\n");
    out.push_str("Informational only. Does not affect the gate result in v1.\n\n");
    out.push_str("| File | Covered Opportunities | Total Opportunities | Cover |\n");
    out.push_str("| --- | ---: | ---: | ---: |\n");
    for (path, totals) in &result.totals_by_file {
        let percent = if totals.total == 0 {
            100.0
        } else {
            (totals.covered as f64 / totals.total as f64) * 100.0
        };
        out.push_str(&format!(
            "| `{}` | {} | {} | {:.2}% |\n",
            path.display(),
            totals.covered,
            totals.total,
            percent
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, path::PathBuf};

    use crate::model::{FileTotals, GateResult, MetricKind, Threshold};

    use super::render;

    #[test]
    fn renders_markdown_tables() {
        let result = GateResult {
            metric: MetricKind::Region,
            covered: 1,
            total: 2,
            percent: 50.0,
            threshold: Threshold {
                metric: MetricKind::Region,
                minimum_percent: 90.0,
            },
            passed: false,
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
        };

        let rendered = render(&result, "origin/main...HEAD");
        assert!(rendered.contains("| Result | Metric | Changed Coverage | Threshold |"));
        assert!(rendered.contains("| `src/lib.rs` | 1 | 2 | 50.00% |"));
        assert!(rendered.contains("### Overall Coverage"));
    }

    #[test]
    fn groups_duplicate_spans_with_counts() {
        let result = GateResult {
            metric: MetricKind::Region,
            covered: 1,
            total: 3,
            percent: 33.33,
            threshold: Threshold {
                metric: MetricKind::Region,
                minimum_percent: 90.0,
            },
            passed: false,
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
        };

        let rendered = render(&result, "origin/main...HEAD");
        assert!(rendered.contains("`5-6(2)`"));
    }
}
