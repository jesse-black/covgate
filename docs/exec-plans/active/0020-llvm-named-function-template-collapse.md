---
description: "Implement LLVM named-function generic/template instantiation collapsing according to docs/design-docs/named-function-metric.md."
---

# LLVM Named Function Template Collapse

## Goal
- LLVM-backed `named-functions` treats generic/template instantiations as one authored function: covered if any instantiation is covered, uncovered only if all reported instantiations are uncovered.
- Raw `functions` behavior remains unchanged.

## Scope
- In: LLVM JSON parser and metric computation changes needed to collapse named generic/template instantiations for totals and changed-diff gates.
- In: Regression tests proving generic instantiations collapse without excluding user-authored generics.
- Out: Istanbul and Coverlet generic/template collapsing.
- Out: C/C++ parser implementation.
- Out: Config schema, CLI flag, gate default, fixture regeneration, or unrelated named-function policy changes.

## Relevant Areas
- `docs/design-docs/named-function-metric.md` — source design for the classification hierarchy and format-specific matrix.
- `src/coverage/llvm_json.rs` — demangles LLVM function names, builds function opportunities, and computes per-file `functions` / `named-functions` totals.
- `src/model.rs` — owns coverage opportunity shape; may need a stable named-function identity in addition to `is_named_function`.
- `src/metrics.rs` — computes changed metrics from opportunities; `NamedFunction` currently filters raw function opportunities by `is_named_function`.
- `tests/coverage_parse.rs` and/or inline tests in `src/coverage/llvm_json.rs` — parser behavior and private classification helpers.
- `tests/cli_metrics.rs` — end-to-end gate behavior if a copied-fixture or existing fixture can express the scenario without fixture-specific conditionals.

## Open Questions
- None yet.

## Steps
- [ ] Start with a failing regression test that shows two LLVM function records for the same authored generic function currently count as two `named-functions`.
- [ ] Add a focused helper in `src/coverage/llvm_json.rs` that derives a named-function identity from a normalized LLVM name:
  - returns `None` for span-only records and `{...}` synthetic/anonymous path segments.
  - strips generic/template instantiation syntax such as Rust/C++ demangled `<...>` from the authored function identity.
  - does not match framework names such as `serde`, `toml`, or `clap`.
- [ ] Preserve raw function accounting:
  - keep one `Function` opportunity per backend function record.
  - keep `MetricKind::Function` totals based on backend function record count.
- [ ] Add a representation for collapsed named-function identity that `metrics.rs` can group by for `MetricKind::NamedFunction`.
  - Prefer extending `CoverageOpportunity` with an optional stable named-function identity over parallel side maps.
  - Ensure ordinary named functions also get an identity so all named-function diff metric behavior is computed through one path.
- [ ] Update `src/coverage/llvm_json.rs` named-function totals to collapse by `(path, named_function_identity)`.
  - Covered count increments once when any record in the group is covered.
  - Total count increments once per group.
- [ ] Update `src/metrics.rs` so changed `NamedFunction` metrics group changed function opportunities by `(path, named_function_identity)`.
  - A group is changed if any grouped opportunity overlaps the diff.
  - A group is covered if any grouped changed opportunity is covered.
  - If a grouped named function is uncovered, report one representative uncovered opportunity with the authored identity's source span.
- [ ] Keep synthetic/anonymous exclusions before generic/template collapsing.
- [ ] Add tests for:
  - generic instantiations collapse to one named-function total.
  - at least one covered instantiation makes the collapsed named function covered.
  - all uncovered instantiations make the collapsed named function uncovered.
  - closure/async `{...}` entries remain excluded.
  - raw `functions` totals still count each backend function record.
- [ ] Update docs only if implementation discovers a necessary correction to `docs/design-docs/named-function-metric.md`.

## Validation
- `cargo fmt --check`
- `cargo test llvm_json::tests::identifies_named_functions_correctly`
- `cargo test llvm_json::tests::verifies_named_function_totals`
- `cargo test --test coverage_parse named_function`
- `cargo test --test cli_metrics named_function`
- `cargo xtask validate`

## Discoveries
- The existing LLVM parser uses `FunctionKey::NormalizedName { normalized_name, start_line, start_col, end_line, end_col }` for raw function deduplication, and `is_llvm_function_named` only excludes span-only records and `{...}` segments.
- `src/metrics.rs` currently computes `NamedFunction` changed metrics by filtering raw function opportunities with `is_named_function`, so generic/template collapse must be represented in opportunities or metric grouping, not only in parser totals.
- `functions` and `named-functions` intentionally have different semantics: `functions` remains backend-record accounting, while `named-functions` is review-relevant authored-function accounting.

## Review
- [ ] None yet.

## Definition of Done

### Planner
- [x] Plan is consistent, up to date, decision-complete, and ready to hand off.

### Generator
- [ ] Goal achieved: LLVM-backed `named-functions` collapses generic/template instantiations without changing raw `functions`.
- [ ] All planned steps are complete.
- [ ] All validation commands pass.
- [ ] Handed off to an independent reviewer (MUST use the `evaluator-execplan` skill via a subagent or separate agent, not the generator agent).

### Evaluator
- [ ] Standard review posture applied.
- [ ] Adheres to the principles of `docs/CODESTYLE.md`.
- [ ] Adheres to the principles of `docs/TESTING.md`.
- [ ] All review findings have been addressed.
