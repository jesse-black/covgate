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
- [x] Start with a failing regression test that shows two LLVM function records for the same authored generic function currently count as two `named-functions`.
- [x] Add a focused helper in `src/coverage/llvm_json.rs` that derives a named-function identity from a normalized LLVM name:
  - returns `None` for span-only records and `{...}` synthetic/anonymous path segments.
  - strips generic/template instantiation syntax such as Rust/C++ demangled `<...>` from the authored function identity.
  - does not match framework names such as `serde`, `toml`, or `clap`.
- [x] Preserve raw function accounting:
  - keep one `Function` opportunity per backend function record.
  - keep `MetricKind::Function` totals based on backend function record count.
- [x] Add a representation for collapsed named-function identity that `metrics.rs` can group by for `MetricKind::NamedFunction`.
  - Prefer extending `CoverageOpportunity` with an optional stable named-function identity over parallel side maps.
  - Ensure ordinary named functions also get an identity so all named-function diff metric behavior is computed through one path.
- [x] Update `src/coverage/llvm_json.rs` named-function totals to collapse by `(path, named_function_identity)`.
  - Covered count increments once when any record in the group is covered.
  - Total count increments once per group.
- [x] Update `src/metrics.rs` so changed `NamedFunction` metrics group changed function opportunities by `(path, named_function_identity)`.
  - A group is changed if any grouped opportunity overlaps the diff.
  - A group is covered if any grouped changed opportunity is covered.
  - If a grouped named function is uncovered, report one representative uncovered opportunity with the authored identity's source span.
- [x] Keep synthetic/anonymous exclusions before generic/template collapsing.
- [x] Add tests for:
  - generic instantiations collapse to one named-function total.
  - at least one covered instantiation makes the collapsed named function covered.
  - all uncovered instantiations make the collapsed named function uncovered.
  - closure/async `{...}` entries remain excluded.
  - raw `functions` totals still count each backend function record.
- [x] Update docs only if implementation discovers a necessary correction to `docs/design-docs/named-function-metric.md`.

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
- Added `llvm_json::tests::collapses_named_function_template_instantiations`; it fails on the current behavior with named total `2` instead of expected collapsed total `1`.
- Non-LLVM parsers now populate `named_function_identity` without changing their named-function totals; Istanbul uses function ids to avoid out-of-scope same-name collapsing.
- Validation passed: `cargo fmt --check`, `cargo test llvm_json::tests::identifies_named_functions_correctly`, `cargo test llvm_json::tests::verifies_named_function_totals`, `cargo test --test coverage_parse named_function`, `cargo test --test cli_metrics named_function`, and `cargo xtask validate`.
- Review fixes validated with `cargo test --test coverage_parse llvm_named_function`, `cargo test llvm_json::tests::derives_named_function_identity_without_template_arguments`, `cargo test llvm_json::tests::verifies_named_function_totals`, `cargo test changed_named_function_metric`, and `cargo xtask validate`.

## Review
- [x] Finding P1: `src/coverage/llvm_json.rs` strips every top-level `<...>` group from LLVM names before deciding the named-function identity. That also rewrites Rust qualified trait impl names such as `<crate::Type as core::fmt::Debug>::fmt` to `::fmt`, so distinct authored trait methods in the same file can collapse into one `named-functions` opportunity even though they are ordinary source functions, not generic instantiations. Add a failing LLVM parser regression for qualified trait impl names and narrow the stripping rule to generic/template instantiation syntax only.
- [x] Finding P2: The new parser-behavior regression tests `collapses_named_function_template_instantiations` and `collapsed_template_instantiations_are_uncovered_when_all_variants_are_uncovered` live inline in `src/coverage/llvm_json.rs`, but they exercise `parse_with_repo_root` behavior and do not need private helper access. `docs/TESTING.md` says public parser behavior belongs in `tests/coverage_parse.rs`; keep only private helper classification tests inline.
- [x] Evaluator re-review: prior P1/P2 findings are addressed; no new findings against this ExecPlan, `docs/CODESTYLE.md`, or `docs/TESTING.md`.

## Definition of Done

### Planner
- [x] Plan is consistent, up to date, decision-complete, and ready to hand off.

### Generator
- [x] Goal achieved: LLVM-backed `named-functions` collapses generic/template instantiations without changing raw `functions`.
- [x] All planned steps are complete.
- [x] All validation commands pass.
- [x] Handed off to an independent reviewer (MUST use the `evaluator-execplan` skill via a subagent or separate agent, not the generator agent).

### Evaluator
- [x] Standard review posture applied.
- [x] Adheres to the principles of `docs/CODESTYLE.md`.
- [x] Adheres to the principles of `docs/TESTING.md`.
- [x] All review findings have been addressed.
