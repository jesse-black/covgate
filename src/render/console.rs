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
    out.push_str(&format!("Metric: {}\n", result.metric.as_str()));
    out.push_str("-------------\n");

    let grouped = group_spans(
        &result
            .uncovered_changed_opportunities
            .iter()
            .map(|o| &o.span)
            .collect::<Vec<_>>(),
    );
    for (path, totals) in &result.changed_totals_by_file {
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
                "{path_display} ({percent:.2}%): uncovered changed spans {}\n",
                spans.missed.join(", ")
            ));
        } else {
            out.push_str(&format!("{path_display} ({percent:.2}%)\n"));
        }
    }

    out.push_str("-------------\n");
    out.push_str(&format!(
        "Changed {}: {}\n",
        result.metric.label(),
        result.total
    ));
    out.push_str(&format!(
        "Covered {}: {}\n",
        result.metric.label(),
        result.covered
    ));
    out.push_str(&format!("Coverage: {:.2}%\n", result.percent));
    out.push_str(&format!(
        "Threshold: {:.2}%\n",
        result.threshold.minimum_percent
    ));
    out.push_str("-------------");
    out
}

struct FileSummary {
    missed: Vec<String>,
}

fn group_spans(spans: &[&SourceSpan]) -> BTreeMap<String, FileSummary> {
    let mut grouped = BTreeMap::new();
    for span in spans {
        let entry = grouped
            .entry(span.path.display().to_string())
            .or_insert(FileSummary { missed: Vec::new() });
        entry
            .missed
            .push(format!("{}-{}", span.start_line, span.end_line));
    }
    grouped
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, path::PathBuf};

    use crate::model::{FileTotals, GateResult, MetricKind, Threshold};

    use super::render;

    #[test]
    fn renders_console_summary() {
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
            totals_by_file: BTreeMap::new(),
        };

        let rendered = render(&result, "origin/main...HEAD");
        assert!(rendered.contains("Diff Coverage: FAIL"));
        assert!(rendered.contains("src/lib.rs (50.00%)"));
        assert!(rendered.contains("Threshold: 90.00%"));
    }
}
