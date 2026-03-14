# Add C/C++ and Swift LLVM coverage fixtures

Save this in-progress ExecPlan at `docs/exec-plans/active/covgate-cpp-swift-llvm-fixtures.md` while the work is being designed or implemented in this repository.

This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds.

Maintain this document in accordance with `docs/PLANS.md`. Re-read that file before revising this plan and keep this plan aligned with its rules.

## Purpose / Big Picture

After this change, `covgate` will have real copied-fixture integration scenarios for LLVM JSON generated from C/C++ and Swift projects in addition to the existing Rust fixtures. A contributor will be able to run the CLI test suite and see the same metric rules exercised across multiple languages that all emit LLVM-style coverage data, rather than relying on one Rust-only view of the product.

This matters because the product promise is not “works on one Rust-shaped LLVM export.” The tool claims to evaluate LLVM JSON coverage in a language-agnostic way. The new behavior should prove that with live fixture repositories that build, test, emit coverage, and then run `covgate` against diffs for C/C++, Swift, and Rust. It should also close a current gap in branch coverage testing: C/C++ and Swift LLVM reports are expected to include branch data, so the repository should add positive branch-threshold cases rather than only negative “metric not available” Rust cases.

You will know this work is complete when a contributor can run the repository CLI tests from the repository root and observe:

    cargo test --test cli_metrics
    cargo test --test cli_interface

and see metric-oriented cases reuse Rust, C/C++, and Swift fixtures wherever the scenario semantics are the same, while interface-oriented cases remain focused on CLI and config behavior without unnecessary multi-language duplication.

## Progress

- [x] (2026-03-14 11:30Z) Create a dedicated active ExecPlan for adding C/C++ and Swift LLVM fixture coverage and record the test-organization expectations requested by the user.
- [x] (2026-03-14 11:30Z) Update `docs/TESTING.md` to document fixture-matrix metric testing and the split between metric tests and CLI interface tests.
- [ ] Add copied fixture repositories under `tests/fixtures/cpp/` and `tests/fixtures/swift/` with baseline repo content, overlay changes, and generated LLVM JSON coverage artifacts.
- [ ] Refactor the current monolithic `tests/cli.rs` coverage into separate metric-focused and interface-focused integration test files with shared helpers.
- [ ] Add positive branch-threshold and uncovered-branch-budget CLI tests using the new branch-capable fixtures.
- [ ] Re-run the full repository validation stack and confirm the new fixture matrix is stable and documented.

## Surprises & Discoveries

- Observation: The current repository has placeholder fixture roots for `.NET` and Vitest, but no checked-in C/C++ or Swift fixtures yet.
  Evidence: `find tests -maxdepth 3 -type f | sort` currently shows `tests/fixtures/dotnet/README.md` and `tests/fixtures/vitest/README.md`, but no C/C++ or Swift fixture trees.

- Observation: All current CLI integration coverage lives in one file and is Rust-specific.
  Evidence: `tests/cli.rs` hardcodes `tests/fixtures/rust/<name>` in `fixture_root()` and mixes metric behavior tests with CLI/config behavior tests such as `uses_repo_config_defaults_for_base_and_threshold`.

- Observation: Current branch CLI coverage is intentionally negative because the Rust LLVM fixtures do not contain non-empty branch arrays.
  Evidence: `tests/cli.rs` includes `assert_fixture_has_no_branch_coverage()` and uses it only in `branch_threshold_switch_reports_metric_not_available` and `uncovered_branch_budget_switch_reports_metric_not_available`.

## Decision Log

- Decision: Metric-oriented CLI tests must define a list of fixtures to exercise, and each test case should run against all compatible fixtures whenever possible.
  Rationale: The product promise for metrics is cross-language behavior on normalized LLVM JSON. Repeating the same metric assertion across Rust, C/C++, and Swift gives stronger confidence than language-specific one-offs. “Whenever possible” means a test should include all fixtures unless the scenario is fundamentally incompatible with a fixture’s capabilities, such as Rust fixtures that intentionally lack branch data for “metric not available” coverage.
  Date/Author: 2026-03-14 / Codex

- Decision: CLI metric tests and CLI interface tests should live in separate integration-test files.
  Rationale: Metric behavior is where fixture-matrix reuse pays off. Interface tests such as config precedence, `--base`, diff file handling, and Markdown output primarily validate the command surface and do not need to run across every language fixture. Splitting these concerns keeps each test file easier to understand and prevents needless build/test duplication.
  Date/Author: 2026-03-14 / Codex

- Decision: The repository should add positive branch coverage tests once C/C++ and Swift fixtures are present.
  Rationale: The current negative Rust-only cases prove graceful handling when branch data is absent, but they do not prove that branch metrics actually work. The new branch-capable fixtures should be used to add pass and fail cases for both `--fail-under-branches` and `--fail-uncovered-branches`.
  Date/Author: 2026-03-14 / Codex

- Decision: The new fixture families should check in generated `coverage.json` artifacts and use them during normal test runs instead of regenerating coverage on every `cargo test` invocation.
  Rationale: The repository wants live, auditable fixture projects, but routine integration runs should stay fast and deterministic. Checked-in coverage artifacts preserve the exact report shape under test while still allowing deliberate regeneration whenever a fixture changes.
  Date/Author: 2026-03-14 / Codex

## Outcomes & Retrospective

At this stage the outcome is a concrete execution plan and an explicit testing philosophy update, not yet implemented fixture code. The important product direction is now documented: language-agnostic LLVM support must be proven with live C/C++ and Swift fixtures, and future CLI test structure should separate metric-matrix coverage from interface-only behavior.

The main implementation risk is fixture maintenance cost. Live multi-language fixtures can become noisy if every test rebuilds its own language-specific command pipeline independently. This plan reduces that risk by requiring shared helpers, language-specific fixture metadata, and a deliberate split between tests that must iterate across fixtures and tests that should stay single-purpose.

## Context and Orientation

`covgate` is a Rust CLI that currently parses LLVM JSON and compares changed coverage against a Git diff. The existing integration coverage lives entirely in `tests/cli.rs`. That file sets up a temporary Git repository, copies a checked-in Rust fixture from `tests/fixtures/rust/`, runs the built `covgate` binary, and asserts on CLI output. It currently contains three broad kinds of tests mixed together:

Metric behavior tests check whether threshold rules pass or fail for lines, regions, uncovered counts, and branch metric availability. Examples include `basic_fail_rust_fixture`, `line_metric_fails_when_below_threshold`, and `branch_threshold_switch_reports_metric_not_available`.

CLI interface tests check whether the user interface behaves correctly regardless of language-specific coverage details. Examples include `markdown_summary_rust_fixture`, `uses_repo_config_defaults_for_base_and_threshold`, `mixed_cli_over_toml_precedence`, and `cli_threshold_overrides_repo_config_default`.

Fixture plumbing helpers such as `setup_fixture_worktree`, `write_worktree_diff`, and `run_covgate` are currently embedded at the bottom of `tests/cli.rs` and assume a Rust fixture root.

In this plan, a “fixture” means a checked-in language project under `tests/fixtures/<language>/<scenario>/` that contains at least a baseline repository skeleton in `repo/`, a set of changed files in `overlay/`, and a checked-in `coverage.json` file generated from the language’s native test coverage tooling. A “metric test” means a CLI test whose core claim is about coverage semantics, such as “region threshold fails when observed region coverage is below the configured minimum.” An “interface test” means a CLI test whose core claim is about flags, config precedence, output file creation, or diff-source selection rather than language-specific metric math.

The plan assumes the repository setup work already completed in `scripts/setup-agent-env.sh`: C/C++ tooling now includes `clang`, `llvm-profdata`, and `llvm-cov`, and Swift tooling is installed via `swiftly`. Those tools make it possible to generate LLVM JSON from live C/C++ and Swift fixture projects during fixture authoring. The checked-in tests should consume committed `coverage.json` artifacts for speed and determinism. Fixture regeneration should be a deliberate maintenance workflow when a fixture changes, not part of every routine `cargo test` run.

## Plan of Work

Begin by creating two new live fixture families under `tests/fixtures/cpp/` and `tests/fixtures/swift/`. Each family should mirror the existing Rust layout closely enough that shared helpers can treat fixtures generically. At minimum, create one passing scenario and one failing scenario that differ in changed coverage the same way the current Rust `basic-pass` and `basic-fail` fixtures do. The baseline repository should contain a tiny library plus tests. The overlay should introduce a small diff that changes exactly the lines or branches the CLI assertions care about. Then generate and check in `coverage.json` from the language’s native coverage flow. For C/C++, compile and run tests with Clang source-based coverage, merge profiles with `llvm-profdata`, and export JSON with `llvm-cov export`. For Swift, run `swift test --enable-code-coverage`, locate the produced profdata and test binary, and export with `llvm-cov export`. The committed integration tests should read those checked-in coverage artifacts rather than regenerating them on every test invocation.

While authoring those fixtures, ensure the C/C++ and Swift scenarios intentionally expose branch coverage in the generated LLVM JSON. Do not merely accept whatever the toolchain emits by accident. The fixture authoring notes in the repository should make clear which source constructs create observable branch points, such as an `if` statement with both covered and uncovered paths. The goal is that branch thresholds can be tested positively, not only as “metric unavailable.” At least one pass scenario and one fail scenario should include branch data that can drive `--fail-under-branches` and `--fail-uncovered-branches`.

Next, refactor the integration tests so coverage semantics and interface semantics are separated. Move the metric-threshold tests into a new file such as `tests/cli_metrics.rs`. That file should define a fixture descriptor type, for example a Rust struct that records the language, fixture name, coverage capabilities, and any scenario-specific expectations. Metric tests should iterate a list of descriptors and run the same assertion against every compatible fixture. For example, the region fail-under test should exercise Rust, C/C++, and Swift fail fixtures. The region pass test should exercise Rust, C/C++, and Swift pass fixtures. Line fail-under and uncovered-line tests should do the same if those fixtures expose comparable changed-line semantics. Branch metric tests should iterate only the branch-capable fixtures for positive pass/fail cases, while the existing Rust-only “metric not available” scenarios should remain as targeted negative tests because they intentionally demonstrate unsupported metric data in those reports.

In parallel, move interface-oriented tests into a new file such as `tests/cli_interface.rs`. This file should cover Markdown output, `--base` behavior, config defaults, mixed CLI-over-TOML precedence, and similar command-surface concerns. These tests should use the minimum number of fixtures needed to prove the behavior. They do not need to iterate every language fixture because the claim is about argument parsing, config precedence, or output writing, not the correctness of metric normalization across languages. The plan explicitly prefers keeping these tests single-fixture unless a specific interface bug has historically been language-sensitive.

To support both files, extract the common test harness into a shared helper module under `tests/common/` or another integration-test-visible path. The shared module should stop assuming a Rust fixture root and instead accept a fixture descriptor or a language-plus-name pair. It should expose helper functions for copying fixture trees, initializing Git repositories, generating diffs, rewriting absolute paths in checked-in coverage JSON when needed, and running the `covgate` binary. The helper layer should also include capability checks such as “this fixture has branch coverage” so metric tests can programmatically decide whether a scenario should be included in the matrix or intentionally excluded.

After the fixture and test split are in place, add positive branch tests for parity with existing line and region tests. At minimum, `tests/cli_metrics.rs` should include one branch fail-under passing scenario, one branch fail-under failing scenario, one uncovered-branch-budget passing scenario, and one uncovered-branch-budget failing scenario using the branch-capable C/C++ and Swift fixtures. Keep the existing Rust negative tests, but rename them or comment them clearly so they read as a separate unsupported-metric behavior check rather than the only branch coverage validation in the repository.

Finally, update repository documentation where needed so future contributors understand the split. `docs/TESTING.md` should already explain that metric tests iterate across compatible fixtures and that interface tests stay separate. If fixture-specific regeneration notes become non-obvious during implementation, add a concise `README.md` under `tests/fixtures/cpp/` and `tests/fixtures/swift/` describing how the checked-in `coverage.json` was produced and how to regenerate it safely.

## Concrete Steps

Work from the repository root at `/home/jesse/git/covgate`.

1. Author the new fixture directories and generate their committed coverage artifacts.

    Create directories following this shape:

        tests/fixtures/cpp/basic-pass/repo/
        tests/fixtures/cpp/basic-pass/overlay/
        tests/fixtures/cpp/basic-pass/coverage.json
        tests/fixtures/cpp/basic-fail/repo/
        tests/fixtures/cpp/basic-fail/overlay/
        tests/fixtures/cpp/basic-fail/coverage.json
        tests/fixtures/swift/basic-pass/repo/
        tests/fixtures/swift/basic-pass/overlay/
        tests/fixtures/swift/basic-pass/coverage.json
        tests/fixtures/swift/basic-fail/repo/
        tests/fixtures/swift/basic-fail/overlay/
        tests/fixtures/swift/basic-fail/coverage.json

    Generate the coverage artifacts from the fixture project directories using language-native commands. Representative authoring commands are:

        clang -fprofile-instr-generate -fcoverage-mapping ...
        LLVM_PROFILE_FILE=fixture.profraw ctest
        llvm-profdata merge -sparse fixture.profraw -o fixture.profdata
        llvm-cov export ./path/to/test-binary -instr-profile=fixture.profdata > coverage.json

        swift test --enable-code-coverage
        llvm-cov export <swift-test-binary> -instr-profile <default.profdata> > coverage.json

    Expected outcome: each new fixture directory contains a committed `coverage.json` whose file paths normalize correctly under the copied worktree used by the integration harness.

2. Split and generalize the CLI integration harness.

    Move the generic helper functions out of `tests/cli.rs` into a shared helper module. Introduce a fixture descriptor abstraction that lets tests select `rust/basic-pass`, `cpp/basic-pass`, or `swift/basic-pass` without duplicating harness code.

    Expected outcome: both metric-focused and interface-focused test files can call the same helper functions, and no helper assumes the fixture root is always `tests/fixtures/rust`.

3. Create the metric matrix test file.

    Add `tests/cli_metrics.rs` and migrate threshold semantics there. Each metric-oriented test should define the fixtures it exercises as an explicit list and then iterate that list. When a scenario is language-agnostic, the list should include every compatible fixture. When a scenario depends on a capability that some fixtures intentionally lack, the list should include only the compatible fixtures and the test should state why.

    Expected outcome: tests for regions, lines, uncovered counts, and branches visibly exercise multiple languages rather than one hardcoded Rust fixture.

4. Create the interface-focused test file.

    Add `tests/cli_interface.rs` and move command-surface tests there. Keep these tests focused on one representative fixture unless the behavior truly depends on language-specific coverage shape.

    Expected outcome: config precedence, Markdown output, diff source, and absolute-path normalization behavior remain covered without forcing every interface test to build a cross-language matrix.

5. Add branch-positive assertions and retire the single-file layout.

    Add passing and failing branch-threshold cases for both `--fail-under-branches` and `--fail-uncovered-branches` using the branch-capable fixtures. Keep the Rust negative branch-availability tests. Remove or shrink the old `tests/cli.rs` file so the new file split is the canonical structure.

    Expected outcome: branch metric support is proven by both positive and negative CLI behavior, and the integration test layout matches the documented testing philosophy.

6. Run repository validation and capture the observable result.

    Run:

        cargo test --test cli_metrics
        cargo test --test cli_interface
        cargo test
        cargo xtask validate

    Expected outcome: the new integration tests pass, the full repository suite passes, and the result demonstrates that LLVM JSON metric behavior is consistent across the available language fixtures.

## Validation and Acceptance

Acceptance is complete only when all of the following behaviors are true.

Running `cargo test --test cli_metrics` from the repository root executes metric-threshold cases that reuse the same scenario across all compatible fixtures. A region fail-under case should not be Rust-only once equivalent C/C++ and Swift fixtures exist. A line uncovered-budget case should likewise exercise every compatible fixture. A branch-positive case should exercise at least the branch-capable C/C++ and Swift fixtures. The test code should make those fixture lists visible in the test body or helper data so a reader can see which languages each scenario covers.

Running `cargo test --test cli_interface` executes CLI-surface tests such as config precedence, base reference defaults, diff-source behavior, Markdown output, and absolute-path normalization without forcing each of those tests through every language fixture. The file split should make it obvious that these tests validate interface behavior rather than metric parity.

Running the branch metric CLI tests must demonstrate both positive and negative behavior. Positive behavior means a branch-capable fixture can pass or fail `--fail-under-branches` and `--fail-uncovered-branches` with clear output. Negative behavior means the Rust fixture still produces the “requested metric branch is not available in the report” error when that is the intended scenario.

The helper layer must support all three fixture families through one interface. A novice should be able to add a future LLVM language fixture by creating a new fixture tree, generating and checking in its `coverage.json`, and adding one descriptor entry, not by copy-pasting an entire Rust-specific harness.

Documentation must clearly state the expected split: metric tests iterate across compatible fixtures whenever possible; interface tests remain separate and do not need to iterate the whole language matrix unless the behavior under test genuinely depends on it.

## Idempotence and Recovery

This work is intentionally additive and should be safe to repeat. Re-running fixture generation commands should overwrite only the checked-in coverage artifacts for the targeted fixture directories. Re-running the integration tests should recreate temporary worktrees and Git repositories from the checked-in `repo/`, `overlay/`, and committed `coverage.json` files without mutating the committed fixture baselines or rebuilding every fixture from scratch.

If a fixture’s coverage artifact no longer matches the copied worktree because paths changed, regenerate only that fixture’s `coverage.json` and then rerun the targeted test file before rerunning `cargo xtask validate`. If the test split introduces confusion during migration, keep the old `tests/cli.rs` temporarily as a thin re-export or placeholder only long enough to keep `cargo test` green, then remove the duplication once the new files are stable.

## Artifacts and Notes

Representative examples of the intended test organization after implementation:

    #[test]
    fn region_threshold_fails_when_below_threshold() {
        for fixture in fail_fixtures_with_regions() {
            assert_region_fail_under_failure(fixture, 60.0);
        }
    }

    #[test]
    fn uses_repo_config_defaults_for_base_and_threshold() {
        let fixture = rust_basic_fail_fixture();
        assert_repo_config_defaults_behavior(fixture);
    }

Representative examples of positive and negative branch intent:

    #[test]
    fn branch_threshold_passes_for_branch_capable_fixtures() {
        for fixture in pass_fixtures_with_branches() {
            assert_branch_fail_under_pass(fixture, 50.0);
        }
    }

    #[test]
    fn branch_threshold_reports_metric_not_available_for_rust_fixture() {
        assert_branch_metric_unavailable(rust_basic_fail_fixture());
    }

These examples are not exact code requirements, but they show the expected shape: explicit fixture lists for metric tests, single-purpose interface tests, and both positive and negative branch coverage.

## Interfaces and Dependencies

Use the existing integration-test stack in Rust stable. Do not add a new external test framework unless implementation uncovers a concrete limitation in the standard integration-test approach.

The fixture harness should end with a shared helper interface that can represent fixture language, fixture scenario name, coverage artifact path, and metric capabilities. Whether this is a struct, enum, or helper function family is an implementation detail, but the interface must let metric tests iterate compatible fixtures without repeating path logic.

The final integration test layout should include at least:

- `tests/cli_metrics.rs` for threshold and metric-availability behavior
- `tests/cli_interface.rs` for config, diff, output, and command-surface behavior
- a shared helper module under `tests/` that both files import

The fixture tree should end with at least:

- `tests/fixtures/rust/` as the existing LLVM fixture family
- `tests/fixtures/cpp/` as the new C/C++ LLVM fixture family
- `tests/fixtures/swift/` as the new Swift LLVM fixture family

Keep the existing Rust “branch metric unavailable” scenarios even after branch-capable fixtures exist. Those negative cases remain part of the supported behavior because some LLVM producers do not emit every metric family.

At the bottom of this plan, append a revision note every time the plan changes materially, describing what changed and why.

Revision note: Initial plan created to add real C/C++ and Swift LLVM JSON fixtures, document metric-matrix testing across compatible fixtures, and require a split between metric semantics tests and CLI interface tests.

Revision note: Updated the plan to make checked-in `coverage.json` artifacts the default for routine test runs, with regeneration treated as a deliberate fixture-maintenance workflow for speed and determinism.
