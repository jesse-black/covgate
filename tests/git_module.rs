mod support;

use std::fs;
use std::sync::Mutex;

use tempfile::tempdir;

use covgate::git::{
    RECORDED_BASE_REF, create_ref, discover_base_ref, record_base_ref, resolve_head_sha,
    resolve_ref_sha,
};
use support::run_git;

static CWD_LOCK: Mutex<()> = Mutex::new(());

struct CwdGuard(std::path::PathBuf);
impl Drop for CwdGuard {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.0);
    }
}

fn with_temp_git_repo<F>(f: F)
where
    F: FnOnce(&std::path::Path),
{
    let _lock = CWD_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());

    let temp = tempdir().expect("tempdir should exist");
    let repo = temp.path();
    fs::write(repo.join("README.md"), "initial\n").expect("fixture file should be written");
    run_git(repo, &["init"]);
    run_git(repo, &["config", "user.email", "covgate@example.com"]);
    run_git(repo, &["config", "user.name", "Covgate Tests"]);
    run_git(repo, &["add", "."]);
    run_git(repo, &["commit", "-m", "initial"]);

    let previous = std::env::current_dir().expect("cwd should resolve");
    let _guard = CwdGuard(previous);
    std::env::set_current_dir(repo).expect("should chdir into repo");
    f(repo);
}

fn with_path_override<F>(path: &str, f: F)
where
    F: FnOnce(),
{
    let original = std::env::var("PATH").ok();
    // SAFETY: tests hold CWD_LOCK to serialize process-global env/cwd mutation.
    unsafe { std::env::set_var("PATH", path) };
    f();
    match original {
        Some(value) => {
            // SAFETY: tests hold CWD_LOCK to serialize process-global env/cwd mutation.
            unsafe { std::env::set_var("PATH", value) };
        }
        None => {
            // SAFETY: tests hold CWD_LOCK to serialize process-global env/cwd mutation.
            unsafe { std::env::remove_var("PATH") };
        }
    }
}

#[test]
fn record_base_creates_and_is_idempotent() {
    with_temp_git_repo(|repo| {
        let head_before = resolve_ref_sha("HEAD")
            .expect("head should resolve")
            .expect("head sha");
        let first = record_base_ref().expect("record-base should succeed");
        assert_eq!(first, head_before);

        fs::write(repo.join("next.txt"), "next\n").expect("next file should write");
        run_git(repo, &["add", "."]);
        run_git(repo, &["commit", "-m", "next"]);

        let second = record_base_ref().expect("record-base second run should succeed");
        assert_eq!(second, first);
    });
}

#[test]
fn record_base_refreshes_when_branch_changes() {
    with_temp_git_repo(|repo| {
        run_git(repo, &["branch", "-M", "main"]);

        let first = record_base_ref().expect("record-base should succeed on main");

        run_git(repo, &["checkout", "-b", "task/two"]);
        fs::write(repo.join("task-two.txt"), "task two\n").expect("task-two file should write");
        run_git(repo, &["add", "."]);
        run_git(repo, &["commit", "-m", "task two"]);

        let refreshed = record_base_ref().expect("record-base should refresh on branch change");
        assert_ne!(refreshed, first);

        let recorded = resolve_ref_sha(RECORDED_BASE_REF)
            .expect("recorded ref should resolve")
            .expect("recorded ref should exist");
        assert_eq!(recorded, refreshed);
    });
}

#[test]
fn record_base_recreates_missing_branch_marker_without_refreshing_ref() {
    with_temp_git_repo(|repo| {
        run_git(repo, &["branch", "-M", "main"]);
        let recorded = record_base_ref().expect("record-base should succeed");

        let marker_path_output = std::process::Command::new("git")
            .args([
                "rev-parse",
                "--git-path",
                "refs/worktree/covgate/base.branch",
            ])
            .current_dir(repo)
            .output()
            .expect("git rev-parse should run");
        assert!(marker_path_output.status.success());
        let marker_path = String::from_utf8(marker_path_output.stdout)
            .expect("marker path should be utf8")
            .trim()
            .to_string();
        fs::remove_file(&marker_path).expect("marker should be removable");

        let second = record_base_ref().expect("record-base should still succeed");
        assert_eq!(second, recorded);

        let marker_branch = fs::read_to_string(marker_path).expect("marker should be rewritten");
        assert_eq!(marker_branch.trim(), "main");
    });
}

#[test]
fn record_base_detached_head_uses_ancestor_check_path() {
    with_temp_git_repo(|repo| {
        run_git(repo, &["branch", "-M", "main"]);
        let recorded = record_base_ref().expect("record-base should succeed");

        run_git(repo, &["checkout", "--detach", "HEAD"]);

        let second = record_base_ref().expect("record-base should remain stable on detached head");
        assert_eq!(second, recorded);
    });
}

#[test]
fn record_base_detached_head_refreshes_when_recorded_commit_is_not_ancestor() {
    with_temp_git_repo(|repo| {
        run_git(repo, &["branch", "-M", "main"]);

        fs::write(
            repo.join("main-a.txt"),
            "main-a
",
        )
        .expect("file should write");
        run_git(repo, &["add", "."]);
        run_git(repo, &["commit", "-m", "main a"]);
        let main_a = resolve_ref_sha("HEAD")
            .expect("head should resolve")
            .expect("head sha should exist");
        record_base_ref().expect("record-base should succeed on main");

        run_git(repo, &["checkout", "-b", "side", "HEAD~1"]);
        fs::write(
            repo.join("side-b.txt"),
            "side-b
",
        )
        .expect("file should write");
        run_git(repo, &["add", "."]);
        run_git(repo, &["commit", "-m", "side b"]);
        let side_b = resolve_ref_sha("HEAD")
            .expect("head should resolve")
            .expect("head sha should exist");
        assert_ne!(side_b, main_a);

        run_git(repo, &["checkout", "--detach", "HEAD"]);

        let refreshed =
            record_base_ref().expect("record-base should refresh on detached divergent commit");
        assert_eq!(refreshed, side_b);
    });
}

#[test]
fn discover_base_prefers_recorded_ref() {
    with_temp_git_repo(|repo| {
        let main_sha = resolve_ref_sha("HEAD")
            .expect("main sha query should work")
            .expect("head should resolve");
        run_git(repo, &["branch", "-M", "main"]);
        run_git(repo, &["branch", "origin/main", &main_sha]);
        record_base_ref().expect("record-base should succeed");

        let discovered = discover_base_ref().expect("discovery should succeed");
        assert_eq!(discovered.as_deref(), Some(RECORDED_BASE_REF));
    });
}

#[test]
fn resolve_and_create_ref_error_paths_are_actionable() {
    with_temp_git_repo(|_| {
        let missing = resolve_ref_sha("refs/worktree/covgate/missing")
            .expect("query should run on missing ref");
        assert!(missing.is_none());

        let err = create_ref("refs/worktree/covgate/base", "not-a-real-target")
            .expect_err("create_ref should fail");
        assert!(err.to_string().contains("failed to update git ref"));
    });
}

#[test]
fn resolve_head_fails_outside_git_repo() {
    let _lock = CWD_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());

    let temp = tempdir().expect("tempdir should exist");
    let previous = std::env::current_dir().expect("cwd should resolve");
    let _guard = CwdGuard(previous);
    std::env::set_current_dir(temp.path()).expect("should chdir to temp");

    let err = resolve_head_sha().expect_err("HEAD lookup should fail outside git repo");
    let message = err.to_string();
    assert!(
        message.contains("failed to resolve HEAD commit")
            || message.contains("failed to run git rev-parse for HEAD"),
        "message={message}"
    );
}

#[test]
fn resolve_head_ref_and_create_ref_report_when_git_command_is_missing() {
    let _lock = CWD_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());

    with_path_override("", || {
        let head_err = resolve_head_sha().expect_err("resolve_head_sha should fail");
        assert!(
            head_err
                .to_string()
                .contains("failed to run git rev-parse for HEAD")
        );

        let resolve_err = resolve_ref_sha("HEAD").expect_err("resolve_ref_sha should fail");
        assert!(
            resolve_err
                .to_string()
                .contains("failed to run git rev-parse")
        );

        let create_err =
            create_ref("refs/worktree/covgate/base", "HEAD").expect_err("create_ref should fail");
        assert!(
            create_err
                .to_string()
                .contains("failed to run git update-ref")
        );
    });
}

#[test]
fn resolve_head_and_ref_report_non_utf8_command_output() {
    let _lock = CWD_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());

    let temp = tempdir().expect("tempdir should exist");
    let fake_git = temp.path().join("git");
    fs::write(&fake_git, "#!/usr/bin/env bash\nprintf '\\377'\nexit 0\n")
        .expect("fake git script should be written");

    let mut perms = fs::metadata(&fake_git)
        .expect("metadata should exist")
        .permissions();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        perms.set_mode(0o755);
    }
    fs::set_permissions(&fake_git, perms).expect("permissions should be updated");

    let original_path = std::env::var("PATH").unwrap_or_default();
    let path = format!("{}:{}", temp.path().to_string_lossy(), original_path);
    with_path_override(&path, || {
        let head_err = resolve_head_sha().expect_err("resolve_head_sha should fail on non-utf8");
        let head_message = head_err.to_string();
        assert!(
            head_message.contains("git rev-parse output was not valid utf-8")
                || head_message.contains("failed to resolve HEAD commit"),
            "message={head_message}"
        );

        let ref_err = resolve_ref_sha("HEAD").expect_err("resolve_ref_sha should fail on non-utf8");
        assert!(
            ref_err
                .to_string()
                .contains("git rev-parse output was not valid utf-8")
        );
    });
}
