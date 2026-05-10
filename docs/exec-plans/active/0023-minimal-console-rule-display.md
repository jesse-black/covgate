---
description: "ExecPlan for making minimal console rule summaries unaligned and rendering uncovered-count gates with uncovered-count observations."
---

# Minimal Console Rule Display

## Goal
- Minimal console output uses compact, unaligned rule summary lines, and uncovered-count gates display their observed uncovered count instead of percent metric evidence.

## Scope
- In: normal/minimal console rule summary formatting, scoped gate labels in minimal console output, uncovered-count gate observed-value display, focused renderer tests, and CLI output expectations affected by the new minimal format.
- Out: verbose console table-style output, Markdown output, gate semantics, parser metric definitions, config schema, and threshold defaults.

## Relevant Areas
- `docs/design-docs/token-efficient-output.md` — source design decision for unaligned minimal output and uncovered-count display.
- `src/render/console.rs` — minimal console rule summary formatting and failure-focused output.
- `tests/render_console.rs` — renderer-level output shape and regression coverage.
- `tests/cli_interface.rs`, `tests/cli_metrics.rs` — end-to-end expectations that may assert minimal console output.
- `docs/exec-plans/active/0021-split-changed-metrics-from-gates.md` — nearby model split that should remain compatible with rule-observation rendering.

## Open Questions
- None yet.

## Steps
- [ ] In `tests/render_console.rs`: replace `aligns_comparators_vertically` with a failing test named `percent_rule_renders_compact_unaligned` that asserts a `Percent` rule outcome produces a single-line string matching `"PASS Regions: 100.00% (3/3) ≥ 90.00%"` exactly (no leading/trailing spaces beyond the verdict, no double-spaces between fields, no fixed-width padding).
- [ ] In `tests/render_console.rs`: add a failing test named `uncovered_count_rule_renders_observed_count_not_percent` that asserts a `UncoveredCount` outcome with `observed_uncovered_count = 2` produces a line containing `"2 uncovered"` and the comparator `"≤ 5"` (the budget threshold), and does NOT contain a percentage or covered/total count.
- [ ] In `tests/render_console.rs`: add a failing test named `scoped_label_renders_compact_without_padding` that asserts a scoped `[frontend] PASS Lines: 100.00% (5/5) ≥ 90.00%` line — label prefix followed immediately by the compact rule line, no extra alignment padding between label and verdict.
- [ ] In `tests/cli_metrics.rs`: update every `stdout.contains("PASS  MetricName:")` and `stdout.contains("FAIL  MetricName:")` double-space assertion to single-space (e.g., `"PASS Regions:"`, `"FAIL Lines:"`). Keep all exit-code checks, comparator assertions (`"≱"`, `"≰"`, `"≤"`, `"≥"` with threshold values), and the `stderr` metric-unavailable check — these prove CLI flag-to-rule wiring and pass/fail semantics that renderer tests cannot exercise end-to-end.
- [ ] In `tests/cli_interface.rs`: update every `stdout.contains("PASS  MetricName:")`, `stdout.contains("FAIL  MetricName:")`, `stdout.contains("[label] PASS  MetricName:")`, and `stdout.contains("[label] FAIL  MetricName:")` double-space assertion to single-space. The 19 affected assertions span lines 566, 608, 612, 615, 617, 644, 647, 747, 749, 837, 870, 902, 937, 973, 975, 1012, 1063, and 1093.
- [ ] Update `src/render/console.rs` so `render_rule_summary` builds compact record-like lines from the `RuleOutcome` family: drop the `format!("{}  {:<11} {:>7} {:>13}{:<11}", ...)` padded table format; for `Percent` rules emit `"{status} {metric_label}: {observed} ({covered}/{total}) {comparator} {threshold:.2}%"`; for `UncoveredCount` rules emit `"{status} {metric_label}: {observed_uncovered_count} uncovered {comparator} {maximum_count}"`.
- [ ] Run focused tests during the edit loop, then run full validation because this changes Rust output behavior.

## Validation
- `cargo test --test render_console`
- `cargo test --test cli_interface`
- `cargo test --test cli_metrics`
- `cargo xtask validate`

## Discoveries
- The token-efficient output design now explicitly rejects fixed-width table alignment in minimal console output.
- Uncovered-count gates should display observed uncovered count directly; percent evidence belongs to percent rules or changed metric evidence, not uncovered-count rule summaries.
- Plan 21's former CLI metric-test finding now belongs here: excessive repeated CLI output assertions create churn. Prefer fewer, sharper CLI tests plus renderer-level formatting tests.
- Because implementation may happen on a fresh branch where unchanged files can lose coverage, do not compensate by adding low-value tests only to satisfy line coverage. Rely on the existing 96% global coverage gate enforced by `cargo xtask llvm-cov` and `cargo xtask validate`.
- Audit of `tests/cli_metrics.rs` (18 tests, 21 stdout/stderr assertions) and `tests/cli_interface.rs` (14+ stdout assertions containing PASS/FAIL metric labels): all existing double-space `"PASS  MetricName:"` / `"FAIL  MetricName:"` patterns come from the `format!("{}  {:<11} {:>7} {:>13}{:<11}", ...)` fixed-width table in `render_rule_summary` (lines 95–104 of `src/render/console.rs`). Every such pattern must become single-space after the refactor; none of these assertions can be deleted because they each prove distinct flag-to-rule wiring.
- The `aligns_comparators_vertically` test in `tests/render_console.rs` directly asserts the behavior being removed; it must be replaced, not updated.
- `UncoveredCount` outcome in the current renderer produces an empty `counts` string but still pads the `observed` field as a right-aligned 7-char field inside a 13-char counts column — the design calls for `"<N> uncovered"` in a flat string with no numeric covered/total counts, so the Percent and UncoveredCount branches need fully separate output templates.

## Review
- None yet.

## Definition of Done

### Planner
- [x] Plan is consistent, up to date, decision-complete, and ready to hand off.

### Generator
- [ ] Goal achieved: minimal console rule summaries are compact and uncovered-count gates display uncovered-count observations.
- [ ] All planned steps are complete.
- [ ] All validation commands pass.
- [ ] Handed off to an independent reviewer (MUST use the `evaluator-execplan` skill via a subagent or separate agent, not the generator agent).

### Evaluator
- [ ] Standard review posture applied.
- [ ] Adheres to the principles of `docs/CODESTYLE.md`.
- [ ] Adheres to the principles of `docs/TESTING.md`.
- [ ] All review findings have been addressed.
