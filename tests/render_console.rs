mod helpers;

use covgate::model::{
    CheckResult, ComputedMetric, CoverageOpportunity, FileTotals, GateEvaluation, MetricKind,
    OpportunityKind, RuleOutcome, SourceSpan, Verbosity,
};
use covgate::render::console::render;
use helpers::{percent_outcome, uncovered_outcome};
use std::{collections::BTreeMap, path::PathBuf};

fn single_scope_result(
    metrics: Vec<ComputedMetric>,
    rules: Vec<RuleOutcome>,
    passed: bool,
) -> CheckResult {
    CheckResult {
        gates: vec![GateEvaluation {
            label: None,
            rules,
            passed,
        }],
        gate_metrics: vec![covgate::model::GateMetricEvidence {
            metrics: metrics.clone(),
        }],
        changed_metrics: metrics.clone(),
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

    let rendered = render(&result, "origin/main...HEAD", Verbosity::Normal);
    assert!(!rendered.contains("Diff Coverage: FAIL"));
    assert!(rendered.contains("src/lib.rs (50.00% region)"));
    assert!(rendered.contains("FAIL  Regions:"));
    assert!(rendered.contains("50.00%"));
    assert!(rendered.contains("(1/2)"));
    assert!(rendered.contains("≱ 90.00%"));
}

fn line_metric(path: &str, covered: usize, total: usize, missed_line: u32) -> ComputedMetric {
    ComputedMetric {
        metric: MetricKind::Line,
        covered,
        total,
        percent: (covered as f64 / total as f64) * 100.0,
        uncovered_changed_opportunities: vec![CoverageOpportunity {
            kind: OpportunityKind::Line,
            span: SourceSpan {
                path: PathBuf::from(path),
                start_line: missed_line,
                end_line: missed_line,
                start_col: None,
                end_col: None,
            },
            covered: false,
            is_named_function: None,
            named_function_identity: None,
        }],
        changed_totals_by_file: BTreeMap::from([(
            PathBuf::from(path),
            FileTotals { covered, total },
        )]),
        totals_by_file: BTreeMap::new(),
    }
}

fn multi_gate_line_result() -> CheckResult {
    let ui_metric = line_metric("ui/button.tsx", 1, 2, 12);
    let backend_metric = line_metric("server/api.ts", 1, 2, 40);
    CheckResult {
        gates: vec![
            GateEvaluation {
                label: Some("ui".to_string()),
                rules: vec![percent_outcome(MetricKind::Line, 75.0, false, 50.0, 1, 2)],
                passed: false,
            },
            GateEvaluation {
                label: Some("backend".to_string()),
                rules: vec![percent_outcome(MetricKind::Line, 50.0, true, 50.0, 1, 2)],
                passed: true,
            },
        ],
        gate_metrics: vec![
            covgate::model::GateMetricEvidence {
                metrics: vec![ui_metric.clone()],
            },
            covgate::model::GateMetricEvidence {
                metrics: vec![backend_metric.clone()],
            },
        ],
        changed_metrics: vec![ui_metric, backend_metric],
        overall_metrics: Vec::new(),
        passed: false,
    }
}

#[test]
fn verbose_falls_back_to_global_metrics_without_gate_evidence() {
    let mut result = multi_gate_line_result();
    result.gate_metrics = Vec::new();
    result.overall_metrics = vec![ComputedMetric {
        metric: MetricKind::Line,
        covered: 8,
        total: 10,
        percent: 80.0,
        uncovered_changed_opportunities: Vec::new(),
        changed_totals_by_file: BTreeMap::new(),
        totals_by_file: BTreeMap::new(),
    }];

    let rendered = render(&result, "origin/main...HEAD", Verbosity::Verbose);

    assert!(rendered.contains("ui/button.tsx"));
    assert!(rendered.contains("server/api.ts"));
    assert!(rendered.contains("Gate: ui"));
    assert!(rendered.contains("Gate: backend"));
    assert!(rendered.contains("Overall Coverage"));
    assert!(rendered.contains("80.00% (8/10)"));
}

#[test]
fn minimal_failures_fall_back_to_global_metrics_without_gate_evidence() {
    let mut result = multi_gate_line_result();
    result.gate_metrics = Vec::new();

    let rendered = render(&result, "origin/main...HEAD", Verbosity::Normal);
    let failure_details = rendered
        .split("[ui] FAIL")
        .next()
        .expect("failure details should precede rule summaries");

    assert!(failure_details.contains("ui/button.tsx"));
    assert!(failure_details.contains("server/api.ts"));
}

#[test]
fn verbose_renders_zero_total_file_details_as_full_coverage() {
    let result = single_scope_result(
        vec![ComputedMetric {
            metric: MetricKind::Line,
            covered: 0,
            total: 0,
            percent: 100.0,
            uncovered_changed_opportunities: Vec::new(),
            changed_totals_by_file: BTreeMap::from([(
                PathBuf::from("src/empty.rs"),
                FileTotals {
                    covered: 0,
                    total: 0,
                },
            )]),
            totals_by_file: BTreeMap::new(),
        }],
        vec![percent_outcome(MetricKind::Line, 80.0, true, 100.0, 0, 0)],
        true,
    );

    let rendered = render(&result, "origin/main...HEAD", Verbosity::Verbose);

    assert!(rendered.contains("src/empty.rs (100.00%) [line]"));
}

#[test]
fn minimal_failures_only_show_files_from_failing_gate() {
    let result = multi_gate_line_result();

    let rendered = render(&result, "origin/main...HEAD", Verbosity::Normal);
    let failure_details = rendered
        .split("[ui] FAIL")
        .next()
        .expect("failure details should precede rule summaries");

    assert!(failure_details.contains("ui/button.tsx"));
    assert!(!failure_details.contains("server/api.ts"));
}

#[test]
fn verbose_file_details_stay_under_their_gate_labels() {
    let result = multi_gate_line_result();

    let rendered = render(&result, "origin/main...HEAD", Verbosity::Verbose);
    let ui_section = rendered
        .split("Gate: ui\n")
        .nth(1)
        .expect("ui gate section should exist")
        .split("Gate: backend\n")
        .next()
        .expect("ui section should end before backend gate");
    let backend_section = rendered
        .split("Gate: backend\n")
        .nth(1)
        .expect("backend gate section should exist");

    assert!(ui_section.contains("ui/button.tsx"));
    assert!(!ui_section.contains("server/api.ts"));
    assert!(backend_section.contains("server/api.ts"));
    assert!(!backend_section.contains("ui/button.tsx"));
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

    let rendered = render(&result, "origin/main...HEAD", Verbosity::Normal);
    assert!(rendered.contains("PASS  Lines:"));
    assert!(rendered.contains("N/A"));
    assert!(rendered.contains("(0/0)"));
    assert!(!rendered.contains("100.00%"));
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
        vec![percent_outcome(MetricKind::Region, 90.0, false, 50.0, 1, 2)],
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
            percent_outcome(MetricKind::Region, 90.0, false, 10.0, 100, 1000),
            uncovered_outcome(MetricKind::Function, 0, true, 0),
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
