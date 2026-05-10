use covgate::coverage::parse_with_repo_root;
use covgate::metrics::compute_changed_metric;
use covgate::model::{
    ChangedFile, CoverageOpportunity, CoverageReport, FileTotals, LineRange, MetricKind,
    OpportunityKind, SourceSpan,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

#[test]
fn computes_changed_region_metric() {
    let report = CoverageReport {
        opportunities: vec![
            CoverageOpportunity {
                kind: OpportunityKind::Region,
                span: SourceSpan {
                    path: PathBuf::from("src/lib.rs"),
                    start_line: 2,
                    end_line: 3,
                    start_col: None,
                    end_col: None,
                },
                covered: true,
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
        totals_by_file: BTreeMap::from([(
            MetricKind::Region,
            BTreeMap::from([(
                PathBuf::from("src/lib.rs"),
                FileTotals {
                    covered: 1,
                    total: 2,
                },
            )]),
        )]),
    };
    let diff = vec![ChangedFile {
        path: PathBuf::from("src/lib.rs"),
        changed_lines: vec![LineRange { start: 1, end: 6 }],
    }];

    let included_paths = diff
        .iter()
        .map(|file| file.path.clone())
        .collect::<BTreeSet<_>>();
    let metric = compute_changed_metric(&report, &diff, &included_paths, MetricKind::Region)
        .expect("metric works");
    assert_eq!(metric.covered, 1);
    assert_eq!(metric.total, 2);
    assert_eq!(metric.uncovered_changed_opportunities.len(), 1);
    let file_totals = metric
        .changed_totals_by_file
        .get(&PathBuf::from("src/lib.rs"))
        .expect("changed totals by file");
    assert_eq!(file_totals.covered, 1);
    assert_eq!(file_totals.total, 2);
}

#[test]
fn metric_with_only_zero_totals_is_treated_as_unavailable() {
    let report = CoverageReport {
        opportunities: Vec::new(),
        totals_by_file: BTreeMap::from([(
            MetricKind::Branch,
            BTreeMap::from([(
                PathBuf::from("src/lib.rs"),
                FileTotals {
                    covered: 0,
                    total: 0,
                },
            )]),
        )]),
    };

    let error = compute_changed_metric(&report, &[], &BTreeSet::new(), MetricKind::Branch)
        .expect_err("branch metric with only zero totals should be unavailable");

    assert_eq!(
        error.to_string(),
        "requested metric branch is not available in the report"
    );
}

#[test]
fn changed_branch_metric_counts_multiline_vitest_branch_outcomes() {
    let report = parse_with_repo_root(
        include_str!("fixtures/vitest/empty-branch-locations/coverage.json"),
        std::path::Path::new("."),
    )
    .expect("fixture should parse");
    let diff = vec![ChangedFile {
        path: PathBuf::from("src/auth/authService.ts"),
        changed_lines: vec![LineRange { start: 11, end: 11 }],
    }];

    let included_paths = diff
        .iter()
        .map(|file| file.path.clone())
        .collect::<BTreeSet<_>>();
    let metric = compute_changed_metric(&report, &diff, &included_paths, MetricKind::Branch)
        .expect("metric works");

    assert_eq!(metric.covered, 1);
    assert_eq!(metric.total, 2);
}

#[test]
fn changed_line_metric_keeps_uncovered_fixture_seed_call_visible() {
    let report = parse_with_repo_root(
        include_str!("fixtures/vitest/empty-branch-locations/coverage.json"),
        std::path::Path::new("."),
    )
    .expect("fixture should parse");
    let diff = vec![ChangedFile {
        path: PathBuf::from("src/fixtures/fixtureSeed.ts"),
        changed_lines: vec![LineRange { start: 20, end: 20 }],
    }];

    let included_paths = diff
        .iter()
        .map(|file| file.path.clone())
        .collect::<BTreeSet<_>>();
    let metric = compute_changed_metric(&report, &diff, &included_paths, MetricKind::Line)
        .expect("metric works");

    assert_eq!(metric.covered, 0);
    assert_eq!(metric.total, 1);
    assert_eq!(metric.uncovered_changed_opportunities.len(), 1);
}

#[test]
fn changed_named_function_metric_collapses_template_instantiations() {
    let report = CoverageReport {
        opportunities: vec![
            CoverageOpportunity {
                kind: OpportunityKind::Function,
                span: SourceSpan {
                    path: PathBuf::from("src/lib.rs"),
                    start_line: 10,
                    end_line: 12,
                    start_col: Some(1),
                    end_col: Some(1),
                },
                covered: false,
                is_named_function: Some(true),
                named_function_identity: Some("covgate::metrics::parse".to_string()),
            },
            CoverageOpportunity {
                kind: OpportunityKind::Function,
                span: SourceSpan {
                    path: PathBuf::from("src/lib.rs"),
                    start_line: 20,
                    end_line: 22,
                    start_col: Some(1),
                    end_col: Some(1),
                },
                covered: true,
                is_named_function: Some(true),
                named_function_identity: Some("covgate::metrics::parse".to_string()),
            },
        ],
        totals_by_file: BTreeMap::from([(
            MetricKind::NamedFunction,
            BTreeMap::from([(
                PathBuf::from("src/lib.rs"),
                FileTotals {
                    covered: 1,
                    total: 1,
                },
            )]),
        )]),
    };
    let diff = vec![ChangedFile {
        path: PathBuf::from("src/lib.rs"),
        changed_lines: vec![LineRange { start: 10, end: 22 }],
    }];

    let included_paths = diff
        .iter()
        .map(|file| file.path.clone())
        .collect::<BTreeSet<_>>();
    let metric = compute_changed_metric(&report, &diff, &included_paths, MetricKind::NamedFunction)
        .expect("metric works");

    assert_eq!(metric.covered, 1);
    assert_eq!(metric.total, 1);
    assert_eq!(metric.uncovered_changed_opportunities.len(), 0);
    let file_totals = metric
        .changed_totals_by_file
        .get(&PathBuf::from("src/lib.rs"))
        .expect("changed totals by file");
    assert_eq!(file_totals.covered, 1);
    assert_eq!(file_totals.total, 1);
}

#[test]
fn changed_named_function_metric_reports_one_uncovered_template_group() {
    let report = CoverageReport {
        opportunities: vec![
            CoverageOpportunity {
                kind: OpportunityKind::Function,
                span: SourceSpan {
                    path: PathBuf::from("src/lib.rs"),
                    start_line: 20,
                    end_line: 22,
                    start_col: Some(1),
                    end_col: Some(1),
                },
                covered: false,
                is_named_function: Some(true),
                named_function_identity: Some("covgate::metrics::parse".to_string()),
            },
            CoverageOpportunity {
                kind: OpportunityKind::Function,
                span: SourceSpan {
                    path: PathBuf::from("src/lib.rs"),
                    start_line: 10,
                    end_line: 12,
                    start_col: Some(1),
                    end_col: Some(1),
                },
                covered: false,
                is_named_function: Some(true),
                named_function_identity: Some("covgate::metrics::parse".to_string()),
            },
        ],
        totals_by_file: BTreeMap::from([(
            MetricKind::NamedFunction,
            BTreeMap::from([(
                PathBuf::from("src/lib.rs"),
                FileTotals {
                    covered: 0,
                    total: 1,
                },
            )]),
        )]),
    };
    let diff = vec![ChangedFile {
        path: PathBuf::from("src/lib.rs"),
        changed_lines: vec![LineRange { start: 10, end: 22 }],
    }];

    let included_paths = diff
        .iter()
        .map(|file| file.path.clone())
        .collect::<BTreeSet<_>>();
    let metric = compute_changed_metric(&report, &diff, &included_paths, MetricKind::NamedFunction)
        .expect("metric works");

    assert_eq!(metric.covered, 0);
    assert_eq!(metric.total, 1);
    assert_eq!(metric.uncovered_changed_opportunities.len(), 1);
    assert_eq!(
        metric.uncovered_changed_opportunities[0].span.start_line,
        10
    );
}
