# Reproduce and fix native-summary parity drift for Coverlet and Istanbul fixtures

Save this in-progress ExecPlan in `docs/exec-plans/active/covgate-native-summary-parity-fixture-repros.md`. Move it to `docs/exec-plans/completed/covgate-native-summary-parity-fixture-repros.md` only after the reproducer fixtures, failing tests, helper fixes, validation runs, and retrospective notes are complete.

This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds.

Maintain this document in accordance with `docs/PLANS.md`. Re-read that file before revising this plan and keep this plan aligned with its rules.

## Purpose / Big Picture

`covgate` already parses Coverlet and Istanbul coverage into its internal line metrics, but the overall-summary parity tests currently derive their “native” totals with helper code that does not match the parser semantics for line counting. That creates a dangerous false-positive path: a parity test can say “native and covgate match” even when the helper is counting the wrong thing.

After this work, a novice should be able to regenerate one .NET fixture and one Vitest fixture that explicitly reproduce the mismatches under review, run a targeted parity test that fails before the helper fix, apply the helper fix, and then rerun the same test to prove that `covgate`’s overall line totals are being checked against the correct native interpretation. The user-visible result is stronger confidence that the Markdown summary parity tests really validate native-format behavior instead of accidental agreement with flawed test helpers.

## Progress

- [x] (2026-03-18 00:35Z) Re-read `docs/PLANS.md`, `docs/TESTING.md`, `src/coverage/coverlet_json.rs`, `src/coverage/istanbul_json.rs`, `tests/support/mod.rs`, and `tests/overall_summary.rs` to confirm the current mismatch and ground this plan in the checked-in code.
- [x] (2026-03-18 00:40Z) Confirmed the exact helper drift: Coverlet native line totals still count raw per-method `Lines` entries, while Istanbul native line totals still count raw `s` entries instead of `statementMap`-derived line ranges.
- [ ] Add explicit fixture scenarios that reproduce the Coverlet duplicate-line case and the Istanbul multi-line / multi-statement-on-one-line case using native toolchains and checked-in `coverage.json` artifacts.
- [ ] Add failing tests that isolate the helper mismatch before any fix lands.
- [ ] Update the native-summary helper logic in `tests/support/mod.rs` so Coverlet and Istanbul line totals match parser semantics exactly.
- [ ] Re-run targeted tests, then `cargo xtask quick`, then `cargo xtask validate`.
- [ ] Close this plan with a retrospective that records the final fixture shapes, the fixed helper rules, and any follow-up gaps.

## Surprises & Discoveries

- Observation: The parser-side logic is already correct for the review comments under discussion; the drift lives in the parity test helpers, not in `covgate`’s ingestion path.
  Evidence: `src/coverage/coverlet_json.rs` merges duplicate method-reported lines into a per-file `BTreeMap<u32, bool>`, and `src/coverage/istanbul_json.rs` expands line coverage from `statementMap` ranges before counting totals.

- Observation: The current line parity test in `tests/overall_summary.rs` is broad enough to cover `.NET` and Vitest fixtures, but it will only expose this bug if the checked-in fixtures actually contain the problematic report shapes.
  Evidence: the shared test iterates every line-capable fixture through `MetricFixtureCase::native_overall_totals()` and compares it against the Markdown summary path.

- Observation: The repository’s testing policy already requires exactly the kind of reproduction this review feedback needs: real fixture source projects, native toolchains, checked-in artifacts, and TDD for bugs.
  Evidence: `docs/TESTING.md` requires adding a failing test first and regenerating fixture coverage artifacts through xtask rather than hand-authoring JSON.

## Decision Log

- Decision: Treat this as a fixture-and-test-helper bug, not a parser bug.
  Rationale: The checked-in parser modules already implement the semantics described in the review feedback, so the smallest correct fix is to make the native-summary test helpers mirror those semantics.
  Date/Author: 2026-03-18 / Codex

- Decision: Add dedicated reproduction fixtures instead of mutating the existing `basic-pass` and `basic-fail` scenarios to carry extra edge-case meaning.
  Rationale: The current basic fixtures should remain small smoke tests. Review regressions deserve scenario names that make the edge case obvious to future readers, such as duplicate Coverlet method lines or Istanbul statement/line divergence.
  Date/Author: 2026-03-18 / Codex

- Decision: Keep the reproducer artifacts native-generated through xtask even if hand-editing the JSON would be faster.
  Rationale: `docs/TESTING.md` explicitly forbids hand-authored fixture coverage JSON for routine repository validation. The whole point of this work is to prove real tool output behavior.
  Date/Author: 2026-03-18 / Codex

- Decision: Add focused tests for the helper semantics in addition to relying on the broad overall-summary parity suite.
  Rationale: The matrix-style parity test proves end-to-end behavior, but a smaller helper-focused test will make future regressions easier to diagnose without having to infer the cause from fixture-wide Markdown mismatches.
  Date/Author: 2026-03-18 / Codex

## Outcomes & Retrospective

Implementation has not started yet. The important outcome so far is a sharper problem statement: the review feedback is still valid because the test helper logic does not match the parser logic, and the current fixtures are not yet guaranteed to exercise the divergence.

The main lesson at this stage is that parity tests are only trustworthy when both sides of the comparison express the same native semantics. “Native total” is not a vague idea here. For Coverlet it means unique source lines per file after method-level duplication is collapsed, and for Istanbul it means unique source lines reached by statement ranges, not raw statement counter count.

## Context and Orientation

`covgate` parses coverage formats in `src/coverage/`. The Coverlet adapter lives in `src/coverage/coverlet_json.rs`, and the Istanbul adapter lives in `src/coverage/istanbul_json.rs`. Both convert ecosystem-specific coverage payloads into `CoverageOpportunity` records and per-file totals that later feed gate calculations and Markdown rendering.

The overall-summary parity tests live in `tests/overall_summary.rs`. They do not read parser internals directly. Instead, they build `MetricFixtureCase` values from checked-in fixtures, compute a supposed native total through helper functions in `tests/support/mod.rs`, run the CLI to emit Markdown, parse the Markdown summary, and assert equality. That means any bug in the helper logic can make the parity suite compare `covgate` against the wrong “native” answer.

The current helper mismatch is specific to line metrics. In this plan, “Coverlet duplicate-line case” means a `.NET` coverage artifact where two methods in the same source file report the same line number in their `Lines` maps. `covgate` currently merges those by line number before counting totals, so the test helper must do the same. “Istanbul statement/line divergence case” means a Vitest coverage artifact where the `statementMap` does not have a one-to-one relationship with source lines, such as multiple statements on one line or one statement spanning multiple lines. `covgate` currently expands statement ranges into source lines before counting totals, so the test helper must do the same.

Relevant files the implementer will need to navigate are:

- `src/coverage/coverlet_json.rs` for the actual Coverlet line-counting behavior.
- `src/coverage/istanbul_json.rs` for the actual Istanbul line-counting behavior.
- `tests/support/mod.rs` for the helper functions `coverlet_native_overall_totals` and `istanbul_native_overall_totals` that currently drift from those semantics.
- `tests/overall_summary.rs` for the end-to-end parity assertions.
- `tests/fixtures/dotnet/` for checked-in .NET fixture repositories and coverage artifacts.
- `tests/fixtures/vitest/` for checked-in Vitest fixture repositories and coverage artifacts.
- `xtask/src/main.rs` for fixture regeneration entry points and any new scenario registration needed to rebuild the reproducer artifacts.

## Plan of Work

Start by creating two new fixture scenarios with native toolchains and deliberately shaped source code. For `.NET`, add a new fixture under `tests/fixtures/dotnet/` whose source file contains at least two methods that Coverlet will report against the same source line. Expression-bodied members, multiple lambdas on one line, or two tiny methods intentionally written on one line are acceptable as long as the generated `coverage.json` proves the same line number appears in multiple method `Lines` maps for one file. The fixture should include a pass/fail shape only if needed for the existing harness; otherwise a single reproduction-oriented scenario is enough.

For Vitest, add a new fixture under `tests/fixtures/vitest/` whose source file forces line counting to diverge from raw statement counting. The safest shape is one statement spanning multiple lines plus another pair of statements sharing one source line. That guarantees both directions of drift are visible: statement count greater than line count in one area and line count greater than statement count in another. The source should stay small and deterministic, and coverage must still be generated through `vitest run --coverage` so the checked-in artifact is real Istanbul output.

Once the new fixture directories exist, wire them into xtask regeneration if they are not automatically discovered. The plan is complete only if a novice can run `cargo xtask regen-fixture-coverage dotnet/<scenario>` and `cargo xtask regen-fixture-coverage vitest/<scenario>` from the repository root and reproduce the checked-in artifacts without manual JSON edits.

Next, add failing tests before fixing the helper logic. One layer of tests should stay end-to-end: extend `tests/overall_summary.rs` or the fixture lists in `tests/support/mod.rs` so the new reproducer scenarios participate in line parity checks. A second layer should be narrower and diagnostic. Add a helper-focused test module, likely near `tests/support/mod.rs` or in a new integration test, that reads the reproducer JSON and asserts the currently expected native line totals for each scenario. Before the fix, these tests should fail because the helper still counts per-method Coverlet `Lines` entries and raw Istanbul `s` entries.

After the failure is demonstrated, fix `tests/support/mod.rs` by making the native helper semantics mirror the parser code. For Coverlet line totals, accumulate per-file line numbers in a deduplicating map and merge coverage with boolean OR so a line is considered covered if any method reports hits on that line. For Istanbul line totals, iterate `statementMap`, look up each statement’s hit count in `s`, expand the statement’s `start.line..=end.line` range into a deduplicating per-file map, and mark lines covered if any covering statement has hits. Keep function and branch logic unchanged unless the reproducer shows an unexpected secondary mismatch.

Finish by rerunning the new targeted failing tests, then the broader overall-summary suite, then the repository-standard commands from `AGENTS.md` and `docs/TESTING.md`. If the new fixtures expose an additional mismatch in Markdown rendering or parser behavior, record that discovery in this document before broadening scope.

## Concrete Steps

Run all commands from the repository root, the directory containing `Cargo.toml`.

1. Inspect current fixture and helper entry points before editing.

    rg -n "coverlet_native_overall_totals|istanbul_native_overall_totals|overall_summary_line_totals" tests src
    find tests/fixtures/dotnet -maxdepth 3 -type f | sort
    find tests/fixtures/vitest -maxdepth 3 -type f | sort
    rg -n "regen-fixture-coverage|dotnet|vitest" xtask/src/main.rs

   Expected result: identify where new fixture ids must be added and confirm the current helper logic that will be made to fail.

2. Create reproduction fixture source changes and regenerate native artifacts.

    cargo xtask regen-fixture-coverage dotnet/duplicate-lines
    cargo xtask regen-fixture-coverage vitest/statement-line-divergence

   Expected result: checked-in `coverage.json` files are updated under the new fixture directories. The .NET artifact should show at least one repeated line number across multiple methods. The Vitest artifact should show a `statementMap` where line-range expansion is not equivalent to counting `s` entries.

3. Add failing tests before fixing the helper.

    cargo test overall_summary_line_totals_match_native_summary_for_all_line_capable_fixtures -- --nocapture
    cargo test native_overall_totals -- --nocapture

   Expected result before the fix: at least one new test fails, and the failure message makes clear that the helper-native total differs from the Markdown summary for the new reproducer fixture.

4. Implement the helper fix and rerun the narrow suites.

    cargo test overall_summary_line_totals_match_native_summary_for_all_line_capable_fixtures -- --nocapture
    cargo test native_overall_totals -- --nocapture

   Expected result after the fix: the same tests now pass, proving the helper semantics match the parser semantics.

5. Run the standard repository validation loop.

    cargo xtask quick
    cargo xtask validate

   Expected result: the repository passes the normal development and pre-ship validation commands without relaxing any gates.

## Validation and Acceptance

This plan is complete only when all of the following are true and observable by a novice:

A checked-in `.NET` fixture exists whose native Coverlet artifact reproduces duplicate line numbers across methods in one file, and a checked-in Vitest fixture exists whose native Istanbul artifact reproduces statement-to-line counting divergence.

The line parity suite fails before the helper fix and passes after it. The failure must be caused by the new reproducer fixtures, not by synthetic JSON or a hand-waved explanation.

`tests/support/mod.rs` computes Coverlet native line totals by deduplicating per-file line numbers across methods and computes Istanbul native line totals by expanding `statementMap` line ranges rather than counting raw `s` entries.

`cargo xtask quick` passes during development and `cargo xtask validate` passes before the plan is closed.

The final state must prove a real behavioral claim, not just a refactor: `tests/overall_summary.rs` should once again compare `covgate`’s Markdown output against a helper that matches the actual native-format semantics for the reproduced cases.

## Idempotence and Recovery

Fixture regeneration must stay idempotent. Re-running the two xtask commands for the new scenarios should only update those scenarios’ checked-in `coverage.json` artifacts and should never require manual edits to the generated JSON.

If the first chosen source shape does not actually produce the intended native artifact, recover by changing the fixture source code and regenerating coverage, not by editing the JSON output. Keep the plan notes up to date with the final source pattern that worked.

If the broad parity test is too opaque when it fails, keep the smaller helper-focused tests even after the end-to-end parity suite passes. They are the safest recovery aid for future contributors debugging fixture-specific summary drift.

If a new fixture reveals a parser bug instead of only a helper bug, stop and record that surprise in this document before changing scope. The current intent is to fix helper parity, but the living plan must stay honest if the evidence widens the problem.

## Artifacts and Notes

Representative failure this plan should capture before the fix:

    $ cargo test overall_summary_line_totals_match_native_summary_for_all_line_capable_fixtures -- --nocapture
    thread 'overall_summary_line_totals_match_native_summary_for_all_line_capable_fixtures' panicked at ...
    fixture dotnet/duplicate-lines metric line
    left: OverallTotals { covered: 3, total: 4 }
    right: OverallTotals { covered: 3, total: 3 }

Representative success after the fix:

    $ cargo test overall_summary_line_totals_match_native_summary_for_all_line_capable_fixtures -- --nocapture
    test overall_summary_line_totals_match_native_summary_for_all_line_capable_fixtures ... ok

Representative helper rule for Istanbul after the fix:

    native line total = count of unique source lines reached by statementMap ranges
    native covered lines = unique source lines for which at least one covering statement has hits > 0

Keep the actual transcripts concise and replace the example numbers above with the real fixture outputs once implementation is underway.

## Interfaces and Dependencies

Use the existing repository mechanisms. Do not introduce a second fixture-generation workflow or a one-off script.

The final implementation should leave these interfaces and responsibilities clear:

- `tests/support/mod.rs::coverlet_native_overall_totals(parsed, "line") -> Option<OverallTotals>` must deduplicate line numbers per file across methods and OR coverage hits for duplicate lines.
- `tests/support/mod.rs::istanbul_native_overall_totals(parsed, "line") -> Option<OverallTotals>` must derive line totals from `statementMap` ranges joined with `s` hit counts.
- `tests/overall_summary.rs` must continue to use `MetricFixtureCase::native_overall_totals()` and `MetricFixtureCase::covgate_markdown_overall_totals()` as the end-to-end comparison entry points.
- `xtask/src/main.rs` must provide regeneration support for any newly added `dotnet/...` and `vitest/...` reproducer fixtures.
- `tests/fixtures/dotnet/...` and `tests/fixtures/vitest/...` must remain native-tool-generated fixture trees with checked-in `coverage.json` artifacts.

At the bottom of this plan, append a revision note every time the plan changes materially, describing what changed and why.

Revision note: Initial plan created to turn valid review feedback about Coverlet and Istanbul native-summary helper drift into native-generated repro fixtures, failing parity tests, and a precise helper fix.
