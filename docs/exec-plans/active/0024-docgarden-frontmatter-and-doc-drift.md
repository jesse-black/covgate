---
description: "ExecPlan for updating routed documentation metadata, fixing docgarden lint failures, and reconciling current gate-output docs after Plan 21."
---

# Docgarden Frontmatter And Doc Drift Cleanup

## Goal
- Repository Markdown docs pass `docgarden lint`, every routed Markdown file has useful `description` frontmatter, and current behavior docs no longer conflict with Plan 21 gate-output semantics.

## Scope
- In: add missing `description` frontmatter, fix `docgarden lint` link errors, update current/normative docs that still describe removed verbose output or gate-column changed metric tables.
- Out: Rust behavior changes, CLI behavior changes, test assertion hardening from Plan 21 Finding 15, rewriting completed ExecPlans for historical behavior except adding required frontmatter.

## Relevant Areas
- `.agents/description-frontmatter-authoring/SKILL.md` — required workflow for writing route-oriented description frontmatter.
- `docs/design-docs/path-scoped-gates.md` — known stale current design doc from Plan 21 Finding 16.
- `docs/design-docs/token-efficient-output.md` — nearby current console-output design doc; inspect for consistency after Plan 21.
- `ARCHITECTURE.md`, `README.md`, `docs/reference/` — current/reference docs to inspect for stale `GateResult`, `--verbose`, gate-scoped metric evidence, or gate-column changed metric table claims.
- `docs/exec-plans/completed/` — add frontmatter only; preserve historical body text unless `docgarden lint` reports a structural issue.

## Open Questions
- None yet.

## Steps
- [ ] Run `docgarden lint docs --color never` and record the exact missing-frontmatter and link-failure list in Discoveries.
- [ ] For each Markdown file missing `description` frontmatter, spawn one subagent with a disjoint write scope for that file; instruct it to read the file and use `.agents/description-frontmatter-authoring/SKILL.md` to write positive routing frontmatter.
- [ ] Fix unresolved links reported by `docgarden lint`; preserve useful citation text while making links repository-relative when the target is in this repo.
- [ ] For the unresolved `CoberturaParser.cs` and `CoberturaParser.java` references in `docs/reference/coverlet-method-summary-semantics.md`, find the upstream GitHub source URLs and replace the nonexistent local absolute paths with those GitHub links.
- [ ] Update `docs/design-docs/path-scoped-gates.md` to reflect current Plan 21 behavior: all configured gates render rule rows, zero-opportunity percent observations render `N/A (0/0)`, console output is compact-only, and changed metric tables do not include gate columns or per-gate totals.
- [ ] Inspect `docs/design-docs/token-efficient-output.md`, `ARCHITECTURE.md`, `README.md`, and `docs/reference/` for current-doc drift around `--verbose`, `GateResult`, `GateScopeResult`, `gate_metrics`, `GateMetricEvidence`, per-gate changed metric evidence, and `Gate | File` changed metric tables; update only live/current docs, not historical completed plan bodies.
- [ ] Re-run `docgarden lint docs --color never` and fix remaining documentation lint failures.

## Validation
- `docgarden lint docs --color never`
- `git diff --check`

## Discoveries
- Initial planner sweep: `docgarden lint docs --color never` reports missing `description` frontmatter for `docs/TESTING.md`, `docs/TODO.md`, `docs/TOOLS.md`, all five `docs/design-docs/*.md`, completed ExecPlans `0001` through `0015`, and all eight files under `docs/reference/`.
- Initial planner sweep: `docgarden lint docs --color never` reports unresolved links in `docs/reference/coverage-parser-support-matrix.md` and `docs/reference/coverlet-method-summary-semantics.md`.
- User clarification: the `CoberturaParser.cs` and `CoberturaParser.java` lint errors should be fixed by finding real GitHub source links, not by deleting the links or converting them to plain text.
- Initial planner sweep: stale current behavior text appears in `docs/design-docs/path-scoped-gates.md` lines 87, 97-99, 147, and 151-175.
- Initial planner sweep: stale `--verbose` and `GateResult` references in completed ExecPlans are historical record and should not be rewritten except for required frontmatter.

## Review
- None yet.

## Definition of Done

### Planner
- [x] Plan is consistent, up to date, decision-complete, and ready to hand off.

### Generator
- [ ] Goal achieved: documentation lint passes and current docs match Plan 21 gate-output semantics.
- [ ] All planned steps are complete.
- [ ] All validation commands pass.
- [ ] Handed off to an independent reviewer (MUST use the `evaluator-execplan` skill via a subagent or separate agent, not the generator agent).

### Evaluator
- [ ] Standard review posture applied.
- [ ] Adheres to the principles of `docs/CODESTYLE.md`.
- [ ] Adheres to the principles of `docs/TESTING.md`.
- [ ] All review findings have been addressed.
