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

### Finding 1 — High: Multi-gate console failure rendering uses global metrics without gate attribution

`render_failures` in `src/render/console.rs:165-193` now collects all failed metric kinds
from all gates and then calls `group_uncovered_by_file(&result.changed_metrics, &failed_metrics)`.
`result.changed_metrics` holds global changed files (all gates combined). In a multi-gate scenario
where gate A (covering `a.ts`, `b.ts`) fails lines and gate B (covering `c.rs`, `d.rs`) passes,
the failure section lists uncovered spans from every changed file — including files that belong
only to the passing gate. The old per-scope gate-label header (`[gate-name]`) is also gone, so
there is no gate attribution in the failure section at all.

No existing test exercises multi-gate console failure rendering (the path-scoped-gates CLI tests
check Markdown output only). This regression is untested.

**Required action (TDD):** Add a failing test for multi-gate console minimal failure rendering
that verifies only the failing gate's files appear, then fix `render_failures` to operate on
gate-scoped metrics instead of global `changed_metrics`.

### Finding 2 — Medium: Verbose output loses per-gate file attribution

`render_verbose` (`src/render/console.rs:17-143`) now displays file-level changed metric details
from `result.changed_metrics` (global) in a single block before showing per-gate rule outcomes.
In a multi-gate run, a reader cannot tell which files are covered by which gate; the per-gate
"Gate: {label}" header appears only before rule outcomes, not before the file details.

The old code iterated `scope.metrics` per scope and emitted file details immediately under the
gate label. The new structure severs the file → gate connection.

No multi-gate verbose test exists. This is a testing gap that also masks the scope regression
from Finding 1.

**Required action (TDD):** Add a multi-gate verbose test that verifies file attribution stays
with the correct gate section, then update `render_verbose` accordingly.

### Finding 3 — Medium: Duplicate test helpers violate CODESTYLE "one fact, one place"

`percent_outcome` and `uncovered_outcome` are defined identically in both
`tests/render_console.rs:25-61` and `tests/render_markdown.rs` (same signatures, same body).
CODESTYLE.md principle 2 ("One fact, one place — duplication is debt that compounds silently")
and TESTING.md's guidance that shared helper code should live in a reusable test module both
require extracting these into a shared `tests/helpers/` module or `tests/test_builders.rs`.

**Required action:** Extract the two helpers into a shared test module and import from both
renderer test files.

### Finding 4 — Low: `format_percent` carries a dead `_covered` parameter

`src/render/console.rs:373` declares `fn format_percent(percent: f64, _covered: usize, total: usize)`.
The `_covered` parameter is never read; only `total` and `percent` are used. The `_` prefix
suppresses the compiler warning but does not justify the parameter's existence. Every call site
passes a `covered` value that the function ignores, adding confusion and unnecessary coupling.

CODESTYLE.md principle 4 ("Earn every layer") applies to function parameters too.

**Required action:** Remove the `_covered` parameter and update all call sites.

### Finding 5 — Low: `uncovered_outcome` helper populates percent fields inconsistently

`tests/render_console.rs:58-60` (and the identical copy in `render_markdown.rs`) sets
`observed_percent: 100.0, observed_covered_count: 0, observed_total_count: 0`. These fields are
never read by the renderer for `UncoveredCount` rules, so no test currently fails — but the
state is internally inconsistent: if `total == 0`, `format_percent` returns "N/A", not "100.00%".
The inconsistency is a trap for anyone later adding render coverage of these fields for
`UncoveredCount` rules.

**Required action (addressable with Finding 3):** When extracting the shared helper, use
`observed_covered_count: observed_uncovered_count, observed_total_count: observed_uncovered_count`
(or zero them all and set `observed_percent: 0.0`) so the fields are self-consistent.

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
- [ ] Adheres to the principles of `docs/CODESTYLE.md`. (Findings 3, 4 open)
- [ ] Adheres to the principles of `docs/TESTING.md`. (Findings 1, 2, 3 open)
- [ ] All review findings have been addressed.
