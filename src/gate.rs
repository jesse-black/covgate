use anyhow::Result;

use crate::model::{ComputedMetric, GateResult, GateRule, RuleOutcome};

pub fn evaluate(metrics: Vec<ComputedMetric>, rules: &[GateRule]) -> Result<GateResult> {
    let mut outcomes = Vec::new();
    let mut all_passed = true;

    for rule in rules {
        let metric = metrics
            .iter()
            .find(|m| m.metric == rule.metric())
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "configured rule for {} is not supported by the loaded report",
                    rule.metric().as_str()
                )
            })?;

        let rule_passed = match rule {
            GateRule::Percent {
                minimum_percent, ..
            } => metric.percent + f64::EPSILON >= *minimum_percent,
            GateRule::UncoveredCount { maximum_count, .. } => {
                metric.uncovered_changed_opportunities.len() <= *maximum_count
            }
        };

        if !rule_passed {
            all_passed = false;
        }

        outcomes.push(RuleOutcome {
            rule: rule.clone(),
            passed: rule_passed,
            observed_percent: metric.percent,
            observed_uncovered_count: metric.uncovered_changed_opportunities.len(),
        });
    }

    Ok(GateResult {
        metrics,
        rules: outcomes,
        passed: all_passed,
    })
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, path::PathBuf};

    use crate::model::{ComputedMetric, GateRule, MetricKind};

    use super::evaluate;

    #[test]
    fn fails_below_percent_threshold() {
        let result = evaluate(
            vec![ComputedMetric {
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
            }],
            &[GateRule::Percent {
                metric: MetricKind::Region,
                minimum_percent: 90.0,
            }],
        )
        .expect("evaluate should succeed");

        assert!(!result.passed);
        assert!(!result.rules[0].passed);
    }

    #[test]
    fn fails_above_uncovered_count_threshold() {
        let result = evaluate(
            vec![ComputedMetric {
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
                            start_line: 10,
                            end_line: 11,
                        },
                        covered: false,
                    },
                ],
                changed_totals_by_file: BTreeMap::new(),
                totals_by_file: BTreeMap::new(),
            }],
            &[GateRule::UncoveredCount {
                metric: MetricKind::Region,
                maximum_count: 1,
            }],
        )
        .expect("evaluate should succeed");

        assert!(!result.passed);
        assert!(!result.rules[0].passed);
    }

    #[test]
    fn multiple_rules_fail_if_any_fails() {
        let result = evaluate(
            vec![ComputedMetric {
                metric: MetricKind::Region,
                covered: 9,
                total: 10,
                percent: 90.0,
                uncovered_changed_opportunities: vec![crate::model::CoverageOpportunity {
                    kind: crate::model::OpportunityKind::Region,
                    span: crate::model::SourceSpan {
                        path: PathBuf::from("src/lib.rs"),
                        start_line: 5,
                        end_line: 6,
                    },
                    covered: false,
                }],
                changed_totals_by_file: BTreeMap::new(),
                totals_by_file: BTreeMap::new(),
            }],
            &[
                GateRule::Percent {
                    metric: MetricKind::Region,
                    minimum_percent: 80.0,
                },
                GateRule::UncoveredCount {
                    metric: MetricKind::Region,
                    maximum_count: 0,
                },
            ],
        )
        .expect("evaluate should succeed");

        assert!(!result.passed);
        assert!(result.rules[0].passed);
        assert!(!result.rules[1].passed);
    }

    #[test]
    fn mismatched_metric_returns_error() {
        let error = evaluate(
            vec![ComputedMetric {
                metric: MetricKind::Region,
                covered: 1,
                total: 2,
                percent: 50.0,
                uncovered_changed_opportunities: Vec::new(),
                changed_totals_by_file: BTreeMap::new(),
                totals_by_file: BTreeMap::new(),
            }],
            &[GateRule::Percent {
                metric: MetricKind::Line,
                minimum_percent: 90.0,
            }],
        )
        .expect_err("should return error");

        assert!(
            error
                .to_string()
                .contains("not supported by the loaded report")
        );
    }
}
