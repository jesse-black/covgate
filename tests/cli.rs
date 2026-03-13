use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

use tempfile::tempdir;

fn fixture_root(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("rust")
        .join(name)
}

#[test]
fn basic_fail_rust_fixture() {
    let temp = tempdir().expect("tempdir should exist");
    let worktree = setup_fixture_worktree(temp.path(), "basic-fail");
    let diff_file = write_worktree_diff(temp.path(), &worktree);

    let output = run_covgate(
        &worktree,
        "basic-fail",
        &[
            "--diff-file".to_string(),
            diff_file.to_string_lossy().into_owned(),
            "--fail-under-regions".to_string(),
            "60".to_string(),
        ],
    );

    assert_eq!(
        output.status.code(),
        Some(1),
        "fixture should fail the gate"
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("Diff Coverage: FAIL"));
    assert!(stdout.contains("src/lib.rs"));
    assert!(stdout.contains("Coverage: 50.00%"));
    assert!(stdout.contains("Rule fail-under-regions: FAIL (50.00% ≥ 60.00%)"));
}

#[test]
fn uncovered_regions_pass_fixture() {
    let temp = tempdir().expect("tempdir should exist");
    let worktree = setup_fixture_worktree(temp.path(), "basic-fail");
    let diff_file = write_worktree_diff(temp.path(), &worktree);

    let output = run_covgate(
        &worktree,
        "basic-fail",
        &[
            "--diff-file".to_string(),
            diff_file.to_string_lossy().into_owned(),
            "--fail-uncovered-regions".to_string(),
            "1".to_string(),
        ],
    );

    assert_eq!(
        output.status.code(),
        Some(0),
        "fixture should pass the gate when uncovered budget is met"
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("Diff Coverage: PASS"));
    assert!(stdout.contains("Coverage: 50.00%"));
    assert!(stdout.contains("Rule fail-uncovered-regions: PASS (1 <= 1)"));
}

#[test]
fn uncovered_regions_fail_fixture() {
    let temp = tempdir().expect("tempdir should exist");
    let worktree = setup_fixture_worktree(temp.path(), "basic-fail");
    let diff_file = write_worktree_diff(temp.path(), &worktree);

    let output = run_covgate(
        &worktree,
        "basic-fail",
        &[
            "--diff-file".to_string(),
            diff_file.to_string_lossy().into_owned(),
            "--fail-uncovered-regions".to_string(),
            "0".to_string(),
        ],
    );

    assert_eq!(
        output.status.code(),
        Some(1),
        "fixture should fail the gate when uncovered budget is exceeded"
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("Diff Coverage: FAIL"));
    assert!(stdout.contains("Coverage: 50.00%"));
    assert!(stdout.contains("Rule fail-uncovered-regions: FAIL (1 > 0)"));
}

#[test]
fn multi_rule_one_pass_one_fail() {
    let temp = tempdir().expect("tempdir should exist");
    let worktree = setup_fixture_worktree(temp.path(), "basic-fail");
    let diff_file = write_worktree_diff(temp.path(), &worktree);

    let output = run_covgate(
        &worktree,
        "basic-fail",
        &[
            "--diff-file".to_string(),
            diff_file.to_string_lossy().into_owned(),
            "--fail-under-regions".to_string(),
            "90".to_string(),
            "--fail-uncovered-regions".to_string(),
            "1".to_string(),
        ],
    );

    assert_eq!(
        output.status.code(),
        Some(1),
        "fixture should fail the gate when one rule fails"
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("Diff Coverage: FAIL"));
    assert!(stdout.contains("Rule fail-under-regions: FAIL (50.00% ≥ 90.00%)"));
    assert!(stdout.contains("Rule fail-uncovered-regions: PASS (1 <= 1)"));
}

#[test]
fn basic_pass_rust_fixture() {
    let temp = tempdir().expect("tempdir should exist");
    let worktree = setup_fixture_worktree(temp.path(), "basic-pass");
    let diff_file = write_worktree_diff(temp.path(), &worktree);

    let output = run_covgate(
        &worktree,
        "basic-pass",
        &[
            "--diff-file".to_string(),
            diff_file.to_string_lossy().into_owned(),
            "--fail-under-regions".to_string(),
            "90".to_string(),
        ],
    );

    assert_eq!(
        output.status.code(),
        Some(0),
        "fixture should pass the gate"
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("Diff Coverage: PASS"));
    assert!(stdout.contains("Coverage: 100.00%"));
    assert!(stdout.contains("Rule fail-under-regions: PASS (100.00% ≥ 90.00%)"));
}

#[test]
fn absolute_llvm_paths_match_diff_fixture() {
    let temp = tempdir().expect("tempdir should exist");
    let worktree = setup_fixture_worktree(temp.path(), "basic-pass");
    let diff_file = write_worktree_diff(temp.path(), &worktree);
    let coverage_json = temp.path().join("coverage-absolute.json");
    write_absolute_path_coverage_fixture("basic-pass", &worktree, &coverage_json);

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

    assert_eq!(
        output.status.code(),
        Some(0),
        "absolute-path coverage fixture should still pass the gate"
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("Diff Coverage: PASS"));
    assert!(stdout.contains("Changed regions: 2"));
    assert!(!stdout.contains("Changed regions: 0"));
    assert!(stdout.contains("Coverage: 100.00%"));
}

#[test]
fn markdown_summary_rust_fixture() {
    let temp = tempdir().expect("tempdir should exist");
    let worktree = setup_fixture_worktree(temp.path(), "basic-pass");
    let diff_file = write_worktree_diff(temp.path(), &worktree);
    let markdown_output = temp.path().join("summary.md");

    let output = run_covgate(
        &worktree,
        "basic-pass",
        &[
            "--diff-file".to_string(),
            diff_file.to_string_lossy().into_owned(),
            "--fail-under-regions".to_string(),
            "90".to_string(),
            "--markdown-output".to_string(),
            markdown_output.to_string_lossy().into_owned(),
        ],
    );

    assert_eq!(
        output.status.code(),
        Some(0),
        "markdown output should not change the gate outcome"
    );
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
}

#[test]
fn pr_branch_against_main_fixture() {
    let temp = tempdir().expect("tempdir should exist");
    let fixture = fixture_root("basic-pass");
    let repo_src = fixture.join("repo");
    let overlay_src = fixture.join("overlay");
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
        "basic-pass",
        &[
            "--base".to_string(),
            "main".to_string(),
            "--fail-under-regions".to_string(),
            "90".to_string(),
        ],
    );

    assert_eq!(
        output.status.code(),
        Some(0),
        "branch-versus-main fixture should pass the gate"
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("Diff: main...HEAD"));
    assert!(stdout.contains("Diff Coverage: PASS"));
    assert!(stdout.contains("Coverage: 100.00%"));
}

#[test]
fn uses_repo_config_defaults_for_base_and_threshold() {
    let temp = tempdir().expect("tempdir should exist");
    let fixture = fixture_root("basic-fail");
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
        "base = \"main\"\n[gates]\nfail_under_regions = 40\n",
    )
    .expect("config should be written");

    let output = run_covgate(&worktree, "basic-fail", &[]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "config defaults should allow the gate to pass"
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("Diff: main...HEAD"));
    assert!(stdout.contains("Rule fail-under-regions: PASS (50.00% ≥ 40.00%)"));
    assert!(stdout.contains("Coverage: 50.00%"));
}

#[test]
fn mixed_cli_over_toml_precedence() {
    let temp = tempdir().expect("tempdir should exist");
    let fixture = fixture_root("basic-fail");
    let repo_src = fixture.join("repo");
    let overlay_src = fixture.join("overlay");
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
        "base = \"main\"\n[gates]\nfail_under_regions = 40\nfail_uncovered_regions = 10\n",
    )
    .expect("config should be written");

    let output = run_covgate(
        &worktree,
        "basic-fail",
        &["--fail-uncovered-regions".to_string(), "0".to_string()],
    );

    assert_eq!(
        output.status.code(),
        Some(1),
        "cli override of one rule should fail the gate while leaving the other rule active"
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("Diff: main...HEAD"));
    assert!(stdout.contains("Rule fail-under-regions: PASS (50.00% ≥ 40.00%)"));
    assert!(stdout.contains("Rule fail-uncovered-regions: FAIL (1 > 0)"));
    assert!(stdout.contains("Diff Coverage: FAIL"));
}

#[test]
fn cli_threshold_overrides_repo_config_default() {
    let temp = tempdir().expect("tempdir should exist");
    let fixture = fixture_root("basic-fail");
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
        "base = \"main\"\n[gates]\nfail_under_regions = 40\n",
    )
    .expect("config should be written");

    let output = run_covgate(
        &worktree,
        "basic-fail",
        &["--fail-under-regions".to_string(), "60".to_string()],
    );

    assert_eq!(
        output.status.code(),
        Some(1),
        "cli threshold should override the repo config default"
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("Diff: main...HEAD"));
    assert!(stdout.contains("Rule fail-under-regions: FAIL (50.00% ≥ 60.00%)"));
    assert!(stdout.contains("Diff Coverage: FAIL"));
}

fn setup_fixture_worktree(temp_root: &Path, fixture_name: &str) -> PathBuf {
    let fixture = fixture_root(fixture_name);
    let repo_src = fixture.join("repo");
    let overlay_src = fixture.join("overlay");
    let worktree = temp_root.join("repo");
    copy_tree(&repo_src, &worktree);
    init_git_repo(&worktree);
    copy_tree(&overlay_src, &worktree);
    worktree
}

fn write_worktree_diff(temp_root: &Path, worktree: &Path) -> PathBuf {
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

fn run_covgate(worktree: &Path, fixture_name: &str, extra_args: &[String]) -> Output {
    let coverage_json = fixture_root(fixture_name).join("coverage.json");
    run_covgate_with_coverage(worktree, &coverage_json, extra_args)
}

fn run_covgate_with_coverage(
    worktree: &Path,
    coverage_json: &Path,
    extra_args: &[String],
) -> Output {
    let binary = env!("CARGO_BIN_EXE_covgate");
    let mut command = Command::new(binary);
    command.arg("--coverage-json");
    command.arg(coverage_json);
    command.args(extra_args);
    command.current_dir(worktree);
    command.output().expect("covgate should run")
}

fn write_absolute_path_coverage_fixture(fixture_name: &str, worktree: &Path, destination: &Path) {
    let template = fixture_root(fixture_name).join("coverage.json");
    let absolute_source_path = worktree.join("src").join("lib.rs");
    let updated = fs::read_to_string(template)
        .expect("fixture coverage should be readable")
        .replace(
            "\"src/lib.rs\"",
            &format!("\"{}\"", absolute_source_path.display()),
        );
    fs::write(destination, updated).expect("absolute-path coverage fixture should be written");
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
