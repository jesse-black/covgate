# Restore `covgate` overall coverage parity with native coverage summaries

Save this in-progress ExecPlan in `docs/exec-plans/active/covgate-llvm-summary-parity.md`. Move it to `docs/exec-plans/completed/covgate-llvm-summary-parity.md` only after implementation, validation, and documentation updates are complete.

This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds.

Maintain this document in accordance with `docs/PLANS.md`. Re-read that file before revising this plan and keep this plan aligned with its rules.

## Purpose / Big Picture

After this change, `covgate` will report overall coverage totals that match the native tool totals for the same fixture data instead of silently drifting above or below them. A contributor will be able to run fixture-backed regression tests that invoke both the upstream coverage tool and `covgate`, then see the same covered and total counts for every supported metric that the source format actually exposes.

This matters because the current mismatch is a trust failure, not a formatting issue. The tempting shortcut would be to read LLVM's `summary` or top-level `totals` blocks and print those values directly in the Markdown overall summary. That is explicitly unacceptable for this bug. It would hide the real defect by bypassing `covgate`'s own calculations rather than proving they are correct. `covgate` needs to compute the right answers itself so gate evaluation, Markdown rendering, console output, and future non-LLVM adapters all remain internally consistent.

You will know this work is complete when all of the following are true:

1. A regression test reproduces the current mismatch by comparing native summary output with `covgate` Markdown totals using checked-in fixtures.
2. `cargo xtask regen-fixture-coverage-all` regenerates fixture artifacts that still trigger the failing regression before the bug fix is applied.
3. After the calculation fix, the same tests pass and show that `covgate`-computed totals match the upstream tool's totals for each supported metric/language combination.
4. The implementation does not special-case LLVM Markdown rendering by piping upstream summary numbers through unchanged.

## Progress

- [x] (2026-03-17 00:00Z) Created this repository-specific ExecPlan and recorded the non-negotiable requirement that the fix must repair `covgate` calculations instead of proxying LLVM summary blocks.
- [x] Inspect the current total-calculation pipeline and capture the exact point where `covgate` diverges from upstream counts.
- [x] Add failing regression tests that compare native coverage-tool summaries with `covgate` Markdown overall totals across the fixture matrix.
- [x] Confirm that fixture regeneration via `cargo xtask regen-fixture-coverage-all` still preserves a red test before the fix.
- [x] Repair the calculation bug in the normalization or aggregation layer so the parity tests pass without bypassing `covgate` logic.
- [x] Run the targeted suites and full validation (`cargo xtask validate`).

## Surprises & Discoveries

- Observation: `src/render/markdown.rs` currently computes overall totals by summing `metric.totals_by_file` at render time instead of consuming a separately validated overall-total value.
  Evidence: `src/render/markdown.rs` sums `metric.totals_by_file.values().map(|totals| totals.covered)` and `.map(|totals| totals.total)` before printing the `**Total**` row.

- Observation: LLVM fixture artifacts already contain native summary data, so simply reading those fields would make the Markdown output look correct even if `covgate`'s own normalization remains wrong.
  Evidence: checked-in LLVM fixtures under `tests/fixtures/rust/`, `tests/fixtures/cpp/`, and `tests/fixtures/swift/` contain per-file `summary` blocks and top-level `totals` objects.

- Observation: The repository already has a cross-language live-fixture harness and xtask-driven regeneration flow that can support this regression without inventing new fixture infrastructure.
  Evidence: `tests/support/mod.rs`, `tests/cli_metrics.rs`, and `xtask/src/main.rs` define fixture matrices for Rust, C/C++, Swift, .NET, and Vitest plus `regen-fixture-coverage` and `regen-fixture-coverage-all`.

- Observation: LLVM region drift came from `segments_to_regions()` treating every counted segment window as a distinct region even when the segment was not a region entry or was explicitly marked as a gap region.
  Evidence: the new parity test failed for the C++ and Swift LLVM fixtures until `src/coverage/llvm_json.rs` started honoring the `is_region_entry` and `is_gap_region` flags carried in each segment tuple.


## Decision Log

- Decision: Reject the shortcut of printing LLVM `summary` or top-level `totals` values directly in the Markdown overall summary.
  Rationale: The bug report is about `covgate` disagreeing with the source tool. Using the source tool's precomputed totals only in output would mask the defect and leave `covgate`'s internal totals wrong for gating, future renderers, and debugging.
  Date/Author: 2026-03-17 / Codex

- Decision: Use TDD with fixture-backed parity tests that run both the upstream coverage tool and `covgate`.
  Rationale: This repository's testing rules require bugs to be reproduced first. A parity test provides a stable observable contract: `covgate` totals must match the native tool for the same artifact.
  Date/Author: 2026-03-17 / Codex

- Decision: Exercise every supported metric across every language fixture that can actually emit that metric, and document unsupported combinations explicitly.
  Rationale: `covgate` supports region, line, branch, and function metrics, but not every native format exposes every one of them. LLVM fixtures cover region, line, and function today; branch parity should run on the fixtures that emit branch data; .NET and Vitest should be included for line, branch, and function parity because the trust problem is about `covgate` calculations broadly, not LLVM alone.
  Date/Author: 2026-03-17 / Codex

- Decision: Collect native overall totals from the checked-in fixture artifacts inside the test helper instead of shelling out to external coverage binaries.
  Rationale: the repository already stores authoritative native-summary data for LLVM fixtures and native-format raw artifacts for Coverlet and Istanbul fixtures, so parsing those artifacts keeps the parity matrix deterministic and avoids introducing environment-sensitive toolchain dependencies into the regression.
  Date/Author: 2026-03-17 / Codex

## Outcomes & Retrospective

Implementation is complete. The repository now has an explicit overall-summary parity regression matrix across LLVM, Coverlet, and Istanbul fixtures, and the LLVM adapter now respects segment region-entry and gap markers so its computed region totals match the native summaries again.

The main implementation risk is choosing the wrong comparison surface. If tests only snapshot Markdown text without checking the underlying counts against native tool output, a future formatter change could reintroduce incorrect math. The plan therefore treats Markdown as the user-visible symptom while requiring parity assertions against the computed numeric totals that drive rendering.

## Context and Orientation

`covgate` is a Rust CLI in `src/` that parses coverage reports, normalizes them into `crate::model::CoverageReport`, computes changed and overall metrics, evaluates gates, and optionally writes a Markdown summary. The code paths relevant to this bug are:

- `src/coverage/llvm_json.rs`, `src/coverage/coverlet_json.rs`, and `src/coverage/istanbul_json.rs`: adapters that translate native report formats into `CoverageReport` opportunities and per-file totals.
- `src/model.rs`: core data structures such as `CoverageReport`, `FileTotals`, and `ComputedMetric`.
- `src/metrics.rs`: computes changed coverage for a requested metric and carries forward `totals_by_file`.
- `src/render/markdown.rs`: renders the overall summary tables whose `**Total**` row is currently mismatching native summaries.
- `tests/support/mod.rs`: shared fixture harness for copied worktrees and `covgate` execution.
- `tests/cli_metrics.rs` and `tests/cli_interface.rs`: existing integration coverage split between metric semantics and output/interface behavior.
- `xtask/src/main.rs`: fixture regeneration entry points, especially `cargo xtask regen-fixture-coverage <language>/<scenario>` and `cargo xtask regen-fixture-coverage-all`.

A “native summary” in this plan means the totals reported by the upstream coverage tool for the checked-in fixture artifact, not totals recomputed by `covgate`. For LLVM fixtures that means invoking `llvm-cov report --summary-only` (or the equivalent `cargo llvm-cov report --summary-only` flow for Rust-originated fixture generation if that is what the fixture toolchain already uses). For Coverlet and Istanbul fixtures, use the native totals already expressed by their checked-in JSON plus the adapter's expected semantics, or invoke the closest native command if the repository already has one in the regeneration path. The implementation must document the exact command chosen for each fixture family and why it is authoritative.

An “overall total parity test” means a test that:

1. prepares a real fixture worktree,
2. obtains the upstream tool's overall metric totals for that fixture,
3. runs `covgate` against the same coverage artifact and writes Markdown output,
4. extracts the Markdown `### Overall Coverage` totals,
5. asserts that covered counts, total counts, and percentages match for each metric under test.

The current bug report specifically shows LLVM region totals diverging from `cargo llvm-cov report`, but this plan deliberately broadens the regression surface so we can trust `covgate` totals across supported metrics and languages rather than fixing one visible Markdown row in isolation.

## Plan of Work

Start by tracing the end-to-end path that produces the Markdown `Overall Coverage` totals. Confirm whether the discrepancy is introduced in format adapters such as `src/coverage/llvm_json.rs`, in shared metric aggregation under `src/metrics.rs`, or in render-time recomputation inside `src/render/markdown.rs`. Record the exact mismatch with one concrete fixture before changing code.

Next, add a new regression-focused integration test module under `tests/`, or extend the existing helper module plus `tests/cli_interface.rs`, so the test can run native-tool summary collection and `covgate` side by side. The test must not just compare strings. It should parse the native summary counts into a helper struct and parse the Markdown overall totals into the same struct, then compare the numeric values. Reuse `tests/support/mod.rs` for fixture setup and `run_covgate`, and add helper functions there for extracting Markdown totals and invoking native summary commands.

The fixture matrix must intentionally cover the full supported surface. Use Rust, C/C++, and Swift LLVM fixtures for region parity because LLVM JSON is the only format in this repository that exposes region totals. Use Rust, C/C++, Swift, .NET, and Vitest fixtures for line parity. Use the branch-capable fixtures already encoded in `tests/support/mod.rs` for branch parity. Use every function-capable fixture already encoded in `tests/support/mod.rs` for function parity. If implementation discovers that a fixture family cannot provide an authoritative native summary command, keep that fixture in the matrix only if the test can compare against the native totals embedded in the source artifact without routing around `covgate`'s own computation.

After the red test exists, run `cargo xtask regen-fixture-coverage-all` and rerun the targeted tests before fixing the bug. This step is required because the bug must reproduce against freshly regenerated artifacts, not only against stale checked-in JSON. If regeneration changes fixture shapes, update the test helper parsing to follow the native tool output format, but do not weaken the parity assertions.

Then fix the underlying bug in the calculation layer. The preferred location is the earliest point where `covgate`'s normalized totals stop matching the source report. If `src/render/markdown.rs` is merely exposing bad `totals_by_file`, fix the adapter or aggregation logic rather than teaching the renderer to trust external summary fields. If the renderer is incorrectly recomputing totals from already-corrupted file totals, introduce a single validated overall-total path inside `CoverageReport` or `ComputedMetric` only if the data is still produced by `covgate`'s own normalization rather than copied from LLVM summary blocks.

Finally, add unit tests close to the repaired logic so the root cause is covered independently of the integration parity tests. For example, if an LLVM parser bug is found, add focused tests in `src/coverage/llvm_json.rs`; if a shared aggregation bug is found, add focused tests in `src/metrics.rs` or `src/render/markdown.rs`. Keep the parity integration tests as the user-visible regression proof.

## Concrete Steps

Run all commands from the repository root (the directory containing `Cargo.toml`, `src/`, `tests/`, and `docs/`).

1. Inspect the current total-calculation path and existing fixture capability helpers.

    rg -n "totals_by_file|Overall Coverage|summary-only|llvm-cov|branch_capable|function_capable" src tests xtask docs

    Expected result: identify the exact files and helper functions to edit for native-summary invocation, Markdown-total parsing, and the likely calculation bug.

2. Add failing parity tests before touching the calculation code.

    cargo test overall_summary

    Expected result: a new regression test fails because the native summary totals and `covgate` Markdown totals do not match for at least one fixture/metric combination. The test output should name the fixture id and metric that diverged.

3. Prove the reproducer survives fixture regeneration.

    cargo xtask regen-fixture-coverage-all
    cargo test overall_summary

    Expected result: regenerated fixture artifacts still produce the same failing parity test before the fix. If the failing fixture or metric changes because regeneration changes report shape, update the test expectation to match the new authoritative native totals and keep the test red.

4. Repair the underlying calculations and add focused unit coverage.

    cargo test overall_summary
    cargo test llvm_json
    cargo test metrics

    Expected result: the new parity tests pass and any new root-cause unit tests also pass.

5. Run the broader regression suite and full repository validation.

    cargo test --test cli_metrics
    cargo test --test cli_interface
    cargo test
    cargo xtask validate

    Expected result: all targeted and full-project checks pass, confirming the fix does not regress existing coverage semantics or output behavior.

## Validation and Acceptance

Acceptance is complete only when all of the following behaviors are visible and repeatable.

The parity regression test must execute against copied fixtures, not synthetic-only JSON snippets. For each fixture/metric combination under test, it must run the upstream summary path first, then run `covgate` and read the generated Markdown summary, then compare the numeric totals. A failing assertion must make it obvious which fixture and metric drifted.

The language and metric matrix must be explicit in test code. At minimum:

- Region parity runs across Rust, C/C++, and Swift LLVM fixtures.
- Line parity runs across Rust, C/C++, Swift, .NET, and Vitest fixtures.
- Branch parity runs across every branch-capable fixture already supported by the repository.
- Function parity runs across every function-capable fixture already supported by the repository.

If any language/metric combination is unsupported by the native format, the test code must say so plainly and skip that combination by design rather than silently omitting it.

`cargo xtask regen-fixture-coverage-all` must not “fix” the failure by changing the tests to accept `covgate`'s wrong values. Before the code fix, regenerated fixtures still need to produce a failing parity assertion. After the code fix, the same regeneration command followed by the same test command must pass.

The final implementation must leave `covgate` computing its own totals. It is acceptable to parse native summary output inside the test harness to establish the oracle. It is not acceptable for production code to populate Markdown overall totals by directly passing through LLVM summary blocks while leaving `covgate`'s internal metric math inconsistent.

## Idempotence and Recovery

The fixture parity workflow is designed to be safely repeatable. Re-running `cargo xtask regen-fixture-coverage-all` should only refresh checked-in coverage artifacts. Re-running the parity tests should recreate temporary worktrees and Markdown files without mutating the committed fixture baselines.

If a native summary command fails because the required tool is unavailable, recover by first confirming whether the fixture family should participate in native-command parity or in artifact-level parity. Document that decision in this plan and keep the test matrix explicit. Do not silently downgrade an authoritative comparison to a looser assertion without recording why.

If implementation temporarily introduces a helper struct for overall totals, prefer additive changes and keep the existing renderer behavior until the parity tests can be switched from red to green in one small step. Recovery from a bad intermediate state is to revert only the incomplete helper wiring and keep the failing test intact.

## Artifacts and Notes

Representative failing transcript shape before the fix:

    $ cargo test overall_summary
    thread 'overall_summary_region_totals_match_native_summary_for_llvm_fixtures' panicked at ...
    fixture rust/basic-fail metric region: native totals were covered=3262 total=3376 percent=96.62
    covgate markdown totals were covered=3523 total=3626 percent=97.16

Representative passing transcript shape after the fix:

    $ cargo test overall_summary
    running 4 tests
    test overall_summary_region_totals_match_native_summary_for_llvm_fixtures ... ok
    test overall_summary_line_totals_match_native_summary_for_all_line_capable_fixtures ... ok
    test overall_summary_branch_totals_match_native_summary_for_branch_capable_fixtures ... ok
    test overall_summary_function_totals_match_native_summary_for_function_capable_fixtures ... ok

Representative Markdown parsing target:

    ### Overall Coverage

    #### Region

    | **Total** | **3262** | **3376** | **96.62%** |

The exact totals will depend on the fixture and regenerated artifact. The important property is parity between the native tool and `covgate`, not any hard-coded number from this document.

## Interfaces and Dependencies

Use the existing Rust integration-test stack and fixture helpers. Do not add a new test framework.

The implementation should end with:

- a reusable test helper in `tests/support/mod.rs` for invoking native-summary collection and parsing Markdown overall totals into a common struct;
- one or more integration tests under `tests/` that iterate the explicit fixture matrix described above;
- focused unit tests in the module that actually contained the bug;
- production code that continues to compute totals through `covgate`'s own model and aggregation pipeline.

If the repair requires expanding the model, prefer a small repository-local type such as an internal “overall totals” struct carried alongside `totals_by_file`, with constructors fed by adapter/aggregation logic that `covgate` owns. Do not introduce a production dependency on raw LLVM summary blocks as the source of truth for rendered totals.

Revision note: Initial plan created to address Markdown overall-total mismatches with a strict TDD parity workflow, explicitly reject the shortcut of piping LLVM summary blocks through as production output, and require coverage across supported metrics and language fixtures.
