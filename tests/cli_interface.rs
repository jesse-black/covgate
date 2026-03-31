mod support;

use std::{fs, process::Output};

use tempfile::tempdir;

use crate::support::{
    copy_tree, init_git_repo, run_covgate, run_covgate_raw, run_covgate_with_coverage, run_git,
    rust_basic_fail_fixture, rust_basic_pass_fixture, setup_fixture_worktree,
    write_absolute_path_coverage_fixture, write_worktree_diff,
};

fn run_covgate_raw_with_path(worktree: &std::path::Path, path: &str, args: &[String]) -> Output {
    let binary = env!("CARGO_BIN_EXE_covgate");
    let mut command = std::process::Command::new(binary);
    command.args(args);
    command.current_dir(worktree);
    command.env("PATH", path);
    command.output().expect("covgate should run")
}

#[test]
fn record_base_noops_when_standard_base_ref_is_available() {
    let fixture = rust_basic_pass_fixture();
    let temp = tempdir().expect("tempdir should exist");
    let worktree = setup_fixture_worktree(temp.path(), fixture);
    run_git(&worktree, &["branch", "-M", "main"]);

    let output = run_covgate_raw(&worktree, &["record-base".to_string()]);
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

    let output = run_covgate_raw(&worktree, &["record-base".to_string()]);
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

    let output = run_covgate_raw(&worktree, &["record-base".to_string()]);
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

    let output = run_covgate_raw(temp.path(), &["record-base".to_string()]);
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

    let output = run_covgate_raw_with_path(temp.path(), "", &["record-base".to_string()]);

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

    let output = run_covgate_raw(temp.path(), &[]);
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

    let output = run_covgate_raw(temp.path(), &["check".to_string()]);
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

    let output = run_covgate_raw_with_path(
        temp.path(),
        "",
        &["check".to_string(), "missing.json".to_string()],
    );

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

    let output = run_covgate_raw(temp.path(), &["--help".to_string()]);
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
fn check_help_describes_arguments_and_options() {
    let temp = tempdir().expect("tempdir should exist");

    let output = run_covgate_raw(temp.path(), &["check".to_string(), "--help".to_string()]);
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
    assert!(
        stdout.contains("Minimum changed-region coverage percentage required to pass"),
        "stdout={stdout}"
    );
    assert!(
        stdout.contains("Write a Markdown summary to this file"),
        "stdout={stdout}"
    );
}

#[test]
fn record_base_help_is_user_focused() {
    let temp = tempdir().expect("tempdir should exist");

    let output = run_covgate_raw(
        temp.path(),
        &["record-base".to_string(), "--help".to_string()],
    );
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

    let first = run_covgate_raw(&worktree, &["record-base".to_string()]);
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

    let second = run_covgate_raw(&worktree, &["record-base".to_string()]);
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

    let first = run_covgate_raw(&worktree, &["record-base".to_string()]);
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

    let second = run_covgate_raw(&worktree, &["record-base".to_string()]);
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

    let output = run_covgate(
        &worktree,
        fixture,
        &["--fail-under-regions".to_string(), "90".to_string()],
    );

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

    let output = run_covgate_with_coverage(
        &worktree,
        &fixture.coverage_json(),
        &[
            "--diff-file".to_string(),
            diff_file.to_string_lossy().into_owned(),
            "--fail-under-regions".to_string(),
            "90".to_string(),
        ],
    );

    assert_eq!(output.status.code(), Some(0));
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        !stderr.contains("working tree has uncommitted changes"),
        "stderr={stderr}"
    );
}

#[test]
fn git_base_mode_warns_about_untracked_files() {
    let fixture = rust_basic_pass_fixture();
    let temp = tempdir().expect("tempdir should exist");
    let worktree = setup_fixture_worktree(temp.path(), fixture);

    fs::write(
        worktree.join("new_untracked.rs"),
        "pub fn pending() {}
",
    )
    .expect("untracked file should write");

    let output = run_covgate(
        &worktree,
        fixture,
        &["--fail-under-regions".to_string(), "90".to_string()],
    );

    assert_eq!(output.status.code(), Some(0));
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("Untracked-files warning"),
        "stderr={stderr}"
    );
    assert!(stderr.contains("false pass"), "stderr={stderr}");
    assert!(stderr.contains("Add them with:"), "stderr={stderr}");
    assert!(
        stderr.contains("git add -N new_untracked.rs"),
        "stderr={stderr}"
    );
}

#[test]
fn diff_file_mode_skips_untracked_files_warning() {
    let fixture = rust_basic_pass_fixture();
    let temp = tempdir().expect("tempdir should exist");
    let worktree = setup_fixture_worktree(temp.path(), fixture);
    let diff_file = write_worktree_diff(temp.path(), &worktree);

    fs::write(
        worktree.join("new_untracked.rs"),
        "pub fn pending() {}
",
    )
    .expect("untracked file should write");

    let output = run_covgate_with_coverage(
        &worktree,
        &fixture.coverage_json(),
        &[
            "--diff-file".to_string(),
            diff_file.to_string_lossy().into_owned(),
            "--fail-under-regions".to_string(),
            "90".to_string(),
        ],
    );

    assert_eq!(output.status.code(), Some(0));
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        !stderr.contains("Untracked-files warning"),
        "stderr={stderr}"
    );
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

    let output = run_covgate_raw(&worktree, &["record-base".to_string()]);
    assert_eq!(output.status.code(), Some(0));
    run_git(&worktree, &["branch", "main", "HEAD"]);

    copy_tree(&overlay_src, &worktree);
    run_git(&worktree, &["add", "."]);
    run_git(&worktree, &["commit", "-m", "feature change"]);

    let output = run_covgate(
        &worktree,
        fixture,
        &["--fail-under-regions".to_string(), "90".to_string()],
    );

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(
        stdout.contains("Diff: main...HEAD, staged and unstaged changes"),
        "stdout={stdout}"
    );
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

    let output = run_covgate_raw(&worktree, &["record-base".to_string()]);
    assert_eq!(output.status.code(), Some(0));
    run_git(&worktree, &["branch", "main", "HEAD"]);

    copy_tree(&overlay_src, &worktree);
    run_git(&worktree, &["add", "."]);
    run_git(&worktree, &["commit", "-m", "feature change"]);

    let output = run_covgate(
        &worktree,
        fixture,
        &[
            "--base".to_string(),
            "main".to_string(),
            "--fail-under-regions".to_string(),
            "90".to_string(),
        ],
    );

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(
        stdout.contains("Diff: main...HEAD, staged and unstaged changes"),
        "stdout={stdout}"
    );
}

#[test]
fn failure_text_requires_git_repo_when_run_outside_repository() {
    let fixture = rust_basic_pass_fixture();
    let temp = tempdir().expect("tempdir should exist");

    let output = run_covgate_with_coverage(
        temp.path(),
        &fixture.coverage_json(),
        &["--fail-under-regions".to_string(), "90".to_string()],
    );

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

    let output = run_covgate(
        &worktree,
        fixture,
        &[
            "--diff-file".to_string(),
            diff_file.to_string_lossy().into_owned(),
            "--fail-under-regions".to_string(),
            "90".to_string(),
            "--markdown-output".to_string(),
            markdown_output.to_string_lossy().into_owned(),
        ],
    );

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("Diff Coverage: PASS"));
    assert!(markdown_output.exists(), "markdown file should be written");

    let markdown = fs::read_to_string(markdown_output).expect("markdown should be readable");
    assert!(markdown.contains("## Covgate"));
    assert!(markdown.contains("### Diff Coverage"));
    assert!(markdown.contains("| Result | Rule | Observed | Configured |"));
    assert!(markdown.contains("| PASS | `fail-under-regions` | 100.00% | ≥ 90.00% |"));
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
fn absolute_llvm_paths_match_diff_fixture() {
    let fixture = rust_basic_pass_fixture();
    let temp = tempdir().expect("tempdir should exist");
    let worktree = setup_fixture_worktree(temp.path(), fixture);
    let diff_file = write_worktree_diff(temp.path(), &worktree);
    let coverage_json = temp.path().join("coverage-absolute.json");
    write_absolute_path_coverage_fixture(fixture, &worktree, &coverage_json);

    let output = run_covgate_with_coverage(
        &worktree,
        &coverage_json,
        &[
            "--diff-file".to_string(),
            diff_file.to_string_lossy().into_owned(),
            "--fail-under-regions".to_string(),
            "90".to_string(),
        ],
    );

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("Diff Coverage: PASS"));
    assert!(stdout.contains("Changed regions:"));
    assert!(!stdout.contains("Changed regions: 0"));
    assert!(stdout.contains("Coverage:"));
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
    run_git(&worktree, &["add", "."]);
    run_git(&worktree, &["commit", "-m", "feature change"]);

    let output = run_covgate(
        &worktree,
        fixture,
        &[
            "--base".to_string(),
            "main".to_string(),
            "--fail-under-regions".to_string(),
            "90".to_string(),
        ],
    );

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("Diff: main...HEAD, staged and unstaged changes"));
    assert!(stdout.contains("Diff Coverage: PASS"));
    assert!(stdout.contains("Coverage: 100.00%"));
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
        "base = \"main\"\n[gates]\nfail_under_regions = 0.0\n",
    )
    .expect("config should be written");
    run_git(&worktree, &["add", "covgate.toml"]);
    run_git(&worktree, &["commit", "-m", "add covgate defaults"]);

    let output = run_covgate(&worktree, fixture, &[]);

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("Diff: main...HEAD, staged and unstaged changes"));
    assert!(stdout.contains("Rule fail-under-regions: PASS"));
    assert!(stdout.contains("Coverage:"));
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
        "base = \"main\"\n[gates]\nfail_under_regions = 0.0\n",
    )
    .expect("config should be written");
    run_git(&worktree, &["add", "covgate.toml"]);
    run_git(&worktree, &["commit", "-m", "add covgate defaults"]);

    let nested_dir = worktree.join("src");
    let output = run_covgate(&nested_dir, fixture, &[]);

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("Diff: main...HEAD, staged and unstaged changes"));
    assert!(stdout.contains("Rule fail-under-regions: PASS"));
    assert!(stdout.contains("Coverage:"));
}

#[test]
fn mixed_cli_over_toml_precedence() {
    let fixture = rust_basic_fail_fixture();
    let temp = tempdir().expect("tempdir should exist");
    let fixture_root = fixture.root();
    let repo_src = fixture_root.join("repo");
    let overlay_src = fixture_root.join("overlay");
    let worktree = temp.path().join("repo");
    copy_tree(&repo_src, &worktree);
    init_git_repo(&worktree);
    run_git(&worktree, &["branch", "-M", "main"]);
    run_git(&worktree, &["checkout", "-b", "feature/mixed-cli-override"]);

    copy_tree(&overlay_src, &worktree);
    run_git(&worktree, &["add", "."]);
    run_git(&worktree, &["commit", "-m", "feature change"]);
    fs::write(
        worktree.join("covgate.toml"),
        "base = \"main\"\n[gates]\nfail_under_regions = 0.0\nfail_uncovered_regions = 10\n",
    )
    .expect("config should be written");
    run_git(&worktree, &["add", "covgate.toml"]);
    run_git(&worktree, &["commit", "-m", "add covgate defaults"]);

    let output = run_covgate(
        &worktree,
        fixture,
        &["--fail-uncovered-regions".to_string(), "0".to_string()],
    );

    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("Diff: main...HEAD, staged and unstaged changes"));
    assert!(stdout.contains("Rule fail-under-regions: PASS"));
    assert!(stdout.contains("Rule fail-uncovered-regions: FAIL"));
    assert!(stdout.contains("Diff Coverage: FAIL"));
}

#[test]
fn cli_threshold_overrides_repo_config_default() {
    let fixture = rust_basic_fail_fixture();
    let temp = tempdir().expect("tempdir should exist");
    let fixture_root = fixture.root();
    let repo_src = fixture_root.join("repo");
    let overlay_src = fixture_root.join("overlay");
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
        "base = \"main\"\n[gates]\nfail_under_regions = 0.0\n",
    )
    .expect("config should be written");
    run_git(&worktree, &["add", "covgate.toml"]);
    run_git(&worktree, &["commit", "-m", "add covgate defaults"]);

    let output = run_covgate(
        &worktree,
        fixture,
        &["--fail-under-regions".to_string(), "60".to_string()],
    );

    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("Diff: main...HEAD, staged and unstaged changes"));
    assert!(stdout.contains("Rule fail-under-regions: FAIL"));
    assert!(stdout.contains("Diff Coverage: FAIL"));
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

    let output = run_covgate_with_coverage(
        &worktree,
        &invalid_coverage,
        &[
            "--diff-file".to_string(),
            diff_file.to_string_lossy().into_owned(),
            "--fail-under-lines".to_string(),
            "90".to_string(),
        ],
    );

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
