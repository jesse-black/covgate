use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use tempfile::tempdir;

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("rust")
        .join("basic-fail")
}

#[test]
fn basic_fail_rust_fixture() {
    let temp = tempdir().expect("tempdir should exist");
    let fixture = fixture_root();
    let repo_src = fixture.join("repo");
    let overlay_src = fixture.join("overlay");
    let worktree = temp.path().join("repo");
    copy_tree(&repo_src, &worktree);
    init_git_repo(&worktree);

    copy_tree(&overlay_src, &worktree);

    let diff_output = Command::new("git")
        .args(["diff", "--unified=0", "--no-ext-diff"])
        .current_dir(&worktree)
        .output()
        .expect("git diff should run");
    assert!(diff_output.status.success(), "git diff should succeed");
    let diff_file = temp.path().join("scenario.diff");
    fs::write(&diff_file, diff_output.stdout).expect("diff file should be written");

    let coverage_json = fixture.join("coverage.json");
    let binary = env!("CARGO_BIN_EXE_covgate");
    let output = Command::new(binary)
        .args([
            "--coverage-json",
            coverage_json.to_str().expect("utf8 path"),
            "--diff-file",
            diff_file.to_str().expect("utf8 path"),
            "--fail-under-regions",
            "60",
        ])
        .current_dir(&worktree)
        .output()
        .expect("covgate should run");

    assert_eq!(
        output.status.code(),
        Some(1),
        "fixture should fail the gate"
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("Diff Coverage: FAIL"));
    assert!(stdout.contains("src/lib.rs"));
    assert!(stdout.contains("Coverage: 50.00%"));
}

#[test]
fn uses_repo_config_defaults_for_base_and_threshold() {
    let temp = tempdir().expect("tempdir should exist");
    let fixture = fixture_root();
    let repo_src = fixture.join("repo");
    let overlay_src = fixture.join("overlay");
    let worktree = temp.path().join("repo");
    copy_tree(&repo_src, &worktree);
    init_git_repo(&worktree);
    run_git(&worktree, &["branch", "-M", "main"]);
    run_git(&worktree, &["checkout", "-b", "feature/config-defaults"]);

    copy_tree(&overlay_src, &worktree);
    run_git(&worktree, &["add", "."]);
    run_git(&worktree, &["commit", "-m", "feature change"]);
    fs::write(
        worktree.join("covgate.toml"),
        "base = \"main\"\n[thresholds]\nregions = 40\n",
    )
    .expect("config should be written");

    let coverage_json = fixture.join("coverage.json");
    let binary = env!("CARGO_BIN_EXE_covgate");
    let output = Command::new(binary)
        .args([
            "--coverage-json",
            coverage_json.to_str().expect("utf8 path"),
        ])
        .current_dir(&worktree)
        .output()
        .expect("covgate should run");

    assert_eq!(
        output.status.code(),
        Some(0),
        "config defaults should allow the gate to pass"
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("Diff: main...HEAD"));
    assert!(stdout.contains("Threshold: 40.00%"));
    assert!(stdout.contains("Coverage: 50.00%"));
}

#[test]
fn cli_threshold_overrides_repo_config_default() {
    let temp = tempdir().expect("tempdir should exist");
    let fixture = fixture_root();
    let repo_src = fixture.join("repo");
    let overlay_src = fixture.join("overlay");
    let worktree = temp.path().join("repo");
    copy_tree(&repo_src, &worktree);
    init_git_repo(&worktree);
    run_git(&worktree, &["branch", "-M", "main"]);
    run_git(&worktree, &["checkout", "-b", "feature/cli-override"]);

    copy_tree(&overlay_src, &worktree);
    run_git(&worktree, &["add", "."]);
    run_git(&worktree, &["commit", "-m", "feature change"]);
    fs::write(
        worktree.join("covgate.toml"),
        "base = \"main\"\n[thresholds]\nregions = 40\n",
    )
    .expect("config should be written");

    let coverage_json = fixture.join("coverage.json");
    let binary = env!("CARGO_BIN_EXE_covgate");
    let output = Command::new(binary)
        .args([
            "--coverage-json",
            coverage_json.to_str().expect("utf8 path"),
            "--fail-under-regions",
            "60",
        ])
        .current_dir(&worktree)
        .output()
        .expect("covgate should run");

    assert_eq!(
        output.status.code(),
        Some(1),
        "cli threshold should override the repo config default"
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("Diff: main...HEAD"));
    assert!(stdout.contains("Threshold: 60.00%"));
    assert!(stdout.contains("Diff Coverage: FAIL"));
}

fn init_git_repo(path: &Path) {
    run_git(path, &["init"]);
    run_git(path, &["config", "user.email", "covgate@example.com"]);
    run_git(path, &["config", "user.name", "Covgate Tests"]);
    run_git(path, &["add", "."]);
    run_git(path, &["commit", "-m", "baseline"]);
}

fn run_git(path: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(path)
        .output()
        .expect("git command should run");
    assert!(
        output.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
}

fn copy_tree(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).expect("destination tree should exist");
    for entry in fs::read_dir(source).expect("fixture tree should be readable") {
        let entry = entry.expect("dir entry");
        let file_type = entry.file_type().expect("file type");
        let dest = destination.join(entry.file_name());
        if file_type.is_dir() {
            copy_tree(&entry.path(), &dest);
        } else {
            fs::copy(entry.path(), dest).expect("fixture file should copy");
        }
    }
}
