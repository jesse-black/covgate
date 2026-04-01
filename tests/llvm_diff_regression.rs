mod support;

use std::{env, path::PathBuf, sync::Mutex};

use covgate::{
    coverage, diff,
    metrics::compute_changed_metric,
    model::{ComputedMetric, MetricKind},
};
use tempfile::tempdir;

use crate::support::{
    cpp_basic_fail_fixture, cpp_basic_pass_fixture, run_covgate_raw, rust_basic_fail_fixture,
    rust_basic_pass_fixture, setup_fixture_worktree, swift_basic_fail_fixture,
    swift_basic_pass_fixture, write_absolute_path_coverage_fixture,
    write_rebased_real_llvm_fixture, write_worktree_diff,
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
        let report = coverage::load_from_path(&coverage_json)?;
        let diff = diff::load_changed_lines(&diff::DiffSource::DiffFile(diff_file))?;
        compute_changed_metric(&report, &diff, metric)
    })();

    env::set_current_dir(original_cwd)?;
    result
}

fn load_changed_metric_from_subdir(
    fixture: support::Fixture,
    metric: MetricKind,
    subdir: &str,
) -> anyhow::Result<ComputedMetric> {
    let temp = tempdir()?;
    let worktree = setup_fixture_worktree(temp.path(), fixture);
    let diff_file = write_worktree_diff(temp.path(), &worktree);
    let coverage_json = temp.path().join("coverage-absolute.json");
    write_absolute_path_coverage_fixture(fixture, &worktree, &coverage_json);

    let _lock = CWD_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());
    let original_cwd = env::current_dir()?;
    env::set_current_dir(worktree.join(subdir))?;

    let result = (|| {
        let report = coverage::load_from_path(&coverage_json)?;
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

fn load_real_fixture_changed_metric(
    diff_text: &str,
    metric: MetricKind,
) -> anyhow::Result<ComputedMetric> {
    let temp = tempdir()?;
    let diff_file = temp.path().join("scenario.diff");
    std::fs::write(&diff_file, diff_text)?;
    let coverage_json = temp.path().join("covgate-self-full-rebased.json");
    write_rebased_real_llvm_fixture(&coverage_json);

    let _lock = CWD_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());
    let original_cwd = env::current_dir()?;
    env::set_current_dir(env!("CARGO_MANIFEST_DIR"))?;

    let result = (|| {
        let report = coverage::load_from_path(&coverage_json)?;
        let diff = diff::load_changed_lines(&diff::DiffSource::DiffFile(diff_file))?;
        compute_changed_metric(&report, &diff, metric)
    })();

    env::set_current_dir(original_cwd)?;
    result
}

fn synthetic_diff(path: &str, start_line: u32, added_lines: u32) -> String {
    format!(
        "diff --git a/{path} b/{path}\n--- a/{path}\n+++ b/{path}\n@@ -{start_line},0 +{start_line},{added_lines} @@\n"
    )
}

fn run_real_fixture_gate(diff_text: &str, args: &[&str]) -> std::process::Output {
    let temp = tempdir().expect("tempdir should exist");
    let diff_file = temp.path().join("scenario.diff");
    std::fs::write(&diff_file, diff_text).expect("diff file should be written");
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let coverage_json = temp.path().join("covgate-self-full-rebased.json");
    write_rebased_real_llvm_fixture(&coverage_json);

    let mut covgate_args = vec![
        "check".to_string(),
        coverage_json.to_string_lossy().into_owned(),
        "--diff-file".to_string(),
        diff_file.to_string_lossy().into_owned(),
    ];
    covgate_args.extend(args.iter().map(|arg| (*arg).to_string()));
    run_covgate_raw(&repo_root, &covgate_args)
}

#[test]
fn real_fixture_config_range_tracks_exact_changed_llvm_opportunities() {
    let diff_text = synthetic_diff("src/config.rs", 73, 15);

    let line_metric = load_real_fixture_changed_metric(&diff_text, MetricKind::Line)
        .expect("line metric should load");
    assert_eq!(line_metric.covered, 13);
    assert_eq!(line_metric.total, 13);
    assert!(line_metric.uncovered_changed_opportunities.is_empty());

    let region_metric = load_real_fixture_changed_metric(&diff_text, MetricKind::Region)
        .expect("region metric should load");
    assert_eq!(region_metric.covered, 22);
    assert_eq!(region_metric.total, 28);
    assert_eq!(
        changed_spans(&region_metric),
        vec![
            (PathBuf::from("src/config.rs"), 80, 80),
            (PathBuf::from("src/config.rs"), 80, 80),
            (PathBuf::from("src/config.rs"), 80, 80),
            (PathBuf::from("src/config.rs"), 82, 82),
            (PathBuf::from("src/config.rs"), 82, 82),
            (PathBuf::from("src/config.rs"), 82, 82),
        ]
    );

    let function_metric = load_real_fixture_changed_metric(&diff_text, MetricKind::Function)
        .expect("function metric should load");
    assert_eq!(function_metric.covered, 2);
    assert_eq!(function_metric.total, 4);
    assert_eq!(
        changed_spans(&function_metric),
        vec![
            (PathBuf::from("src/config.rs"), 80, 80),
            (PathBuf::from("src/config.rs"), 82, 82),
        ]
    );
}

#[test]
fn absolute_llvm_paths_still_match_diff_when_invoked_from_subdir() {
    let region_metric =
        load_changed_metric_from_subdir(rust_basic_pass_fixture(), MetricKind::Region, "src")
            .expect("region metric should load from subdir");

    assert!(region_metric.total > 0, "metric={region_metric:?}");
}

#[test]
fn real_fixture_llvm_json_range_tracks_exact_changed_llvm_opportunities() {
    let diff_text = synthetic_diff("src/coverage/llvm_json.rs", 1, 120);

    let line_metric = load_real_fixture_changed_metric(&diff_text, MetricKind::Line)
        .expect("line metric should load");
    assert_eq!(line_metric.covered, 91);
    assert_eq!(line_metric.total, 91);
    assert!(line_metric.uncovered_changed_opportunities.is_empty());

    let region_metric = load_real_fixture_changed_metric(&diff_text, MetricKind::Region)
        .expect("region metric should load");
    assert_eq!(region_metric.covered, 142);
    assert_eq!(region_metric.total, 145);
    assert_eq!(
        changed_spans(&region_metric),
        vec![
            (PathBuf::from("src/coverage/llvm_json.rs"), 65, 65),
            (PathBuf::from("src/coverage/llvm_json.rs"), 92, 92),
            (PathBuf::from("src/coverage/llvm_json.rs"), 116, 116),
        ]
    );

    let function_metric = load_real_fixture_changed_metric(&diff_text, MetricKind::Function)
        .expect("function metric should load");
    assert_eq!(function_metric.covered, 5);
    assert_eq!(function_metric.total, 5);
    assert!(function_metric.uncovered_changed_opportunities.is_empty());
}

#[test]
fn real_fixture_config_second_range_tracks_exact_changed_llvm_opportunities() {
    let diff_text = synthetic_diff("src/config.rs", 108, 40);

    let line_metric = load_real_fixture_changed_metric(&diff_text, MetricKind::Line)
        .expect("line metric should load");
    assert_eq!(line_metric.covered, 26);
    assert_eq!(line_metric.total, 34);
    assert_eq!(
        changed_spans(&line_metric),
        vec![
            (PathBuf::from("src/config.rs"), 131, 131),
            (PathBuf::from("src/config.rs"), 132, 132),
            (PathBuf::from("src/config.rs"), 133, 133),
            (PathBuf::from("src/config.rs"), 134, 134),
            (PathBuf::from("src/config.rs"), 144, 144),
            (PathBuf::from("src/config.rs"), 145, 145),
            (PathBuf::from("src/config.rs"), 146, 146),
            (PathBuf::from("src/config.rs"), 147, 147),
        ]
    );

    let region_metric = load_real_fixture_changed_metric(&diff_text, MetricKind::Region)
        .expect("region metric should load");
    assert_eq!(region_metric.covered, 30);
    assert_eq!(region_metric.total, 38);
    assert_eq!(
        changed_spans(&region_metric),
        vec![
            (PathBuf::from("src/config.rs"), 130, 130),
            (PathBuf::from("src/config.rs"), 130, 131),
            (PathBuf::from("src/config.rs"), 131, 131),
            (PathBuf::from("src/config.rs"), 131, 131),
            (PathBuf::from("src/config.rs"), 143, 143),
            (PathBuf::from("src/config.rs"), 143, 144),
            (PathBuf::from("src/config.rs"), 144, 144),
            (PathBuf::from("src/config.rs"), 144, 144),
        ]
    );

    let function_metric = load_real_fixture_changed_metric(&diff_text, MetricKind::Function)
        .expect("function metric should load");
    assert_eq!(function_metric.covered, 1);
    assert_eq!(function_metric.total, 1);
    assert!(function_metric.uncovered_changed_opportunities.is_empty());
}

#[test]
fn real_fixture_coverlet_json_range_tracks_exact_changed_llvm_opportunities() {
    let diff_text = synthetic_diff("src/coverage/coverlet_json.rs", 1, 120);

    let line_metric = load_real_fixture_changed_metric(&diff_text, MetricKind::Line)
        .expect("line metric should load");
    assert_eq!(line_metric.covered, 88);
    assert_eq!(line_metric.total, 89);
    assert_eq!(
        changed_spans(&line_metric),
        vec![(PathBuf::from("src/coverage/coverlet_json.rs"), 30, 30)]
    );

    let region_metric = load_real_fixture_changed_metric(&diff_text, MetricKind::Region)
        .expect("region metric should load");
    assert_eq!(region_metric.covered, 149);
    assert_eq!(region_metric.total, 151);
    assert_eq!(
        changed_spans(&region_metric),
        vec![
            (PathBuf::from("src/coverage/coverlet_json.rs"), 15, 15),
            (PathBuf::from("src/coverage/coverlet_json.rs"), 30, 30),
        ]
    );

    let function_metric = load_real_fixture_changed_metric(&diff_text, MetricKind::Function)
        .expect("function metric should load");
    assert_eq!(function_metric.covered, 3);
    assert_eq!(function_metric.total, 3);
    assert!(function_metric.uncovered_changed_opportunities.is_empty());
}

#[test]
fn real_fixture_render_markdown_range_tracks_exact_changed_llvm_opportunities() {
    let diff_text = synthetic_diff("src/render/markdown.rs", 1, 120);

    let line_metric = load_real_fixture_changed_metric(&diff_text, MetricKind::Line)
        .expect("line metric should load");
    assert_eq!(line_metric.covered, 87);
    assert_eq!(line_metric.total, 98);
    assert_eq!(
        changed_spans(&line_metric),
        vec![
            (PathBuf::from("src/render/markdown.rs"), 29, 29),
            (PathBuf::from("src/render/markdown.rs"), 30, 30),
            (PathBuf::from("src/render/markdown.rs"), 31, 31),
            (PathBuf::from("src/render/markdown.rs"), 32, 32),
            (PathBuf::from("src/render/markdown.rs"), 33, 33),
            (PathBuf::from("src/render/markdown.rs"), 34, 34),
            (PathBuf::from("src/render/markdown.rs"), 35, 35),
            (PathBuf::from("src/render/markdown.rs"), 36, 36),
            (PathBuf::from("src/render/markdown.rs"), 37, 37),
            (PathBuf::from("src/render/markdown.rs"), 64, 64),
            (PathBuf::from("src/render/markdown.rs"), 111, 111),
        ]
    );

    let region_metric = load_real_fixture_changed_metric(&diff_text, MetricKind::Region)
        .expect("region metric should load");
    assert_eq!(region_metric.covered, 121);
    assert_eq!(region_metric.total, 128);
    assert_eq!(
        changed_spans(&region_metric),
        vec![
            (PathBuf::from("src/render/markdown.rs"), 29, 29),
            (PathBuf::from("src/render/markdown.rs"), 29, 30),
            (PathBuf::from("src/render/markdown.rs"), 30, 30),
            (PathBuf::from("src/render/markdown.rs"), 30, 30),
            (PathBuf::from("src/render/markdown.rs"), 30, 30),
            (PathBuf::from("src/render/markdown.rs"), 64, 64),
            (PathBuf::from("src/render/markdown.rs"), 111, 111),
        ]
    );

    let function_metric = load_real_fixture_changed_metric(&diff_text, MetricKind::Function)
        .expect("function metric should load");
    assert_eq!(function_metric.covered, 3);
    assert_eq!(function_metric.total, 3);
    assert!(function_metric.uncovered_changed_opportunities.is_empty());
}

#[test]
fn real_fixture_config_second_range_cli_gates_fail_and_pass_as_expected() {
    let diff_text = synthetic_diff("src/config.rs", 108, 40);
    let output = run_real_fixture_gate(
        &diff_text,
        &[
            "--fail-under-lines",
            "80",
            "--fail-under-regions",
            "75",
            "--fail-uncovered-functions",
            "0",
        ],
    );

    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(
        stdout.contains("Rule fail-under-lines: FAIL"),
        "stdout={stdout}"
    );
    assert!(
        stdout.contains("Rule fail-under-regions: PASS"),
        "stdout={stdout}"
    );
    assert!(
        stdout.contains("Rule fail-uncovered-functions: PASS"),
        "stdout={stdout}"
    );
}

#[test]
fn real_fixture_coverlet_json_range_cli_gates_pass_as_expected() {
    let diff_text = synthetic_diff("src/coverage/coverlet_json.rs", 1, 120);
    let output = run_real_fixture_gate(
        &diff_text,
        &[
            "--fail-under-lines",
            "98",
            "--fail-uncovered-regions",
            "2",
            "--fail-under-functions",
            "100",
        ],
    );

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(
        stdout.contains("Rule fail-under-lines: PASS"),
        "stdout={stdout}"
    );
    assert!(
        stdout.contains("Rule fail-uncovered-regions: PASS"),
        "stdout={stdout}"
    );
    assert!(
        stdout.contains("Rule fail-under-functions: PASS"),
        "stdout={stdout}"
    );
}

#[test]
fn real_fixture_render_markdown_range_cli_uncovered_line_budget_fails_as_expected() {
    let diff_text = synthetic_diff("src/render/markdown.rs", 1, 120);
    let output = run_real_fixture_gate(
        &diff_text,
        &[
            "--fail-uncovered-lines",
            "10",
            "--fail-under-regions",
            "94",
            "--fail-under-functions",
            "100",
        ],
    );

    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(
        stdout.contains("Rule fail-uncovered-lines: FAIL"),
        "stdout={stdout}"
    );
    assert!(
        stdout.contains("Rule fail-under-regions: PASS"),
        "stdout={stdout}"
    );
    assert!(
        stdout.contains("Rule fail-under-functions: PASS"),
        "stdout={stdout}"
    );
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
