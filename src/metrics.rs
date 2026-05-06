use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

use crate::model::{ChangedFile, ComputedMetric, CoverageReport, FileTotals, MetricKind};

pub fn compute_changed_metric(
    report: &CoverageReport,
    diff: &[ChangedFile],
    included_paths: &BTreeSet<PathBuf>,
    metric: MetricKind,
) -> anyhow::Result<ComputedMetric> {
    let global_totals_by_file = report
        .totals_by_file
        .get(&metric)
        .filter(|totals| totals.values().any(|file_totals| file_totals.total > 0))
        .ok_or_else(|| {
            anyhow::anyhow!(
                "requested metric {} is not available in the report",
                metric.as_str()
            )
        })?;
    let totals_by_file: BTreeMap<PathBuf, FileTotals> = global_totals_by_file
        .iter()
        .filter(|(path, _)| included_paths.contains(*path))
        .map(|(path, totals)| (path.clone(), totals.clone()))
        .collect();

    let mut covered = 0usize;
    let mut total = 0usize;
    let mut uncovered = Vec::new();
    let mut changed_totals_by_file: BTreeMap<PathBuf, FileTotals> = BTreeMap::new();

    let target_kind = metric.to_opportunity_kind();
    let named_only = matches!(metric, MetricKind::NamedFunction);

    for opportunity in &report.opportunities {
        if opportunity.kind != target_kind {
            continue;
        }
        if named_only && !opportunity.is_named_function.unwrap_or(false) {
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
