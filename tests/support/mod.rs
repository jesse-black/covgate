#![allow(dead_code, unused_imports)]

pub mod fixtures;
pub mod git;
pub mod parity;
pub mod runner;

pub use fixtures::{
    Fixture, branch_capable_fail_fixtures, branch_capable_pass_fixtures, cpp_basic_fail_fixture,
    cpp_basic_pass_fixture, dotnet_basic_fail_fixture, dotnet_basic_pass_fixture,
    dotnet_duplicate_lines_fixture, fail_fixtures_with_lines, fail_fixtures_with_regions,
    function_capable_fail_fixtures, function_capable_pass_fixtures, pass_fixtures_with_lines,
    pass_fixtures_with_regions, rust_basic_fail_fixture, rust_basic_pass_fixture,
    swift_basic_fail_fixture, swift_basic_pass_fixture, vitest_basic_fail_fixture,
    vitest_basic_pass_fixture, vitest_empty_branch_locations_fixture,
    vitest_path_scoped_gates_fixture, vitest_statement_line_divergence_fixture,
    vitest_tsx_line_summary_fixture,
};

pub use git::{copy_tree, init_git_repo, run_git, setup_fixture_worktree, write_worktree_diff};

pub use parity::{
    MetricFixtureCase, OverallTotals, write_absolute_path_coverage_fixture,
    write_rebased_real_llvm_fixture,
};

pub use runner::{CovgateCommand, covgate};
