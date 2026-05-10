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
- `tests/cli_interface.rs` — end-to-end expectations that assert minimal console output.

## Open Questions
- None yet.

## Steps
- [ ] In `tests/render_console.rs`: replace `aligns_comparators_vertically` with a failing test named `percent_rule_renders_compact_unaligned` that asserts a `Percent` rule outcome produces a single-line string matching `"PASS Regions: 100.00% (3/3) ≥ 90.00%"` exactly (no leading/trailing spaces beyond the verdict, no double-spaces between fields, no fixed-width padding).
- [ ] In `tests/render_console.rs`: add a failing test named `uncovered_count_rule_renders_observed_count_not_percent` that asserts a `UncoveredCount` outcome with `observed_uncovered_count = 2` produces a line containing `"2 uncovered"` and the comparator `"≤ 5"` (the budget threshold), and does NOT contain a percentage or covered/total count.
- [ ] In `tests/render_console.rs`: add a failing test named `scoped_label_renders_compact_without_padding` that asserts a scoped `[frontend] PASS Lines: 100.00% (5/5) ≥ 90.00%` line — label prefix followed immediately by the compact rule line, no extra alignment padding between label and verdict.
- [ ] In `tests/cli_interface.rs`: update the 13 double-space `"PASS  …"` / `"FAIL  …"` / `"[label] PASS  …"` / `"[label] FAIL  …"` assertions to single-space at lines 573, 615, 619, 622, 624, 651, 654, 818, 851, 883, 918, 973, and 1004.
- [ ] Update `src/render/console.rs` so `render_rule_summary` builds compact record-like lines from the `RuleOutcome` family: drop the `format!("{}  {:<11} {:>7} {:>13}{:<11}", ...)` padded table format; for `Percent` rules emit `"{status} {metric_label}: {observed} ({covered}/{total}) {comparator} {threshold:.2}%"`; for `UncoveredCount` rules emit `"{status} {metric_label}: {observed_uncovered_count} uncovered {comparator} {maximum_count}"`.

## Validation
- `cargo test --test render_console`
- `cargo test --test cli_interface`
- `cargo xtask validate`

## Discoveries
- The `aligns_comparators_vertically` test in `tests/render_console.rs` asserts the behavior being removed; it must be replaced, not updated.
- All double-space `"PASS  …"` / `"FAIL  …"` patterns in `tests/cli_interface.rs` come from the `format!("{}  {:<11} {:>7} {:>13}{:<11}", ...)` padded table in `render_rule_summary`; every assertion must become single-space after the refactor.
- `UncoveredCount` outcome currently pads `observed` as a right-aligned 7-char field inside a 13-char counts column; the new format needs `"<N> uncovered"` in a flat string — Percent and UncoveredCount branches need fully separate output templates.
- Do not add low-value tests to recover coverage lost on unchanged files; rely on the global 96% gate enforced by `cargo xtask validate`.

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
