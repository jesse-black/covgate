# Investigate and resolve the remaining Coverlet function-summary mismatch without disturbing the now-stable line and branch behavior

Save this in-progress ExecPlan in `docs/exec-plans/active/covgate-coverlet-function-summary-investigation.md`. Move it to `docs/exec-plans/completed/covgate-coverlet-function-summary-investigation.md` only after the function semantics are understood, the necessary code or documentation changes land, and validation passes.

This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds.

Maintain this document in accordance with `docs/PLANS.md`. Re-read that file before revising this plan and keep this plan aligned with its rules.

## Purpose / Big Picture

The repository’s current `.NET` evidence now shows that Coverlet line and branch totals align closely with native summary output, which is a valuable narrowing result. A user feeding Coverlet or Cobertura output to `covgate` should be able to trust those totals without fear that the line-summary bug seen in Vitest is silently affecting the .NET parser too. The remaining unresolved question is function coverage: the current real-world comparison shows that `covgate` still counts more functions than the native `.NET` summary tools do.

After this work, a novice should be able to regenerate the relevant `.NET` fixture or compare `covgate` against a real project’s summary path and understand exactly why function totals match or differ. The first job is not to force `covgate` to match ReportGenerator or Cobertura-derived method totals. The first job is to establish whether those totals are actually a trustworthy oracle for Coverlet JSON function semantics, or whether the mismatch is introduced somewhere in the export path from Coverlet JSON to Cobertura XML or to the downstream summary tool. Only after that investigation should the repository decide whether the goal is true parser parity with a native summary, documented irreducible ambiguity, or a narrower fixture-backed contract.

## Progress

- [x] (2026-03-19 18:23Z) Added the native-summary capture path for `.NET` reproducer fixtures so line-summary parity comparisons read a checked-in `native-summary.json` normalized from Cobertura summary attributes.
- [x] (2026-03-23 22:37Z) Compared a real Coverlet/Cobertura summary from a larger `.NET` project against `covgate` output and confirmed that line totals match (`228/255`) and branch totals match (`37/52`).
- [x] (2026-03-23 22:37Z) Recorded the remaining mismatch shape from that same `.NET` project: `covgate` reported function totals `73/82`, while the native method summary reported `71/80`.
- [ ] Investigate whether the function mismatch is already present when comparing raw Coverlet JSON detail against Cobertura XML and ReportGenerator output, before treating any Cobertura- or ReportGenerator-derived method summary as the oracle.
- [ ] Reproduce the function mismatch inside a checked-in open fixture or fixture expansion under `tests/fixtures/dotnet/` so the repository can prove the bug without relying only on a closed-source project summary.
- [ ] Inspect `src/coverage/coverlet_json.rs` function normalization against the native tool semantics and identify whether the current overcount comes from duplicate methods, compiler-generated methods, methods without meaningful source ownership, or another normalization boundary.
- [ ] Add a failing regression test in `src/coverage/coverlet_json.rs`, `tests/overall_summary.rs`, or both, depending on whether the mismatch is parser-local or only visible in end-to-end native summary comparison.
- [ ] Implement the smallest parser or harness fix required to make Coverlet function totals align with the native summary while preserving the already-matching line and branch totals.
- [ ] Run `cargo test overall_summary -- --nocapture`, targeted Coverlet parser tests, `cargo xtask quick`, and `cargo xtask validate` before closing this plan.

## Surprises & Discoveries

- Observation: The strongest new `.NET` evidence narrows the current problem considerably because line totals and branch totals already match native summary output.
  Evidence: the real project comparison reported `covgate` line totals `228/255` and branch totals `37/52`, which agree with the ReportGenerator summary’s `Covered lines: 228`, `Coverable lines: 255`, and `Branch coverage: 37 of 52`.

- Observation: The remaining mismatch is function-specific and not enormous, which suggests a normalization boundary rather than a completely wrong parser model.
  Evidence: `covgate` reported functions `73/82` while the native method summary reported `71/80`, a delta of two extra functions and two extra covered functions.

- Observation: `src/coverage/coverlet_json.rs` currently creates one function opportunity per parsed method object whenever that method has at least one line entry, using the minimum and maximum line numbers in the method as the function span.
  Evidence: the parser iterates each `CoverletMethod`, computes `start_line` and `end_line` from `method.lines.keys()`, and pushes one `FunctionRecord` whenever both bounds exist.

- Observation: The current checked-in `.NET` fixture matrix was built primarily around line and branch edge cases, not around method/function normalization drift against a native summary tool.
  Evidence: the existing reproducer fixture is `tests/fixtures/dotnet/duplicate-lines/`, which was introduced for line-summary parity rather than for function parity.

- Observation: There is no checked-in direct JSON summary artifact for Coverlet function totals today; the repository’s existing `.NET` native summary capture is derived from Cobertura summary attributes for line and branch counts, while the function mismatch evidence comes from downstream summary tooling.
  Evidence: `xtask/src/main.rs` currently normalizes Cobertura line and branch summary attributes into `native-summary.json`, and the real-project function comparison cited in this plan came from ReportGenerator-style method totals rather than from a top-level summary embedded in Coverlet JSON.

## Decision Log

- Decision: Keep the Coverlet function investigation separate from the Istanbul line-summary work.
  Rationale: the new evidence shows the ecosystems have diverged: Coverlet line and branch behavior is currently stable, while Istanbul line behavior needed a parser fix. Mixing them would make it harder for a novice to understand what is still open.
  Date/Author: 2026-03-23 / Codex

- Decision: Do not assume at the outset that Cobertura- or ReportGenerator-derived method totals are the correct oracle for Coverlet JSON function semantics.
  Rationale: unlike the line and branch path, the repository does not yet have a direct checked-in native JSON summary for Coverlet function totals. The first investigation step must determine whether the mismatch comes from `covgate`, from the JSON-to-Cobertura export path, or from the downstream summary tool’s own method semantics.
  Date/Author: 2026-03-23 / Codex

- Decision: Do not change Coverlet line or branch parsing while investigating function normalization.
  Rationale: the recent real-project comparison gives us valuable positive evidence that those metrics are already behaving correctly, and the plan should avoid destabilizing them while chasing a smaller function-specific gap.
  Date/Author: 2026-03-23 / Codex

## Outcomes & Retrospective

No implementation work has landed under this plan yet. The important outcome so far is sharper scope. The repository no longer needs to treat Coverlet as a likely source of the same line-summary drift that affected Istanbul. Instead, the open question is now specifically “why does Coverlet function normalization overcount relative to downstream method totals in at least one realistic project, and are those downstream totals the right oracle in the first place?”

That narrower framing is useful because it changes how the next contributor should spend time. The most valuable work is not another general line-summary audit. It is first an oracle investigation across Coverlet JSON, Cobertura export, and ReportGenerator-style summaries, and only then a focused function-normalization repro if the evidence still points back to `covgate`.

## Context and Orientation

`covgate` parses Coverlet JSON in `src/coverage/coverlet_json.rs`. Coverlet JSON is organized by module, then file, then class, then method. Each method object may contain `Lines` and `Branches`. The current parser turns every distinct line number in `Lines` into a line opportunity, every branch entry in `Branches` into a branch opportunity, and one function opportunity per method whenever the method has at least one line number. For functions, the parser does not currently carry the method name into the internal model. It only uses the minimum and maximum line numbers from the method’s `Lines` map to define a source span.

Overall-summary integration tests live in `tests/overall_summary.rs`, and the fixture harness lives in `tests/support/mod.rs`. The checked-in `.NET` fixture summaries come from `native-summary.json` files generated by `xtask/src/main.rs`, which currently normalize Cobertura summary attributes for line and branch totals. There is no checked-in native function summary artifact yet, because the repository’s previous parity work focused on line and branch counts and because Coverlet JSON itself does not currently give this repository a direct top-level function summary field to trust.

In this plan, “function summary mismatch” means the count of methods or functions reported by a downstream `.NET` summary path such as Cobertura-derived summary tooling or ReportGenerator differs from the `Function` total rendered by `covgate`. “Oracle investigation” means comparing the raw Coverlet JSON detail, the Cobertura export used by existing fixture tooling, and any downstream report summary before deciding which of those, if any, should be treated as authoritative for parser parity. “Normalization” means the repository-owned rule that decides which native method records should become internal `CoverageOpportunity { kind: Function }` entries and which should be merged or excluded.

## Plan of Work

Start by turning the current anecdotal real-project mismatch into repository-owned evidence, but do not assume the downstream method summary is the truth source yet. First compare one real or fixture-sized Coverlet JSON artifact against the Cobertura export and the ReportGenerator method summary to determine whether those layers agree with one another. If raw Coverlet JSON plus a simple repository-local inspection already shows a different method identity count than the downstream summary, record that before changing any parser code. The best outcome of this first step is a clear answer to the question “is the mismatch born in `covgate`, in the JSON-to-Cobertura export path, or in the downstream summary tool’s own semantics?”

Only after the oracle investigation should the plan extend fixture regeneration to capture a function or method summary artifact. If the chosen summary source is ReportGenerator text output, normalize only the stable method-total fields into a small checked-in JSON file and document that the repository is choosing ReportGenerator as the function oracle. If the investigation instead shows that Cobertura export or ReportGenerator introduces semantics that are not recoverable from Coverlet JSON, record that explicitly and narrow the goal before adding a parity test.

Once the fixture exists and the oracle is justified, add a failing regression that compares `covgate` function totals with the captured method totals for that fixture. Keep line and branch assertions in the same test or adjacent tests so any attempted function fix that accidentally changes those stable metrics is caught immediately.

Then inspect `src/coverage/coverlet_json.rs` closely around function construction. Determine whether the mismatch can be fixed by excluding specific method shapes, merging duplicate method identities, or carrying additional identity from the method key rather than only the source span. If the chosen method summary cannot be reproduced exactly from the JSON detail because the export path or the downstream reporter adds semantics that are not visible in the JSON, say so plainly in this plan and narrow the acceptance criteria to the best recoverable semantics rather than silently forcing a fake parity hack.

## Concrete Steps

Run all commands from the repository root, the directory containing `Cargo.toml`.

1. Re-read the current Coverlet parser and fixture harness to understand where function opportunities are created.

    sed -n '1,240p' src/coverage/coverlet_json.rs
    sed -n '340,460p' tests/support/mod.rs
    sed -n '1,220p' tests/overall_summary.rs

   Expected result: the current function path is visible and clearly separate from the line and branch paths.

2. Inspect the current `.NET` fixtures and decide whether to expand one or add a new function-focused fixture.

    find tests/fixtures/dotnet -maxdepth 4 -type f | sort
    sed -n '1,220p' tests/fixtures/dotnet/basic-fail/coverage.json
    sed -n '1,220p' tests/fixtures/dotnet/duplicate-lines/coverage.json

   Expected result: you can explain why the current fixture matrix is or is not enough to reproduce the function mismatch.

3. Add or regenerate the chosen function-focused `.NET` fixture and, only after the oracle investigation, capture a stable method summary artifact.

    cargo xtask regen-fixture-coverage dotnet/<scenario>

   Expected result: the fixture writes `coverage.json` and either a justified checked-in method summary artifact or a documented explanation of why the chosen summary path is not trustworthy enough to use as an oracle.

4. Add the failing regression and prove the mismatch before changing parser code.

    cargo test overall_summary -- --nocapture
    cargo test coverlet_json -- --nocapture

   Expected result before the fix: at least one function-focused assertion fails and points directly at disagreement with the captured native method totals.

5. Implement the smallest parser or harness change needed, then rerun focused tests.

    cargo test overall_summary -- --nocapture
    cargo test coverlet_json -- --nocapture

   Expected result after the fix: function totals align for the repro fixture, and line and branch totals remain unchanged.

6. Run the standard repository validation sweep.

    cargo xtask quick
    cargo xtask validate

   Expected result: the full repository remains green after the function normalization change.

## Validation and Acceptance

This plan is complete only when all of the following are true:

The repository contains a native-generated open `.NET` fixture that reproduces the Coverlet function or method mismatch and also records the result of the oracle investigation across Coverlet JSON, Cobertura export, and the chosen downstream summary path.

If a downstream method summary is chosen as the oracle, the final test path compares `covgate` function totals against that captured method summary artifact rather than against a repository-local guess.

If the mismatch is fixable from the JSON detail, `covgate` now matches the chosen method totals for the repro fixture without disturbing the already-stable line and branch totals.

If the mismatch is not fully derivable from the JSON detail, the plan says so explicitly, the tests prove the best recoverable behavior, and the repository documentation no longer over-claims full native function-summary parity.

`cargo test overall_summary -- --nocapture`, targeted Coverlet parser tests, `cargo xtask quick`, and `cargo xtask validate` all pass after the final implementation state.

## Idempotence and Recovery

Fixture generation must remain idempotent. Re-running `cargo xtask regen-fixture-coverage dotnet/<scenario>` should only rewrite that fixture’s native-generated artifacts. If the chosen native method summary source is external to Coverlet’s raw JSON, xtask must normalize it deterministically so ordinary `cargo test` runs do not depend on ambient tool output formatting changes.

If a candidate fixture does not reproduce the function mismatch, recover by changing the fixture source shape and regenerating the artifacts rather than by weakening the acceptance bar. Keep the plan updated so the next contributor knows which source patterns failed to reproduce the bug.

If investigation shows that the native method summary uses semantics not exposed clearly enough in the JSON export to reproduce exactly, recover by documenting that limitation and preserving line and branch correctness. Do not hide the gap by passing native summary totals through production output while leaving the parser model unexplained.

## Artifacts and Notes

Representative real-project narrowing result that motivated this plan:

    covgate line totals:     228 / 255
    native line totals:      228 / 255
    covgate branch totals:    37 / 52
    native branch totals:     37 / 52
    covgate function totals:  73 / 82
    native method totals:     71 / 80

Representative current Coverlet function construction in `src/coverage/coverlet_json.rs`:

    let start_line = method.lines.keys().copied().min();
    let end_line = method.lines.keys().copied().max();
    if let (Some(start_line), Some(end_line)) = (start_line, end_line) {
        let covered = method.lines.values().any(|hits| *hits > 0);
        function_records.push(FunctionRecord {
            start_line,
            end_line,
            covered,
        });
    }

This excerpt is the most likely first place to inspect because it currently ignores the native method name and treats every line-bearing method record as a distinct function opportunity.

Representative oracle uncertainty that this plan must resolve before a parity target is chosen:

    Coverlet JSON does not currently provide a repository-owned direct top-level function summary.
    The repository’s checked-in `.NET` native summary capture is based on Cobertura summary attributes for lines and branches.
    The current function mismatch evidence comes from downstream summary tooling such as ReportGenerator.

That means the first milestone is not “make `covgate` match ReportGenerator.” The first milestone is “prove whether ReportGenerator or Cobertura export is the right oracle for Coverlet JSON function semantics.”

## Interfaces and Dependencies

The final implementation should keep these interfaces clear:

`src/coverage/coverlet_json.rs` remains the only production parser for Coverlet JSON, and any function-normalization changes must happen there rather than in summary rendering.

`tests/support/mod.rs` should grow a native method-summary loading path only for the fixture scenarios covered by this plan, without disturbing the now-stable line and branch summary path.

`tests/overall_summary.rs` should gain function-summary parity coverage for the chosen `.NET` repro fixture once a stable native method-summary artifact exists.

`xtask/src/main.rs` remains the supported mechanism for regenerating `.NET` fixtures and any new method-summary artifact normalized from the native tool output.

At the bottom of this plan, append a revision note every time the plan changes materially, describing what changed and why.

Revision note: Initial plan created after confirming that Coverlet line and branch totals already match native summary output on a real project while a smaller function or method mismatch remains. The plan isolates that remaining `.NET` function-normalization question from the Istanbul line-summary work so future changes can stay focused and evidence-driven.

Revision note: Updated the initial plan to make the oracle investigation explicit. Before treating Cobertura- or ReportGenerator-derived method totals as the parity target, the plan now requires proving that the mismatch is not introduced by the JSON-to-Cobertura export or downstream reporting path itself.
