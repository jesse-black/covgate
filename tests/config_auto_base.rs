mod support;

use std::fs;
use std::sync::Mutex;

use tempfile::tempdir;

use covgate::{
    cli::Args,
    config::Config,
    diff::DiffSource,
    git::{RECORDED_BASE_REF, record_base_ref},
};

use crate::support::run_git;

static CWD_LOCK: Mutex<()> = Mutex::new(());

struct CwdGuard(std::path::PathBuf);
impl Drop for CwdGuard {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.0);
    }
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
    run_git(repo, &["branch", "-M", "task/config-auto-base"]);

    record_base_ref().expect("record-base should succeed");
    fs::write(
        repo.join("covgate.toml"),
        "[[gates]]\nfail-under-regions = 1\n",
    )
    .expect("config should write");

    let cfg = Config::try_from(Args {
        coverage_report: "coverage.json".into(),
        base: None,
        diff_file: None,
        markdown_output: None,
        no_github_summary: false,
    })
    .expect("config should resolve");

    match cfg.diff_source {
        DiffSource::GitBase(base) => assert_eq!(base, RECORDED_BASE_REF),
        DiffSource::DiffFile(_) => panic!("expected git base"),
    }
}
