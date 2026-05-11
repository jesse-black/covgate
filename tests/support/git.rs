#![allow(dead_code)]

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use super::fixtures::Fixture;

pub fn init_git_repo(path: &Path) {
    run_git(path, &["init"]);
    run_git(path, &["config", "user.email", "covgate@example.com"]);
    run_git(path, &["config", "user.name", "Covgate Tests"]);
    run_git(path, &["add", "."]);
    run_git(path, &["commit", "-m", "baseline"]);
}

pub fn run_git(path: &Path, args: &[&str]) {
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

pub fn copy_tree(source: &Path, destination: &Path) {
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

pub fn write_worktree_diff(temp_root: &Path, worktree: &Path) -> PathBuf {
    let diff_output = Command::new("git")
        .args(["diff", "--unified=0", "--no-ext-diff"])
        .current_dir(worktree)
        .output()
        .expect("git diff should run");
    assert!(diff_output.status.success(), "git diff should succeed");
    let diff_file = temp_root.join("scenario.diff");
    fs::write(&diff_file, diff_output.stdout).expect("diff file should be written");
    diff_file
}

pub fn setup_fixture_worktree(temp_root: &Path, fixture: Fixture) -> PathBuf {
    let fixture_root = fixture.root();
    let repo_src = fixture_root.join("repo");
    let overlay_src = fixture_root.join("overlay");
    let worktree = temp_root.join("repo");
    copy_tree(&repo_src, &worktree);
    init_git_repo(&worktree);
    copy_tree(&overlay_src, &worktree);
    worktree
}
