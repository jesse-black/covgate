mod support;

use std::{env, path::PathBuf, sync::Mutex};

use covgate::{
    coverage, diff,
    metrics::compute_changed_metric,
    model::{ComputedMetric, MetricKind},
};
use tempfile::tempdir;

use crate::support::{
    cpp_basic_fail_fixture, cpp_basic_pass_fixture, rust_basic_fail_fixture,
    rust_basic_pass_fixture, setup_fixture_worktree, swift_basic_fail_fixture,
    swift_basic_pass_fixture, write_absolute_path_coverage_fixture, write_worktree_diff,
};

static CWD_LOCK: Mutex<()> = Mutex::new(());

fn load_changed_metric(
    fixture: support::Fixture,
    metric: MetricKind,
) -> anyhow::Result<ComputedMetric> {
    let temp = tempdir()?;
    let worktree = setup_fixture_worktree(temp.path(), fixture);
    let diff_file = write_worktree_diff(temp.path(), &worktree);
    let coverage_json = temp.path().join("coverage-absolute.json");
    write_absolute_path_coverage_fixture(fixture, &worktree, &coverage_json);

    let _lock = CWD_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());
    let original_cwd = env::current_dir()?;
    env::set_current_dir(&worktree)?;

    let result = (|| {
        let report = coverage::parse_path(&coverage_json)?;
        let diff = diff::load_changed_lines(&diff::DiffSource::DiffFile(diff_file))?;
        compute_changed_metric(&report, &diff, metric)
    })();

    env::set_current_dir(original_cwd)?;
    result
}

fn changed_spans(metric: &ComputedMetric) -> Vec<(PathBuf, u32, u32)> {
    metric
        .uncovered_changed_opportunities
        .iter()
        .map(|op| (op.span.path.clone(), op.span.start_line, op.span.end_line))
        .collect()
}

#[test]
fn rust_basic_fail_fixture_tracks_exact_changed_llvm_opportunities() {
    let fixture = rust_basic_fail_fixture();

    let line_metric = load_changed_metric(fixture, MetricKind::Line).expect("line metric");
    assert_eq!(line_metric.covered, 0);
    assert_eq!(line_metric.total, 2);
    assert_eq!(line_metric.percent, 0.0);
    assert_eq!(
        changed_spans(&line_metric),
        vec![
            (PathBuf::from("src/lib.rs"), 2, 2),
            (PathBuf::from("src/lib.rs"), 3, 3),
        ]
    );

    let region_metric = load_changed_metric(fixture, MetricKind::Region).expect("region metric");
    assert_eq!(region_metric.covered, 0);
    assert_eq!(region_metric.total, 3);
    assert_eq!(region_metric.percent, 0.0);
    assert_eq!(
        changed_spans(&region_metric),
        vec![
            (PathBuf::from("src/lib.rs"), 2, 2),
            (PathBuf::from("src/lib.rs"), 2, 2),
            (PathBuf::from("src/lib.rs"), 3, 3),
        ]
    );

    let function_metric =
        load_changed_metric(fixture, MetricKind::Function).expect("function metric");
    assert_eq!(function_metric.covered, 0);
    assert_eq!(function_metric.total, 1);
    assert_eq!(function_metric.percent, 0.0);
    assert_eq!(
        changed_spans(&function_metric),
        vec![(PathBuf::from("src/lib.rs"), 1, 4)]
    );
}

#[test]
fn rust_basic_pass_fixture_marks_changed_llvm_opportunities_as_covered() {
    let fixture = rust_basic_pass_fixture();

    let line_metric = load_changed_metric(fixture, MetricKind::Line).expect("line metric");
    assert_eq!(line_metric.covered, 2);
    assert_eq!(line_metric.total, 2);
    assert_eq!(line_metric.percent, 100.0);
    assert!(line_metric.uncovered_changed_opportunities.is_empty());

    let region_metric = load_changed_metric(fixture, MetricKind::Region).expect("region metric");
    assert_eq!(region_metric.covered, 3);
    assert_eq!(region_metric.total, 3);
    assert_eq!(region_metric.percent, 100.0);
    assert!(region_metric.uncovered_changed_opportunities.is_empty());

    let function_metric =
        load_changed_metric(fixture, MetricKind::Function).expect("function metric");
    assert_eq!(function_metric.covered, 1);
    assert_eq!(function_metric.total, 1);
    assert_eq!(function_metric.percent, 100.0);
    assert!(function_metric.uncovered_changed_opportunities.is_empty());
}

#[test]
fn cpp_basic_fail_fixture_tracks_exact_changed_llvm_opportunities() {
    let fixture = cpp_basic_fail_fixture();

    let line_metric = load_changed_metric(fixture, MetricKind::Line).expect("line metric");
    assert_eq!(line_metric.covered, 0);
    assert_eq!(line_metric.total, 4);
    assert_eq!(
        changed_spans(&line_metric),
        vec![
            (PathBuf::from("src/lib.cpp"), 2, 2),
            (PathBuf::from("src/lib.cpp"), 3, 3),
            (PathBuf::from("src/lib.cpp"), 4, 4),
            (PathBuf::from("src/lib.cpp"), 5, 5),
        ]
    );

    let region_metric = load_changed_metric(fixture, MetricKind::Region).expect("region metric");
    assert_eq!(region_metric.covered, 0);
    assert_eq!(region_metric.total, 4);
    assert_eq!(
        changed_spans(&region_metric),
        vec![
            (PathBuf::from("src/lib.cpp"), 1, 2),
            (PathBuf::from("src/lib.cpp"), 2, 2),
            (PathBuf::from("src/lib.cpp"), 2, 4),
            (PathBuf::from("src/lib.cpp"), 5, 5),
        ]
    );

    let function_metric =
        load_changed_metric(fixture, MetricKind::Function).expect("function metric");
    assert_eq!(function_metric.covered, 0);
    assert_eq!(function_metric.total, 1);
    assert_eq!(
        changed_spans(&function_metric),
        vec![(PathBuf::from("src/lib.cpp"), 1, 6)]
    );

    let branch_metric = load_changed_metric(fixture, MetricKind::Branch).expect("branch metric");
    assert_eq!(branch_metric.covered, 0);
    assert_eq!(branch_metric.total, 2);
    assert_eq!(
        changed_spans(&branch_metric),
        vec![
            (PathBuf::from("src/lib.cpp"), 2, 2),
            (PathBuf::from("src/lib.cpp"), 2, 2),
        ]
    );
}

#[test]
fn cpp_basic_pass_fixture_marks_changed_llvm_opportunities_as_covered() {
    let fixture = cpp_basic_pass_fixture();

    let line_metric = load_changed_metric(fixture, MetricKind::Line).expect("line metric");
    assert_eq!(line_metric.covered, 4);
    assert_eq!(line_metric.total, 4);
    assert!(line_metric.uncovered_changed_opportunities.is_empty());

    let region_metric = load_changed_metric(fixture, MetricKind::Region).expect("region metric");
    assert_eq!(region_metric.covered, 4);
    assert_eq!(region_metric.total, 4);
    assert!(region_metric.uncovered_changed_opportunities.is_empty());

    let function_metric =
        load_changed_metric(fixture, MetricKind::Function).expect("function metric");
    assert_eq!(function_metric.covered, 1);
    assert_eq!(function_metric.total, 1);
    assert!(function_metric.uncovered_changed_opportunities.is_empty());

    let branch_metric = load_changed_metric(fixture, MetricKind::Branch).expect("branch metric");
    assert_eq!(branch_metric.covered, 2);
    assert_eq!(branch_metric.total, 2);
    assert!(branch_metric.uncovered_changed_opportunities.is_empty());
}

#[test]
fn swift_basic_fail_fixture_tracks_exact_changed_llvm_opportunities() {
    let fixture = swift_basic_fail_fixture();

    let line_metric = load_changed_metric(fixture, MetricKind::Line).expect("line metric");
    assert_eq!(line_metric.covered, 0);
    assert_eq!(line_metric.total, 4);
    assert_eq!(
        changed_spans(&line_metric),
        vec![
            (PathBuf::from("Sources/CovgateDemo/CovgateDemo.swift"), 2, 2,),
            (PathBuf::from("Sources/CovgateDemo/CovgateDemo.swift"), 3, 3,),
            (PathBuf::from("Sources/CovgateDemo/CovgateDemo.swift"), 4, 4,),
            (PathBuf::from("Sources/CovgateDemo/CovgateDemo.swift"), 5, 5,),
        ]
    );

    let region_metric = load_changed_metric(fixture, MetricKind::Region).expect("region metric");
    assert_eq!(region_metric.covered, 0);
    assert_eq!(region_metric.total, 4);
    assert_eq!(
        changed_spans(&region_metric),
        vec![
            (PathBuf::from("Sources/CovgateDemo/CovgateDemo.swift"), 1, 2,),
            (PathBuf::from("Sources/CovgateDemo/CovgateDemo.swift"), 2, 2,),
            (PathBuf::from("Sources/CovgateDemo/CovgateDemo.swift"), 2, 4,),
            (PathBuf::from("Sources/CovgateDemo/CovgateDemo.swift"), 4, 5,),
        ]
    );

    let function_metric =
        load_changed_metric(fixture, MetricKind::Function).expect("function metric");
    assert_eq!(function_metric.covered, 0);
    assert_eq!(function_metric.total, 1);
    assert_eq!(
        changed_spans(&function_metric),
        vec![(PathBuf::from("Sources/CovgateDemo/CovgateDemo.swift"), 1, 6)]
    );
}

#[test]
fn swift_basic_pass_fixture_marks_changed_llvm_opportunities_as_covered() {
    let fixture = swift_basic_pass_fixture();

    let line_metric = load_changed_metric(fixture, MetricKind::Line).expect("line metric");
    assert_eq!(line_metric.covered, 4);
    assert_eq!(line_metric.total, 4);
    assert!(line_metric.uncovered_changed_opportunities.is_empty());

    let region_metric = load_changed_metric(fixture, MetricKind::Region).expect("region metric");
    assert_eq!(region_metric.covered, 4);
    assert_eq!(region_metric.total, 4);
    assert!(region_metric.uncovered_changed_opportunities.is_empty());

    let function_metric =
        load_changed_metric(fixture, MetricKind::Function).expect("function metric");
    assert_eq!(function_metric.covered, 1);
    assert_eq!(function_metric.total, 1);
    assert!(function_metric.uncovered_changed_opportunities.is_empty());
}
