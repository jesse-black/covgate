use std::fs;
use std::sync::Mutex;

use tempfile::tempdir;

use covgate::{
    cli::Args,
    config::Config,
    diff::DiffSource,
    git::{RECORDED_BASE_REF, record_base_ref},
};

static CWD_LOCK: Mutex<()> = Mutex::new(());

struct CwdGuard(std::path::PathBuf);
impl Drop for CwdGuard {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.0);
    }
}

fn run_git(path: &std::path::Path, args: &[&str]) {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(path)
        .output()
        .expect("git should run");
    assert!(
        output.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn config_uses_recorded_base_when_base_is_omitted() {
    let _lock = CWD_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());

    let temp = tempdir().expect("tempdir should exist");
    let repo = temp.path();
    fs::write(repo.join("README.md"), "initial\n").expect("fixture file should write");

    run_git(repo, &["init"]);
    run_git(repo, &["config", "user.email", "covgate@example.com"]);
    run_git(repo, &["config", "user.name", "Covgate Tests"]);
    run_git(repo, &["add", "."]);
    run_git(repo, &["commit", "-m", "initial"]);

    let previous = std::env::current_dir().expect("cwd should resolve");
    let _guard = CwdGuard(previous);
    std::env::set_current_dir(repo).expect("should chdir");

    record_base_ref().expect("record-base should succeed");

    let cfg = Config::try_from(Args {
        coverage_report: "coverage.json".into(),
        base: None,
        diff_file: None,
        fail_under_regions: Some(1.0),
        fail_under_lines: None,
        fail_under_branches: None,
        fail_under_functions: None,
        fail_uncovered_regions: None,
        fail_uncovered_lines: None,
        fail_uncovered_branches: None,
        fail_uncovered_functions: None,
        markdown_output: None,
        allow_dirty_worktree: false,
    })
    .expect("config should resolve");

    match cfg.diff_source {
        DiffSource::GitBase(base) => assert_eq!(base, RECORDED_BASE_REF),
        DiffSource::DiffFile(_) => panic!("expected git base"),
    }
}
