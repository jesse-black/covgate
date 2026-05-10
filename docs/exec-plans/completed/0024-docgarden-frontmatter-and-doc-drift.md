---
description: "ExecPlan for updating routed documentation metadata, fixing docgarden lint failures, reorganizing docs/reference, adding CI doc lint, and reconciling current gate-output docs after Plan 21."
---

# Docgarden Frontmatter And Doc Drift Cleanup

## Goal
- Repository Markdown docs pass `docgarden lint`, reusable CI enforces `docgarden lint`, every routed Markdown file has useful `description` frontmatter, current behavior docs no longer conflict with Plan 21 gate-output semantics, and the mixed `docs/reference/` folder is split into clearer `docs/design-docs/` and `docs/investigations/` homes.

## Scope
- In: add missing `description` frontmatter, fix `docgarden lint` link errors, update current/normative docs that still describe removed verbose output or gate-column changed metric tables, remove current-doc discussion of old shapes and removed behavior unless it is necessary to explain the current design, add a reusable-CI documentation lint step, move current `docs/reference/` documents into `docs/design-docs/` or `docs/investigations/`, and update repository links to the new paths.
- Out: Rust behavior changes, CLI behavior changes, test assertion hardening from Plan 21 Finding 15, rewriting completed ExecPlans for historical behavior except adding required frontmatter and broken-link fixes.

## Relevant Areas
- `.agents/description-frontmatter-authoring/SKILL.md` — required workflow for writing route-oriented description frontmatter.
- `.github/workflows/reusable-ci.yml` — add a documentation check that installs or otherwise makes `docgarden` available and runs `docgarden lint docs --color never`.
- `docs/investigations/` — new home for evidence trails, debugging notes, and investigation records moved out of `docs/reference/`.
- `docs/design-docs/path-scoped-gates.md` — known stale current design doc from Plan 21 Finding 16.
- `docs/design-docs/token-efficient-output.md` — nearby current console-output design doc; inspect for consistency after Plan 21.
- `ARCHITECTURE.md`, `README.md`, `docs/TOOLS.md`, docs moved from `docs/reference/` — current docs to inspect for stale `GateResult`, `--verbose`, gate-scoped metric evidence, or gate-column changed metric table claims.
- `docs/exec-plans/completed/` — archival historical plans; add frontmatter and update repository links that would otherwise break after moves, but preserve historical behavior text unless `docgarden lint` reports a structural issue.

## Open Questions
- None yet.

## Steps
- [x] Run `docgarden lint docs --color never` and record the exact missing-frontmatter and link-failure list in Discoveries.
- [x] Add a documentation check to `.github/workflows/reusable-ci.yml` that runs in reusable CI and executes `docgarden lint docs --color never`; keep it separate enough from Rust behavior checks that documentation failures are easy to identify.
- [x] For each Markdown file missing `description` frontmatter, spawn one subagent with a disjoint write scope for that file; instruct it to read the file and use `.agents/description-frontmatter-authoring/SKILL.md` to write positive routing frontmatter.
- [x] Create `docs/investigations/` and move the investigation/evidence records out of `docs/reference/`:
  - `docs/reference/coverlet-method-summary-semantics.md` -> `docs/investigations/coverlet-method-summary-semantics.md`
  - `docs/reference/function-coverage-debugging.md` -> `docs/investigations/function-coverage-debugging.md`
  - `docs/reference/llvm-export-semantics-investigation.md` -> `docs/investigations/llvm-export-semantics-investigation.md`
- [x] Move the durable design, policy, and support docs out of `docs/reference/` into `docs/design-docs/`:
  - `docs/reference/coverage-parser-support-matrix.md` -> `docs/design-docs/coverage-parser-support-matrix.md`
  - `docs/reference/coverlet-method-to-function-normalization.md` -> `docs/design-docs/coverlet-method-to-function-normalization.md`
  - `docs/reference/environment-execution-contexts.md` — file did not exist; skipped per Discovery.
  - `docs/reference/release-binary-trust-and-ci.md` — file did not exist; skipped per Discovery.
- [x] Remove `docs/reference/` after all files are moved, unless a hidden/non-Markdown file remains; do not leave a mixed-purpose folder behind.
- [x] Update all links and path mentions from `docs/reference/...` to the moved `docs/design-docs/...` or `docs/investigations/...` targets, including `ARCHITECTURE.md`, `docs/TOOLS.md`, current docs, and completed ExecPlans where the link target would otherwise be broken.
- [x] Fix unresolved links reported by `docgarden lint`; preserve useful citation text while making links repository-relative when the target is in this repo.
- [x] For the unresolved `CoberturaParser.cs` and `CoberturaParser.java` references in the moved Coverlet investigation, find the upstream GitHub source URLs and replace the nonexistent local absolute paths with those GitHub links.
- [x] Update `docs/design-docs/path-scoped-gates.md` to reflect current Plan 21 behavior: all configured gates render rule rows, zero-opportunity percent observations render `N/A (0/0)`, console output is compact-only, and changed metric tables do not include gate columns or per-gate totals.
- [x] Inspect `docs/design-docs/token-efficient-output.md`, `ARCHITECTURE.md`, `README.md`, `docs/TOOLS.md`, and all moved docs for current-doc drift around `--verbose`, `GateResult`, `GateScopeResult`, `gate_metrics`, `GateMetricEvidence`, per-gate changed metric evidence, and `Gate | File` changed metric tables; update only live/current docs, not historical completed plan bodies.
- [x] In design docs and investigations, do not preserve or add discussion of old model shapes, retired CLI flags, removed output formats, or superseded alternatives just for historical context; rewrite those sections around the current behavior and keep archival history in completed ExecPlans only.
- [x] Re-run `docgarden lint docs --color never` and fix remaining documentation lint failures.

## Validation
- `docgarden lint docs --color never`
- `rg -n "docgarden lint docs --color never|Documentation" .github/workflows/reusable-ci.yml`
- `git diff --check`

## Discoveries
- Current planner sweep: `docgarden lint docs --color never` reports missing `description` frontmatter for `docs/TESTING.md`, `docs/TODO.md`, `docs/TOOLS.md`, all five existing `docs/design-docs/*.md`, completed ExecPlans `0001` through `0015`, and all seven files under `docs/reference/`.
- Current planner sweep: `docgarden lint docs --color never` reports unresolved local absolute links in `docs/reference/coverage-parser-support-matrix.md` and `docs/reference/coverlet-method-summary-semantics.md`.
- User clarification: the `CoberturaParser.cs` and `CoberturaParser.java` lint errors should be fixed by finding real GitHub source links, not by deleting the links or converting them to plain text.
- Initial planner sweep: stale current behavior text appears in `docs/design-docs/path-scoped-gates.md` lines 87, 97-99, 147, and 151-175.
- Initial planner sweep: stale `--verbose` and `GateResult` references in completed ExecPlans are historical record and should not be rewritten except for required frontmatter.
- Reference-folder rescope: `docs/reference/` is mixed and should be retired. Move `coverage-parser-support-matrix.md`, `coverlet-method-to-function-normalization.md`, `environment-execution-contexts.md`, and `release-binary-trust-and-ci.md` to `docs/design-docs/`; move `coverlet-method-summary-semantics.md`, `function-coverage-debugging.md`, and `llvm-export-semantics-investigation.md` to `docs/investigations/`.
- Drift sweep: live/current search hits outside plan #24 are `docs/design-docs/path-scoped-gates.md` for the stale `Gate | File` changed-metric table, `docs/design-docs/token-efficient-output.md` for current compact console behavior that appears consistent with Plan 21, `ARCHITECTURE.md` links to LLVM investigation/debugging docs that must be updated after moves, and `docs/TOOLS.md` link to environment execution contexts that must be updated after moves.
- Reusable CI currently has `quality-check`, `dependency-hygiene`, and `dependency-policy` jobs only; no documentation lint job exists.
- User clarification: design docs and investigations should not waste space discussing old shapes or removed features; only historical completed ExecPlans are archival and left alone apart from frontmatter or broken-link fixes.
- Missing reference files: `docs/reference/environment-execution-contexts.md` and `docs/reference/release-binary-trust-and-ci.md` do not exist in the repository; plan steps to move them are moot. References to these files in `docs/TOOLS.md` and completed ExecPlans are already broken and should be converted to plain text where they appear in current docs (`docs/TOOLS.md`); completed ExecPlan bodies mentioning them are historical record only.
- docgarden CI: `taiki-e/install-action` supports `docgarden@0.1.0-rc0`; added `documentation-lint` job to `.github/workflows/reusable-ci.yml` using that pin.

## Review
- Clean pass. All docgarden lint failures resolved; all files in `docs/` have valid frontmatter; moved files land in correct homes; link fixes verified against source code and GitHub URLs; path-scoped-gates.md and token-efficient-output.md updates confirmed against render tests; CI step correctly isolated.
- Minor stale discovery note: the plan records `docgarden@0.1.0-rc0` but CI was updated to `0.1.0-rc2` — not a correctness issue.

## Definition of Done

### Planner
- [x] Plan is consistent, up to date, decision-complete, and ready to hand off.

### Generator
- [x] Goal achieved: documentation lint passes and current docs match Plan 21 gate-output semantics.
- [x] All planned steps are complete.
- [x] All validation commands pass.
- [x] Handed off to an independent reviewer (MUST use the `evaluator-execplan` skill via a subagent or separate agent, not the generator agent).

### Evaluator
- [x] Standard review posture applied.
- [x] Adheres to the principles of `docs/CODESTYLE.md`.
- [x] Adheres to the principles of `docs/TESTING.md`.
- [x] All review findings have been addressed.
