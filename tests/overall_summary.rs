mod support;

use support::{
    MetricFixtureCase, branch_capable_fail_fixtures, branch_capable_pass_fixtures,
    fail_fixtures_with_lines, fail_fixtures_with_regions, function_capable_fail_fixtures,
    function_capable_pass_fixtures, pass_fixtures_with_lines, pass_fixtures_with_regions,
};

#[test]
fn overall_summary_region_totals_match_native_summary_for_llvm_fixtures() {
    let fixtures = fail_fixtures_with_regions()
        .into_iter()
        .chain(pass_fixtures_with_regions())
        .collect::<Vec<_>>();

    for fixture in fixtures {
        let case = MetricFixtureCase::new(fixture, "region");
        let native = case
            .native_overall_totals()
            .expect("native totals should exist");
        let markdown = case
            .covgate_markdown_overall_totals()
            .expect("markdown totals should exist");
        assert_eq!(
            native,
            markdown,
            "fixture {} metric region",
            case.fixture_id()
        );
    }
}

#[test]
fn overall_summary_line_totals_match_native_summary_for_all_line_capable_fixtures() {
    let fixtures = fail_fixtures_with_lines()
        .into_iter()
        .chain(pass_fixtures_with_lines())
        .chain([
            support::dotnet_duplicate_lines_fixture(),
            support::vitest_statement_line_divergence_fixture(),
        ])
        .collect::<Vec<_>>();

    for fixture in fixtures {
        let case = MetricFixtureCase::new(fixture, "line");
        let native = case
            .native_overall_totals()
            .expect("native totals should exist");
        let markdown = case
            .covgate_markdown_overall_totals()
            .expect("markdown totals should exist");
        assert_eq!(
            native,
            markdown,
            "fixture {} metric line",
            case.fixture_id()
        );
    }
}

#[test]
fn native_summary_artifacts_exist_for_line_repro_fixtures() {
    for fixture in [
        support::dotnet_duplicate_lines_fixture(),
        support::vitest_statement_line_divergence_fixture(),
    ] {
        assert!(
            fixture.root().join("native-summary.json").exists(),
            "fixture {} should include a captured native summary artifact",
            fixture.id()
        );
    }
}

#[test]
fn overall_summary_branch_totals_match_native_summary_for_branch_capable_fixtures() {
    let fixtures = branch_capable_fail_fixtures()
        .into_iter()
        .chain(branch_capable_pass_fixtures())
        .collect::<Vec<_>>();

    for fixture in fixtures {
        let case = MetricFixtureCase::new(fixture, "branch");
        let native = case
            .native_overall_totals()
            .expect("native totals should exist");
        let markdown = case
            .covgate_markdown_overall_totals()
            .expect("markdown totals should exist");
        assert_eq!(
            native,
            markdown,
            "fixture {} metric branch",
            case.fixture_id()
        );
    }
}

#[test]
fn overall_summary_function_totals_match_native_summary_for_function_capable_fixtures() {
    let fixtures = function_capable_fail_fixtures()
        .into_iter()
        .chain(function_capable_pass_fixtures())
        .collect::<Vec<_>>();

    for fixture in fixtures {
        let case = MetricFixtureCase::new(fixture, "function");
        let native = case
            .native_overall_totals()
            .expect("native totals should exist");
        let markdown = case
            .covgate_markdown_overall_totals()
            .expect("markdown totals should exist");
        assert_eq!(
            native,
            markdown,
            "fixture {} metric function",
            case.fixture_id()
        );
    }
}
