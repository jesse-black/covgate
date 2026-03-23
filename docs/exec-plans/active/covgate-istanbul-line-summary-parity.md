# Lock Istanbul line-summary behavior to native Vitest v8 semantics and realistic fixture-backed regressions

Save this in-progress ExecPlan in `docs/exec-plans/active/covgate-istanbul-line-summary-parity.md`. Move it to `docs/exec-plans/completed/covgate-istanbul-line-summary-parity.md` only after the remaining validation, documentation updates, and any final fixture expansions are complete.

This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds.

Maintain this document in accordance with `docs/PLANS.md`. Re-read that file before revising this plan and keep this plan aligned with its rules.

## Purpose / Big Picture

`covgate` now accepts larger Istanbul JSON artifacts from Vitest v8 and no longer reports obviously impossible line coverage in the small checked-in `fixtureSeed.ts` repro, but the more important user promise is stronger than “the parser no longer crashes.” A user should be able to run Vitest with the default v8 coverage provider, hand the resulting Istanbul JSON to `covgate`, and trust that the overall line totals in `covgate`’s Markdown summary reflect the same line semantics that Vitest itself reports in `coverage-summary.json`.

After the work already completed under this plan, a novice can run the checked-in Vitest summary regressions and see that `covgate` now matches the realistic multi-file `empty-branch-locations` fixture as well as the compact `statement-line-divergence` fixture. The remaining work is to preserve that confidence by documenting the semantics clearly, keeping the fixture-backed parity regressions explicit, and widening validation only when a new native-generated repro shows a remaining mismatch.

## Progress

- [x] (2026-03-22 00:00Z) Re-read `docs/PLANS.md`, `ARCHITECTURE.md`, `docs/TESTING.md`, `src/coverage/istanbul_json.rs`, `tests/support/mod.rs`, and the completed native-summary parity plans to restate the Istanbul problem as “native Vitest line-summary parity,” not just “statement math cleanup.”
- [x] (2026-03-22 00:15Z) Confirmed the existing toy Vitest fixtures were too small to expose the real parser and summary problems reported from larger projects.
- [x] (2026-03-22 18:30Z) Expanded the repository’s Vitest coverage evidence with the dedicated `tests/fixtures/vitest/empty-branch-locations/` fixture and updated parser tests so empty `branchMap.locations` no longer abort deserialization.
- [x] (2026-03-23 02:15Z) Reproduced the misleading Istanbul line case where `covgate` reported a covered line in `src/fixtures/fixtureSeed.ts` even though the checked-in coverage artifact marked the nested statement on line 20 as uncovered.
- [x] (2026-03-23 02:35Z) Added parser and metric regressions in `src/coverage/istanbul_json.rs` and `src/metrics.rs` proving that the uncovered nested `fixtureSeed.ts` call remains visible as an uncovered line opportunity.
- [x] (2026-03-23 03:10Z) Compared the checked-in Vitest native summary artifact for `empty-branch-locations` against candidate parser semantics and confirmed that native line totals match unique statement start lines rather than the union of full statement spans.
- [x] (2026-03-23 03:25Z) Updated `src/coverage/istanbul_json.rs` so Istanbul line totals use unique statement start lines with duplicate starts merged as covered when any statement at that start line has hits.
- [x] (2026-03-23 03:40Z) Added `tests/fixtures/vitest/empty-branch-locations` to `tests/overall_summary.rs` so the realistic multi-file Vitest fixture now participates in line-summary parity regressions.
- [x] (2026-03-23 03:55Z) Ran `cargo test istanbul_json -- --nocapture`, `cargo test overall_summary -- --nocapture`, and `cargo xtask quick` after the semantic change and observed all checks pass.
- [x] (2026-03-23 23:55Z) Decided that the next Istanbul confidence expansion will include a dedicated TSX-backed Vitest fixture rather than stopping at `.ts` coverage evidence.
- [ ] Document the final Istanbul line model in repository reference material so future contributors understand that the current parser follows native Vitest `coverage-summary.json` line totals, not union-of-span executable lines.
- [ ] Add at least one native-generated TSX-backed Vitest fixture under `tests/fixtures/vitest/` that exercises JSX or TSX-heavy source and feeds into the same Istanbul line-summary parity path as the existing `.ts` fixtures.
- [ ] Regenerate the new TSX fixture through `cargo xtask regen-fixture-coverage vitest/<scenario>`, check in its `coverage.json` and `native-summary.json`, and extend `tests/overall_summary.rs` so the TSX fixture participates in line-summary parity assertions.
- [ ] If the TSX fixture exposes new parser drift, add the failing regression first and then adjust `src/coverage/istanbul_json.rs` so the parser still matches the captured native summary artifact.
- [ ] Run `cargo xtask validate` after the documentation update or any new fixture work before closing this plan.

## Surprises & Discoveries

- Observation: The first parser fix that preferred the narrowest covering statement span removed the worst masking bug, but it still did not match the realistic Vitest native summary totals.
  Evidence: the `empty-branch-locations` native summary artifact records line totals `28/36`, while the intermediate span-based parser still produced `51/59` until the line model changed again.

- Observation: On the checked-in Vitest fixtures currently in this repository, native line totals align exactly with unique statement start lines.
  Evidence: `tests/fixtures/vitest/empty-branch-locations/native-summary.json` records `28/36`, and a direct comparison against the checked-in `coverage.json` showed that unique statement start lines also produce `28/36`; `tests/fixtures/vitest/statement-line-divergence/native-summary.json` similarly matches `9/12`.

- Observation: The original `fixtureSeed.ts` masking bug came from an enclosing covered `if` statement span marking line 20 as covered even though the nested statement on the same line had zero hits.
  Evidence: in `tests/fixtures/vitest/empty-branch-locations/coverage.json`, statement `1` spans lines 19 through 22 and is covered, while statement `2` starts on line 20 and has `0` hits.

- Observation: The environment used for local investigation did not expose `node`, `npm`, or Linuxbrew on `PATH`, even though `~/.zprofile` configures both Homebrew and `fnm`.
  Evidence: `which node`, `which npm`, and `brew --prefix node` all failed in this agent session, but probing `~/.local/share/fnm/node-versions/v24.14.0/installation/bin/node` confirmed an installed `fnm`-managed Node binary exists.

- Observation: The repository currently has realistic `.ts` Vitest fixtures, but no dedicated `.tsx` or JSX-backed Istanbul repro yet.
  Evidence: the checked-in Vitest fixtures under `tests/fixtures/vitest/` use `.js` and `.ts` sources today, and the user explicitly called out the absence of a TSX fixture during review of this plan.

## Decision Log

- Decision: Treat the checked-in Vitest `native-summary.json` artifact as the primary oracle for Istanbul line-summary parity, rather than reasoning from raw statement maps alone.
  Rationale: the entire bug class is that raw statement-map reconstruction can drift from what Vitest itself reports, so the native summary must stay the truth source for line-parity confidence.
  Date/Author: 2026-03-23 / Codex

- Decision: Use unique statement start lines as the current Istanbul line model.
  Rationale: this is the simplest parser-owned model that matches the realistic checked-in Vitest native summaries now in the repository and fixes the misleading `fixtureSeed.ts` “100% covered” case.
  Date/Author: 2026-03-23 / Codex

- Decision: Keep the `empty-branch-locations` fixture in the overall line-summary parity matrix instead of treating it as parser-only evidence.
  Rationale: it is the first realistic multi-file Vitest fixture in the repository that proves the line-summary model against native totals, so it should guard future changes directly.
  Date/Author: 2026-03-23 / Codex

- Decision: Do not widen the scope back to general “Istanbul summary parity across all upstream tools” without a new concrete repro.
  Rationale: the evidence gathered so far is specific to Vitest v8’s native summary behavior. The safest next step is to document and hold that behavior, not to over-claim universal Istanbul semantics.
  Date/Author: 2026-03-23 / Codex

- Decision: Add a dedicated TSX-backed Vitest fixture as part of this plan rather than leaving TSX coverage as an optional future follow-up.
  Rationale: the repository now has stronger confidence for `.ts` fixtures, but TSX and JSX-heavy source are common real-world shapes where statement-start semantics may still drift. The plan should close only after that source shape is represented in the native-summary parity matrix.
  Date/Author: 2026-03-23 / Codex

## Outcomes & Retrospective

The parser and regression work completed so far materially improved user trust. `covgate` no longer rejects the realistic Vitest branch-location shape, it no longer masks the uncovered nested `fixtureSeed.ts` call as a covered line, and its overall line totals now match the captured Vitest native summaries for both the compact line-summary fixture and the larger multi-file fixture.

The main lesson is that the important semantic boundary for Istanbul in this repository is not “all lines touched by a statement span.” The practical, user-visible contract is “whatever Vitest v8 itself reports in `coverage-summary.json` for line totals,” and the checked-in fixtures now provide concrete evidence that unique statement start lines match that contract on the exercised artifacts.

This plan remains open because the repository still needs a small amount of follow-through: document the final model plainly, add the promised TSX-backed Vitest fixture, keep the fixture-backed regressions explicit, and only close after a full `cargo xtask validate` pass on the final fixture and documentation state.

## Context and Orientation

`covgate` is a Rust command-line tool in `src/` that parses native coverage artifacts into a shared internal `CoverageReport`, computes changed and overall metrics, and renders console and Markdown output. The Istanbul adapter lives in `src/coverage/istanbul_json.rs`. That file is responsible for translating Vitest v8’s Istanbul JSON shape into line, branch, and function `CoverageOpportunity` records plus per-file totals. The Markdown total row is rendered later in `src/render/markdown.rs`, but those totals are only sums of what the parser produced; the renderer does not invent semantics.

The realistic open Vitest fixtures live under `tests/fixtures/vitest/`. `tests/fixtures/vitest/statement-line-divergence/` is the small focused line-summary repro. `tests/fixtures/vitest/empty-branch-locations/` is the larger multi-file TypeScript fixture that reproduces empty branch locations and now also serves as the main realistic line-summary parity check. Each of those fixtures contains a checked-in `coverage.json` from `coverage-final.json` and a checked-in `native-summary.json` normalized from Vitest’s `coverage-summary.json`.

`tests/support/mod.rs` contains `MetricFixtureCase`, which knows how to load the fixture’s captured native summary totals and how to run `covgate` against the fixture worktree to parse Markdown totals. `tests/overall_summary.rs` is the integration boundary that proves Markdown overall totals match the captured native totals. `src/metrics.rs` proves changed-line behavior against the same parser output.

In this plan, “line-summary parity” means that the `### Overall Coverage` line totals in `covgate`’s Markdown match the line totals recorded in the fixture’s checked-in `native-summary.json`, which is normalized from Vitest’s own `coverage-summary.json`. “Unique statement start lines” means: for each statement in the Istanbul `statementMap`, count only `statement.start.line` as the line opportunity, merge duplicate starts on the same file line, and mark that start line as covered if any statement beginning on that line has hits.

## Plan of Work

Keep the current parser model in `src/coverage/istanbul_json.rs` as the implementation of record, and make the remaining work about preserving and explaining it clearly. Update repository-facing documentation to state that the Istanbul line metric currently follows captured Vitest v8 native summary semantics as exercised by the checked-in fixtures, and that the implementation uses unique statement start lines because that is what matches the native summaries now stored in the repository.

Review `tests/overall_summary.rs`, `tests/support/mod.rs`, and the Vitest fixture README so a novice can see exactly which fixtures act as native-summary parity oracles. If a short comment or additional prose is needed to explain why `empty-branch-locations` now belongs in the line-summary parity matrix, add that explanation in the relevant test helper or README, keeping the wording focused on observable behavior rather than implementation folklore.

Then add a dedicated TSX-backed fixture under `tests/fixtures/vitest/` that keeps the source tree small but clearly JSX- or TSX-heavy enough to exercise statement-start behavior in a realistic frontend shape. Regenerate it through `cargo xtask regen-fixture-coverage vitest/<scenario>`, check in both `coverage.json` and `native-summary.json`, and extend the same parity tests instead of introducing a new ad hoc comparison path.

Only after the TSX fixture is in place should the plan decide whether more fixture expansion is necessary. If the TSX fixture is green under the current parser, record that as stronger evidence that the unique-statement-start-line model is holding across both `.ts` and `.tsx` source. If it is red, add the failing regression first and then adjust the parser.

Before closing the plan, run the repository’s full validation command and record the final result. Do not close the plan on targeted tests alone because this parser participates in CLI behavior, summary rendering, and fixture regeneration assumptions that the broader validation pass exercises.

## Concrete Steps

Run all commands from the repository root, the directory containing `Cargo.toml`.

1. Re-read the current Istanbul parser and the native-summary parity tests before any further edits.

    sed -n '1,220p' src/coverage/istanbul_json.rs
    sed -n '1,220p' tests/overall_summary.rs
    sed -n '340,460p' tests/support/mod.rs

   Expected result: the parser clearly counts line totals from `statement.start.line`, and the realistic Vitest fixtures appear in the line-summary parity matrix.

2. Verify the focused regressions that protect the fixed behavior before adding TSX.

    cargo test istanbul_json -- --nocapture
    cargo test changed_line_metric_keeps_uncovered_fixture_seed_call_visible -- --nocapture
    cargo test overall_summary_line_totals_match_native_summary_for_all_line_capable_fixtures -- --nocapture

   Expected result: the parser tests, changed-line `fixtureSeed.ts` regression, and overall-summary parity matrix all pass.

3. Add the new TSX-backed fixture and wire it into the parity matrix.

    cargo xtask regen-fixture-coverage vitest/<tsx-scenario>
    cargo test overall_summary_line_totals_match_native_summary_for_all_line_capable_fixtures -- --nocapture

   Expected result: the new fixture writes native-generated `coverage.json` and `native-summary.json`, and the line-summary parity test either passes immediately or fails with an explicit TSX-specific mismatch that can drive a parser regression.

4. If documentation, test-helper, or parser updates are made, rerun the fast repository loop.

    cargo xtask quick

   Expected result: all existing checks pass, proving the explanatory changes did not drift behavior.

5. Before closing the plan, run the full repository validation sweep.

    cargo xtask validate

   Expected result: all validation commands pass. If anything fails, keep this plan active and record the blocker in `Progress`, `Surprises & Discoveries`, and `Decision Log`.

## Validation and Acceptance

This plan is complete only when a novice can prove all of the following from the checked-in repository state:

The Istanbul parser in `src/coverage/istanbul_json.rs` accepts realistic Vitest v8 artifacts that contain empty branch locations and computes line totals that match the checked-in Vitest native summary artifacts.

The `fixtureSeed.ts` regression remains visible as an uncovered line opportunity rather than being masked by a larger enclosing covered statement span.

The realistic `tests/fixtures/vitest/empty-branch-locations/` scenario remains part of the line-summary parity matrix in `tests/overall_summary.rs`, and at least one dedicated TSX-backed Vitest fixture also participates in that same matrix, so future parser changes are forced to match captured native summary totals across both `.ts` and `.tsx` source shapes.

`cargo test istanbul_json -- --nocapture`, `cargo test overall_summary -- --nocapture`, and `cargo xtask validate` all pass after the final documentation state is committed.

## Idempotence and Recovery

The parser and test steps in this plan are safe to rerun. Re-running the focused tests or `cargo xtask quick` should not mutate checked-in artifacts. Re-running `cargo xtask regen-fixture-coverage vitest/<scenario>` is also safe when a new native repro is needed, because xtask rewrites the checked-in `coverage.json` and `native-summary.json` deterministically from native tool output.

If a future native-generated Vitest artifact disagrees with the current unique-statement-start-line model, recover by checking in that artifact and its `native-summary.json` first, then extending `tests/overall_summary.rs` to make the disagreement explicit before changing the parser again. Do not “fix” the mismatch by editing the captured native summary artifact or by bypassing the fixture-backed parity path.

If the environment used for regeneration still lacks `node` on `PATH`, recover by using the discovered absolute `fnm`-managed Node path or by exporting the `fnm` installation bin directory explicitly inside the regeneration command environment. Do not hand-edit Istanbul artifacts as a substitute for native regeneration.

If the new TSX fixture turns out to be too framework-heavy or unstable, recover by shrinking the source tree while keeping the source type TSX. The plan's goal is not a large frontend app; it is a minimal, native-generated TSX repro that still exercises JSX-style statement mapping.

## Artifacts and Notes

Representative native-summary evidence that motivated the current parser model:

    tests/fixtures/vitest/empty-branch-locations/native-summary.json
    {
      "line": {
        "covered": 28,
        "total": 36
      }
    }

Representative focused regression proving the nested uncovered line remains visible:

    cargo test changed_line_metric_keeps_uncovered_fixture_seed_call_visible -- --nocapture
    test metrics::tests::changed_line_metric_keeps_uncovered_fixture_seed_call_visible ... ok

Representative parity proof after the semantic change:

    cargo test overall_summary -- --nocapture
    test overall_summary_line_totals_match_native_summary_for_all_line_capable_fixtures ... ok

Representative follow-up expectation for this still-open plan:

    tests/fixtures/vitest/<tsx-scenario>/repo/src/...
    tests/fixtures/vitest/<tsx-scenario>/overlay/src/...
    tests/fixtures/vitest/<tsx-scenario>/coverage.json
    tests/fixtures/vitest/<tsx-scenario>/native-summary.json

## Interfaces and Dependencies

The final repository state for this plan should preserve these interfaces:

`src/coverage/istanbul_json.rs` must continue to expose `parse_str_with_repo_root(input, repo_root) -> Result<CoverageReport>` and implement line totals using unique statement start lines merged by file line.

`tests/support/mod.rs::MetricFixtureCase::native_overall_totals()` must continue to prefer a checked-in `native-summary.json` artifact when one exists for a Vitest fixture.

`tests/overall_summary.rs` must continue to include both `support::vitest_statement_line_divergence_fixture()` and `support::vitest_empty_branch_locations_fixture()` in the line-summary parity coverage, and it must gain a dedicated TSX-backed Vitest fixture entry before this plan closes.

`xtask/src/main.rs` remains the only supported regeneration path for native Vitest fixture artifacts and native summary capture.

At the bottom of this plan, append a revision note every time the plan changes materially, describing what changed and why.

Revision note: Initial plan created after the Istanbul parser no longer crashed on empty branch locations and after the line model was corrected to match realistic Vitest native summaries. The plan separates the now-fixed Istanbul line-summary work from the still-open Coverlet function investigation so each ecosystem can be reasoned about on its own terms.

Revision note: Updated the plan after deciding explicitly that the remaining Istanbul confidence work will include a TSX-backed fixture. The plan now treats TSX coverage as a required follow-up rather than an optional future enhancement.
