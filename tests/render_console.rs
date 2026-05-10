mod helpers;

use covgate::model::{
    CheckResult, ComputedMetric, CoverageOpportunity, FileTotals, GateEvaluation, MetricKind,
    OpportunityKind, SourceSpan,
};
use covgate::render::console::render;
use helpers::{percent_outcome, uncovered_outcome};
use std::{collections::BTreeMap, path::PathBuf};

#[test]
fn renders_console_summary_minimal() {
    let result = CheckResult {
        gates: vec![GateEvaluation {
            label: None,
            rules: vec![percent_outcome(MetricKind::Region, 90.0, false, 50.0, 1, 2)],
            passed: false,
        }],
        changed_metrics: vec![ComputedMetric {
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
        overall_metrics: Vec::new(),
        passed: false,
    };

    let rendered = render(&result, "origin/main...HEAD");
    assert!(!rendered.contains("Diff Coverage: FAIL"));
    assert!(rendered.contains("src/lib.rs (50.00% region)"));
    assert!(rendered.contains("FAIL Regions:"));
    assert!(rendered.contains("50.00%"));
    assert!(rendered.contains("(1/2)"));
    assert!(rendered.contains("≱ 90.00%"));
}

#[test]
fn renders_zero_total_percent_rules_as_na() {
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
    assert!(rendered.contains("PASS Lines:"));
    assert!(rendered.contains("N/A"));
    assert!(rendered.contains("(0/0)"));
    assert!(!rendered.contains("100.00%"));
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
        overall_metrics: Vec::new(),
        passed: false,
    };

    let rendered = render(&result, "origin/main...HEAD");
    assert!(rendered.contains("5-6(2)"));
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
        overall_metrics: Vec::new(),
        passed: false,
    };

    let rendered = render(&result, "origin/main...HEAD");
    let spans_row = rendered
        .lines()
        .find(|line| line.contains("regions:"))
        .expect("spans row should exist");
    assert!(spans_row.find("48").expect("48") < spans_row.find("102").expect("102"));
}

#[test]
fn omits_non_gated_metrics_from_minimal_output() {
    let result = CheckResult {
        gates: vec![GateEvaluation {
            label: None,
            rules: vec![percent_outcome(MetricKind::Region, 90.0, false, 50.0, 1, 2)],
            passed: false,
        }],
        changed_metrics: vec![
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
        overall_metrics: Vec::new(),
        passed: false,
    };

    let rendered = render(&result, "origin/main...HEAD");
    assert!(rendered.contains("Regions:"));
    assert!(!rendered.contains("Lines:"));
}

#[test]
fn percent_rule_renders_compact_unaligned() {
    let result = CheckResult {
        gates: vec![GateEvaluation {
            label: None,
            rules: vec![percent_outcome(MetricKind::Region, 90.0, true, 100.0, 3, 3)],
            passed: true,
        }],
        changed_metrics: vec![ComputedMetric {
            metric: MetricKind::Region,
            covered: 3,
            total: 3,
            percent: 100.0,
            uncovered_changed_opportunities: Vec::new(),
            changed_totals_by_file: BTreeMap::new(),
            totals_by_file: BTreeMap::new(),
        }],
        overall_metrics: Vec::new(),
        passed: true,
    };

    let rendered = render(&result, "origin/main...HEAD");
    let summary_line = rendered
        .lines()
        .find(|line| line.contains("PASS"))
        .expect("summary line");
    assert_eq!(summary_line, "PASS Regions: 100.00% (3/3) ≥ 90.00%");
}

#[test]
fn uncovered_count_rule_renders_observed_count_not_percent() {
    let result = CheckResult {
        gates: vec![GateEvaluation {
            label: None,
            rules: vec![uncovered_outcome(MetricKind::Function, 5, true, 2)],
            passed: true,
        }],
        changed_metrics: vec![ComputedMetric {
            metric: MetricKind::Function,
            covered: 8,
            total: 10,
            percent: 80.0,
            uncovered_changed_opportunities: Vec::new(),
            changed_totals_by_file: BTreeMap::new(),
            totals_by_file: BTreeMap::new(),
        }],
        overall_metrics: Vec::new(),
        passed: true,
    };

    let rendered = render(&result, "origin/main...HEAD");
    let summary_line = rendered
        .lines()
        .find(|line| line.contains("PASS"))
        .expect("summary line");
    assert!(
        summary_line.contains("2 uncovered"),
        "should contain '2 uncovered', got: {summary_line}"
    );
    assert!(
        summary_line.contains("≤ 5"),
        "should contain '≤ 5', got: {summary_line}"
    );
    assert!(
        !summary_line.contains('%'),
        "should NOT contain percent, got: {summary_line}"
    );
    assert!(
        !summary_line.contains("(8/10)"),
        "should NOT contain covered/total, got: {summary_line}"
    );
}

#[test]
fn scoped_label_renders_compact_without_padding() {
    let result = CheckResult {
        gates: vec![GateEvaluation {
            label: Some("frontend".to_string()),
            rules: vec![percent_outcome(MetricKind::Line, 90.0, true, 100.0, 5, 5)],
            passed: true,
        }],
        changed_metrics: vec![ComputedMetric {
            metric: MetricKind::Line,
            covered: 5,
            total: 5,
            percent: 100.0,
            uncovered_changed_opportunities: Vec::new(),
            changed_totals_by_file: BTreeMap::new(),
            totals_by_file: BTreeMap::new(),
        }],
        overall_metrics: Vec::new(),
        passed: true,
    };

    let rendered = render(&result, "origin/main...HEAD");
    let summary_line = rendered
        .lines()
        .find(|line| line.contains("PASS"))
        .expect("summary line");
    assert_eq!(
        summary_line,
        "[frontend] PASS Lines: 100.00% (5/5) ≥ 90.00%"
    );
}
