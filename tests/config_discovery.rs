use std::{fs, path::PathBuf, sync::Mutex};

use covgate::{cli::Args, config::Config, diff::DiffSource};
use tempfile::tempdir;

static CWD_LOCK: Mutex<()> = Mutex::new(());

struct CwdGuard(PathBuf);

impl Drop for CwdGuard {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.0);
    }
}

fn run_git(repo: &std::path::Path, args: &[&str]) {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(repo)
        .output()
        .expect("git should run");
    assert!(output.status.success(), "stderr={:?}", output.stderr);
}

fn args_for_config_discovery() -> Args {
    Args {
        coverage_report: "coverage.json".into(),
        base: None,
        diff_file: Some("scenario.diff".into()),
        fail_under_regions: Some(90.0),
        fail_under_lines: None,
        fail_under_branches: None,
        fail_under_functions: None,
        fail_uncovered_regions: None,
        fail_uncovered_lines: None,
        fail_uncovered_branches: None,
        fail_uncovered_functions: None,
        markdown_output: None,
    }
}

#[test]
fn loads_config_from_parent_directory() {
    let _lock = CWD_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());

    let temp = tempdir().expect("tempdir should exist");
    let nested = temp.path().join("nested").join("deeper");
    fs::create_dir_all(&nested).expect("nested dir should exist");
    fs::write(
        temp.path().join("covgate.toml"),
        "markdown_output = \"summary.md\"\n[gates]\nfail_under_lines = 80\n",
    )
    .expect("config should write");

    let previous = std::env::current_dir().expect("cwd should resolve");
    let _guard = CwdGuard(previous);
    std::env::set_current_dir(&nested).expect("should chdir into nested directory");

    let config = Config::try_from(args_for_config_discovery()).expect("config should resolve");

    assert_eq!(config.markdown_output, Some(PathBuf::from("summary.md")));
    assert!(matches!(config.diff_source, DiffSource::DiffFile(_)));
}

#[test]
fn does_not_walk_past_repo_root_when_config_is_missing_inside_repo() {
    let _lock = CWD_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());

    let temp = tempdir().expect("tempdir should exist");
    let outer = temp.path().join("outer");
    let repo_root = outer.join("repo");
    let nested = repo_root.join("nested").join("deeper");
    fs::create_dir_all(&nested).expect("nested dir should exist");
    run_git(&repo_root, &["init"]);
    fs::write(
        outer.join("covgate.toml"),
        "markdown_output = \"outside.md\"\n[gates]\nfail_under_lines = 80\n",
    )
    .expect("outer config should write");

    let previous = std::env::current_dir().expect("cwd should resolve");
    let _guard = CwdGuard(previous);
    std::env::set_current_dir(&nested).expect("should chdir into nested directory");

    let config = Config::try_from(args_for_config_discovery()).expect("config should resolve");

    assert_eq!(config.markdown_output, None);
}

#[test]
fn still_walks_past_parent_boundaries_when_repo_root_is_unknown() {
    let _lock = CWD_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());

    let temp = tempdir().expect("tempdir should exist");
    let outer = temp.path().join("outer");
    let nested = outer.join("repo").join("nested");
    fs::create_dir_all(&nested).expect("nested dir should exist");
    fs::write(
        outer.join("covgate.toml"),
        "markdown_output = \"outside.md\"\n[gates]\nfail_under_lines = 80\n",
    )
    .expect("outer config should write");

    let previous = std::env::current_dir().expect("cwd should resolve");
    let _guard = CwdGuard(previous);
    std::env::set_current_dir(&nested).expect("should chdir into nested directory");

    let config = Config::try_from(args_for_config_discovery()).expect("config should resolve");

    assert_eq!(config.markdown_output, Some(PathBuf::from("outside.md")));
}

#[test]
fn reports_read_errors_for_discovered_config_candidates() {
    let _lock = CWD_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());

    let temp = tempdir().expect("tempdir should exist");
    let nested = temp.path().join("nested");
    fs::create_dir_all(&nested).expect("nested dir should exist");
    fs::create_dir(temp.path().join("covgate.toml")).expect("config path should be a directory");

    let previous = std::env::current_dir().expect("cwd should resolve");
    let _guard = CwdGuard(previous);
    std::env::set_current_dir(&nested).expect("should chdir into nested directory");

    let error = Config::try_from(args_for_config_discovery()).expect_err("config should fail");
    let error_text = format!("{error:#}");

    assert!(error_text.contains("failed to read config file"));
    assert!(error_text.contains("covgate.toml"));
}

#[test]
fn reports_parse_errors_for_discovered_config_candidates() {
    let _lock = CWD_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());

    let temp = tempdir().expect("tempdir should exist");
    let nested = temp.path().join("nested");
    fs::create_dir_all(&nested).expect("nested dir should exist");
    fs::write(temp.path().join("covgate.toml"), "not = [valid toml")
        .expect("invalid config should write");

    let previous = std::env::current_dir().expect("cwd should resolve");
    let _guard = CwdGuard(previous);
    std::env::set_current_dir(&nested).expect("should chdir into nested directory");

    let error = Config::try_from(args_for_config_discovery()).expect_err("config should fail");
    let error_text = format!("{error:#}");

    assert!(error_text.contains("failed to parse config file"));
    assert!(error_text.contains("covgate.toml"));
}
