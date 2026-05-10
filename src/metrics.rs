use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

use crate::model::{
    ChangedFile, ComputedMetric, CoverageOpportunity, CoverageReport, FileTotals, MetricKind,
};

pub fn compute_changed_metric(
    report: &CoverageReport,
    diff: &[ChangedFile],
    included_paths: &BTreeSet<PathBuf>,
    metric: MetricKind,
) -> anyhow::Result<ComputedMetric> {
    if matches!(metric, MetricKind::NamedFunction) {
        return compute_changed_named_function_metric(report, diff, included_paths);
    }

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

    let mut covered = 0usize;
    let mut total = 0usize;
    let mut uncovered = Vec::new();
    let mut changed_totals_by_file: BTreeMap<PathBuf, FileTotals> = BTreeMap::new();

    let target_kind = metric.to_opportunity_kind();

    for opportunity in &report.opportunities {
        if opportunity.kind != target_kind {
            continue;
        }
        if !included_paths.contains(&opportunity.span.path) {
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
        totals_by_file: global_totals_by_file.clone(),
    })
}

fn compute_changed_named_function_metric(
    report: &CoverageReport,
    diff: &[ChangedFile],
    included_paths: &BTreeSet<PathBuf>,
) -> anyhow::Result<ComputedMetric> {
    let global_totals_by_file = report
        .totals_by_file
        .get(&MetricKind::NamedFunction)
        .filter(|totals| totals.values().any(|file_totals| file_totals.total > 0))
        .ok_or_else(|| {
            anyhow::anyhow!(
                "requested metric {} is not available in the report",
                MetricKind::NamedFunction.as_str()
            )
        })?;

    let mut changed_groups: BTreeMap<(PathBuf, String), NamedFunctionGroup> = BTreeMap::new();
    for opportunity in &report.opportunities {
        if opportunity.kind != MetricKind::NamedFunction.to_opportunity_kind() {
            continue;
        }
        let Some(identity) = opportunity.named_function_identity.as_ref() else {
            continue;
        };
        if !included_paths.contains(&opportunity.span.path) {
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

        let entry = changed_groups
            .entry((opportunity.span.path.clone(), identity.clone()))
            .or_insert_with(|| NamedFunctionGroup {
                covered: false,
                representative: opportunity.clone(),
            });
        entry.covered |= opportunity.covered;
        if opportunity.span.key() < entry.representative.span.key() {
            entry.representative = opportunity.clone();
        }
    }

    let mut covered = 0usize;
    let total = changed_groups.len();
    let mut uncovered = Vec::new();
    let mut changed_totals_by_file: BTreeMap<PathBuf, FileTotals> = BTreeMap::new();
    for ((path, _identity), group) in changed_groups {
        let entry = changed_totals_by_file.entry(path).or_insert(FileTotals {
            covered: 0,
            total: 0,
        });
        entry.total += 1;
        if group.covered {
            covered += 1;
            entry.covered += 1;
        } else {
            uncovered.push(group.representative);
        }
    }

    let percent = if total == 0 {
        100.0
    } else {
        (covered as f64 / total as f64) * 100.0
    };

    Ok(ComputedMetric {
        metric: MetricKind::NamedFunction,
        covered,
        total,
        percent,
        uncovered_changed_opportunities: uncovered,
        changed_totals_by_file,
        totals_by_file: global_totals_by_file.clone(),
    })
}

struct NamedFunctionGroup {
    covered: bool,
    representative: CoverageOpportunity,
}

pub fn compute_overall_metric(
    report: &CoverageReport,
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

    for file_totals in totals_by_file.values() {
        covered += file_totals.covered;
        total += file_totals.total;
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
        uncovered_changed_opportunities: Vec::new(),
        changed_totals_by_file: BTreeMap::new(),
        totals_by_file: totals_by_file.clone(),
    })
}
