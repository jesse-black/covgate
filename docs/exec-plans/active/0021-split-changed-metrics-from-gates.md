---
description: "ExecPlan for separating gate policy evaluations from changed metric evidence, renaming GateResult to CheckResult, and fixing Markdown/console 0/0 coverage rendering."
---

# Split Changed Metrics From Gate Evaluations

## Goal
- `covgate` result modeling separates gate policy outcomes from changed metric evidence; Markdown lists all configured gates while metric tables remain metric-only, and `0/0` percent observations render as `N/A` in console and Markdown.

## Scope
- In: rename the top-level result to `CheckResult`, split changed metrics out of gate evaluations, render all configured gate rules in Markdown, remove gate columns from metric tables, add `N/A (0/0)` observed output for zero-opportunity percent rules, remove the `--verbose` CLI flag and all verbose console rendering.
- Out: config schema changes, parser metric definition changes, and fake per-gate metric rows for gates with no changed files.

## Architectural Intent

These decisions are binding on all implementation and evaluation work in this plan:

- **`GateEvaluation` is policy-only.** It carries `label`, `rules` (`Vec<RuleOutcome>`), and `passed`. It does not carry metrics. This boundary keeps policy outcomes and metric evidence from re-coupling.
- **`CheckResult` has one source of metric evidence: `changed_metrics`.** This is a flat `Vec<ComputedMetric>` covering all changed files across all gates, irrespective of gate scope. There is no per-gate metric array in the result model.
- **Multi-gate failure rendering uses global metrics.** Failure file lists draw from `changed_metrics`; files from passing gates may appear. Gate attribution belongs only in rule-outcome summary lines (`[gate-name] PASS/FAIL ...`), not in file-level failure lists.
- **No `GateMetricEvidence` struct.** This single-field wrapper does not earn its abstraction (CODESTYLE principle 4). It must not exist in the final model.
- **Console output is compact only.** The `--verbose` flag and `render_verbose` path are removed. Markdown is the format for detailed human inspection. The `Verbosity` enum, `verbose` CLI field, and all verbose helpers (`render_metric_file_details`, `render_changed_metric_totals`, `render_rule_outcomes`) are deleted.

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
- [ ] Remove `GateMetricEvidence` from `src/model.rs` and the `gate_metrics` field from `CheckResult`.
- [ ] Remove `render_failures` global/gate-scoped branch logic from `src/render/console.rs`; `render_failures` uses only `result.changed_metrics`; remove `gate_metrics_for` and `failed_gate_metrics` helpers.
- [ ] Remove `--verbose` flag from `src/cli.rs` and `verbose` field from `Config`; remove `Verbosity` enum from `src/model.rs`; simplify `src/lib.rs::run` to call console render without verbosity.
- [ ] Delete `render_verbose`, `render_metric_file_details`, `render_changed_metric_totals`, `render_rule_outcomes` from `src/render/console.rs`; simplify `pub fn render` to call `render_minimal` directly.
- [ ] Remove tests whose premise conflicts with the global-metrics or compact-only architecture: `minimal_failures_only_show_files_from_failing_gate`, `verbose_file_details_stay_under_their_gate_labels`, `minimal_failures_fall_back_to_global_metrics_without_gate_evidence`, `verbose_falls_back_to_global_metrics_without_gate_evidence`, `renders_console_summary_verbose`, `verbose_renders_zero_total_file_details_as_full_coverage`; remove `multi_gate_line_result` if unused.
- [ ] Update 9 CLI tests in `tests/cli_interface.rs` that pass `--verbose`: remove the flag; replace verbose-only stdout assertions (`"Diff Coverage: PASS/FAIL"`, `"Coverage: X%"`, `"Changed regions: N"`, `"Rule X: PASS/FAIL"` verbose format) with compact output assertions or markdown output assertions; add `--markdown-output` where a test specifically needs to verify coverage counts or percentages that compact output does not expose.
- [ ] Fix `uncovered_outcome` in `tests/helpers/mod.rs` to use `observed_covered_count: 0, observed_total_count: 0, observed_percent: 0.0`.
- [ ] Run focused tests and `cargo xtask validate`.

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
- Review follow-up keeps `GateEvaluation` policy-only and adds `GateMetricEvidence` on `CheckResult` so console rendering can use gate-scoped metric evidence without re-coupling policy outcomes and changed metric tables.
- Shared renderer outcome helpers now live in `tests/helpers/mod.rs`; `uncovered_outcome` uses self-consistent percent counts.
- Review follow-up validation passed: `cargo test --test render_console`, `cargo test --test render_markdown`, `cargo test --test gate`, `cargo test --test cli_interface path_scoped_gates`, `cargo test path_scoped_gates`, and `cargo xtask validate`.
- Final validation required a small clippy cleanup in the already-modified `xtask/src/main.rs`; `cargo xtask validate` now passes.
- Architectural pass (2026-05-10): `GateEvaluation` stays policy-only; `gate_metrics`/`GateMetricEvidence` are removed; `changed_metrics` is the single source of metric evidence; multi-gate failure output uses global metrics — gate attribution appears only in rule summaries. Findings 6, 7, and 8 are addressed by this simplification; tests added in Finding 1 and 2 fixes conflict with this architecture and must be removed.
- Verbose removal (2026-05-10): `--verbose` flag and `render_verbose` path are deleted. Markdown is the human inspection format; compact console is the CI signal. 9 CLI tests in `tests/cli_interface.rs` use `--verbose` and check verbose-only strings; they require updating. Verbose unit tests (`renders_console_summary_verbose`, `verbose_renders_zero_total_file_details_as_full_coverage`) are removed.

## Review

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

### Generator Response
- [x] Finding 3 addressed by extracting shared renderer outcome helpers to `tests/helpers/mod.rs`.
- [x] Finding 4 addressed by removing the unused `covered` parameter from console `format_percent`.
- [x] Finding 5 addressed by making the shared `uncovered_outcome` helper internally consistent.

### Finding 6 — Low: `GateMetricEvidence` is a single-field wrapper that does not earn its abstraction

`GateMetricEvidence { pub metrics: Vec<ComputedMetric> }` has exactly one field. Every call site either
constructs it with `GateMetricEvidence { metrics }` or immediately extracts `.metrics`:

- `gate_metrics_for`: `.get(index).map(|evidence| evidence.metrics.as_slice()).unwrap_or(...)`
- `failed_gate_metrics`: yields `(evidence, failed_metrics)`, caller then passes `&evidence.metrics`
- `src/lib.rs:84`: `gate_metrics.push(GateMetricEvidence { metrics })`

CODESTYLE principle 4 and the rule "NEVER introduce a `Foo { only_field: T }` struct that callers
immediately destructure — return `T` directly" apply here. `CheckResult.gate_metrics` should be
`Vec<Vec<ComputedMetric>>`, or the field should be eliminated as part of resolving Finding 8.

**Required action:** Remove `GateMetricEvidence` and inline the `Vec<ComputedMetric>` directly in
`CheckResult.gate_metrics`, updating all construction and access sites.

### Finding 7 — Low: `uncovered_outcome` helper's percent fields are still internally inconsistent

Finding 5's fix changed from `{percent: 100.0, covered: 0, total: 0}` to
`{percent: 0.0, covered: N, total: N}` (where N = `observed_uncovered_count`). When `N > 0`,
`covered == total == N` but `percent == 0.0`, which is inconsistent: equal covered and total implies
100% coverage, not 0%. The original trap is gone but a new one was introduced — any future code
reading these fields for `UncoveredCount` rules would get a self-contradictory state.

The consistent fix is `{covered: 0, total: 0, percent: 0.0}`, making `format_percent(0.0, 0)` return
"N/A" — correctly encoding "percent is not applicable for UncoveredCount rules."

**Required action:** In `tests/helpers/mod.rs`, change `uncovered_outcome` to use
`observed_covered_count: 0, observed_total_count: 0, observed_percent: 0.0`.

### Finding 8 — Medium: Parallel `gates`/`gate_metrics` vectors allow an invalid intermediate state

`CheckResult` holds `gates: Vec<GateEvaluation>` and `gate_metrics: Vec<GateMetricEvidence>` as
parallel arrays with an unenforced invariant: either `gate_metrics.is_empty()` (global fallback)
or `gate_metrics.len() == gates.len()`. Nothing in the type enforces this:

- `gate_metrics_for` silently falls back to `changed_metrics` when an index is missing.
- `failed_gate_metrics` uses `zip`, which silently truncates when lengths differ.

A caller populating `gate_metrics` for fewer gates than `gates.len()` would produce incorrect failure
output with no compile-time or runtime error. CODESTYLE principle 3 requires making invalid states
unrepresentable.

**Required action (TDD):** Add a failing test that documents the expected behavior, then redesign
`CheckResult` to make the invariant unrepresentable — for example, by placing
`Option<Vec<ComputedMetric>>` on `GateEvaluation` directly (gate-scoped metrics alongside the
evaluation that produced them) rather than in a parallel array. This also resolves Finding 6.

## Definition of Done

### Planner
- [x] Plan is consistent, up to date, decision-complete, and ready to hand off.

### Generator
- [x] Goal achieved: gate policy outcomes and changed metric evidence are separate in the result model and output.
- [ ] All planned steps are complete.
- [ ] All validation commands pass.
- [ ] Handed off to an independent reviewer (MUST use the `evaluator-execplan` skill via a subagent or separate agent, not the generator agent).

### Evaluator
- [ ] Standard review posture applied.
- [ ] Adheres to the principles of `docs/CODESTYLE.md`.
- [ ] Adheres to the principles of `docs/TESTING.md`.
- [ ] All review findings have been addressed.
