mod support;

use std::{fs, path::PathBuf};

use tempfile::tempdir;

use crate::support::{
    copy_tree, covgate, init_git_repo, run_git, rust_basic_fail_fixture, rust_basic_pass_fixture,
    setup_fixture_worktree, vitest_path_scoped_gates_fixture, write_absolute_path_coverage_fixture,
    write_worktree_diff,
};

fn setup_path_scoped_fixture() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let fixture = vitest_path_scoped_gates_fixture();
    let temp = tempdir().expect("tempdir should exist");
    let worktree = setup_fixture_worktree(temp.path(), fixture);
    let diff_file = write_worktree_diff(temp.path(), &worktree);
    (temp, worktree, diff_file)
}

#[test]
fn record_base_noops_when_standard_base_ref_is_available() {
    let fixture = rust_basic_pass_fixture();
    let temp = tempdir().expect("tempdir should exist");
    let worktree = setup_fixture_worktree(temp.path(), fixture);
    run_git(&worktree, &["branch", "-M", "main"]);

    let output = covgate(&worktree).arg("record-base").run();
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(
        stdout.contains("record-base` is unnecessary"),
        "stdout={stdout}"
    );
    assert!(
        stdout.contains("Base ref `main` is available"),
        "stdout={stdout}"
    );

    let ref_sha = std::process::Command::new("git")
        .args(["rev-parse", "--verify", "refs/worktree/covgate/base"])
        .current_dir(&worktree)
        .output()
        .expect("git rev-parse should run");
    assert!(!ref_sha.status.success(), "stdout={stdout}");
}

#[test]
fn record_base_creates_worktree_ref_in_constrained_repo() {
    let fixture = rust_basic_pass_fixture();
    let temp = tempdir().expect("tempdir should exist");
    let worktree = setup_fixture_worktree(temp.path(), fixture);
    run_git(&worktree, &["branch", "-M", "task/record-base"]);

    let output = covgate(&worktree).arg("record-base").run();
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("Recorded base commit"), "stdout={stdout}");
    assert!(
        stdout.contains("refs/worktree/covgate/base"),
        "stdout={stdout}"
    );

    let ref_sha = std::process::Command::new("git")
        .args(["rev-parse", "--verify", "refs/worktree/covgate/base"])
        .current_dir(&worktree)
        .output()
        .expect("git rev-parse should run");
    assert!(ref_sha.status.success(), "stderr={:?}", ref_sha.stderr);
}

#[test]
fn record_base_does_not_break_git_ref_enumeration() {
    let fixture = rust_basic_pass_fixture();
    let temp = tempdir().expect("tempdir should exist");
    let worktree = setup_fixture_worktree(temp.path(), fixture);
    run_git(&worktree, &["branch", "-M", "task/record-base"]);

    let output = covgate(&worktree).arg("record-base").run();
    assert_eq!(output.status.code(), Some(0));

    let show_ref = std::process::Command::new("git")
        .args(["show-ref"])
        .current_dir(&worktree)
        .output()
        .expect("git show-ref should run");
    assert!(
        show_ref.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&show_ref.stderr)
    );
}

#[test]
fn record_base_fails_outside_git_repo() {
    let temp = tempdir().expect("tempdir should exist");

    let output = covgate(temp.path()).arg("record-base").run();
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("covgate requires a git repository to run"),
        "stderr={stderr}"
    );
}

#[test]
fn record_base_fails_fast_when_git_is_missing() {
    let temp = tempdir().expect("tempdir should exist");

    let output = covgate(temp.path())
        .arg("record-base")
        .env("PATH", "")
        .run();

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("covgate requires `git` in PATH to run"),
        "stderr={stderr}"
    );
}

#[test]
fn missing_check_subcommand_is_reported_as_clap_usage_error() {
    let temp = tempdir().expect("tempdir should exist");

    let output = covgate(temp.path()).run();
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("Usage: covgate <COMMAND>"),
        "stderr={stderr}"
    );
    assert!(stderr.contains("Commands:"), "stderr={stderr}");
}

#[test]
fn missing_check_coverage_report_is_reported_as_clap_usage_error() {
    let temp = tempdir().expect("tempdir should exist");

    let output = covgate(temp.path()).arg("check").run();
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("the following required arguments were not provided"),
        "stderr={stderr}"
    );
    assert!(stderr.contains("<COVERAGE_REPORT>"), "stderr={stderr}");
}

#[test]
fn check_fails_fast_when_git_is_missing() {
    let temp = tempdir().expect("tempdir should exist");

    let output = covgate(temp.path())
        .args(["check", "missing.json"])
        .env("PATH", "")
        .run();

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("covgate requires `git` in PATH to run"),
        "stderr={stderr}"
    );
}

#[test]
fn help_lists_record_base_as_subcommand() {
    let temp = tempdir().expect("tempdir should exist");

    let output = covgate(temp.path()).arg("--help").run();
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("Commands:"), "stdout={stdout}");
    assert!(stdout.contains("check"), "stdout={stdout}");
    assert!(stdout.contains("record-base"), "stdout={stdout}");
    assert!(!stdout.contains("./covgate.toml"), "stdout={stdout}");
    assert!(
        !stdout.contains("Supported defaults in v1"),
        "stdout={stdout}"
    );
    assert!(!stdout.contains("Agent workflow"), "stdout={stdout}");
}

#[test]
fn version_switches_report_current_binary_version() {
    let temp = tempdir().expect("tempdir should exist");

    for args in [vec!["--version".to_string()], vec!["-V".to_string()]] {
        let output = covgate(temp.path())
            .args(args.iter().map(String::as_str))
            .run();
        assert_eq!(output.status.code(), Some(0), "args={args:?}");
        let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
        assert!(
            stdout.contains(env!("CARGO_PKG_VERSION")),
            "args={args:?} stdout={stdout}"
        );
    }
}

#[test]
fn check_help_describes_arguments_and_options() {
    let temp = tempdir().expect("tempdir should exist");

    let output = covgate(temp.path()).args(["check", "--help"]).run();
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("Arguments:"), "stdout={stdout}");
    assert!(stdout.contains("Coverage report path"), "stdout={stdout}");
    assert!(stdout.contains("Options:"), "stdout={stdout}");
    assert!(
        stdout.contains("Git base reference to diff against"),
        "stdout={stdout}"
    );
    assert!(
        stdout.contains("Precomputed unified diff file"),
        "stdout={stdout}"
    );
    assert!(!stdout.contains("--fail-under-regions"), "stdout={stdout}");
    assert!(
        stdout.contains("Write a Markdown summary to this file"),
        "stdout={stdout}"
    );
}

#[test]
fn record_base_help_is_user_focused() {
    let temp = tempdir().expect("tempdir should exist");

    let output = covgate(temp.path()).args(["record-base", "--help"]).run();
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(
        stdout.contains("stable task-start base for constrained cloud-agent worktrees"),
        "stdout={stdout}"
    );
    assert!(
        stdout.contains("Run it once at the start of a task before"),
        "stdout={stdout}"
    );
    assert!(stdout.contains("making Git changes"), "stdout={stdout}");
    assert!(
        stdout.contains("covgate check <coverage-report>"),
        "stdout={stdout}"
    );
    assert!(
        !stdout.contains("refs/worktree/covgate/base"),
        "stdout={stdout}"
    );
}

#[test]
fn record_base_is_idempotent() {
    let fixture = rust_basic_pass_fixture();
    let temp = tempdir().expect("tempdir should exist");
    let worktree = setup_fixture_worktree(temp.path(), fixture);
    run_git(&worktree, &["branch", "-M", "task/idempotent"]);

    let first = covgate(&worktree).arg("record-base").run();
    assert_eq!(first.status.code(), Some(0));
    let first_ref = std::process::Command::new("git")
        .args(["rev-parse", "--verify", "refs/worktree/covgate/base"])
        .current_dir(&worktree)
        .output()
        .expect("git rev-parse should run");
    let first_sha = String::from_utf8(first_ref.stdout).expect("sha should be utf8");

    fs::write(worktree.join("idempotent.txt"), "change\n").expect("file should write");
    run_git(&worktree, &["add", "."]);
    run_git(&worktree, &["commit", "-m", "change after record-base"]);

    let second = covgate(&worktree).arg("record-base").run();
    assert_eq!(second.status.code(), Some(0));
    let second_stdout = String::from_utf8(second.stdout).expect("stdout should be utf8");
    assert!(second_stdout.contains("Base already recorded"));

    let second_ref = std::process::Command::new("git")
        .args(["rev-parse", "--verify", "refs/worktree/covgate/base"])
        .current_dir(&worktree)
        .output()
        .expect("git rev-parse should run");
    let second_sha = String::from_utf8(second_ref.stdout).expect("sha should be utf8");
    assert_eq!(second_sha.trim(), first_sha.trim());
}

#[test]
fn record_base_refreshes_after_branch_switch() {
    let fixture = rust_basic_pass_fixture();
    let temp = tempdir().expect("tempdir should exist");
    let worktree = setup_fixture_worktree(temp.path(), fixture);
    run_git(&worktree, &["branch", "-M", "task/base"]);

    let first = covgate(&worktree).arg("record-base").run();
    assert_eq!(first.status.code(), Some(0));
    let first_ref = std::process::Command::new("git")
        .args(["rev-parse", "--verify", "refs/worktree/covgate/base"])
        .current_dir(&worktree)
        .output()
        .expect("git rev-parse should run");
    let first_sha = String::from_utf8(first_ref.stdout).expect("sha should be utf8");

    run_git(&worktree, &["checkout", "-b", "task/refresh"]);
    fs::write(worktree.join("refresh.txt"), "refresh\n").expect("file should write");
    run_git(&worktree, &["add", "."]);
    run_git(&worktree, &["commit", "-m", "refresh branch work"]);

    let second = covgate(&worktree).arg("record-base").run();
    assert_eq!(second.status.code(), Some(0));
    let second_stdout = String::from_utf8(second.stdout).expect("stdout should be utf8");
    assert!(
        second_stdout.contains("Refreshed base commit"),
        "stdout={second_stdout}"
    );
    assert!(
        second_stdout.contains("for branch task/refresh"),
        "stdout={second_stdout}"
    );

    let second_ref = std::process::Command::new("git")
        .args(["rev-parse", "--verify", "refs/worktree/covgate/base"])
        .current_dir(&worktree)
        .output()
        .expect("git rev-parse should run");
    let second_sha = String::from_utf8(second_ref.stdout).expect("sha should be utf8");
    assert_ne!(second_sha.trim(), first_sha.trim());
}

#[test]
fn covgate_includes_dirty_worktree_changes_by_default() {
    let fixture = rust_basic_pass_fixture();
    let temp = tempdir().expect("tempdir should exist");
    let worktree = setup_fixture_worktree(temp.path(), fixture);

    fs::write(
        worktree.join("dirty.txt"),
        "dirty
",
    )
    .expect("dirty file should write");
    fs::write(
        worktree.join("covgate.toml"),
        "[[gates]]\nfail-under-regions = 90\n",
    )
    .expect("config should be written");

    let output = covgate(&worktree).check(&fixture.coverage_json()).run();

    assert_eq!(output.status.code(), Some(0));
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        !stderr.contains("working tree has uncommitted changes"),
        "stderr={stderr}"
    );
}

#[test]
fn diff_file_mode_skips_dirty_worktree_guard() {
    let fixture = rust_basic_pass_fixture();
    let temp = tempdir().expect("tempdir should exist");
    let worktree = setup_fixture_worktree(temp.path(), fixture);
    let diff_file = write_worktree_diff(temp.path(), &worktree);
    fs::write(
        worktree.join("covgate.toml"),
        "[[gates]]\nfail-under-regions = 90\n",
    )
    .expect("config should be written");

    let output = covgate(&worktree)
        .check(&fixture.coverage_json())
        .args([
            "--diff-file".into(),
            diff_file.to_string_lossy().into_owned(),
        ])
        .run();

    assert_eq!(output.status.code(), Some(0));
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        !stderr.contains("working tree has uncommitted changes"),
        "stderr={stderr}"
    );
}

#[test]
fn git_base_mode_errors_on_coverage_untracked_files() {
    let fixture = rust_basic_pass_fixture();
    let temp = tempdir().expect("tempdir should exist");
    let worktree = setup_fixture_worktree(temp.path(), fixture);
    fs::write(
        worktree.join("covgate.toml"),
        "[[gates]]\nfail-under-regions = 90\n",
    )
    .expect("config should be written");
    run_git(&worktree, &["add", "covgate.toml"]);
    run_git(&worktree, &["commit", "-m", "add covgate config"]);

    run_git(&worktree, &["rm", "--cached", "src/lib.rs"]);

    let output = covgate(&worktree).check(&fixture.coverage_json()).run();

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(stderr.contains("false pass"), "stderr={stderr}");
    assert!(stderr.contains("git add -N src/lib.rs"), "stderr={stderr}");
}

#[test]
fn git_base_mode_passes_for_uncovered_untracked_file() {
    let fixture = rust_basic_pass_fixture();
    let temp = tempdir().expect("tempdir should exist");
    let worktree = setup_fixture_worktree(temp.path(), fixture);
    fs::write(
        worktree.join("covgate.toml"),
        "[[gates]]\nfail-under-regions = 90\n",
    )
    .expect("config should be written");
    run_git(&worktree, &["add", "covgate.toml"]);
    run_git(&worktree, &["commit", "-m", "add covgate config"]);

    fs::write(worktree.join("new_untracked.rs"), "pub fn pending() {}\n")
        .expect("untracked file should write");

    let output = covgate(&worktree).check(&fixture.coverage_json()).run();

    assert_eq!(output.status.code(), Some(0));
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(!stderr.contains("false pass"), "stderr={stderr}");
}

#[test]
fn diff_file_mode_skips_untracked_files_check() {
    let fixture = rust_basic_pass_fixture();
    let temp = tempdir().expect("tempdir should exist");
    let worktree = setup_fixture_worktree(temp.path(), fixture);
    let diff_file = write_worktree_diff(temp.path(), &worktree);

    run_git(&worktree, &["rm", "--cached", "src/lib.rs"]);
    fs::write(
        worktree.join("covgate.toml"),
        "[[gates]]\nfail-under-regions = 90\n",
    )
    .expect("config should be written");

    let output = covgate(&worktree)
        .check(&fixture.coverage_json())
        .args([
            "--diff-file".into(),
            diff_file.to_string_lossy().into_owned(),
        ])
        .run();

    assert_eq!(output.status.code(), Some(0));
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(!stderr.contains("false pass"), "stderr={stderr}");
}

#[test]
fn automatic_base_prefers_standard_branch_ref_over_recorded_worktree_ref() {
    let fixture = rust_basic_pass_fixture();
    let temp = tempdir().expect("tempdir should exist");
    let fixture_root = fixture.root();
    let repo_src = fixture_root.join("repo");
    let overlay_src = fixture_root.join("overlay");
    let worktree = temp.path().join("repo");
    copy_tree(&repo_src, &worktree);
    init_git_repo(&worktree);
    run_git(&worktree, &["branch", "-M", "task/recorded-base"]);

    let output = covgate(&worktree).arg("record-base").run();
    assert_eq!(output.status.code(), Some(0));
    run_git(&worktree, &["branch", "main", "HEAD"]);

    copy_tree(&overlay_src, &worktree);
    fs::write(
        worktree.join("covgate.toml"),
        "[[gates]]\nfail-under-regions = 90\n",
    )
    .expect("config should be written");
    run_git(&worktree, &["add", "."]);
    run_git(&worktree, &["commit", "-m", "feature change"]);

    let output = covgate(&worktree).check(&fixture.coverage_json()).run();

    assert_eq!(output.status.code(), Some(0));
}

#[test]
fn explicit_base_overrides_recorded_worktree_ref() {
    let fixture = rust_basic_pass_fixture();
    let temp = tempdir().expect("tempdir should exist");
    let fixture_root = fixture.root();
    let repo_src = fixture_root.join("repo");
    let overlay_src = fixture_root.join("overlay");
    let worktree = temp.path().join("repo");
    copy_tree(&repo_src, &worktree);
    init_git_repo(&worktree);
    run_git(&worktree, &["branch", "-M", "task/explicit-base"]);

    let output = covgate(&worktree).arg("record-base").run();
    assert_eq!(output.status.code(), Some(0));
    run_git(&worktree, &["branch", "main", "HEAD"]);

    copy_tree(&overlay_src, &worktree);
    fs::write(
        worktree.join("covgate.toml"),
        "[[gates]]\nfail-under-regions = 90\n",
    )
    .expect("config should be written");
    run_git(&worktree, &["add", "."]);
    run_git(&worktree, &["commit", "-m", "feature change"]);

    let output = covgate(&worktree)
        .check(&fixture.coverage_json())
        .args(["--base", "main"])
        .run();

    assert_eq!(output.status.code(), Some(0));
}

#[test]
fn failure_text_requires_git_repo_when_run_outside_repository() {
    let fixture = rust_basic_pass_fixture();
    let temp = tempdir().expect("tempdir should exist");

    let output = covgate(temp.path()).check(&fixture.coverage_json()).run();

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("covgate requires a git repository to run"),
        "stderr={stderr}"
    );
}

#[test]
fn markdown_summary_rust_fixture() {
    let fixture = rust_basic_pass_fixture();
    let temp = tempdir().expect("tempdir should exist");
    let worktree = setup_fixture_worktree(temp.path(), fixture);
    let diff_file = write_worktree_diff(temp.path(), &worktree);
    let markdown_output = temp.path().join("summary.md");
    fs::write(
        worktree.join("covgate.toml"),
        "[[gates]]\nfail-under-regions = 90\n",
    )
    .expect("config should be written");

    let output = covgate(&worktree)
        .check(&fixture.coverage_json())
        .args([
            "--diff-file".into(),
            diff_file.to_string_lossy().into_owned(),
            "--markdown-output".into(),
            markdown_output.to_string_lossy().into_owned(),
        ])
        .run();

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("PASS Regions:"));
    assert!(markdown_output.exists(), "markdown file should be written");

    let markdown = fs::read_to_string(markdown_output).expect("markdown should be readable");
    assert!(markdown.contains("## Covgate"));
    assert!(markdown.contains("### Diff Coverage"));
    assert!(markdown.contains("| Result | Rule | Observed | Configured |"));
    assert!(markdown.contains("| ✅PASS | `fail-under-regions` | 100.00% ("));
    assert!(markdown.contains(") | ≥ 90.00% |"));
    assert!(markdown.contains(
        "| File | Covered Changed Regions | Changed Regions | Coverage | Missed Changed Spans |"
    ));
    assert!(markdown.contains("### Overall Coverage"));
    assert!(markdown.contains("#### Region"));
    assert!(markdown.contains("#### Line"));
    assert!(markdown.contains("#### Function"));
    assert!(markdown.contains("| File | Covered Regions | Regions | Missed Regions | Coverage |"));
    assert!(markdown.contains("| **Total** | **"));
}

#[test]
fn markdown_output_to_stdout() {
    let fixture = rust_basic_pass_fixture();
    let temp = tempdir().expect("tempdir should exist");
    let worktree = setup_fixture_worktree(temp.path(), fixture);
    let diff_file = write_worktree_diff(temp.path(), &worktree);
    fs::write(
        worktree.join("covgate.toml"),
        "[[gates]]\nfail-under-regions = 90\n",
    )
    .expect("config should be written");

    let output = covgate(&worktree)
        .check(&fixture.coverage_json())
        .args([
            "--diff-file".into(),
            diff_file.to_string_lossy().into_owned(),
            "--markdown-output".into(),
            "-".to_string(),
        ])
        .run();

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("PASS Regions:"), "stdout={stdout}");
    assert!(stdout.contains("## Covgate"), "stdout={stdout}");
    assert!(
        stdout.contains("| Result | Rule | Observed | Configured |"),
        "stdout={stdout}"
    );
}

#[test]
fn markdown_output_to_file_regression() {
    let fixture = rust_basic_pass_fixture();
    let temp = tempdir().expect("tempdir should exist");
    let worktree = setup_fixture_worktree(temp.path(), fixture);
    let diff_file = write_worktree_diff(temp.path(), &worktree);
    let markdown_output = temp.path().join("summary.md");
    fs::write(
        worktree.join("covgate.toml"),
        "[[gates]]\nfail-under-regions = 90\n",
    )
    .expect("config should be written");

    let output = covgate(&worktree)
        .check(&fixture.coverage_json())
        .args([
            "--diff-file".into(),
            diff_file.to_string_lossy().into_owned(),
            "--markdown-output".into(),
            markdown_output.to_string_lossy().into_owned(),
        ])
        .run();

    assert_eq!(output.status.code(), Some(0));
    let markdown = fs::read_to_string(markdown_output).expect("markdown should be readable");
    assert!(markdown.contains("## Covgate"));
    assert!(markdown.contains("| Result | Rule | Observed | Configured |"));
}

#[test]
fn github_step_summary_auto_detected() {
    let fixture = rust_basic_pass_fixture();
    let temp = tempdir().expect("tempdir should exist");
    let worktree = setup_fixture_worktree(temp.path(), fixture);
    let diff_file = write_worktree_diff(temp.path(), &worktree);
    let summary_output = temp.path().join("github-step-summary.md");
    fs::write(
        worktree.join("covgate.toml"),
        "[[gates]]\nfail-under-regions = 90\n",
    )
    .expect("config should be written");

    let output = covgate(&worktree)
        .check(&fixture.coverage_json())
        .args([
            "--diff-file".into(),
            diff_file.to_string_lossy().into_owned(),
        ])
        .env(
            "GITHUB_STEP_SUMMARY",
            summary_output.to_str().expect("path should be utf8"),
        )
        .run();

    assert_eq!(output.status.code(), Some(0));
    let markdown = fs::read_to_string(summary_output).expect("summary should be readable");
    assert!(markdown.contains("## Covgate"));
    assert!(markdown.contains("| Result | Rule | Observed | Configured |"));
}

#[test]
fn github_step_summary_suppressed_by_flag() {
    let fixture = rust_basic_pass_fixture();
    let temp = tempdir().expect("tempdir should exist");
    let worktree = setup_fixture_worktree(temp.path(), fixture);
    let diff_file = write_worktree_diff(temp.path(), &worktree);
    let summary_output = temp.path().join("github-step-summary.md");
    fs::write(
        worktree.join("covgate.toml"),
        "[[gates]]\nfail-under-regions = 90\n",
    )
    .expect("config should be written");

    let output = covgate(&worktree)
        .check(&fixture.coverage_json())
        .args([
            "--diff-file".into(),
            diff_file.to_string_lossy().into_owned(),
            "--no-github-summary".into(),
        ])
        .env(
            "GITHUB_STEP_SUMMARY",
            summary_output.to_str().expect("path should be utf8"),
        )
        .run();

    assert_eq!(output.status.code(), Some(0));
    assert!(
        !summary_output.exists(),
        "summary file should not be written"
    );
}

#[test]
fn github_step_summary_and_explicit_output_both_write() {
    let fixture = rust_basic_pass_fixture();
    let temp = tempdir().expect("tempdir should exist");
    let worktree = setup_fixture_worktree(temp.path(), fixture);
    let diff_file = write_worktree_diff(temp.path(), &worktree);
    let markdown_output = temp.path().join("summary.md");
    let summary_output = temp.path().join("github-step-summary.md");
    fs::write(
        worktree.join("covgate.toml"),
        "[[gates]]\nfail-under-regions = 90\n",
    )
    .expect("config should be written");

    let output = covgate(&worktree)
        .check(&fixture.coverage_json())
        .args([
            "--diff-file".into(),
            diff_file.to_string_lossy().into_owned(),
            "--markdown-output".into(),
            markdown_output.to_string_lossy().into_owned(),
        ])
        .env(
            "GITHUB_STEP_SUMMARY",
            summary_output.to_str().expect("path should be utf8"),
        )
        .run();

    assert_eq!(output.status.code(), Some(0));
    let markdown = fs::read_to_string(markdown_output).expect("markdown should be readable");
    let summary = fs::read_to_string(summary_output).expect("summary should be readable");
    assert!(markdown.contains("## Covgate"));
    assert!(summary.contains("## Covgate"));
}

#[test]
fn github_step_summary_matching_explicit_output_writes_once() {
    let fixture = rust_basic_pass_fixture();
    let temp = tempdir().expect("tempdir should exist");
    let worktree = setup_fixture_worktree(temp.path(), fixture);
    let diff_file = write_worktree_diff(temp.path(), &worktree);
    let markdown_output = temp.path().join("summary.md");
    fs::write(
        worktree.join("covgate.toml"),
        "[[gates]]\nfail-under-regions = 90\n",
    )
    .expect("config should be written");

    let output = covgate(&worktree)
        .check(&fixture.coverage_json())
        .args([
            "--diff-file".into(),
            diff_file.to_string_lossy().into_owned(),
            "--markdown-output".into(),
            markdown_output.to_string_lossy().into_owned(),
        ])
        .env(
            "GITHUB_STEP_SUMMARY",
            markdown_output.to_str().expect("path should be utf8"),
        )
        .run();

    assert_eq!(output.status.code(), Some(0));
    let markdown = fs::read_to_string(markdown_output).expect("markdown should be readable");
    assert_eq!(markdown.matches("## Covgate").count(), 1, "{markdown}");
}

#[test]
fn markdown_output_file_write_error_is_reported() {
    let fixture = rust_basic_pass_fixture();
    let temp = tempdir().expect("tempdir should exist");
    let worktree = setup_fixture_worktree(temp.path(), fixture);
    let diff_file = write_worktree_diff(temp.path(), &worktree);
    fs::write(
        worktree.join("covgate.toml"),
        "[[gates]]\nfail-under-regions = 90\n",
    )
    .expect("config should be written");

    let output = covgate(&worktree)
        .check(&fixture.coverage_json())
        .args([
            "--diff-file".into(),
            diff_file.to_string_lossy().into_owned(),
            "--markdown-output".into(),
            temp.path().to_string_lossy().into_owned(),
        ])
        .run();

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("failed to write markdown output"),
        "stderr={stderr}"
    );
    assert!(
        stderr.contains(&temp.path().display().to_string()),
        "stderr={stderr}"
    );
}

#[test]
fn github_step_summary_write_error_is_reported() {
    let fixture = rust_basic_pass_fixture();
    let temp = tempdir().expect("tempdir should exist");
    let worktree = setup_fixture_worktree(temp.path(), fixture);
    let diff_file = write_worktree_diff(temp.path(), &worktree);
    fs::write(
        worktree.join("covgate.toml"),
        "[[gates]]\nfail-under-regions = 90\n",
    )
    .expect("config should be written");

    let output = covgate(&worktree)
        .check(&fixture.coverage_json())
        .args([
            "--diff-file".into(),
            diff_file.to_string_lossy().into_owned(),
        ])
        .env("GITHUB_STEP_SUMMARY", "/dev/full")
        .run();

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("failed to write GitHub step summary"),
        "stderr={stderr}"
    );
    assert!(stderr.contains("/dev/full"), "stderr={stderr}");
}

#[test]
fn github_step_summary_open_error_is_reported() {
    let fixture = rust_basic_pass_fixture();
    let temp = tempdir().expect("tempdir should exist");
    let worktree = setup_fixture_worktree(temp.path(), fixture);
    let diff_file = write_worktree_diff(temp.path(), &worktree);
    fs::write(
        worktree.join("covgate.toml"),
        "[[gates]]\nfail-under-regions = 90\n",
    )
    .expect("config should be written");

    let output = covgate(&worktree)
        .check(&fixture.coverage_json())
        .args([
            "--diff-file".into(),
            diff_file.to_string_lossy().into_owned(),
        ])
        .env(
            "GITHUB_STEP_SUMMARY",
            temp.path().to_str().expect("path should be utf8"),
        )
        .run();

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("failed to open GitHub step summary"),
        "stderr={stderr}"
    );
    assert!(
        stderr.contains(&temp.path().display().to_string()),
        "stderr={stderr}"
    );
}

#[test]
fn path_scoped_gates_render_labeled_minimal_output() {
    let fixture = vitest_path_scoped_gates_fixture();
    let (_temp, worktree, diff_file) = setup_path_scoped_fixture();
    fs::write(
        worktree.join("covgate.toml"),
        "[[gates]]\nname = \"js-logic\"\ninclude = [\"**/*.ts\"]\nexclude = [\"**/*.tsx\"]\nfail-under-lines = 30\nfail-under-branches = 20\nfail-under-functions = 40\n\n[[gates]]\nname = \"js-ui\"\ninclude = [\"**/*.tsx\"]\nfail-under-lines = 90\nfail-under-branches = 100\nfail-under-functions = 70\n",
    )
    .expect("config should be written");

    let output = covgate(&worktree)
        .check(&fixture.coverage_json())
        .args([
            "--diff-file".into(),
            diff_file.to_string_lossy().into_owned(),
        ])
        .run();

    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("[js-logic] PASS Lines:"), "stdout={stdout}");
    assert!(
        stdout.contains("[js-logic] PASS Branches:"),
        "stdout={stdout}"
    );
    assert!(stdout.contains("[js-ui] FAIL Lines:"), "stdout={stdout}");
    assert!(
        stdout.contains("[js-ui] FAIL Functions:"),
        "stdout={stdout}"
    );
}

#[test]
fn path_scoped_gates_accept_single_string_include_and_exclude() {
    let fixture = vitest_path_scoped_gates_fixture();
    let (_temp, worktree, diff_file) = setup_path_scoped_fixture();
    fs::write(
        worktree.join("covgate.toml"),
        "[[gates]]\nname = \"js-logic\"\ninclude = \"**/*\"\nexclude = \"**/*.tsx\"\nfail-under-lines = 30\n\n[[gates]]\nname = \"js-ui\"\ninclude = \"**/*.tsx\"\nfail-under-lines = 70\n",
    )
    .expect("config should be written");

    let output = covgate(&worktree)
        .check(&fixture.coverage_json())
        .args([
            "--diff-file".into(),
            diff_file.to_string_lossy().into_owned(),
        ])
        .run();

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("[js-logic] PASS Lines:"), "stdout={stdout}");
    assert!(stdout.contains("[js-ui] PASS Lines:"), "stdout={stdout}");
}

#[test]
fn path_scoped_gates_markdown_adds_gate_column() {
    let fixture = vitest_path_scoped_gates_fixture();
    let (temp, worktree, diff_file) = setup_path_scoped_fixture();
    let markdown_output = temp.path().join("summary.md");
    fs::write(
        worktree.join("covgate.toml"),
        "[[gates]]\nname = \"js-ui\"\ninclude = [\"**/*.tsx\"]\nfail-under-lines = 70\n\n[[gates]]\nfail-under-lines = 40\n",
    )
    .expect("config should be written");

    let output = covgate(&worktree)
        .check(&fixture.coverage_json())
        .args([
            "--diff-file".into(),
            diff_file.to_string_lossy().into_owned(),
            "--markdown-output".into(),
            markdown_output.to_string_lossy().into_owned(),
        ])
        .run();

    assert_eq!(output.status.code(), Some(1));
    let markdown = fs::read_to_string(markdown_output).expect("markdown should be readable");
    assert!(markdown.contains("| Gate | Result | Rule | Observed | Configured |"));
    assert!(
        markdown.contains("| `js-ui` | ✅PASS | `fail-under-lines` | 80.00% (4/5) | ≥ 70.00% |")
    );
    assert!(
        markdown.contains("| `default` | ❌FAIL | `fail-under-lines` | 33.33% (2/6) | ≥ 40.00% |")
    );
    assert!(markdown.contains(
        "| File | Covered Changed Lines | Changed Lines | Coverage | Missed Changed Spans |"
    ));
    assert!(!markdown.contains("| Gate | File |"));
    assert!(!markdown.contains("default Total"));
}

#[test]
fn path_scoped_gates_markdown_labels_single_named_fallback() {
    let fixture = vitest_path_scoped_gates_fixture();
    let (temp, worktree, diff_file) = setup_path_scoped_fixture();
    let markdown_output = temp.path().join("summary.md");
    fs::write(
        worktree.join("covgate.toml"),
        "[[gates]]\nname = \"ui\"\ninclude = [\"**/*.jsx\"]\nfail-under-lines = 80\nfail-under-regions = 80\n\n[[gates]]\nname = \"logic\"\nfail-under-lines = 30\n",
    )
    .expect("config should be written");

    let output = covgate(&worktree)
        .check(&fixture.coverage_json())
        .args([
            "--diff-file".into(),
            diff_file.to_string_lossy().into_owned(),
            "--markdown-output".into(),
            markdown_output.to_string_lossy().into_owned(),
        ])
        .run();

    assert_eq!(output.status.code(), Some(0));
    let markdown = fs::read_to_string(markdown_output).expect("markdown should be readable");
    assert!(markdown.contains("| Gate | Result | Rule | Observed | Configured |"));
    assert!(markdown.contains("| `ui` | ✅PASS | `fail-under-lines` | N/A (0/0) | ≥ 80.00% |"));
    assert!(markdown.contains("| `ui` | ✅PASS | `fail-under-regions` | N/A (0/0) | ≥ 80.00% |"));
    assert!(markdown.contains("| `logic` | ✅PASS | `fail-under-lines` |"));
    assert!(markdown.contains("| ≥ 30.00% |"));
    assert!(markdown.contains(
        "| File | Covered Changed Lines | Changed Lines | Coverage | Missed Changed Spans |"
    ));
    assert!(!markdown.contains("| Gate | File |"));
    assert!(!markdown.contains("logic Total"));
}

#[test]
fn path_scoped_gates_reject_overlap_on_changed_files() {
    let fixture = vitest_path_scoped_gates_fixture();
    let (_temp, worktree, diff_file) = setup_path_scoped_fixture();
    fs::write(
        worktree.join("covgate.toml"),
        "[[gates]]\nname = \"ts-all\"\ninclude = [\"**/*.ts\"]\nfail-under-lines = 10\n\n[[gates]]\nname = \"src-ts\"\ninclude = [\"src/**/*.ts\"]\nfail-under-lines = 10\n",
    )
    .expect("config should be written");

    let output = covgate(&worktree)
        .check(&fixture.coverage_json())
        .args([
            "--diff-file".into(),
            diff_file.to_string_lossy().into_owned(),
        ])
        .run();

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(stderr.contains("src/math.ts"), "stderr={stderr}");
    assert!(
        stderr.contains("matches multiple scoped gates"),
        "stderr={stderr}"
    );
    assert!(stderr.contains("ts-all"), "stderr={stderr}");
    assert!(stderr.contains("src-ts"), "stderr={stderr}");
}

#[test]
fn path_scoped_gates_require_a_fallback_for_unmatched_supported_files() {
    let fixture = vitest_path_scoped_gates_fixture();
    let (_temp, worktree, diff_file) = setup_path_scoped_fixture();
    fs::write(
        worktree.join("covgate.toml"),
        "[[gates]]\nname = \"js-ui\"\ninclude = [\"**/*.tsx\"]\nfail-under-lines = 70\n",
    )
    .expect("config should be written");

    let output = covgate(&worktree)
        .check(&fixture.coverage_json())
        .args([
            "--diff-file".into(),
            diff_file.to_string_lossy().into_owned(),
        ])
        .run();

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(stderr.contains("src/math.ts"), "stderr={stderr}");
    assert!(
        stderr.contains("does not match any scoped gate"),
        "stderr={stderr}"
    );
    assert!(
        stderr.contains("no fallback gate is configured"),
        "stderr={stderr}"
    );
}

#[test]
fn absolute_llvm_paths_match_diff_fixture() {
    let fixture = rust_basic_pass_fixture();
    let temp = tempdir().expect("tempdir should exist");
    let worktree = setup_fixture_worktree(temp.path(), fixture);
    let diff_file = write_worktree_diff(temp.path(), &worktree);
    let coverage_json = temp.path().join("coverage-absolute.json");
    write_absolute_path_coverage_fixture(fixture, &worktree, &coverage_json);
    fs::write(
        worktree.join("covgate.toml"),
        "[[gates]]\nfail-under-regions = 90\n",
    )
    .expect("config should be written");

    let output = covgate(&worktree)
        .check(&coverage_json)
        .args([
            "--diff-file".into(),
            diff_file.to_string_lossy().into_owned(),
        ])
        .run();

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("PASS Regions:"));
    assert!(!stdout.contains("(0/"));
}

#[test]
fn pr_branch_against_main_fixture() {
    let fixture = rust_basic_pass_fixture();
    let temp = tempdir().expect("tempdir should exist");
    let fixture_root = fixture.root();
    let repo_src = fixture_root.join("repo");
    let overlay_src = fixture_root.join("overlay");
    let worktree = temp.path().join("repo");
    copy_tree(&repo_src, &worktree);
    init_git_repo(&worktree);
    run_git(&worktree, &["branch", "-M", "main"]);
    run_git(&worktree, &["checkout", "-b", "feature/pr-fixture"]);
    copy_tree(&overlay_src, &worktree);
    fs::write(
        worktree.join("covgate.toml"),
        "[[gates]]\nfail-under-regions = 90\n",
    )
    .expect("config should be written");
    run_git(&worktree, &["add", "."]);
    run_git(&worktree, &["commit", "-m", "feature change"]);

    let output = covgate(&worktree)
        .check(&fixture.coverage_json())
        .args(["--base", "main"])
        .run();

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("PASS Regions:"));
    assert!(stdout.contains("100.00%"));
}

#[test]
fn uses_repo_config_defaults_for_base_and_threshold() {
    let fixture = rust_basic_fail_fixture();
    let temp = tempdir().expect("tempdir should exist");
    let fixture_root = fixture.root();
    let repo_src = fixture_root.join("repo");
    let overlay_src = fixture_root.join("overlay");
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
        "base = \"main\"\n[[gates]]\nfail-under-regions = 0.0\n",
    )
    .expect("config should be written");
    run_git(&worktree, &["add", "covgate.toml"]);
    run_git(&worktree, &["commit", "-m", "add covgate defaults"]);

    let output = covgate(&worktree).check(&fixture.coverage_json()).run();

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("PASS Regions:"));
}

#[test]
fn uses_repo_config_defaults_from_parent_directory() {
    let fixture = rust_basic_fail_fixture();
    let temp = tempdir().expect("tempdir should exist");
    let fixture_root = fixture.root();
    let repo_src = fixture_root.join("repo");
    let overlay_src = fixture_root.join("overlay");
    let worktree = temp.path().join("repo");
    copy_tree(&repo_src, &worktree);
    init_git_repo(&worktree);
    run_git(&worktree, &["branch", "-M", "main"]);
    run_git(
        &worktree,
        &["checkout", "-b", "feature/config-parent-defaults"],
    );

    copy_tree(&overlay_src, &worktree);
    run_git(&worktree, &["add", "."]);
    run_git(&worktree, &["commit", "-m", "feature change"]);
    fs::write(
        worktree.join("covgate.toml"),
        "base = \"main\"\n[[gates]]\nfail-under-regions = 0.0\n",
    )
    .expect("config should be written");
    run_git(&worktree, &["add", "covgate.toml"]);
    run_git(&worktree, &["commit", "-m", "add covgate defaults"]);

    let nested_dir = worktree.join("src");
    let output = covgate(&nested_dir).check(&fixture.coverage_json()).run();

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("PASS Regions:"));
}

#[test]
fn unknown_coverage_json_shape_reports_supported_formats() {
    let fixture = rust_basic_fail_fixture();
    let temp = tempdir().expect("tempdir should exist");
    let worktree = setup_fixture_worktree(temp.path(), fixture);
    let diff_file = write_worktree_diff(temp.path(), &worktree);
    let invalid_coverage = temp.path().join("unknown-coverage.json");
    fs::write(&invalid_coverage, "{\"hello\":\"world\"}")
        .expect("invalid coverage fixture should be written");
    fs::write(
        worktree.join("covgate.toml"),
        "[[gates]]\nfail-under-lines = 90\n",
    )
    .expect("config should be written");

    let output = covgate(&worktree)
        .check(&invalid_coverage)
        .args([
            "--diff-file".into(),
            diff_file.to_string_lossy().into_owned(),
        ])
        .run();

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("unsupported coverage format"),
        "stderr={stderr}"
    );
    assert!(stderr.contains("LLVM JSON export"), "stderr={stderr}");
    assert!(stderr.contains("Coverlet native JSON"), "stderr={stderr}");
    assert!(stderr.contains("Istanbul native JSON"), "stderr={stderr}");
}

#[test]
fn minimal_pass_output_is_token_efficient() {
    let fixture = rust_basic_pass_fixture();
    let temp = tempdir().expect("tempdir should exist");
    let worktree = setup_fixture_worktree(temp.path(), fixture);
    fs::write(
        worktree.join("covgate.toml"),
        "[[gates]]\nfail-under-regions = 90\n",
    )
    .expect("config should be written");

    let output = covgate(&worktree).check(&fixture.coverage_json()).run();

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");

    // Should contain the summary line
    assert!(stdout.contains("PASS Regions:"));
    assert!(stdout.contains("100.00%"));
    assert!(stdout.contains("(3/3)"));
    assert!(stdout.contains("≥ 90.00%"));

    // Should NOT contain the "Diff Coverage: PASS" header
    assert!(!stdout.contains("Diff Coverage: PASS"));

    // Should NOT contain the "Changed regions:" header
    assert!(!stdout.contains("Changed regions:"));

    // Should NOT contain rulers
    assert!(!stdout.contains("---"));
}

#[test]
fn minimal_fail_output_is_focused() {
    let fixture = rust_basic_fail_fixture();
    let temp = tempdir().expect("tempdir should exist");
    let worktree = setup_fixture_worktree(temp.path(), fixture);
    fs::write(
        worktree.join("covgate.toml"),
        "[[gates]]\nfail-under-regions = 100\n",
    )
    .expect("config should be written");

    let output = covgate(&worktree).check(&fixture.coverage_json()).run();

    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    // Should contain the summary line with FAIL
    assert!(stdout.contains("FAIL Regions:"));
    assert!(stdout.contains("0.00%"));
    assert!(stdout.contains("(0/3)"));
    assert!(stdout.contains("≱ 100.00%"));

    // Should show the failing file
    assert!(stdout.contains("src/lib.rs"));

    // Should show missed regions in that file
    assert!(stdout.contains("2:9-12"));

    // Should NOT contain the "Diff Coverage: FAIL" header
    assert!(!stdout.contains("Diff Coverage: FAIL"));

    // Should NOT contain the "Rule fail-under-regions: FAIL" line
    assert!(!stdout.contains("Rule fail-under-regions: FAIL"));
}

#[test]
fn overall_coverage_remains_global_when_scoped_gates_are_configured() {
    let fixture = vitest_path_scoped_gates_fixture();
    let (temp, worktree, diff_file) = setup_path_scoped_fixture();
    let markdown_output = temp.path().join("summary.md");

    // Configure scoped gates that only cover a subset of files
    fs::write(
        worktree.join("covgate.toml"),
        "[[gates]]\nname = \"js-ui\"\ninclude = [\"**/*.tsx\"]\nfail-under-lines = 70\n\n[[gates]]\nfail-under-lines = 10\n",
    )
    .expect("config should be written");

    let output = covgate(&worktree)
        .check(&fixture.coverage_json())
        .args([
            "--diff-file".into(),
            diff_file.to_string_lossy().into_owned(),
            "--markdown-output".into(),
            markdown_output.to_string_lossy().into_owned(),
        ])
        .run();

    if !markdown_output.exists() {
        panic!(
            "summary.md was not created. exit={:?}\nstdout:\n{}\nstderr:\n{}",
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let markdown = fs::read_to_string(markdown_output).expect("markdown should be readable");

    // The "Overall Coverage" section should still contain the global total for the entire fixture.
    // It should NOT be partitioned by gate or narrowed to changed files only.
    assert!(markdown.contains("### Overall Coverage"));

    // In the broken implementation, the "Overall Coverage" section will have a "Gate" column
    // and multiple "Total" rows (one per gate).
    // The corrected implementation should have one global section.

    // Assert that the global total (15 lines) is present in a standard table row.
    // Broken implementation will have "| **js-ui Total** |" or "| **default Total** |" instead of "| **Total** |".
    assert!(
        markdown.contains("| **Total** | **9** | **15** | **6** | **60.00% 🟡** |"),
        "Markdown 'Overall Coverage' should contain a single global 'Total' row, but was:\n{markdown}"
    );

    // Assert that gate labels are NOT present in the Overall Coverage section tables.
    let overall_section = &markdown[markdown.find("### Overall Coverage").unwrap()..];
    assert!(
        !overall_section.contains("| Gate |"),
        "Overall Coverage should not have a 'Gate' column"
    );
    assert!(
        !overall_section.contains("js-ui"),
        "Overall Coverage should not contain gate labels"
    );
}
