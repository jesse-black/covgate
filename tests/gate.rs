use std::{collections::BTreeMap, path::PathBuf};

use covgate::gate::evaluate;
use covgate::model::{ComputedMetric, GateRule, MetricKind};

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
                covgate::model::FileTotals {
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
                covgate::model::CoverageOpportunity {
                    kind: covgate::model::OpportunityKind::Region,
                    span: covgate::model::SourceSpan {
                        path: PathBuf::from("src/lib.rs"),
                        start_line: 5,
                        end_line: 6,
                        start_col: None,
                        end_col: None,
                    },
                    covered: false,
                },
                covgate::model::CoverageOpportunity {
                    kind: covgate::model::OpportunityKind::Region,
                    span: covgate::model::SourceSpan {
                        path: PathBuf::from("src/lib.rs"),
                        start_line: 10,
                        end_line: 11,
                        start_col: None,
                        end_col: None,
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
            uncovered_changed_opportunities: vec![covgate::model::CoverageOpportunity {
                kind: covgate::model::OpportunityKind::Region,
                span: covgate::model::SourceSpan {
                    path: PathBuf::from("src/lib.rs"),
                    start_line: 5,
                    end_line: 6,
                    start_col: None,
                    end_col: None,
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
