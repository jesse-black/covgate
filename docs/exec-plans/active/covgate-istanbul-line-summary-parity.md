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
- [ ] Document the final Istanbul line model in repository reference material so future contributors understand that the current parser follows native Vitest `coverage-summary.json` line totals, not union-of-span executable lines.
- [ ] Decide whether to add one more realistic Vitest fixture that stresses repeated statement start lines across TSX or JSX-heavy source, only if a native-generated user repro demonstrates coverage drift that the current fixture set still misses.
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

## Outcomes & Retrospective

The parser and regression work completed so far materially improved user trust. `covgate` no longer rejects the realistic Vitest branch-location shape, it no longer masks the uncovered nested `fixtureSeed.ts` call as a covered line, and its overall line totals now match the captured Vitest native summaries for both the compact line-summary fixture and the larger multi-file fixture.

The main lesson is that the important semantic boundary for Istanbul in this repository is not “all lines touched by a statement span.” The practical, user-visible contract is “whatever Vitest v8 itself reports in `coverage-summary.json` for line totals,” and the checked-in fixtures now provide concrete evidence that unique statement start lines match that contract on the exercised artifacts.

This plan remains open because the repository still needs a small amount of follow-through: document the final model plainly, keep the fixture-backed regressions explicit, and only close after a full `cargo xtask validate` pass on the final documentation state.

## Context and Orientation

`covgate` is a Rust command-line tool in `src/` that parses native coverage artifacts into a shared internal `CoverageReport`, computes changed and overall metrics, and renders console and Markdown output. The Istanbul adapter lives in `src/coverage/istanbul_json.rs`. That file is responsible for translating Vitest v8’s Istanbul JSON shape into line, branch, and function `CoverageOpportunity` records plus per-file totals. The Markdown total row is rendered later in `src/render/markdown.rs`, but those totals are only sums of what the parser produced; the renderer does not invent semantics.

The realistic open Vitest fixtures live under `tests/fixtures/vitest/`. `tests/fixtures/vitest/statement-line-divergence/` is the small focused line-summary repro. `tests/fixtures/vitest/empty-branch-locations/` is the larger multi-file TypeScript fixture that reproduces empty branch locations and now also serves as the main realistic line-summary parity check. Each of those fixtures contains a checked-in `coverage.json` from `coverage-final.json` and a checked-in `native-summary.json` normalized from Vitest’s `coverage-summary.json`.

`tests/support/mod.rs` contains `MetricFixtureCase`, which knows how to load the fixture’s captured native summary totals and how to run `covgate` against the fixture worktree to parse Markdown totals. `tests/overall_summary.rs` is the integration boundary that proves Markdown overall totals match the captured native totals. `src/metrics.rs` proves changed-line behavior against the same parser output.

In this plan, “line-summary parity” means that the `### Overall Coverage` line totals in `covgate`’s Markdown match the line totals recorded in the fixture’s checked-in `native-summary.json`, which is normalized from Vitest’s own `coverage-summary.json`. “Unique statement start lines” means: for each statement in the Istanbul `statementMap`, count only `statement.start.line` as the line opportunity, merge duplicate starts on the same file line, and mark that start line as covered if any statement beginning on that line has hits.

## Plan of Work

Keep the current parser model in `src/coverage/istanbul_json.rs` as the implementation of record, and make the remaining work about preserving and explaining it clearly. Update repository-facing documentation to state that the Istanbul line metric currently follows captured Vitest v8 native summary semantics as exercised by the checked-in fixtures, and that the implementation uses unique statement start lines because that is what matches the native summaries now stored in the repository.

Review `tests/overall_summary.rs`, `tests/support/mod.rs`, and the Vitest fixture README so a novice can see exactly which fixtures act as native-summary parity oracles. If a short comment or additional prose is needed to explain why `empty-branch-locations` now belongs in the line-summary parity matrix, add that explanation in the relevant test helper or README, keeping the wording focused on observable behavior rather than implementation folklore.

Only if a new user-provided native-generated repro remains red should this plan expand to add another fixture or to revisit the parser model. If that happens, create the new fixture under `tests/fixtures/vitest/`, regenerate it through `cargo xtask regen-fixture-coverage vitest/<scenario>`, add its native summary artifact, and extend the same parity tests instead of introducing a new ad hoc comparison path.

Before closing the plan, run the repository’s full validation command and record the final result. Do not close the plan on targeted tests alone because this parser participates in CLI behavior, summary rendering, and fixture regeneration assumptions that the broader validation pass exercises.

## Concrete Steps

Run all commands from the repository root, the directory containing `Cargo.toml`.

1. Re-read the current Istanbul parser and the native-summary parity tests before any further edits.

    sed -n '1,220p' src/coverage/istanbul_json.rs
    sed -n '1,220p' tests/overall_summary.rs
    sed -n '340,460p' tests/support/mod.rs

   Expected result: the parser clearly counts line totals from `statement.start.line`, and the realistic Vitest fixtures appear in the line-summary parity matrix.

2. Verify the focused regressions that protect the fixed behavior.

    cargo test istanbul_json -- --nocapture
    cargo test changed_line_metric_keeps_uncovered_fixture_seed_call_visible -- --nocapture
    cargo test overall_summary_line_totals_match_native_summary_for_all_line_capable_fixtures -- --nocapture

   Expected result: the parser tests, changed-line `fixtureSeed.ts` regression, and overall-summary parity matrix all pass.

3. If documentation or small explanatory test-helper updates are made, rerun the fast repository loop.

    cargo xtask quick

   Expected result: all existing checks pass, proving the explanatory changes did not drift behavior.

4. Before closing the plan, run the full repository validation sweep.

    cargo xtask validate

   Expected result: all validation commands pass. If anything fails, keep this plan active and record the blocker in `Progress`, `Surprises & Discoveries`, and `Decision Log`.

## Validation and Acceptance

This plan is complete only when a novice can prove all of the following from the checked-in repository state:

The Istanbul parser in `src/coverage/istanbul_json.rs` accepts realistic Vitest v8 artifacts that contain empty branch locations and computes line totals that match the checked-in Vitest native summary artifacts.

The `fixtureSeed.ts` regression remains visible as an uncovered line opportunity rather than being masked by a larger enclosing covered statement span.

The realistic `tests/fixtures/vitest/empty-branch-locations/` scenario remains part of the line-summary parity matrix in `tests/overall_summary.rs`, so future parser changes are forced to match its captured native summary totals.

`cargo test istanbul_json -- --nocapture`, `cargo test overall_summary -- --nocapture`, and `cargo xtask validate` all pass after the final documentation state is committed.

## Idempotence and Recovery

The parser and test steps in this plan are safe to rerun. Re-running the focused tests or `cargo xtask quick` should not mutate checked-in artifacts. Re-running `cargo xtask regen-fixture-coverage vitest/<scenario>` is also safe when a new native repro is needed, because xtask rewrites the checked-in `coverage.json` and `native-summary.json` deterministically from native tool output.

If a future native-generated Vitest artifact disagrees with the current unique-statement-start-line model, recover by checking in that artifact and its `native-summary.json` first, then extending `tests/overall_summary.rs` to make the disagreement explicit before changing the parser again. Do not “fix” the mismatch by editing the captured native summary artifact or by bypassing the fixture-backed parity path.

If the environment used for regeneration still lacks `node` on `PATH`, recover by using the discovered absolute `fnm`-managed Node path or by exporting the `fnm` installation bin directory explicitly inside the regeneration command environment. Do not hand-edit Istanbul artifacts as a substitute for native regeneration.

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

## Interfaces and Dependencies

The final repository state for this plan should preserve these interfaces:

`src/coverage/istanbul_json.rs` must continue to expose `parse_str_with_repo_root(input, repo_root) -> Result<CoverageReport>` and implement line totals using unique statement start lines merged by file line.

`tests/support/mod.rs::MetricFixtureCase::native_overall_totals()` must continue to prefer a checked-in `native-summary.json` artifact when one exists for a Vitest fixture.

`tests/overall_summary.rs` must continue to include both `support::vitest_statement_line_divergence_fixture()` and `support::vitest_empty_branch_locations_fixture()` in the line-summary parity coverage.

`xtask/src/main.rs` remains the only supported regeneration path for native Vitest fixture artifacts and native summary capture.

At the bottom of this plan, append a revision note every time the plan changes materially, describing what changed and why.

Revision note: Initial plan created after the Istanbul parser no longer crashed on empty branch locations and after the line model was corrected to match realistic Vitest native summaries. The plan separates the now-fixed Istanbul line-summary work from the still-open Coverlet function investigation so each ecosystem can be reasoned about on its own terms.
