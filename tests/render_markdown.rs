mod helpers;

use covgate::model::{
    CheckResult, ComputedMetric, FileTotals, GateEvaluation, MetricKind, OpportunityKind,
    SourceSpan,
};
use covgate::render::markdown::render;
use helpers::{percent_outcome, uncovered_outcome};
use std::{collections::BTreeMap, path::PathBuf};

fn multi_scope_result(
    gates: Vec<GateEvaluation>,
    changed_metrics: Vec<ComputedMetric>,
    overall_metrics: Vec<ComputedMetric>,
) -> CheckResult {
    let passed = gates.iter().all(|gate| gate.passed);
    CheckResult {
        gates,
        changed_metrics,
        overall_metrics,
        passed,
    }
}

#[test]
fn renders_gate_column_for_single_labeled_scope() {
    let metric = ComputedMetric {
        metric: MetricKind::Line,
        covered: 3,
        total: 3,
        percent: 100.0,
        uncovered_changed_opportunities: Vec::new(),
        changed_totals_by_file: BTreeMap::from([(
            PathBuf::from("src/logic.ts"),
            FileTotals {
                covered: 3,
                total: 3,
            },
        )]),
        totals_by_file: BTreeMap::from([(
            PathBuf::from("src/logic.ts"),
            FileTotals {
                covered: 3,
                total: 3,
            },
        )]),
    };
    let result = multi_scope_result(
        vec![GateEvaluation {
            label: Some("logic".to_string()),
            rules: vec![percent_outcome(MetricKind::Line, 95.0, true, 100.0, 3, 3)],
            passed: true,
        }],
        vec![metric],
        Vec::new(),
    );

    let rendered = render(&result, "origin/main...HEAD");

    assert!(rendered.contains("| Gate | Result | Rule | Observed | Configured |"));
    assert!(
        rendered.contains("| `logic` | ✅PASS | `fail-under-lines` | 100.00% (3/3) | ≥ 95.00% |")
    );
    assert!(rendered.contains(
        "| File | Covered Changed Lines | Changed Lines | Coverage | Missed Changed Spans |"
    ));
    assert!(rendered.contains("| `src/logic.ts` | 3 | 3 | 100.00% 🟢 |  |"));
}

#[test]
fn renders_markdown_tables() {
    let metric = ComputedMetric {
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
            named_function_identity: None,
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
    let result = CheckResult {
        gates: vec![GateEvaluation {
            label: None,
            rules: vec![percent_outcome(MetricKind::Region, 90.0, false, 50.0, 1, 2)],
            passed: false,
        }],
        changed_metrics: vec![metric.clone()],
        overall_metrics: vec![metric],
        passed: false,
    };

    let rendered = render(&result, "origin/main...HEAD");
    assert!(rendered.contains("| Result | Rule | Observed | Configured |"));
    assert!(rendered.contains("| ❌FAIL | `fail-under-regions` | 50.00% (1/2) | ≥ 90.00% |"));
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
    let metrics = vec![
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
    ];
    let result = CheckResult {
        gates: vec![GateEvaluation {
            label: None,
            rules: vec![percent_outcome(MetricKind::Region, 90.0, false, 50.0, 1, 2)],
            passed: false,
        }],
        changed_metrics: metrics.clone(),
        overall_metrics: metrics,
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
fn renders_global_unlabeled_overall_coverage_for_multi_scope_results() {
    let ui_changed_region_metric = ComputedMetric {
        metric: MetricKind::Region,
        covered: 1,
        total: 2,
        percent: 50.0,
        uncovered_changed_opportunities: Vec::new(),
        changed_totals_by_file: BTreeMap::from([(
            PathBuf::from("ui/button.tsx"),
            FileTotals {
                covered: 1,
                total: 2,
            },
        )]),
        totals_by_file: BTreeMap::new(),
    };
    let backend_changed_region_metric = ComputedMetric {
        metric: MetricKind::Region,
        covered: 2,
        total: 2,
        percent: 100.0,
        uncovered_changed_opportunities: Vec::new(),
        changed_totals_by_file: BTreeMap::from([(
            PathBuf::from("server/api.ts"),
            FileTotals {
                covered: 2,
                total: 2,
            },
        )]),
        totals_by_file: BTreeMap::new(),
    };
    let overall_region_metric = ComputedMetric {
        metric: MetricKind::Region,
        covered: 0,
        total: 0,
        percent: 0.0,
        uncovered_changed_opportunities: Vec::new(),
        changed_totals_by_file: BTreeMap::new(),
        totals_by_file: BTreeMap::from([
            (
                PathBuf::from("server/api.ts"),
                FileTotals {
                    covered: 8,
                    total: 10,
                },
            ),
            (
                PathBuf::from("ui/button.tsx"),
                FileTotals {
                    covered: 3,
                    total: 5,
                },
            ),
            (
                PathBuf::from("shared/util.ts"),
                FileTotals {
                    covered: 4,
                    total: 4,
                },
            ),
        ]),
    };
    let overall_line_metric = ComputedMetric {
        metric: MetricKind::Line,
        covered: 0,
        total: 0,
        percent: 0.0,
        uncovered_changed_opportunities: Vec::new(),
        changed_totals_by_file: BTreeMap::new(),
        totals_by_file: BTreeMap::from([
            (
                PathBuf::from("server/api.ts"),
                FileTotals {
                    covered: 40,
                    total: 50,
                },
            ),
            (
                PathBuf::from("ui/button.tsx"),
                FileTotals {
                    covered: 12,
                    total: 20,
                },
            ),
            (
                PathBuf::from("shared/util.ts"),
                FileTotals {
                    covered: 16,
                    total: 16,
                },
            ),
        ]),
    };
    let result = multi_scope_result(
        vec![
            GateEvaluation {
                label: Some("js-ui".to_string()),
                rules: vec![percent_outcome(MetricKind::Region, 60.0, false, 50.0, 1, 2)],
                passed: false,
            },
            GateEvaluation {
                label: None,
                rules: vec![percent_outcome(MetricKind::Region, 90.0, true, 100.0, 1, 1)],
                passed: true,
            },
        ],
        vec![ui_changed_region_metric, backend_changed_region_metric],
        vec![overall_region_metric, overall_line_metric],
    );

    let rendered = render(&result, "origin/main...HEAD");
    let overall = rendered
        .split("### Overall Coverage\n\n")
        .nth(1)
        .expect("overall coverage section should exist");

    assert!(rendered.contains("| Gate | Result | Rule | Observed | Configured |"));
    assert!(
        rendered.contains("| `js-ui` | ❌FAIL | `fail-under-regions` | 50.00% (1/2) | ≥ 60.00% |")
    );
    assert!(
        rendered
            .contains("| `default` | ✅PASS | `fail-under-regions` | 100.00% (1/1) | ≥ 90.00% |")
    );
    assert!(overall.contains("#### Region"));
    assert!(overall.contains("#### Line"));
    assert!(overall.contains("| `shared/util.ts` | 4 | 4 | 0 | 100.00% 🟢 |"));
    assert!(overall.contains("| `shared/util.ts` | 16 | 16 | 0 | 100.00% 🟢 |"));
    assert!(!overall.contains("| Gate |"));
    assert!(!overall.contains("`js-ui`"));
    assert!(!overall.contains("`default`"));
}

#[test]
fn renders_rule_status_with_unicode_icons() {
    let result = CheckResult {
        gates: vec![GateEvaluation {
            label: None,
            rules: vec![
                percent_outcome(MetricKind::Region, 90.0, true, 100.0, 2, 2),
                uncovered_outcome(MetricKind::Region, 0, false, 1),
            ],
            passed: false,
        }],
        changed_metrics: vec![ComputedMetric {
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
        overall_metrics: Vec::new(),
        passed: false,
    };

    let rendered = render(&result, "origin/main...HEAD");
    assert!(rendered.contains("| ✅PASS | `fail-under-regions` | 100.00% (2/2) | ≥ 90.00% |"));
    assert!(rendered.contains("| ❌FAIL | `fail-uncovered-regions` | 1 | ≤ 0 |"));
}

#[test]
fn renders_zero_total_percent_rules_as_na_with_counts() {
    let result = CheckResult {
        gates: vec![GateEvaluation {
            label: None,
            rules: vec![percent_outcome(MetricKind::Line, 80.0, true, 100.0, 0, 0)],
            passed: true,
        }],
        changed_metrics: vec![ComputedMetric {
            metric: MetricKind::Line,
            covered: 0,
            total: 0,
            percent: 100.0,
            uncovered_changed_opportunities: Vec::new(),
            changed_totals_by_file: BTreeMap::new(),
            totals_by_file: BTreeMap::new(),
        }],
        overall_metrics: Vec::new(),
        passed: true,
    };

    let rendered = render(&result, "origin/main...HEAD");
    assert!(rendered.contains("| ✅PASS | `fail-under-lines` | N/A (0/0) | ≥ 80.00% |"));
    assert!(!rendered.contains("| ✅PASS | `fail-under-lines` | 100.00%"));
}

#[test]
fn renders_coverage_with_threshold_circles() {
    let metric = ComputedMetric {
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
    };
    let result = CheckResult {
        gates: vec![GateEvaluation {
            label: None,
            rules: Vec::new(),
            passed: true,
        }],
        changed_metrics: vec![metric.clone()],
        overall_metrics: vec![metric],
        passed: true,
    };

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
    let result = CheckResult {
        gates: vec![GateEvaluation {
            label: None,
            rules: vec![percent_outcome(
                MetricKind::Region,
                90.0,
                false,
                33.33,
                1,
                3,
            )],
            passed: false,
        }],
        changed_metrics: vec![ComputedMetric {
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
                    named_function_identity: None,
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
                    named_function_identity: None,
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
        overall_metrics: Vec::new(),
        passed: false,
    };

    let rendered = render(&result, "origin/main...HEAD");
    assert!(rendered.contains("`5-6(2)`"));
}

#[test]
fn sorts_spans_numerically() {
    let result = CheckResult {
        gates: vec![GateEvaluation {
            label: None,
            rules: vec![percent_outcome(
                MetricKind::Region,
                90.0,
                false,
                33.33,
                1,
                3,
            )],
            passed: false,
        }],
        changed_metrics: vec![ComputedMetric {
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
                    named_function_identity: None,
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
                    named_function_identity: None,
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
        overall_metrics: Vec::new(),
        passed: false,
    };

    let rendered = render(&result, "origin/main...HEAD");
    let row = rendered
        .lines()
        .find(|line| line.starts_with("| `src/lib.rs` |"))
        .expect("file row should exist");
    assert!(row.find("`48`").expect("48") < row.find("`102`").expect("102"));
}
