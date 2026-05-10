---
description: "ExecPlan for separating gate policy evaluations from changed metric evidence, renaming GateResult to CheckResult, and fixing Markdown/console 0/0 coverage rendering."
---

# Split Changed Metrics From Gate Evaluations

## Goal
- `covgate` result modeling separates gate policy outcomes from changed metric evidence; Markdown lists all configured gates while metric tables remain metric-only, and `0/0` percent observations render as `N/A` in console and Markdown.

## Scope
- In: rename the top-level result to `CheckResult`, split changed metrics out of gate evaluations, render all configured gate rules in Markdown, remove gate columns from metric tables, and add `N/A (0/0)` observed output for zero-opportunity percent rules.
- Out: config schema changes, CLI flag changes, parser metric definition changes, and fake per-gate metric rows for gates with no changed files.

## Relevant Areas
- `src/model.rs`, `src/gate.rs`, `src/lib.rs` — result model, rule evaluation, and run orchestration.
- `src/render/markdown.rs`, `src/render/console.rs` — gate/rule summary and metric evidence rendering.
- `tests/render_markdown.rs`, `tests/render_console.rs`, `tests/cli_interface.rs`, `tests/gate.rs` — renderer and fixture-backed behavior coverage.

## Open Questions
- None yet.

## Steps
- [x] Add failing renderer/integration tests for complete gate summaries, metric-only Markdown tables, and `N/A (0/0)` percent observations.
- [x] Rename `GateResult` to `CheckResult` and split `changed_metrics` from gate evaluations.
- [x] Refactor gate evaluation so `GateEvaluation` contains policy outcomes only, while scoped metrics remain internal to rule evaluation.
- [x] Update Markdown rendering so the gate summary includes all configured gates and metric sections render only `changed_metrics`.
- [x] Update console rendering so `0/0` percent observations render as `N/A`.
- [x] Remove the earlier `participates` field approach from model, evaluation, renderers, and tests.
- [x] Run focused tests and `cargo xtask validate`.

## Validation
- `cargo test --test render_markdown`
- `cargo test --test render_console`
- `cargo test --test cli_interface path_scoped_gates`
- `cargo test path_scoped_gates`
- `cargo xtask validate`

## Discoveries
- Current worktree contains an earlier `participates`-based attempt that must be replaced rather than extended.
- Focused tests passed for `render_markdown`, `render_console`, `cli_interface path_scoped_gates`, and `path_scoped_gates`; full validation remains.
- `cargo xtask validate` passed after updating stale CLI output expectations.

## Review
- None yet.

## Definition of Done

### Planner
- [x] Plan is consistent, up to date, decision-complete, and ready to hand off.

### Generator
- [x] Goal achieved: gate policy outcomes and changed metric evidence are separate in the result model and output.
- [x] All planned steps are complete.
- [x] All validation commands pass.
- [ ] Handed off to an independent reviewer (MUST use the `evaluator-execplan` skill via a subagent or separate agent, not the generator agent).

### Evaluator
- [ ] Standard review posture applied.
- [ ] Adheres to the principles of `docs/CODESTYLE.md`.
- [ ] Adheres to the principles of `docs/TESTING.md`.
- [ ] All review findings have been addressed.
