mod support;

use std::{fs, path::PathBuf, sync::Mutex};

use covgate::{
    cli::Args,
    config::{Config, OutputSink},
    diff::DiffSource,
};
use tempfile::tempdir;

use crate::support::run_git;

static CWD_LOCK: Mutex<()> = Mutex::new(());

struct CwdGuard(PathBuf);

impl Drop for CwdGuard {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.0);
    }
}

fn args_for_config_discovery() -> Args {
    Args {
        coverage_report: "coverage.json".into(),
        base: None,
        diff_file: Some("scenario.diff".into()),
        markdown_output: None,
        no_github_summary: false,
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
        "markdown-output = \"summary.md\"\n[[gates]]\nfail-under-lines = 80\n",
    )
    .expect("config should write");

    let previous = std::env::current_dir().expect("cwd should resolve");
    let _guard = CwdGuard(previous);
    std::env::set_current_dir(&nested).expect("should chdir into nested directory");

    let config = Config::try_from(args_for_config_discovery()).expect("config should resolve");

    assert_eq!(
        config.markdown_output,
        Some(OutputSink::File(PathBuf::from("summary.md")))
    );
    assert!(matches!(config.diff_source, DiffSource::DiffFile(_)));
}

#[test]
fn config_markdown_output_dash_resolves_to_stdout() {
    let _lock = CWD_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());

    let temp = tempdir().expect("tempdir should exist");
    fs::write(
        temp.path().join("covgate.toml"),
        "markdown-output = \"-\"\n[[gates]]\nfail-under-lines = 80\n",
    )
    .expect("config should write");

    let previous = std::env::current_dir().expect("cwd should resolve");
    let _guard = CwdGuard(previous);
    std::env::set_current_dir(temp.path()).expect("should chdir into config directory");

    let config = Config::try_from(args_for_config_discovery()).expect("config should resolve");

    assert_eq!(config.markdown_output, Some(OutputSink::Stdout));
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
        "markdown-output = \"outside.md\"\n[[gates]]\nfail-under-lines = 80\n",
    )
    .expect("outer config should write");

    let previous = std::env::current_dir().expect("cwd should resolve");
    let _guard = CwdGuard(previous);
    std::env::set_current_dir(&nested).expect("should chdir into nested directory");

    let error =
        Config::try_from(args_for_config_discovery()).expect_err("config should not resolve");

    assert!(
        error.to_string().contains("at least one rule is required"),
        "error={error:?}"
    );
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
        "markdown-output = \"outside.md\"\n[[gates]]\nfail-under-lines = 80\n",
    )
    .expect("outer config should write");

    let previous = std::env::current_dir().expect("cwd should resolve");
    let _guard = CwdGuard(previous);
    std::env::set_current_dir(&nested).expect("should chdir into nested directory");

    let config = Config::try_from(args_for_config_discovery()).expect("config should resolve");

    assert_eq!(
        config.markdown_output,
        Some(OutputSink::File(PathBuf::from("outside.md")))
    );
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
