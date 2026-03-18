# Restore `covgate` overall coverage parity with native coverage summaries

Save this in-progress ExecPlan in `docs/exec-plans/active/covgate-llvm-summary-parity.md`. Move it to `docs/exec-plans/completed/covgate-llvm-summary-parity.md` only after implementation, validation, and documentation updates are complete.

This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds.

Maintain this document in accordance with `docs/PLANS.md`. Re-read that file before revising this plan and keep this plan aligned with its rules.

## Purpose / Big Picture

This work is not about making Markdown summaries show the right numbers. It is about proving that `covgate`'s own calculations are correct.

Matching LLVM's summary output is only the symptom we can observe. It is not the goal by itself. If `covgate` can be taught to print the same totals by copying LLVM summary data, we still learn nothing about whether `covgate`'s normalization, aggregation, and diff-gating calculations are correct. That shortcut would make the output look trustworthy while leaving the underlying math unproven. The real bar is higher: `covgate` must derive correct totals itself so overall summaries, diff coverage, gating, and future adapters can all be trusted together.

You will know this work is complete when all of the following are true:

1. A real regression test reproduces the current mismatch against an authoritative LLVM export shape.
2. The failing test stays red until a calculation fix is applied.
3. After the fix, `covgate`'s computed totals match the upstream totals for every supported metric under test.
4. Production code does not bypass `covgate`'s calculations by passing LLVM summary numbers through unchanged.
5. We can explain why the old calculation was wrong and why the new one is correct.

## Progress

- [x] (2026-03-17 00:00Z) Created this repository-specific ExecPlan and recorded the non-negotiable requirement that the fix must repair `covgate` calculations instead of proxying LLVM summary blocks.
- [x] Inspect the current total-calculation pipeline and the existing fixture-backed parity tests.
- [x] Confirm the original parity suite is too weak: it compares against embedded fixture totals and does not reproduce the real LLVM drift.
- [x] Add a checked-in multi-file LLVM export from the repository's own `cargo llvm-cov report --json` output plus a real parity test in `tests/llvm_real_parity.rs`.
- [x] Prove the real LLVM repro fails on all exposed metrics: region, line, and function.
- [x] Try the tempting shortcut of using LLVM per-file summary totals, then explicitly revert it after confirming it violates the purpose of this plan.
- [ ] Expand LLVM-focused tests so they cover multi-file reports, non-trivial function populations, and mixed segment flag combinations rather than only one-file fixtures with `functions.count == 1`.
- [x] Add initial diff-focused LLVM regression tests that assert exact changed opportunities for the small Rust basic fixtures instead of only asserting threshold-based pass/fail behavior.
- [ ] Identify the real calculation defect in LLVM normalization or aggregation and fix it without passing through summary fields.
  Completed: function parity now matches LLVM on the real repro after switching LLVM function deduplication from pure span identity to normalized function-name identity when a stable LLVM name is available.
  Remaining: region and line parity still fail on the real repro, and the investigation now has to answer whether LLVM summary semantics for those metrics are fully derivable from exported detail at all.
- [ ] Run the targeted suites and full validation (`cargo xtask validate`) after the real calculation fix lands.
- [ ] Follow up separately on summary UX so Markdown output can render every metric available in the loaded report, even when only a subset is actively gated.

## Surprises & Discoveries

- Observation: The original parity suite was not proving what we needed. For LLVM it used embedded fixture `totals` blocks as the oracle, and the LLVM fixtures were too small to exercise the real failing shape.
  Evidence: `tests/support/mod.rs` reads `data[0].totals` directly, and the Rust/C++/Swift LLVM fixtures all have trivial function totals.

- Observation: A real multi-file LLVM export from this repository reproduces the drift across all exposed metrics, not just regions.
  Evidence: `tests/llvm_real_parity.rs` currently fails with native totals `region 3285/3408`, `line 2890/2957`, and `function 160/165`, while `covgate` reports `region 3252/3355`, `line 2865/2910`, and `function 159/164`.

- Observation: The tempting summary-backed adapter change does make the parity test pass, but it does not prove our calculations are correct, so it is not an acceptable fix for this plan.
  Evidence: the branch briefly passed by preferring LLVM per-file `summary` data in production, and that change was then reverted.

- Observation: The remaining gap is not explained by obvious missing data or simple rule flips.
  Evidence: the real export has no LLVM `expansions`, counting all non-gap segments overshoots badly, and counting raw function-region tuples overshoots even more.

- Observation: `covgate` currently renders only metrics selected by active rules. That is a UX issue worth fixing later, but it is not the correctness fix tracked here.
  Evidence: `run()` derives `requested_metrics` from `config.rules` before rendering.

- Observation: LLVM function totals for Rust files align better with normalized LLVM function names than with pure source-span identity.
  Evidence: on the real LLVM repro, `src/metrics.rs` had 14 raw function records, 7 unique spans, and a native function total of 8. Normalizing LLVM names by removing the crate-hash disambiguator produced 8 stable function identities, which matches the native total.

- Observation: After fixing function identity, the honest real repro is now red only for regions and lines.
  Evidence: `tests/llvm_real_parity.rs` now reports only `region native=3285/3408 vs covgate=3252/3355` and `line native=2890/2957 vs covgate=2865/2910`.

- Observation: The remaining live line mismatch is concentrated in LLVM segment semantics, especially in `src/coverage/llvm_json.rs`, rather than in shared line aggregation everywhere.
  Evidence: comparing the validate export against LLVM's annotated text output showed `src/metrics.rs` and `src/render/markdown.rs` line sets already match LLVM's executable-line view, while `src/coverage/llvm_json.rs` still diverges substantially.

- Observation: LLVM's own rendered text view and LLVM per-file summary counts do not always agree on line totals for the same live coverage run.
  Evidence: on a live `cargo llvm-cov report` run, `src/config.rs` rendered as `322/337` covered/total executable lines in the text report while the JSON file summary reported `309/342`. `src/coverage/llvm_json.rs` showed `786/788` in the text view versus `832/834` in the file summary.

- Observation: This means a parser can match LLVM's visible executable-line rendering for a file and still disagree with LLVM summary totals.
  Evidence: reproducing `src/config.rs` from the live export showed `covgate`'s current line derivation matched the text report exactly for that file, but still disagreed with `summary.lines`.

- Observation: Upstream LLVM users have already reported that exact line-covered sets are not fully exposed in JSON export detail today.
  Evidence: llvm/llvm-project issue `#126307` asks LLVM to expose which lines it considers covered because tools such as `cargo-llvm-cov` currently have to infer them imperfectly from JSON export data.

- Observation: Replacing file-segment line derivation with a simple union of function-region lines does not resolve the remaining repro gap for the highest-drift files.
  Evidence: on the checked-in real repro, `src/coverage/llvm_json.rs`, `src/config.rs`, and `src/metrics.rs` all produced the same line totals from segment windows and from function-region union, while still disagreeing with LLVM file summaries.

- Observation: The high-drift live files have no LLVM `expansions` or `branches`, so those fields are not hiding the remaining line mismatch there.
  Evidence: the inspected live files `src/metrics.rs`, `src/config.rs`, and `src/coverage/llvm_json.rs` all had `expansions = []` and `branches = []`, yet summary line totals still exceeded both segment-derived and function-region-union line sets.

- Observation: On the live files with the largest region drift, gap segments are absent and counting all non-entry segments would overshoot badly.
  Evidence: `src/config.rs`, `src/coverage/llvm_json.rs`, and `src/coverage/coverlet_json.rs` all had `gap = 0`, but their summary-region deltas over the current parser were only `+9` to `+11` compared with dozens of non-entry segments.

- Observation: LLVM's LCOV export also carries two different line views: `LF/LH` matches JSON summary line totals, while concrete `DA:` line entries are fewer and track the executable-line-style view instead.
  Evidence: on the same live run, `src/metrics.rs` had LCOV `LF/LH = 133/133` with only `127` `DA:` entries; `src/config.rs` had `LF/LH = 342/309` with `337` `DA:` entries; and `src/coverage/llvm_json.rs` had `LF/LH = 834/832` with `822` `DA:` entries.

- Observation: `covgate`'s actual diff gate is driven by changed opportunities plus diff-line overlap, not by overall summary totals alone.
  Evidence: `src/metrics.rs::compute_changed_metric` filters `CoverageOpportunity` records by `SourceSpan::overlaps_line_range`, computes changed covered/total counts from those opportunities, and `src/gate.rs::evaluate` uses that changed metric for pass/fail decisions.

- Observation: The current integration suite mostly proves whole-fixture threshold behavior, not the exact changed-opportunity set in the hardest LLVM cases.
  Evidence: `tests/cli_metrics.rs` runs threshold-based pass/fail checks across fixtures, while only `src/metrics.rs` unit tests directly assert changed-opportunity counting on small synthetic inputs.

- Observation: The repository now has its first LLVM integration tests that assert exact changed-opportunity membership and covered-state on real fixture diffs.
  Evidence: `tests/llvm_diff_regression.rs` verifies exact changed line, region, and function opportunities for the Rust `basic-fail` and `basic-pass` LLVM fixtures by parsing coverage, loading the real unified diff, and asserting on `compute_changed_metric()`.


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

- Decision: Keep the real LLVM repro even while it is red, and do not “fix” it by passing summary data through production code.
  Rationale: a failing real repro is more valuable than a green test that only proves we can copy LLVM's answers.
  Date/Author: 2026-03-18 / Codex

- Decision: Use normalized LLVM function names, when present, as the primary function identity for LLVM totals and opportunities, with span fallback only when a stable name is unavailable.
  Rationale: the real repro showed that pure span-based deduplication undercounts Rust LLVM function totals, while normalized-name identity matches LLVM's native function totals without relying on summary pass-through.
  Date/Author: 2026-03-18 / Codex

- Decision: Treat remaining line and region parity as a semantics investigation first, not as permission to keep tweaking parser heuristics blindly.
  Rationale: the live evidence now shows that LLVM JSON detail, LLVM rendered text, and LLVM summary counts are not trivially interchangeable. We need to establish which semantics are actually recoverable from exported detail before claiming any parser fix proves `covgate` is correct.
  Date/Author: 2026-03-18 / Codex

- Decision: Shift the confidence target for the remaining LLVM work toward changed-opportunity correctness for diff gating, with summary parity treated as strong but secondary evidence where LLVM semantics are ambiguous.
  Rationale: gate outcomes are computed from changed opportunities and span overlap, not from overall summary rows alone. If LLVM exports multiple competing line views, the direct proof for `covgate` is that changed opportunities match the concrete gating semantics we can observe.
  Date/Author: 2026-03-18 / Codex

## Outcomes & Retrospective

Implementation is still in progress. The useful outcome so far is not a fix; it is a better problem statement. The repository now has a real LLVM parity repro that fails for the right reason, and the plan is explicit that making summaries look right is not enough.

The biggest lesson from this churn is that the regression surface matters. A green test against tiny fixtures or passed-through summary data can create false confidence. This plan now treats that as a primary risk and keeps the branch focused on proving the calculations. The first honest calculation fix has now landed on the function side, which narrows the remaining investigation to LLVM region and line semantics. The newest lesson is that even LLVM's own exported detail and summary views may not encode the same information directly, so the next milestone must establish what can actually be proven from the JSON export before we call any line/region change a fix.

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

1. Keep the real LLVM repro red until a real calculation fix is found.
2. Trace the discrepancy as early as possible in the pipeline:
   adapter normalization in `src/coverage/llvm_json.rs`,
   shared aggregation in `src/metrics.rs`,
   renderer recomputation in `src/render/markdown.rs`.
3. Prefer fixes that make one internally owned calculation path correct for both overall totals and diff coverage.
4. Add focused unit tests near the actual bug in addition to the large parity repro.
5. Treat any production use of LLVM `summary` data as a rejected shortcut unless it is part of a proven calculation model owned by `covgate`.

Near-term next steps:

1. Keep the new function-identity fix and use it as the baseline.
2. Separate the remaining investigation into two questions:
   first, which line/region semantics are recoverable from LLVM export detail and therefore fair game for `covgate` to compute itself;
   second, whether the current parser matches those recoverable semantics.
3. Compare live LLVM text output, LCOV concrete `DA:` entries, live LLVM file summaries, and exported detail side-by-side for the highest-drift files in `src/coverage/llvm_json.rs`, `src/coverage/coverlet_json.rs`, `src/coverage/istanbul_json.rs`, and `src/config.rs`.
4. Add changed-diff-focused tests that prove line, region, and function opportunities are included or excluded correctly for real changed lines, even before full summary parity is resolved.
5. Add one or more targeted parser tests only after the exact recoverable rule is understood from that comparison.
6. Re-run `tests/llvm_real_parity.rs`, `cargo xtask quick`, and then `cargo xtask validate` after each real parser fix attempt.

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

6. If the investigation shows that LLVM summary line or region totals are not derivable from exported detail with enough fidelity, record that explicitly and revisit the acceptance bar for “proof of correctness” in this plan before changing production code.

    Expected result: the plan states clearly whether remaining parity failure is a parser bug, a missing export-detail limitation, or both.

7. Add or expand diff-focused regression tests that exercise real changed-line scenarios against LLVM-backed fixtures and prove the gate decision itself is correct for line, region, and function metrics.

    Expected result: the repository has tests that fail when `CoverageOpportunity` extraction, covered-state assignment, or diff-overlap logic is wrong, even if overall summary semantics remain partially ambiguous.

## Validation and Acceptance

Acceptance is complete only when all of the following behaviors are visible and repeatable.

The parity regression must execute against real artifacts, not only synthetic snippets, and it must make the failing metric obvious.

The language and metric matrix must be explicit in test code. At minimum:

- Region parity runs across Rust, C/C++, and Swift LLVM fixtures.
- Line parity runs across Rust, C/C++, Swift, .NET, and Vitest fixtures.
- Branch parity runs across every branch-capable fixture already supported by the repository.
- Function parity runs across every function-capable fixture already supported by the repository.

If any language/metric combination is unsupported by the native format, the test code must say so plainly and skip that combination by design rather than silently omitting it.

`cargo xtask regen-fixture-coverage-all` must not make the test green by redefining the oracle to match `covgate`'s wrong values.

The final implementation must leave `covgate` proving its own calculations. It is acceptable for tests to read native summary data as the oracle. It is not acceptable for production code to make overall summaries correct while leaving `covgate`'s underlying metric math unproven.

If the investigation shows that LLVM export detail does not actually expose enough information to derive the same line or region totals that LLVM summary reports, the plan must say so plainly and adjust the acceptance criteria around what `covgate` can honestly prove. That would still reject summary pass-through as a fake fix; it would simply acknowledge an upstream visibility limitation rather than pretending the current JSON export contains more semantic detail than it does.

For the diff gate itself, acceptance should increasingly focus on whether changed opportunities are extracted, marked covered or uncovered, and overlapped against changed lines correctly. That is the direct path to trustworthy gate results in `covgate`, even if LLVM summary totals remain only partially reproducible.

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

Representative semantic-blocker evidence from the live investigation:

    $ cargo llvm-cov report --text --output-dir /tmp/llvmtext-covgate
    $ cargo llvm-cov report --json --output-path /tmp/liveexport.json

    src/config.rs
      text executable lines:    covered=322 total=337
      file summary lines:       covered=309 total=342

    src/coverage/llvm_json.rs
      text executable lines:    covered=786 total=788
      file summary lines:       covered=832 total=834

This evidence means “match the visible text view” and “match the summary totals” are not automatically the same task.

## Interfaces and Dependencies

Use the existing Rust integration-test stack and fixture helpers. Do not add a new test framework.

The implementation should end with:

- a reusable test helper in `tests/support/mod.rs` for invoking native-summary collection and parsing Markdown overall totals into a common struct;
- one or more integration tests under `tests/` that iterate the explicit fixture matrix described above;
- focused unit tests in the module that actually contained the bug;
- production code that continues to compute totals through `covgate`'s own model and aggregation pipeline.

If the repair requires expanding the model, prefer a small repository-local type such as an internal “overall totals” struct carried alongside `totals_by_file`, with constructors fed by adapter/aggregation logic that `covgate` owns. Do not introduce a production dependency on raw LLVM summary blocks as the source of truth for rendered totals.

Revision note: Initial plan created to address Markdown overall-total mismatches with a strict TDD parity workflow, explicitly reject the shortcut of piping LLVM summary blocks through as production output, and require coverage across supported metrics and language fixtures.

Revision note: Reopened the plan on 2026-03-18 after confirming the existing fixture suite still passes while a real `cargo llvm-cov` report continues to disagree with `covgate`. Recorded the main gap: the tests rely on tiny checked-in fixture summaries and do not yet reproduce the current discrepancy seen in live LLVM exports.

Revision note: Added a follow-up task for rendering all available metrics in summaries after confirming that current output is limited to metrics selected by active gate rules. Kept that work explicitly out of scope for the parity fix so correctness and UX stay separable.

Revision note: Added a checked-in real LLVM export fixture plus a failing integration test that reproduces the current parity gap for region, line, and function totals. This replaced the earlier weak fixture-only regression surface.

Revision note: A temporary summary-backed adapter change was tried and then reverted. The plan now states this explicitly so future work does not confuse “output matches LLVM” with “`covgate` calculations are proven correct.”

Revision note: Recorded the first honest calculation fix: LLVM function identity now uses normalized LLVM names rather than pure span deduplication when names are available, which resolves the function mismatch in the real repro. The remaining work is now explicitly scoped to region and line semantics.

Revision note: Added live-investigation evidence that LLVM text rendering and LLVM summary totals can disagree for the same file, and linked that investigation to `docs/reference/llvm-export-semantics-investigation.md` so future work does not assume exported detail, text views, and summary counts are automatically equivalent.

Revision note: Recorded that switching line derivation from file segments to simple function-region union does not fix the remaining real repro gap, so the next step must isolate a smaller LLVM semantic pattern rather than just swapping one whole-model heuristic for another.

Revision note: Added more negative findings from the live investigation: the largest-drift files do not use expansions or branch records, and the remaining region gap is too small to justify counting all non-entry segments. The plan now explicitly requires a narrower LLVM semantic rule before any production parser change.

Revision note: Added LCOV evidence that LLVM itself exports summary-style line counts (`LF/LH`) separately from concrete listed line entries (`DA:`), reinforcing that summary parity and executable-line derivation are not automatically the same problem.

Revision note: Refocused the remaining confidence question around changed-opportunity correctness for diff gating, because `covgate`'s actual pass/fail logic is driven by changed opportunities plus span overlap rather than by overall summary totals alone.

Revision note: Recorded the current testing gap: most integration coverage tests assert whole-fixture pass/fail behavior, so the next high-value LLVM regressions should assert exact changed opportunities for explicit diff scenarios rather than only threshold outcomes.

Revision note: Added `tests/llvm_diff_regression.rs` as the first diff-focused LLVM integration regression file. It locks down exact changed line, region, and function opportunities for the Rust basic fixtures and gives the plan a direct gating-correctness proof path that does not depend on overall summary parity.
