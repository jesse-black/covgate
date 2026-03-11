use crate::model::{ComputedMetric, GateResult, Threshold};

pub fn evaluate(metric: ComputedMetric, threshold: Threshold) -> GateResult {
    let passed = metric.percent + f64::EPSILON >= threshold.minimum_percent;
    GateResult {
        metric: metric.metric,
        covered: metric.covered,
        total: metric.total,
        percent: metric.percent,
        threshold,
        passed,
        uncovered_changed_opportunities: metric.uncovered_changed_opportunities,
        changed_totals_by_file: metric.changed_totals_by_file,
        totals_by_file: metric.totals_by_file,
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, path::PathBuf};

    use crate::model::{ComputedMetric, MetricKind, Threshold};

    use super::evaluate;

    #[test]
    fn fails_below_threshold() {
        let result = evaluate(
            ComputedMetric {
                metric: MetricKind::Region,
                covered: 1,
                total: 2,
                percent: 50.0,
                uncovered_changed_opportunities: Vec::new(),
                changed_totals_by_file: BTreeMap::new(),
                totals_by_file: BTreeMap::from([(
                    PathBuf::from("src/lib.rs"),
                    crate::model::FileTotals {
                        covered: 1,
                        total: 2,
                    },
                )]),
            },
            Threshold {
                metric: MetricKind::Region,
                minimum_percent: 90.0,
            },
        );

        assert!(!result.passed);
    }
}
