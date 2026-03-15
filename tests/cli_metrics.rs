mod support;

use tempfile::tempdir;

use crate::support::{
    assert_fixture_has_no_branch_coverage, branch_capable_fail_fixtures,
    branch_capable_pass_fixtures, fail_fixtures_with_lines, fail_fixtures_with_regions,
    function_capable_fail_fixtures, function_capable_pass_fixtures, pass_fixtures_with_lines,
    pass_fixtures_with_regions, run_covgate, rust_basic_fail_fixture, setup_fixture_worktree,
    write_worktree_diff,
};

#[test]
fn region_threshold_fails_when_below_threshold() {
    for fixture in fail_fixtures_with_regions() {
        let temp = tempdir().expect("tempdir should exist");
        let worktree = setup_fixture_worktree(temp.path(), fixture);
        let diff_file = write_worktree_diff(temp.path(), &worktree);

        let output = run_covgate(
            &worktree,
            fixture,
            &[
                "--diff-file".to_string(),
                diff_file.to_string_lossy().into_owned(),
                "--fail-under-regions".to_string(),
                "90".to_string(),
            ],
        );

        assert_eq!(
            output.status.code(),
            Some(1),
            "{} should fail the region gate",
            fixture.id()
        );
        let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
        assert!(
            stdout.contains("Diff Coverage: FAIL"),
            "fixture={}",
            fixture.id()
        );
        assert!(
            stdout.contains("Rule fail-under-regions: FAIL"),
            "fixture={} stdout={}",
            fixture.id(),
            stdout
        );
    }
}

#[test]
fn uncovered_regions_budget_passes_when_met() {
    for fixture in fail_fixtures_with_regions() {
        let temp = tempdir().expect("tempdir should exist");
        let worktree = setup_fixture_worktree(temp.path(), fixture);
        let diff_file = write_worktree_diff(temp.path(), &worktree);

        let output = run_covgate(
            &worktree,
            fixture,
            &[
                "--diff-file".to_string(),
                diff_file.to_string_lossy().into_owned(),
                "--fail-uncovered-regions".to_string(),
                "100".to_string(),
            ],
        );

        assert_eq!(
            output.status.code(),
            Some(0),
            "{} should pass uncovered-region budget",
            fixture.id()
        );
        let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
        assert!(stdout.contains("Rule fail-uncovered-regions: PASS"));
    }
}

#[test]
fn uncovered_regions_budget_fails_when_exceeded() {
    for fixture in fail_fixtures_with_regions() {
        let temp = tempdir().expect("tempdir should exist");
        let worktree = setup_fixture_worktree(temp.path(), fixture);
        let diff_file = write_worktree_diff(temp.path(), &worktree);

        let output = run_covgate(
            &worktree,
            fixture,
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
            "{} should fail uncovered-region budget",
            fixture.id()
        );
        let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
        assert!(stdout.contains("Rule fail-uncovered-regions: FAIL"));
    }
}

#[test]
fn line_threshold_fails_when_below_threshold() {
    for fixture in fail_fixtures_with_lines() {
        let temp = tempdir().expect("tempdir should exist");
        let worktree = setup_fixture_worktree(temp.path(), fixture);
        let diff_file = write_worktree_diff(temp.path(), &worktree);

        let output = run_covgate(
            &worktree,
            fixture,
            &[
                "--diff-file".to_string(),
                diff_file.to_string_lossy().into_owned(),
                "--fail-under-lines".to_string(),
                "100".to_string(),
            ],
        );

        assert_eq!(output.status.code(), Some(1), "fixture={}", fixture.id());
        let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
        assert!(stdout.contains("Rule fail-under-lines: FAIL"));
        assert!(stdout.contains("Line Coverage:"));
    }
}

#[test]
fn uncovered_line_budget_fails_when_exceeded() {
    for fixture in fail_fixtures_with_lines() {
        let temp = tempdir().expect("tempdir should exist");
        let worktree = setup_fixture_worktree(temp.path(), fixture);
        let diff_file = write_worktree_diff(temp.path(), &worktree);

        let output = run_covgate(
            &worktree,
            fixture,
            &[
                "--diff-file".to_string(),
                diff_file.to_string_lossy().into_owned(),
                "--fail-uncovered-lines".to_string(),
                "0".to_string(),
            ],
        );

        assert_eq!(output.status.code(), Some(1), "fixture={}", fixture.id());
        let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
        assert!(stdout.contains("Rule fail-uncovered-lines: FAIL"));
    }
}

#[test]
fn line_threshold_passes_for_all_pass_fixtures() {
    for fixture in pass_fixtures_with_lines() {
        let temp = tempdir().expect("tempdir should exist");
        let worktree = setup_fixture_worktree(temp.path(), fixture);
        let diff_file = write_worktree_diff(temp.path(), &worktree);

        let output = run_covgate(
            &worktree,
            fixture,
            &[
                "--diff-file".to_string(),
                diff_file.to_string_lossy().into_owned(),
                "--fail-under-lines".to_string(),
                "90".to_string(),
            ],
        );

        assert_eq!(output.status.code(), Some(0), "fixture={}", fixture.id());
        let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
        assert!(stdout.contains("Rule fail-under-lines: PASS"));
    }
}

#[test]
fn branch_threshold_passes_for_branch_capable_fixtures() {
    for fixture in branch_capable_pass_fixtures() {
        let temp = tempdir().expect("tempdir should exist");
        let worktree = setup_fixture_worktree(temp.path(), fixture);
        let diff_file = write_worktree_diff(temp.path(), &worktree);

        let output = run_covgate(
            &worktree,
            fixture,
            &[
                "--diff-file".to_string(),
                diff_file.to_string_lossy().into_owned(),
                "--fail-under-branches".to_string(),
                "90".to_string(),
            ],
        );

        assert_eq!(output.status.code(), Some(0), "fixture={}", fixture.id());
        let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
        assert!(stdout.contains("Branch Coverage:"), "stdout={stdout}");
        assert!(stdout.contains("Rule fail-under-branches: PASS"));
    }
}

#[test]
fn branch_threshold_fails_for_branch_capable_fixtures_when_below_threshold() {
    for fixture in branch_capable_fail_fixtures() {
        let temp = tempdir().expect("tempdir should exist");
        let worktree = setup_fixture_worktree(temp.path(), fixture);
        let diff_file = write_worktree_diff(temp.path(), &worktree);

        let output = run_covgate(
            &worktree,
            fixture,
            &[
                "--diff-file".to_string(),
                diff_file.to_string_lossy().into_owned(),
                "--fail-under-branches".to_string(),
                "101".to_string(),
            ],
        );

        assert_eq!(output.status.code(), Some(1), "fixture={}", fixture.id());
        let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
        assert!(stdout.contains("Branch Coverage:"), "stdout={stdout}");
        assert!(stdout.contains("Rule fail-under-branches: FAIL"));
    }
}

#[test]
fn uncovered_branch_budget_passes_for_branch_capable_fixtures() {
    for fixture in branch_capable_fail_fixtures() {
        let temp = tempdir().expect("tempdir should exist");
        let worktree = setup_fixture_worktree(temp.path(), fixture);
        let diff_file = write_worktree_diff(temp.path(), &worktree);

        let output = run_covgate(
            &worktree,
            fixture,
            &[
                "--diff-file".to_string(),
                diff_file.to_string_lossy().into_owned(),
                "--fail-uncovered-branches".to_string(),
                "100".to_string(),
            ],
        );

        assert_eq!(output.status.code(), Some(0), "fixture={}", fixture.id());
        let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
        assert!(stdout.contains("Rule fail-uncovered-branches: PASS"));
    }
}

#[test]
fn uncovered_branch_budget_fails_for_branch_capable_fixtures() {
    for fixture in branch_capable_fail_fixtures() {
        let temp = tempdir().expect("tempdir should exist");
        let worktree = setup_fixture_worktree(temp.path(), fixture);
        let diff_file = write_worktree_diff(temp.path(), &worktree);

        let output = run_covgate(
            &worktree,
            fixture,
            &[
                "--diff-file".to_string(),
                diff_file.to_string_lossy().into_owned(),
                "--fail-uncovered-branches".to_string(),
                "0".to_string(),
            ],
        );

        assert_eq!(output.status.code(), Some(1), "fixture={}", fixture.id());
        let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
        assert!(stdout.contains("Rule fail-uncovered-branches: FAIL"));
    }
}

#[test]
fn branch_metric_unavailable_for_rust_fixture() {
    let fixture = rust_basic_fail_fixture();
    assert_fixture_has_no_branch_coverage(fixture);

    let temp = tempdir().expect("tempdir should exist");
    let worktree = setup_fixture_worktree(temp.path(), fixture);
    let diff_file = write_worktree_diff(temp.path(), &worktree);

    let output = run_covgate(
        &worktree,
        fixture,
        &[
            "--diff-file".to_string(),
            diff_file.to_string_lossy().into_owned(),
            "--fail-under-branches".to_string(),
            "100".to_string(),
        ],
    );

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(stderr.contains("requested metric branch is not available in the report"));
}

#[test]
fn region_threshold_passes_for_all_pass_fixtures() {
    for fixture in pass_fixtures_with_regions() {
        let temp = tempdir().expect("tempdir should exist");
        let worktree = setup_fixture_worktree(temp.path(), fixture);
        let diff_file = write_worktree_diff(temp.path(), &worktree);

        let output = run_covgate(
            &worktree,
            fixture,
            &[
                "--diff-file".to_string(),
                diff_file.to_string_lossy().into_owned(),
                "--fail-under-regions".to_string(),
                "90".to_string(),
            ],
        );

        assert_eq!(output.status.code(), Some(0), "fixture={}", fixture.id());
        let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
        assert!(stdout.contains("Rule fail-under-regions: PASS"));
    }
}

#[test]
fn function_threshold_fails_when_below_threshold() {
    for fixture in function_capable_fail_fixtures() {
        let temp = tempdir().expect("tempdir should exist");
        let worktree = setup_fixture_worktree(temp.path(), fixture);
        let diff_file = write_worktree_diff(temp.path(), &worktree);

        let output = run_covgate(
            &worktree,
            fixture,
            &[
                "--diff-file".to_string(),
                diff_file.to_string_lossy().into_owned(),
                "--fail-under-functions".to_string(),
                "100".to_string(),
            ],
        );

        let expected_status = if fixture.language == "dotnet" { 0 } else { 1 };
        assert_eq!(
            output.status.code(),
            Some(expected_status),
            "fixture={}",
            fixture.id()
        );
        let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
        if expected_status == 1 {
            assert!(stdout.contains("Rule fail-under-functions: FAIL"));
        } else {
            assert!(stdout.contains("Rule fail-under-functions: PASS"));
        }
        assert!(stdout.contains("Function Coverage:"));
    }
}

#[test]
fn function_threshold_passes_for_all_pass_fixtures() {
    for fixture in function_capable_pass_fixtures() {
        let temp = tempdir().expect("tempdir should exist");
        let worktree = setup_fixture_worktree(temp.path(), fixture);
        let diff_file = write_worktree_diff(temp.path(), &worktree);

        let output = run_covgate(
            &worktree,
            fixture,
            &[
                "--diff-file".to_string(),
                diff_file.to_string_lossy().into_owned(),
                "--fail-under-functions".to_string(),
                "100".to_string(),
            ],
        );

        assert_eq!(output.status.code(), Some(0), "fixture={}", fixture.id());
        let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
        assert!(stdout.contains("Rule fail-under-functions: PASS"));
    }
}

#[test]
fn uncovered_function_budget_fails_when_exceeded() {
    for fixture in function_capable_fail_fixtures() {
        let temp = tempdir().expect("tempdir should exist");
        let worktree = setup_fixture_worktree(temp.path(), fixture);
        let diff_file = write_worktree_diff(temp.path(), &worktree);

        let output = run_covgate(
            &worktree,
            fixture,
            &[
                "--diff-file".to_string(),
                diff_file.to_string_lossy().into_owned(),
                "--fail-uncovered-functions".to_string(),
                "0".to_string(),
            ],
        );

        let expected_status = if fixture.language == "dotnet" { 0 } else { 1 };
        assert_eq!(
            output.status.code(),
            Some(expected_status),
            "fixture={}",
            fixture.id()
        );
        let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
        if expected_status == 1 {
            assert!(stdout.contains("Rule fail-uncovered-functions: FAIL"));
        } else {
            assert!(stdout.contains("Rule fail-uncovered-functions: PASS"));
        }
    }
}
