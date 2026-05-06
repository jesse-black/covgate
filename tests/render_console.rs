use covgate::model::{
    ComputedMetric, CoverageOpportunity, FileTotals, GateResult, GateRule, GateScopeResult,
    MetricKind, OpportunityKind, RuleOutcome, SourceSpan, Verbosity,
};
use covgate::render::console::render;
use std::{collections::BTreeMap, path::PathBuf};

fn single_scope_result(
    metrics: Vec<ComputedMetric>,
    rules: Vec<RuleOutcome>,
    passed: bool,
) -> GateResult {
    GateResult {
        scopes: vec![GateScopeResult {
            label: None,
            metrics: metrics.clone(),
            rules,
            passed,
        }],
        overall_metrics: metrics,
        passed,
    }
}

#[test]
fn renders_console_summary_minimal() {
    let result = single_scope_result(
        vec![ComputedMetric {
            metric: MetricKind::Region,
            covered: 1,
            total: 2,
            percent: 50.0,
            uncovered_changed_opportunities: vec![CoverageOpportunity {
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
            totals_by_file: BTreeMap::new(),
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

    let rendered = render(&result, "origin/main...HEAD", Verbosity::Normal);
    assert!(!rendered.contains("Diff Coverage: FAIL"));
    assert!(rendered.contains("src/lib.rs (50.00% region)"));
    assert!(rendered.contains("FAIL  Regions:      50.00%         (1/2)  ≱ 90.00%"));
}

#[test]
fn renders_console_summary_verbose() {
    let result = single_scope_result(
        vec![ComputedMetric {
            metric: MetricKind::Region,
            covered: 1,
            total: 2,
            percent: 50.0,
            uncovered_changed_opportunities: vec![CoverageOpportunity {
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
            totals_by_file: BTreeMap::new(),
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

    let rendered = render(&result, "origin/main...HEAD", Verbosity::Verbose);
    assert!(rendered.contains("Diff Coverage: FAIL"));
    assert!(rendered.contains("src/lib.rs (50.00%)"));
    assert!(rendered.contains("Rule fail-under-regions: FAIL (50.00% ≱ 90.00%)"));
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
                CoverageOpportunity {
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
                CoverageOpportunity {
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

    let rendered = render(&result, "origin/main...HEAD", Verbosity::Normal);
    assert!(rendered.contains("5-6(2)"));
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
                CoverageOpportunity {
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
                CoverageOpportunity {
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

    let rendered = render(&result, "origin/main...HEAD", Verbosity::Normal);
    let spans_row = rendered
        .lines()
        .find(|line| line.contains("regions:"))
        .expect("spans row should exist");
    assert!(spans_row.find("48").expect("48") < spans_row.find("102").expect("102"));
}

#[test]
fn omits_non_gated_metrics_from_minimal_output() {
    let result = single_scope_result(
        vec![
            ComputedMetric {
                metric: MetricKind::Region,
                covered: 1,
                total: 2,
                percent: 50.0,
                uncovered_changed_opportunities: Vec::new(),
                changed_totals_by_file: BTreeMap::new(),
                totals_by_file: BTreeMap::new(),
            },
            ComputedMetric {
                metric: MetricKind::Line,
                covered: 1,
                total: 1,
                percent: 100.0,
                uncovered_changed_opportunities: Vec::new(),
                changed_totals_by_file: BTreeMap::new(),
                totals_by_file: BTreeMap::new(),
            },
        ],
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

    let rendered = render(&result, "origin/main...HEAD", Verbosity::Normal);
    assert!(rendered.contains("Regions:"));
    assert!(!rendered.contains("Lines:"));
}

#[test]
fn aligns_comparators_vertically() {
    let result = single_scope_result(
        vec![
            ComputedMetric {
                metric: MetricKind::Region,
                covered: 100,
                total: 1000,
                percent: 10.0,
                uncovered_changed_opportunities: Vec::new(),
                changed_totals_by_file: BTreeMap::new(),
                totals_by_file: BTreeMap::new(),
            },
            ComputedMetric {
                metric: MetricKind::Function,
                covered: 1,
                total: 1,
                percent: 100.0,
                uncovered_changed_opportunities: Vec::new(),
                changed_totals_by_file: BTreeMap::new(),
                totals_by_file: BTreeMap::new(),
            },
        ],
        vec![
            RuleOutcome {
                rule: GateRule::Percent {
                    metric: MetricKind::Region,
                    minimum_percent: 90.0,
                },
                passed: false,
                observed_percent: 10.0,
                observed_uncovered_count: 900,
            },
            RuleOutcome {
                rule: GateRule::UncoveredCount {
                    metric: MetricKind::Function,
                    maximum_count: 0,
                },
                passed: true,
                observed_percent: 100.0,
                observed_uncovered_count: 0,
            },
        ],
        false,
    );

    let rendered = render(&result, "diff", Verbosity::Normal);
    let lines: Vec<_> = rendered
        .lines()
        .filter(|line| line.contains("PASS") || line.contains("FAIL"))
        .collect();
    assert_eq!(lines.len(), 2);

    let pos1 = lines[0]
        .chars()
        .position(|ch| ch == '≱' || ch == '≥')
        .expect("first comparator");
    let pos2 = lines[1]
        .chars()
        .position(|ch| ch == '≤' || ch == '≰')
        .expect("second comparator");
    assert_eq!(
        pos1, pos2,
        "Comparators should be at the same horizontal position.\nLine 1: {}\nLine 2: {}",
        lines[0], lines[1]
    );
}
