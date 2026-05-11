mod support;

use std::{env, fs, sync::Mutex};

use tempfile::tempdir;

use covgate::{cli::Args, config::Config, run};

static CWD_LOCK: Mutex<()> = Mutex::new(());

struct CwdGuard(std::path::PathBuf);

impl Drop for CwdGuard {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.0);
    }
}

fn git_base_args(coverage_report: std::path::PathBuf) -> Args {
    Args {
        coverage_report,
        base: Some("HEAD".to_string()),
        diff_file: None,
        markdown_output: None,
        no_github_summary: false,
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
    fs::write(
        worktree.join("covgate.toml"),
        "[[gates]]\nfail-under-regions = 90\n",
    )
    .expect("config should write");

    let config = Config::try_from(Args {
        coverage_report: fixture.coverage_json(),
        base: None,
        diff_file: Some(diff_file),
        markdown_output: None,
        no_github_summary: false,
    })
    .expect("config should resolve");

    let code = run(config).expect("run should succeed");

    assert_eq!(code, 0);
}

#[test]
fn run_with_git_base_errors_on_coverage_untracked_files() {
    let _lock = CWD_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());
    let fixture = support::rust_basic_pass_fixture();
    let temp = tempdir().expect("tempdir should exist");
    let worktree = support::setup_fixture_worktree(temp.path(), fixture);
    support::run_git(&worktree, &["rm", "--cached", "src/lib.rs"]);
    let previous = env::current_dir().expect("cwd should resolve");
    let _guard = CwdGuard(previous);
    env::set_current_dir(&worktree).expect("should chdir into worktree");
    fs::write(
        worktree.join("covgate.toml"),
        "[[gates]]\nfail-under-regions = 90\n",
    )
    .expect("config should write");

    let config =
        Config::try_from(git_base_args(fixture.coverage_json())).expect("config should resolve");
    let err = run(config).expect_err("run should fail when coverage-present file is untracked");

    assert!(
        err.to_string().contains("git add -N src/lib.rs"),
        "error={err}"
    );
}

#[test]
fn run_with_git_base_passes_for_uncovered_untracked_file_with_spaces() {
    let _lock = CWD_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());
    let fixture = support::rust_basic_pass_fixture();
    let temp = tempdir().expect("tempdir should exist");
    let worktree = support::setup_fixture_worktree(temp.path(), fixture);
    fs::write(worktree.join("space name.rs"), "pub fn pending() {}\n")
        .expect("untracked file should write");
    let previous = env::current_dir().expect("cwd should resolve");
    let _guard = CwdGuard(previous);
    env::set_current_dir(&worktree).expect("should chdir into worktree");
    fs::write(
        worktree.join("covgate.toml"),
        "[[gates]]\nfail-under-regions = 90\n",
    )
    .expect("config should write");

    let config =
        Config::try_from(git_base_args(fixture.coverage_json())).expect("config should resolve");
    let code = run(config).expect("run should succeed");

    assert_eq!(code, 0);
}

#[test]
fn run_with_git_base_quotes_coverage_paths_with_spaces_in_error_command() {
    let _lock = CWD_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());
    let fixture = support::rust_basic_pass_fixture();
    let temp = tempdir().expect("tempdir should exist");
    let worktree = support::setup_fixture_worktree(temp.path(), fixture);
    let coverage_path = temp.path().join("coverage-with-space.json");
    let original =
        fs::read_to_string(fixture.coverage_json()).expect("fixture coverage should be readable");
    fs::write(
        &coverage_path,
        original.replace("\"src/lib.rs\"", "\"src/my lib.rs\""),
    )
    .expect("modified coverage should be written");
    fs::write(
        worktree.join("src").join("my lib.rs"),
        "pub fn pending() {}\n",
    )
    .expect("untracked file with space should write");
    let previous = env::current_dir().expect("cwd should resolve");
    let _guard = CwdGuard(previous);
    env::set_current_dir(&worktree).expect("should chdir into worktree");
    fs::write(
        worktree.join("covgate.toml"),
        "[[gates]]\nfail-under-regions = 90\n",
    )
    .expect("config should write");

    let config = Config::try_from(git_base_args(coverage_path)).expect("config should resolve");
    let err = run(config)
        .expect_err("run should fail when coverage-present untracked file has spaces in path");

    assert!(
        err.to_string().contains("git add -N 'src/my lib.rs'"),
        "error={err}"
    );
}

#[test]
fn run_with_git_base_passes_when_no_untracked_files_exist() {
    let _lock = CWD_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());
    let fixture = support::rust_basic_pass_fixture();
    let temp = tempdir().expect("tempdir should exist");
    let worktree = support::setup_fixture_worktree(temp.path(), fixture);
    let previous = env::current_dir().expect("cwd should resolve");
    let _guard = CwdGuard(previous);
    env::set_current_dir(&worktree).expect("should chdir into worktree");
    fs::write(
        worktree.join("covgate.toml"),
        "[[gates]]\nfail-under-regions = 90\n",
    )
    .expect("config should write");

    let config =
        Config::try_from(git_base_args(fixture.coverage_json())).expect("config should resolve");
    let code = run(config).expect("run should succeed");

    assert_eq!(code, 0);
}

#[test]
fn run_with_git_base_requires_git_repo_for_coverage_path_normalization() {
    let _lock = CWD_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());
    let fixture = support::rust_basic_pass_fixture();
    let temp = tempdir().expect("tempdir should exist");
    let previous = env::current_dir().expect("cwd should resolve");
    let _guard = CwdGuard(previous);
    env::set_current_dir(temp.path()).expect("should chdir into tempdir");
    fs::write(
        temp.path().join("covgate.toml"),
        "[[gates]]\nfail-under-regions = 90\n",
    )
    .expect("config should write");

    let config =
        Config::try_from(git_base_args(fixture.coverage_json())).expect("config should resolve");
    let err = run(config).expect_err("run should fail");
    assert!(
        err.to_string()
            .contains("covgate requires a git repository to run"),
        "error={err:?}"
    );
}
