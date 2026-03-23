# Reach 100% self-coverage and remove low-value self-referential tests

Save new in-progress ExecPlans in `docs/exec-plans/active/covgate-self-coverage-to-100.md`. Move the file to `docs/exec-plans/completed/covgate-self-coverage-to-100.md` when the work is complete.

This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds.

Maintain this document in accordance with `docs/PLANS.md` and follow the repository testing workflow in `docs/TESTING.md`.

## Purpose / Big Picture

After this work, a contributor should be able to run one self-coverage command from the repository root and see that the Rust crate is fully exercised by the checked-in test suite rather than merely clearing the current 88% region floor. The visible outcome is that `cargo llvm-cov --json --summary-only --output-path /tmp/covgate-self-coverage.json` reports 100% covered lines, regions, and functions for the `covgate` crate, and `cargo xtask validate` enforces that result so it cannot silently drift back down later.

This plan also removes one test that currently adds little confidence: `tests/llvm_real_parity.rs::real_multi_file_llvm_export_markdown_totals_match_covgate_calculations`. That test mostly proves that Markdown output matches `covgate`'s own internal totals. The replacement work must cover real remaining behavior gaps instead, especially renderer edge cases and control-flow branches that the current suite does not exercise.

## Progress

- [x] (2026-03-19 22:10Z) Re-read `docs/PLANS.md`, `docs/TESTING.md`, `xtask/src/main.rs`, `tests/llvm_real_parity.rs`, and the main source modules to understand the current self-coverage workflow and the low-value LLVM renderer-consistency test.
- [x] (2026-03-19 22:18Z) Captured a fresh self-coverage baseline with `cargo llvm-cov --json --summary-only --output-path /tmp/covgate-self-coverage.json` and `cargo llvm-cov report --text --show-missing-lines --output-path /tmp/covgate-self-coverage.txt`.
- [ ] Replace or remove `real_multi_file_llvm_export_markdown_totals_match_covgate_calculations` and add higher-signal tests that cover currently uncovered renderer behavior.
- [ ] Add focused tests for the remaining uncovered configuration, Git, diff, model, and parser branches identified in the baseline.
- [ ] Raise the repository validation gate from the current 88% region floor to strict 100% self-coverage enforcement once the suite is green.
- [ ] Re-run targeted tests, `cargo xtask quick`, `cargo llvm-cov --json --summary-only --output-path /tmp/covgate-self-coverage.json`, and `cargo xtask validate`, then update this plan with the final achieved totals.

## Surprises & Discoveries

- Observation: The remaining distance to 100% is much smaller than it first sounds.
  Evidence: The fresh baseline from `/tmp/covgate-self-coverage.json` reported lines `3188/3242` (98.33%), regions `3576/3689` (96.94%), and functions `180/185` (97.30%), so the plan only needs to cover 54 lines, 113 regions, and 5 functions.

- Observation: The largest remaining gaps are concentrated in a few modules rather than spread evenly across the codebase.
  Evidence: `cargo llvm-cov report --text --show-missing-lines` reported uncovered lines only in `src/config.rs`, `src/git.rs`, `src/model.rs`, `src/render/console.rs`, `src/render/markdown.rs`, `src/coverage/coverlet_json.rs`, `src/coverage/istanbul_json.rs`, `src/coverage/llvm_json.rs`, and `src/diff.rs`.

- Observation: Some uncovered lines live in test-only assertion scaffolding, not in production behavior.
  Evidence: `src/config.rs` lines `300` and `341` are `panic!("expected git base")` arms inside tests. Those lines count against self-coverage without proving meaningful repository behavior, so part of the work is to simplify that test code rather than trying to force an impossible execution path.

- Observation: The existing LLVM Markdown parity test is intentionally self-referential.
  Evidence: `tests/llvm_real_parity.rs::real_multi_file_llvm_export_markdown_totals_match_covgate_calculations` parses the real LLVM export, sums `report.totals_by_file`, renders Markdown, and asserts that the Markdown totals equal `covgate`'s own totals. That is useful as a renderer consistency check, but it is not an external semantic oracle and it does not directly cover the currently missing renderer branches.

## Decision Log

- Decision: Treat 100% self-coverage as a repository-owned behavior target, not just a one-time reporting exercise.
  Rationale: If the suite reaches 100% but `xtask validate` continues enforcing only `--fail-under-regions=88`, the repository can immediately drift back down without noticing.
  Date/Author: 2026-03-19 / Codex

- Decision: Replace or remove the low-value LLVM Markdown self-consistency test instead of preserving it for raw coverage numbers.
  Rationale: The user explicitly wants that test removed or replaced, and the repository already has stronger LLVM confidence coverage in `tests/llvm_diff_regression.rs` plus the architecture-level disagreement test in `tests/llvm_real_parity.rs`.
  Date/Author: 2026-03-19 / Codex

- Decision: Prefer high-signal unit and integration tests that exercise real control-flow branches over synthetic “touch the line” tests.
  Rationale: The goal is trustworthy self-coverage. Every new test should either prove a real user-facing behavior, a parser edge case, or a renderer/output contract that could plausibly regress.
  Date/Author: 2026-03-19 / Codex

- Decision: Clean up unreachable or low-value test-only branches when they meaningfully count against self-coverage.
  Rationale: For example, `panic!("expected git base")` arms in success-path tests add denominator without adding product confidence. Rewriting those assertions to `assert!(matches!(...))` is a better long-term shape than inventing artificial execution paths.
  Date/Author: 2026-03-19 / Codex

## Outcomes & Retrospective

This section is intentionally incomplete until the implementation finishes. At completion, summarize the final coverage totals, the replacement for the removed LLVM Markdown test, the validation gate changes, and any residual reasons not to enforce 100% on every reported metric.

## Context and Orientation

`covgate` is a Rust command-line tool in `src/` that reads a native coverage report, computes changed coverage opportunities from a Git diff, evaluates gate rules, and renders console and Markdown output. The self-coverage work in this plan is about coverage of `covgate` itself, not coverage of fixture projects under `tests/fixtures/`.

The self-coverage workflow already exists in `xtask/src/main.rs`. Today `cargo xtask validate` runs:

    cargo llvm-cov --json --output-path <tempfile> --fail-under-regions=88
    cargo run --bin covgate -- check <tempfile>

That means the repository already collects its own coverage report but only enforces an 88% region threshold. This plan upgrades that path to a strict 100% standard after the missing behavior is covered.

The current missing lines from `cargo llvm-cov report --text --show-missing-lines` are:

    src/config.rs: 90, 131, 132, 133, 134, 144, 145, 146, 147, 196, 197, 198, 199, 300, 341
    src/coverage/coverlet_json.rs: 30
    src/coverage/istanbul_json.rs: 52, 138
    src/coverage/llvm_json.rs: 370, 403
    src/diff.rs: 43
    src/git.rs: 71, 87, 123, 132, 190
    src/model.rs: 100, 101, 103, 107
    src/render/console.rs: 28, 113
    src/render/markdown.rs: 64, 111, 155

Those line numbers matter because they point directly at the real remaining work:

`src/config.rs` still lacks tests for the `--base` plus `--diff-file` conflict and for TOML fallback on line, branch, and uncovered-branch rules. Two uncovered lines are in tests that use panic-based match arms and should be simplified.

`src/git.rs` still has branch-marker and helper paths that are not exercised by the existing `tests/git_module.rs` and `tests/cli_interface.rs` cases, especially the “recorded base ref created while on a named branch” path and helper calls that currently only run in one direction.

`src/model.rs` has no direct test for `SourceSpan::display`, which is a tiny method but still user-visible through renderer output formatting.

`src/render/console.rs` and `src/render/markdown.rs` still lack explicit tests for zero-total file summaries rendering as `100.00%` and for the empty-string branch of the `title_case` helper. Those are exactly the kinds of lines that the current LLVM Markdown self-consistency test does not cover.

`src/diff.rs` still lacks a test where `git diff` itself exits non-zero after a successful `git merge-base`.

The parser modules are nearly complete already. The remaining missed lines there are narrow skip branches: non-object Coverlet class entries in `src/coverage/coverlet_json.rs`, the no-branch-summary path in `src/coverage/istanbul_json.rs`, the “path equals repo root” branch in the same file, and descending segment windows in `src/coverage/llvm_json.rs`.

The existing high-value LLVM confidence tests live in `tests/llvm_diff_regression.rs`. The architecture-signaling disagreement test lives in `tests/llvm_real_parity.rs::real_multi_file_llvm_export_documents_summary_semantics_disagreement`. Keep those. The low-value renderer-consistency test in the same file is the one this plan removes or replaces.

## Plan of Work

Start by locking in the baseline and preserving it in this document. Run the self-coverage commands from the repository root, capture the totals, and keep the missing-line list current in this plan as the implementation evolves. That gives every later change a measurable target.

The first implementation milestone should focus on renderer and small helper gaps because they are isolated and easy to verify. In `src/render/console.rs` and `src/render/markdown.rs`, add unit tests that construct a `GateResult` containing at least one file with `covered = 0` and `total = 0`, then assert that both changed-coverage and overall-coverage tables render `100.00%`. Also add direct helper tests for `title_case("")` returning the empty string. In `src/model.rs`, add a direct test for `SourceSpan::display()` so those user-visible formatting lines stop depending on incidental renderer coverage.

When that renderer work is ready, remove or replace `tests/llvm_real_parity.rs::real_multi_file_llvm_export_markdown_totals_match_covgate_calculations`. If the test is kept in some form, it must be rewritten so it covers a real regression surface that the repository does not already prove elsewhere. The preferred replacement is not another whole-report consistency check; it is a renderer- or CLI-facing test that proves formatting or metric-presence behavior the current suite misses. If no such rewrite cleanly fits there, delete the test and let the new renderer unit tests carry the useful coverage instead.

The second milestone should cover the configuration and Git control-flow gaps. In `src/config.rs`, add unit tests for three specific behaviors: the `--base` and `--diff-file` conflict error from `resolve_diff_source`, TOML-only `fail_under_lines` and `fail_under_branches` fallback, and TOML-only `fail_uncovered_branches` fallback. While editing those tests, replace `match ... { GitBase(base) => ..., DiffFile(_) => panic!(...) }` with `assert!(matches!(...))` or another non-panicking assertion style so the test-only uncovered lines disappear honestly.

For Git and diff handling, extend `tests/git_module.rs`, `tests/cli_interface.rs`, or `tests/diff_module.rs` with the smallest realistic reproductions of the uncovered paths. Add a first-recording case on a named branch that proves `record_base_ref()` writes the branch marker on initial creation. Add or extend a helper-path test so `resolve_git_path`, `resolve_current_branch`, and `is_ancestor` all execute through the currently uncovered success/error reporting lines. Add a `load_changed_lines(DiffSource::GitBase(...))` test where `git merge-base` succeeds but the later `git diff` call fails with a non-zero status, and assert the exact actionable error text.

The third milestone should close the parser edge cases. In `src/coverage/coverlet_json.rs`, add a parser unit test with a class entry that is not a JSON object so the `let Some(classes) = class_value.as_object() else { continue; }` branch is exercised directly. In `src/coverage/istanbul_json.rs`, add one test where no branch records exist so the `branch_totals_by_file` insertion path is skipped intentionally, and one test that passes a file path exactly equal to the repo root string so the normalization branch returning `PathBuf::new()` is exercised. In `src/coverage/llvm_json.rs`, add a segment-window fixture where the next segment line number is less than the current one so both descending-window `continue` branches are covered in a parser-focused test instead of by manufacturing an unrealistic integration scenario.

After the targeted gaps are closed, rerun self-coverage and inspect the new missing-line report. If a few uncovered lines remain in test scaffolding or tiny helpers, prefer simplifying or tightening the relevant tests over adding throwaway assertions. The end state should be that the remaining suite is still readable and each added test has an obvious reason to exist.

Once self-coverage is actually at 100%, update `xtask/src/main.rs` so `validate()` enforces the new standard. Replace the current `cargo llvm-cov --json --output-path ... --fail-under-regions=88` invocation with a stricter check that enforces 100% for the metrics we claim to have fully covered. If `cargo llvm-cov` can enforce lines, regions, and functions in a single invocation, use that. If it requires multiple checks, add them explicitly and keep the failure reporting readable. The plan only completes after `cargo xtask validate` fails when self-coverage drops below 100 again.

## Concrete Steps

Run all commands from the repository root, the directory containing `Cargo.toml`.

1. Refresh the self-coverage baseline and keep the outputs available while implementing.

       cargo llvm-cov --json --summary-only --output-path /tmp/covgate-self-coverage.json
       cargo llvm-cov report --text --show-missing-lines --output-path /tmp/covgate-self-coverage.txt

   Expected result: the JSON report contains top-level totals and the text report lists the exact uncovered source lines. At the baseline captured for this plan, the totals were:

       lines     3188/3242   98.33%
       regions   3576/3689   96.94%
       functions 180/185     97.30%

2. Replace or remove the LLVM Markdown self-consistency test and add higher-value renderer tests.

       cargo test llvm_real_parity -- --nocapture
       cargo test render -- --nocapture

   Expected result: the disagreement test in `tests/llvm_real_parity.rs` still passes, the low-value self-consistency test is gone or replaced, and the renderer tests now cover zero-total percentage rendering and empty-string title casing.

3. Add missing configuration, Git, diff, model, and parser tests in small slices.

       cargo test config -- --nocapture
       cargo test git_module -- --nocapture
       cargo test diff_module -- --nocapture
       cargo test llvm_json -- --nocapture
       cargo test istanbul_json -- --nocapture
       cargo test coverlet_json -- --nocapture

   Expected result: each command either introduces a new failing reproducer before the code or test cleanup, or passes after the intended gap is covered. Keep the new tests focused on one missing branch each.

4. Re-run the fast repository loop.

       cargo xtask quick

   Expected result: formatting, Clippy, and the full test suite pass before tightening the validation gate.

5. Re-run self-coverage and inspect the remaining gap.

       cargo llvm-cov --json --summary-only --output-path /tmp/covgate-self-coverage.json
       cargo llvm-cov report --text --show-missing-lines --output-path /tmp/covgate-self-coverage.txt

   Expected result: all reported source lines, regions, and functions are covered. If not, update this plan with the new missing-line list before continuing.

6. Tighten `cargo xtask validate` to enforce the 100% standard and prove it still passes.

       cargo xtask validate

   Expected result: the validation summary reports all checks passed, including the stricter self-coverage step.

## Validation and Acceptance

This plan is complete only when all of the following are true and visible to a novice running commands from the repository root.

`cargo llvm-cov --json --summary-only --output-path /tmp/covgate-self-coverage.json` reports full coverage for the crate under test. The acceptance bar is `count == covered` for lines, regions, and functions in the top-level `data[0].totals` object.

`cargo llvm-cov report --text --show-missing-lines --output-path /tmp/covgate-self-coverage.txt` no longer lists uncovered lines from `src/`.

The low-value test `tests/llvm_real_parity.rs::real_multi_file_llvm_export_markdown_totals_match_covgate_calculations` is either deleted or replaced by a test that covers a distinct real behavior gap and is documented in this plan's decision log.

The new tests are behavior-oriented. They must not merely call the same helper under test to restate its result. For renderers, the expected output strings must be asserted directly. For parsers, the expected normalized opportunities or totals must be asserted directly. For CLI and Git flows, the expected user-visible status and error text must be asserted directly.

`cargo xtask quick` passes during development and `cargo xtask validate` passes after the stricter self-coverage enforcement is in place.

## Idempotence and Recovery

The coverage-report commands in this plan are safe to rerun. They write temporary files in `/tmp` and do not mutate the repository.

Test additions should be made incrementally and kept narrow. If a new test unexpectedly exposes a real bug instead of a mere missing branch, stop broadening the self-coverage work and fix that behavior with TDD before continuing.

If tightening `cargo xtask validate` to 100% causes the repository to fail because a final branch is still uncovered, revert only the validation-threshold change, finish the missing test or simplification, and then reapply the stricter threshold once the self-coverage command is genuinely green.

If the chosen replacement for the LLVM Markdown self-consistency test does not clearly improve confidence, delete the old test first, add the higher-value unit or integration test in a separate commit or patch, and document the swap in the `Decision Log`.

## Artifacts and Notes

Representative baseline captured while drafting this plan:

    $ cargo llvm-cov --json --summary-only --output-path /tmp/covgate-self-coverage.json
    ...
    Finished report saved to /tmp/covgate-self-coverage.json

    $ jq '.data[0].totals' /tmp/covgate-self-coverage.json
    {
      "functions": { "count": 185, "covered": 180, "percent": 97.2972972972973 },
      "lines": { "count": 3242, "covered": 3188, "percent": 98.33436150524368 },
      "regions": { "count": 3689, "covered": 3576, "percent": 96.93683925182977 }
    }

Representative missing-line excerpt captured while drafting this plan:

    $ cargo llvm-cov report --text --show-missing-lines --output-path /tmp/covgate-self-coverage.txt
    Uncovered Lines:
    /home/jesse/git/covgate/src/config.rs: 90, 131, 132, 133, 134, 144, 145, 146, 147, 196, 197, 198, 199, 300, 341
    /home/jesse/git/covgate/src/git.rs: 71, 87, 123, 132, 190
    /home/jesse/git/covgate/src/render/console.rs: 28, 113
    /home/jesse/git/covgate/src/render/markdown.rs: 64, 111, 155

Representative current low-value test targeted by this plan:

    tests/llvm_real_parity.rs::real_multi_file_llvm_export_markdown_totals_match_covgate_calculations

It currently asserts that Markdown totals equal `report.totals_by_file` totals for a real LLVM export. That is acceptable as an internal consistency check, but weaker than direct renderer edge-case tests and weaker than the existing LLVM diff regression suite for semantic confidence.

## Interfaces and Dependencies

Use the existing repository modules and test layout. Do not introduce a second coverage harness.

The primary files expected to change are:

- `tests/llvm_real_parity.rs` for removing or replacing the low-value LLVM Markdown self-consistency test.
- `src/render/console.rs` and `src/render/markdown.rs` test modules for new renderer edge-case coverage.
- `src/model.rs` test module for direct `SourceSpan::display` coverage.
- `src/config.rs` test module for diff-source conflict and TOML fallback coverage.
- `tests/git_module.rs`, `tests/diff_module.rs`, and possibly `tests/cli_interface.rs` for realistic Git and diff branch coverage.
- `src/coverage/coverlet_json.rs`, `src/coverage/istanbul_json.rs`, and `src/coverage/llvm_json.rs` test modules for parser skip-branch coverage.
- `xtask/src/main.rs` for raising the self-coverage enforcement in `validate()`.

Prefer existing helper APIs and real fixture workflows:

- Use `cargo llvm-cov` for self-coverage measurement.
- Use the current temp-directory and real-Git-repository style already present in `tests/support/mod.rs`, `tests/git_module.rs`, and `tests/cli_interface.rs`.
- Keep parser tests close to the parser module they exercise unless the behavior is specifically end-to-end.

Revision note: Initial plan created after capturing a fresh self-coverage baseline, identifying the remaining uncovered source lines by module, and recording the requirement to remove or replace the low-value LLVM Markdown self-consistency test as part of the path to 100%.
