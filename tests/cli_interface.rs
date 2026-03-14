mod support;

use std::fs;

use tempfile::tempdir;

use crate::support::{
    copy_tree, init_git_repo, run_covgate, run_covgate_with_coverage, run_git,
    rust_basic_fail_fixture, rust_basic_pass_fixture, setup_fixture_worktree,
    write_absolute_path_coverage_fixture, write_worktree_diff,
};

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
    assert!(stdout.contains("Diff: main...HEAD"));
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

    let output = run_covgate(&worktree, fixture, &[]);

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("Diff: main...HEAD"));
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

    let output = run_covgate(
        &worktree,
        fixture,
        &["--fail-uncovered-regions".to_string(), "0".to_string()],
    );

    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("Diff: main...HEAD"));
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

    let output = run_covgate(
        &worktree,
        fixture,
        &["--fail-under-regions".to_string(), "60".to_string()],
    );

    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("Diff: main...HEAD"));
    assert!(stdout.contains("Rule fail-under-regions: FAIL"));
    assert!(stdout.contains("Diff Coverage: FAIL"));
}
