use std::{collections::BTreeMap, path::PathBuf};

use crate::model::{ChangedFile, ComputedMetric, CoverageReport, FileTotals, MetricKind};

pub fn compute_changed_metric(
    report: &CoverageReport,
    diff: &[ChangedFile],
    metric: MetricKind,
) -> anyhow::Result<ComputedMetric> {
    let totals_by_file = report
        .totals_by_file
        .get(&metric)
        .filter(|totals| totals.values().any(|file_totals| file_totals.total > 0))
        .ok_or_else(|| {
            anyhow::anyhow!(
                "requested metric {} is not available in the report",
                metric.as_str()
            )
        })?;

    let mut covered = 0usize;
    let mut total = 0usize;
    let mut uncovered = Vec::new();
    let mut changed_totals_by_file: BTreeMap<PathBuf, FileTotals> = BTreeMap::new();

    let target_kind = metric.to_opportunity_kind();
    for opportunity in &report.opportunities {
        if opportunity.kind != target_kind {
            continue;
        }
        let changed = diff.iter().any(|file| {
            file.path == opportunity.span.path
                && file
                    .changed_lines
                    .iter()
                    .any(|range| opportunity.span.overlaps_line_range(range.start, range.end))
        });
        if !changed {
            continue;
        }
        total += 1;
        let entry = changed_totals_by_file
            .entry(opportunity.span.path.clone())
            .or_insert(FileTotals {
                covered: 0,
                total: 0,
            });
        entry.total += 1;
        if opportunity.covered {
            covered += 1;
            entry.covered += 1;
        } else {
            uncovered.push(opportunity.clone());
        }
    }

    let percent = if total == 0 {
        100.0
    } else {
        (covered as f64 / total as f64) * 100.0
    };

    Ok(ComputedMetric {
        metric,
        covered,
        total,
        percent,
        uncovered_changed_opportunities: uncovered,
        changed_totals_by_file,
        totals_by_file: totals_by_file.clone(),
    })
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, path::PathBuf};

    use crate::model::{
        ChangedFile, CoverageOpportunity, CoverageReport, FileTotals, LineRange, MetricKind,
        OpportunityKind, SourceSpan,
    };

    use super::compute_changed_metric;

    #[test]
    fn computes_changed_region_metric() {
        let report = CoverageReport {
            opportunities: vec![
                CoverageOpportunity {
                    kind: OpportunityKind::Region,
                    span: SourceSpan {
                        path: PathBuf::from("src/lib.rs"),
                        start_line: 2,
                        end_line: 3,
                    },
                    covered: true,
                },
                CoverageOpportunity {
                    kind: OpportunityKind::Region,
                    span: SourceSpan {
                        path: PathBuf::from("src/lib.rs"),
                        start_line: 5,
                        end_line: 6,
                    },
                    covered: false,
                },
            ],
            totals_by_file: BTreeMap::from([(
                MetricKind::Region,
                BTreeMap::from([(
                    PathBuf::from("src/lib.rs"),
                    FileTotals {
                        covered: 1,
                        total: 2,
                    },
                )]),
            )]),
        };
        let diff = vec![ChangedFile {
            path: PathBuf::from("src/lib.rs"),
            changed_lines: vec![LineRange { start: 1, end: 6 }],
        }];

        let metric =
            compute_changed_metric(&report, &diff, MetricKind::Region).expect("metric works");
        assert_eq!(metric.covered, 1);
        assert_eq!(metric.total, 2);
        assert_eq!(metric.uncovered_changed_opportunities.len(), 1);
        let file_totals = metric
            .changed_totals_by_file
            .get(&PathBuf::from("src/lib.rs"))
            .expect("changed totals by file");
        assert_eq!(file_totals.covered, 1);
        assert_eq!(file_totals.total, 2);
    }

    #[test]
    fn metric_with_only_zero_totals_is_treated_as_unavailable() {
        let report = CoverageReport {
            opportunities: Vec::new(),
            totals_by_file: BTreeMap::from([(
                MetricKind::Branch,
                BTreeMap::from([(
                    PathBuf::from("src/lib.rs"),
                    FileTotals {
                        covered: 0,
                        total: 0,
                    },
                )]),
            )]),
        };

        let error = compute_changed_metric(&report, &[], MetricKind::Branch)
            .expect_err("branch metric with only zero totals should be unavailable");

        assert_eq!(
            error.to_string(),
            "requested metric branch is not available in the report"
        );
    }
}
