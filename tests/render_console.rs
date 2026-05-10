mod helpers;

use covgate::model::{
    ComputedMetric, CoverageOpportunity, FileTotals, MetricKind, OpportunityKind, SourceSpan,
};
use covgate::render::console::render;
use helpers::{percent_outcome, single_scope_result, uncovered_outcome};
use std::{collections::BTreeMap, path::PathBuf};

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
                named_function_identity: None,
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
        vec![percent_outcome(MetricKind::Region, 90.0, false, 50.0, 1, 2)],
        false,
    );

    let rendered = render(&result, "origin/main...HEAD");
    assert!(!rendered.contains("Diff Coverage: FAIL"));
    assert!(rendered.contains("src/lib.rs (50.00% region)"));
    assert!(rendered.contains("FAIL  Regions:"));
    assert!(rendered.contains("50.00%"));
    assert!(rendered.contains("(1/2)"));
    assert!(rendered.contains("≱ 90.00%"));
}

#[test]
fn renders_zero_total_percent_rules_as_na() {
    let result = single_scope_result(
        vec![ComputedMetric {
            metric: MetricKind::Line,
            covered: 0,
            total: 0,
            percent: 100.0,
            uncovered_changed_opportunities: Vec::new(),
            changed_totals_by_file: BTreeMap::new(),
            totals_by_file: BTreeMap::new(),
        }],
        vec![percent_outcome(MetricKind::Line, 80.0, true, 100.0, 0, 0)],
        true,
    );

    let rendered = render(&result, "origin/main...HEAD");
    assert!(rendered.contains("PASS  Lines:"));
    assert!(rendered.contains("N/A"));
    assert!(rendered.contains("(0/0)"));
    assert!(!rendered.contains("100.00%"));
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
                    named_function_identity: None,
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
        vec![percent_outcome(
            MetricKind::Region,
            90.0,
            false,
            33.33,
            1,
            3,
        )],
        false,
    );

    let rendered = render(&result, "origin/main...HEAD");
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
                    named_function_identity: None,
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
        vec![percent_outcome(
            MetricKind::Region,
            90.0,
            false,
            33.33,
            1,
            3,
        )],
        false,
    );

    let rendered = render(&result, "origin/main...HEAD");
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
        vec![percent_outcome(MetricKind::Region, 90.0, false, 50.0, 1, 2)],
        false,
    );

    let rendered = render(&result, "origin/main...HEAD");
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
            percent_outcome(MetricKind::Region, 90.0, false, 10.0, 100, 1000),
            uncovered_outcome(MetricKind::Function, 0, true, 0),
        ],
        false,
    );

    let rendered = render(&result, "diff");
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
