# Prove which LLVM line oracle `covgate` should trust for diff gating, then add fixture-backed tests against that oracle

Save this in-progress ExecPlan in `docs/exec-plans/active/covgate-llvm-line-oracle-confidence.md`. Move it to `docs/exec-plans/completed/covgate-llvm-line-oracle-confidence.md` only after the line-oracle investigation, the resulting tests, and validation are complete.

This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds.

Maintain this document in accordance with `docs/PLANS.md`. Re-read that file before revising this plan and keep this plan aligned with its rules.

## Purpose / Big Picture

The repository already has good evidence that `covgate`'s LLVM diff gate is internally coherent on several real-export slices, but the remaining LLVM line-summary mismatch is still not well characterized. The important new lesson from the investigation is that LLVM line discrepancies should not be treated the same way as region discrepancies. For lines, LLVM exposes multiple visible views for the same run: JSON summary totals, LCOV `LF/LH`, LCOV concrete `DA:` line listings, and text-rendered executable lines. Those views can disagree. For regions, the remaining mismatch still looks more like a narrow parser or summary-rule problem.

After this work, a novice should be able to run a fixture-backed test and see exactly which LLVM line oracle `covgate` is claiming to match for diff gating. The result may be “match LLVM text executable lines,” “match LCOV `DA:` concrete line listings,” or a documented equivalence class if those turn out to be the same for the checked-in repros. The important user-visible change is not a prettier summary row. It is a stronger, evidence-backed answer to the question “which changed lines should `covgate` actually gate?”

## Progress

- [x] (2026-03-18 00:00Z) Completed the prior LLVM summary-parity investigation and recorded the architectural decision that normal `covgate` summaries stay calculation-backed rather than passing through LLVM native summary totals.
- [x] (2026-03-18 00:30Z) Added `tests/llvm_diff_regression.rs` real-export diff regressions that assert exact changed line, region, function, and selected CLI gate outcomes for small LLVM fixtures and several slices of the checked-in real LLVM export.
- [x] (2026-03-23 23:30Z) Re-reviewed the LLVM investigation with a line-specific lens and concluded that the repository evidence no longer supports treating LLVM line and region discrepancies as the same class of problem.
- [x] (2026-03-23 23:35Z) Updated `docs/reference/llvm-export-semantics-investigation.md` so it now distinguishes the current line story from the current region story and states more clearly what the current tests do and do not prove.
- [ ] Capture or check in stable LLVM external line-oracle artifacts for the real LLVM repro, preferably LCOV `DA:` listings and, if feasible, LLVM text executable-line output for the same run.
- [ ] Decide which external line oracle should define `covgate` line-gating confidence: LLVM text executable lines, LCOV `DA:` entries, or another directly observable concrete line listing.
- [ ] Add fixture-backed tests that compare changed line opportunities in `tests/llvm_diff_regression.rs` against that chosen external line oracle rather than only against hand-authored expected spans.
- [ ] Reassess whether any remaining line-summary mismatch after those tests should still be treated as a parser defect, an export-detail limitation, or a separate summary-only discrepancy.
- [ ] Run `cargo test llvm_diff_regression -- --nocapture`, `cargo test llvm_real_parity -- --nocapture`, `cargo xtask quick`, and `cargo xtask validate` before closing the plan.

## Surprises & Discoveries

- Observation: the strongest evidence gathered so far for LLVM lines is not “LLVM summaries are inconsistent in the same way as regions.” It is that LLVM exposes multiple visible line views for the same run.
  Evidence: `docs/reference/llvm-export-semantics-investigation.md` now records files where JSON summary lines, LCOV `LF/LH`, LCOV `DA:` counts, and text-rendered executable lines diverge from one another.

- Observation: the current real-export diff regressions increase confidence in `covgate`'s line-gating consistency, but they do not yet bind `covgate` to an external LLVM line oracle.
  Evidence: `tests/llvm_diff_regression.rs` asserts exact changed line spans and CLI outcomes on real LLVM export slices, but those expected line spans are still authored against `covgate`'s own parsed model rather than against LLVM text output or LCOV `DA:` entries.

- Observation: the repository already knows how to keep line and region follow-up separate, because the investigation doc now frames the metrics differently.
  Evidence: `docs/reference/llvm-export-semantics-investigation.md` now says line confidence should come from concrete listed lines, while region confidence should come from segment-pattern fixtures and overlap behavior.

## Decision Log

- Decision: Treat LLVM line confidence as a separate follow-up from LLVM region-summary investigation.
  Rationale: the current evidence shows that line discrepancies involve competing external line views, while region discrepancies currently look more like a narrower summary-rule or parser-boundary question.
  Date/Author: 2026-03-23 / Codex

- Decision: Do not use LLVM JSON summary line totals as the default line-gating oracle for this plan.
  Rationale: the investigation already shows that JSON summary lines can disagree with both text executable lines and LCOV `DA:` concrete listings for the same run, so summary parity alone is not the right confidence target for diff gating.
  Date/Author: 2026-03-23 / Codex

- Decision: Build the next line tests around externally observable concrete line listings, not only around `covgate`'s internally derived changed spans.
  Rationale: the current repository tests already prove internal consistency. The remaining confidence gap is external-oracle alignment.
  Date/Author: 2026-03-23 / Codex

## Outcomes & Retrospective

No implementation work has landed under this plan yet. The valuable outcome so far is sharper problem framing. The repository no longer needs to talk about LLVM lines and LLVM regions as if they were one unresolved “summary parity” issue. Instead, it can move forward with a more honest split:

- lines need an external-oracle confidence plan
- regions need a segment-semantics investigation plan

That split matters because it changes what the next contributor should build. For lines, the highest-value next step is not another summary comparison. It is a reproducible comparison between `covgate`'s changed line opportunities and LLVM's best concrete line listing for the same changed files.

## Context and Orientation

`covgate` parses LLVM JSON in `src/coverage/llvm_json.rs`, builds `CoverageOpportunity` records, and computes changed metrics in `src/metrics.rs` by intersecting those opportunities with diff line ranges. The real multi-file LLVM fixture lives at `tests/fixtures/llvm-real/covgate-self-full.json`. `tests/llvm_real_parity.rs` compares `covgate`'s overall Markdown totals against LLVM JSON summary totals and currently documents that lines and regions disagree while functions match. `tests/llvm_diff_regression.rs` is the stronger diff-focused suite that parses the real LLVM fixture, feeds synthetic diffs into it, and asserts exact changed opportunities and CLI gate outcomes.

In this plan, an “external line oracle” means a directly observable LLVM-produced listing of concrete lines rather than a summary count. The two best candidates already identified in the repository are:

- LLVM text output showing executable lines
- LCOV `DA:` entries listing concrete line records

This plan intentionally distinguishes those from LLVM summary line totals such as JSON `summary.lines` or LCOV `LF/LH`, because the investigation already shows those summary-oriented counts can disagree with concrete line listings.

## Plan of Work

Start by choosing one or two checked-in real LLVM repro files that already have rich diff-focused tests, such as `src/config.rs`, `src/coverage/llvm_json.rs`, and `src/render/markdown.rs`. For each one, collect the corresponding LLVM concrete line oracle from the same coverage run. The preferred artifact is a checked-in LCOV file because `DA:` entries are structured and easier to assert against than human-formatted text. If LLVM text output is also needed because it exposes something LCOV does not, normalize only the line-listing portion into a small repository-owned artifact and document the transformation clearly.

Once the oracle artifact exists, add tests that compare changed line opportunities for those specific diffs against the external line listing. Do not start by changing parser code. The first question is whether `covgate` already matches the concrete oracle on the changed lines exercised by the repo’s real diff regressions. If it does, the remaining overall-summary mismatch should be described as a summary-only discrepancy, not as a line-gating defect. If it does not, the failing test will finally tell us which exact changed lines are missing or extra.

Only after those tests exist should parser changes be considered. If the failing lines show a real parser bug, add the smallest parser-local regression needed and fix it in `src/coverage/llvm_json.rs`. If the tests show that `covgate` already matches LLVM concrete line listings while still disagreeing with summary totals, keep the parser as-is and update the documentation and tests to make that confidence boundary explicit.

## Concrete Steps

Run all commands from the repository root, the directory containing `Cargo.toml`.

1. Re-read the current investigation and the existing LLVM real-fixture tests.

    sed -n '1,260p' docs/reference/llvm-export-semantics-investigation.md
    sed -n '1,220p' tests/llvm_real_parity.rs
    sed -n '1,380p' tests/llvm_diff_regression.rs

   Expected result: the current line-versus-region distinction is visible in the docs, and the real-fixture tests show which files already have explicit changed-line coverage.

2. Capture or check in concrete LLVM line-listing artifacts for the real repro.

    cargo llvm-cov report --lcov --output-path /tmp/covgate-real.lcov
    cargo llvm-cov report --text --output-dir /tmp/covgate-real-text

   Expected result: you can inspect LCOV `DA:` entries and, if needed, text executable-line listings for the same run. If those commands are not available in the current environment, document the exact prerequisite and keep the plan active rather than guessing.

3. Add fixture-backed tests that compare changed line opportunities against the chosen external line oracle.

    cargo test llvm_diff_regression -- --nocapture

   Expected result before any parser change: either the new tests already pass and prove that `covgate` matches the chosen concrete line oracle on the exercised diffs, or they fail with explicit missing or extra changed lines.

4. Only if needed, fix the parser and rerun the focused LLVM suites.

    cargo test llvm_diff_regression -- --nocapture
    cargo test llvm_real_parity -- --nocapture

   Expected result after a real parser fix: the external-oracle line tests pass. If `llvm_real_parity` remains red only on summary totals, that should be documented as expected rather than treated as a failed line-gating confidence target.

5. Run the normal repository validation loop before closing the plan.

    cargo xtask quick
    cargo xtask validate

   Expected result: all validation commands pass, and the repo records clearly whether the remaining LLVM line mismatch is a parser bug or a summary-only discrepancy.

## Validation and Acceptance

This plan is complete only when all of the following are true:

The repository contains a reproducible, fixture-backed external LLVM line oracle for at least the key real-repro files already exercised by `tests/llvm_diff_regression.rs`.

There are tests that compare `covgate` changed line opportunities against that external line oracle rather than only against hand-authored expected spans.

The final repository state states plainly whether `covgate` matches LLVM concrete line listings on the exercised diffs. If it does, the remaining summary mismatch is documented as a summary-only discrepancy. If it does not, the parser fix and tests identify the exact remaining changed-line defect.

`cargo test llvm_diff_regression -- --nocapture`, `cargo test llvm_real_parity -- --nocapture`, `cargo xtask quick`, and `cargo xtask validate` all pass in the final implementation state, except for any explicitly documented and intentionally retained summary-only red assertion that this plan decides to keep or revise.

## Idempotence and Recovery

The investigation steps in this plan must be safe to rerun. Capturing LLVM LCOV or text artifacts should not mutate checked-in coverage fixtures unless the contributor intentionally copies normalized artifacts into the repository. If a first oracle choice turns out to be too unstable or too hard to normalize, recover by checking in the simpler concrete line listing artifact instead of inventing another repository-local summary reconstruction.

If the line-oracle tests prove that `covgate` already matches LLVM concrete lines on the exercised diffs, do not recover by forcing a parser change anyway. In that case, the right recovery is to narrow the claim and adjust the remaining summary-parity tests or documentation to reflect the stronger, narrower confidence target.

## Artifacts and Notes

Representative current state from the investigation:

    real LLVM summary totals:
      lines 2890 / 2957
      regions 3285 / 3408

    current covgate totals on the same fixture:
      lines 2865 / 2910
      regions 3252 / 3355

Representative line-specific ambiguity already recorded in the repo:

    src/config.rs
      text executable lines: 322 / 337
      JSON summary lines:    309 / 342

    src/metrics.rs
      LCOV LF/LH:            133 / 133
      LCOV DA count:         127

These examples are why this plan focuses on external concrete line listings rather than on summary totals alone.

## Interfaces and Dependencies

The final state should preserve these boundaries:

`src/coverage/llvm_json.rs` remains the only production LLVM parser. Any line-parser changes must happen there, not in Markdown rendering or test helpers.

`tests/llvm_diff_regression.rs` should become the main home for real-fixture changed-line-oracle assertions, because it already owns the explicit changed diff slices.

`tests/llvm_real_parity.rs` may continue to document overall-summary disagreement, but it should no longer be the only or strongest signal for LLVM line confidence.

Any new checked-in oracle artifact should be generated from LLVM tooling, normalized minimally, and documented in this plan and in `docs/reference/llvm-export-semantics-investigation.md`.

At the bottom of this plan, append a revision note every time the plan changes materially, describing what changed and why.

Revision note: Initial plan created after re-reviewing the LLVM investigation and concluding that the remaining line discrepancy needs a dedicated external-oracle confidence plan rather than being treated as the same unresolved issue as the region summary discrepancy.
