use covgate::model::{
    ComputedMetric, FileTotals, GateResult, GateRule, GateScopeResult, MetricKind, OpportunityKind,
    RuleOutcome, SourceSpan,
};
use covgate::render::markdown::render;
use std::{collections::BTreeMap, path::PathBuf};

fn single_scope_result(
    metrics: Vec<ComputedMetric>,
    rules: Vec<RuleOutcome>,
    passed: bool,
) -> GateResult {
    GateResult {
        scopes: vec![GateScopeResult {
            label: None,
            metrics,
            rules,
            passed,
        }],
        passed,
    }
}

#[test]
fn renders_markdown_tables() {
    let result = single_scope_result(
        vec![ComputedMetric {
            metric: MetricKind::Region,
            covered: 1,
            total: 2,
            percent: 50.0,
            uncovered_changed_opportunities: vec![covgate::model::CoverageOpportunity {
                kind: OpportunityKind::Region,
                span: SourceSpan {
                    path: PathBuf::from("src/lib.rs"),
                    start_line: 5,
                    end_line: 6,
                    start_col: None,
                    end_col: None,
                },
                covered: false,
                is_named_function: None,
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
        vec![RuleOutcome {
            rule: GateRule::Percent {
                metric: MetricKind::Region,
                minimum_percent: 90.0,
            },
            passed: false,
            observed_percent: 50.0,
            observed_uncovered_count: 1,
        }],
        false,
    );

    let rendered = render(&result, "origin/main...HEAD");
    assert!(rendered.contains("| Result | Rule | Observed | Configured |"));
    assert!(rendered.contains("| ❌FAIL | `fail-under-regions` | 50.00% | ≥ 90.00% |"));
    assert!(rendered.contains(
        "| File | Covered Changed Regions | Changed Regions | Coverage | Missed Changed Spans |"
    ));
    assert!(rendered.contains("| `src/lib.rs` | 1 | 2 | 50.00% 🟡 |"));
    assert!(rendered.contains("| **Total** | **1** | **2** | **50.00% 🟡** |  |"));
    assert!(rendered.contains("| File | Covered Regions | Regions | Missed Regions | Coverage |"));
    assert!(rendered.contains("| `src/lib.rs` | 3 | 4 | 1 | 75.00% 🟡 |"));
    assert!(rendered.contains("| **Total** | **3** | **4** | **1** | **75.00% 🟡** |"));
    assert!(rendered.contains("### Overall Coverage"));
    assert!(!rendered.contains("Informational only. Does not affect the gate result in v1."));
}

#[test]
fn renders_all_nonzero_metrics_in_markdown_summary() {
    let result = single_scope_result(
        vec![
            ComputedMetric {
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
            ComputedMetric {
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
        vec![RuleOutcome {
            rule: GateRule::Percent {
                metric: MetricKind::Region,
                minimum_percent: 90.0,
            },
            passed: false,
            observed_percent: 50.0,
            observed_uncovered_count: 0,
        }],
        false,
    );

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
    let result = single_scope_result(
        vec![ComputedMetric {
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
        vec![
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
        false,
    );

    let rendered = render(&result, "origin/main...HEAD");
    assert!(rendered.contains("| ✅PASS | `fail-under-regions` | 100.00% | ≥ 90.00% |"));
    assert!(rendered.contains("| ❌FAIL | `fail-uncovered-regions` | 1 | ≤ 0 |"));
}

#[test]
fn renders_coverage_with_threshold_circles() {
    let result = single_scope_result(
        vec![ComputedMetric {
            metric: MetricKind::Region,
            covered: 1,
            total: 3,
            percent: 33.33,
            uncovered_changed_opportunities: Vec::new(),
            changed_totals_by_file: BTreeMap::from([
                (
                    PathBuf::from("src/red.rs"),
                    FileTotals {
                        covered: 0,
                        total: 3,
                    },
                ),
                (
                    PathBuf::from("src/yellow.rs"),
                    FileTotals {
                        covered: 1,
                        total: 2,
                    },
                ),
                (
                    PathBuf::from("src/green.rs"),
                    FileTotals {
                        covered: 4,
                        total: 4,
                    },
                ),
            ]),
            totals_by_file: BTreeMap::from([
                (
                    PathBuf::from("src/red.rs"),
                    FileTotals {
                        covered: 2,
                        total: 5,
                    },
                ),
                (
                    PathBuf::from("src/yellow.rs"),
                    FileTotals {
                        covered: 3,
                        total: 4,
                    },
                ),
                (
                    PathBuf::from("src/green.rs"),
                    FileTotals {
                        covered: 5,
                        total: 5,
                    },
                ),
            ]),
        }],
        vec![],
        true,
    );

    let rendered = render(&result, "origin/main...HEAD");
    assert!(rendered.contains("| `src/red.rs` | 0 | 3 | 0.00% 🔴 |"));
    assert!(rendered.contains("| `src/yellow.rs` | 1 | 2 | 50.00% 🟡 |"));
    assert!(rendered.contains("| `src/green.rs` | 4 | 4 | 100.00% 🟢 |"));
    assert!(rendered.contains("| `src/red.rs` | 2 | 5 | 3 | 40.00% 🔴 |"));
    assert!(rendered.contains("| `src/yellow.rs` | 3 | 4 | 1 | 75.00% 🟡 |"));
    assert!(rendered.contains("| `src/green.rs` | 5 | 5 | 0 | 100.00% 🟢 |"));
}

#[test]
fn groups_duplicate_spans_with_counts() {
    let result = single_scope_result(
        vec![ComputedMetric {
            metric: MetricKind::Region,
            covered: 1,
            total: 3,
            percent: 33.33,
            uncovered_changed_opportunities: vec![
                covgate::model::CoverageOpportunity {
                    kind: OpportunityKind::Region,
                    span: SourceSpan {
                        path: PathBuf::from("src/lib.rs"),
                        start_line: 5,
                        end_line: 6,
                        start_col: None,
                        end_col: None,
                    },
                    covered: false,
                    is_named_function: None,
                },
                covgate::model::CoverageOpportunity {
                    kind: OpportunityKind::Region,
                    span: SourceSpan {
                        path: PathBuf::from("src/lib.rs"),
                        start_line: 5,
                        end_line: 6,
                        start_col: None,
                        end_col: None,
                    },
                    covered: false,
                    is_named_function: None,
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
        vec![RuleOutcome {
            rule: GateRule::Percent {
                metric: MetricKind::Region,
                minimum_percent: 90.0,
            },
            passed: false,
            observed_percent: 33.33,
            observed_uncovered_count: 2,
        }],
        false,
    );

    let rendered = render(&result, "origin/main...HEAD");
    assert!(rendered.contains("`5-6(2)`"));
}

#[test]
fn sorts_spans_numerically() {
    let result = single_scope_result(
        vec![ComputedMetric {
            metric: MetricKind::Region,
            covered: 1,
            total: 3,
            percent: 33.33,
            uncovered_changed_opportunities: vec![
                covgate::model::CoverageOpportunity {
                    kind: OpportunityKind::Region,
                    span: SourceSpan {
                        path: PathBuf::from("src/lib.rs"),
                        start_line: 102,
                        end_line: 102,
                        start_col: None,
                        end_col: None,
                    },
                    covered: false,
                    is_named_function: None,
                },
                covgate::model::CoverageOpportunity {
                    kind: OpportunityKind::Region,
                    span: SourceSpan {
                        path: PathBuf::from("src/lib.rs"),
                        start_line: 48,
                        end_line: 48,
                        start_col: None,
                        end_col: None,
                    },
                    covered: false,
                    is_named_function: None,
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
        vec![RuleOutcome {
            rule: GateRule::Percent {
                metric: MetricKind::Region,
                minimum_percent: 90.0,
            },
            passed: false,
            observed_percent: 33.33,
            observed_uncovered_count: 2,
        }],
        false,
    );

    let rendered = render(&result, "origin/main...HEAD");
    let row = rendered
        .lines()
        .find(|line| line.starts_with("| `src/lib.rs` |"))
        .expect("file row should exist");
    assert!(row.find("`48`").expect("48") < row.find("`102`").expect("102"));
}
