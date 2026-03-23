mod support;

use std::{env, fs, sync::Mutex};

use tempfile::tempdir;

use covgate::{
    config::Config,
    diff::DiffSource,
    model::{GateRule, MetricKind},
    run,
};

static CWD_LOCK: Mutex<()> = Mutex::new(());

struct CwdGuard(std::path::PathBuf);
impl Drop for CwdGuard {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.0);
    }
}

fn git_base_config(coverage_report: std::path::PathBuf) -> Config {
    Config {
        coverage_report,
        diff_source: DiffSource::GitBase("HEAD".to_string()),
        rules: vec![GateRule::Percent {
            metric: MetricKind::Region,
            minimum_percent: 90.0,
        }],
        markdown_output: None,
    }
}

#[test]
fn run_with_diff_file_executes_without_untracked_warning_lookup() {
    let _lock = CWD_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());
    let fixture = support::rust_basic_pass_fixture();
    let temp = tempdir().expect("tempdir should exist");
    let worktree = support::setup_fixture_worktree(temp.path(), fixture);
    let diff_file = support::write_worktree_diff(temp.path(), &worktree);
    let previous = env::current_dir().expect("cwd should resolve");
    let _guard = CwdGuard(previous);
    env::set_current_dir(&worktree).expect("should chdir into worktree");

    let code = run(Config {
        coverage_report: fixture.coverage_json(),
        diff_source: DiffSource::DiffFile(diff_file),
        rules: vec![GateRule::Percent {
            metric: MetricKind::Region,
            minimum_percent: 90.0,
        }],
        markdown_output: None,
    })
    .expect("run should succeed");

    assert_eq!(code, 0);
}

#[test]
fn run_with_git_base_checks_untracked_files_before_loading_diff() {
    let _lock = CWD_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());
    let fixture = support::rust_basic_pass_fixture();
    let temp = tempdir().expect("tempdir should exist");
    let worktree = support::setup_fixture_worktree(temp.path(), fixture);
    fs::write(worktree.join("new_untracked.rs"), "pub fn pending() {}\n")
        .expect("untracked file should write");
    let previous = env::current_dir().expect("cwd should resolve");
    let _guard = CwdGuard(previous);
    env::set_current_dir(&worktree).expect("should chdir into worktree");

    let code = run(Config {
        coverage_report: fixture.coverage_json(),
        diff_source: DiffSource::GitBase("HEAD".to_string()),
        rules: vec![GateRule::Percent {
            metric: MetricKind::Region,
            minimum_percent: 90.0,
        }],
        markdown_output: None,
    })
    .expect("run should succeed");

    assert_eq!(code, 0);
}

#[test]
fn run_with_git_base_quotes_paths_in_add_command_when_needed() {
    let _lock = CWD_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());
    let fixture = support::rust_basic_pass_fixture();
    let temp = tempdir().expect("tempdir should exist");
    let worktree = support::setup_fixture_worktree(temp.path(), fixture);
    fs::write(worktree.join("space name.rs"), "pub fn pending() {}\n")
        .expect("untracked file should write");
    let previous = env::current_dir().expect("cwd should resolve");
    let _guard = CwdGuard(previous);
    env::set_current_dir(&worktree).expect("should chdir into worktree");

    let code = run(Config {
        coverage_report: fixture.coverage_json(),
        diff_source: DiffSource::GitBase("HEAD".to_string()),
        rules: vec![GateRule::Percent {
            metric: MetricKind::Region,
            minimum_percent: 90.0,
        }],
        markdown_output: None,
    })
    .expect("run should succeed");

    assert_eq!(code, 0);
}

#[test]
fn run_with_git_base_skips_warning_when_no_untracked_files_exist() {
    let _lock = CWD_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());
    let fixture = support::rust_basic_pass_fixture();
    let temp = tempdir().expect("tempdir should exist");
    let worktree = support::setup_fixture_worktree(temp.path(), fixture);
    let previous = env::current_dir().expect("cwd should resolve");
    let _guard = CwdGuard(previous);
    env::set_current_dir(&worktree).expect("should chdir into worktree");

    let code = run(Config {
        coverage_report: fixture.coverage_json(),
        diff_source: DiffSource::GitBase("HEAD".to_string()),
        rules: vec![GateRule::Percent {
            metric: MetricKind::Region,
            minimum_percent: 90.0,
        }],
        markdown_output: None,
    })
    .expect("run should succeed");

    assert_eq!(code, 0);
}

#[test]
fn run_with_git_base_reports_status_failure_for_untracked_lookup_outside_repo() {
    let _lock = CWD_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());
    let fixture = support::rust_basic_pass_fixture();
    let temp = tempdir().expect("tempdir should exist");
    let previous = env::current_dir().expect("cwd should resolve");
    let _guard = CwdGuard(previous);
    env::set_current_dir(temp.path()).expect("should chdir into tempdir");

    let err = run(git_base_config(fixture.coverage_json())).expect_err("run should fail");
    assert!(
        err.to_string().contains("failed to list untracked files"),
        "error={err:?}"
    );
}
