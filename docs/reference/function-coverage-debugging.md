# Function coverage debugging notes (LLVM / `covgate`)

This note documents why `covgate` reported unexpectedly high uncovered-function counts during dogfooding even when changed-region coverage was very high, and what was changed to fix it.

## Symptom seen in dogfooding

Observed behavior during PR validation was:

- Changed region coverage near 100%.
- Changed function coverage materially lower (for example a few uncovered functions in `src/coverage/llvm_json.rs` and `src/model.rs`).

At first glance this seemed impossible because regions are more granular than functions. In practice, the mismatch came from how LLVM function records were normalized before gating.

## What was going on

Two parser-level issues in LLVM function normalization were contributing noise to function totals.

1. Covered-state source mismatch

`llvm-cov` function records can carry both:

- a top-level function `count`, and
- per-region execution counts in each function region tuple.

Some callable records were effectively executed (non-zero region execution count) while function `count` was still zero in the raw record shape we consumed. Using only top-level `count` could misclassify those as uncovered.

Fix applied:

- Function covered-state is now computed as covered when either:
  - `function.count > 0`, or
  - any function region has `execution_count > 0`.

2. Duplicate callable entries at the same source span

LLVM exports can contain multiple callable records that normalize to the same file + line span. Treating each record independently can double-count one span (for example one uncovered variant + one covered variant), which inflates uncovered-function counts.

Fix applied:

- During parsing, callable records are deduplicated by `(path, start_line, end_line)`.
- Covered-state for duplicates is merged with OR semantics (`covered` if any variant is covered).

3. Ambiguous path suffix fallback (earlier related fix)

When mapping function filenames to known file entries, a naive first-suffix match could pick the wrong file if paths shared suffixes.

Fix already applied before this note:

- Suffix fallback now chooses the longest valid suffix at a path-component boundary.

## Validation that this resolved the issue

We validated in three layers:

1. Regression tests (TDD)

- Added test proving region execution count marks function as covered even when top-level function `count` is zero.
- Added test proving duplicate function spans are merged and reported once.
- Added test proving longest-suffix path resolution.

2. Repository test suite

- `cargo test` passes with the new parser logic and regressions.

3. Coverage-tool sanity check

- `cargo llvm-cov --summary-only` reports TOTAL functions and missed functions.
- `cargo llvm-cov --json` totals (`data[].totals.functions`) were summed and matched the same covered/missed counts as the summary output in the same run.

This confirms that post-fix function totals seen by our quick validation are consistent with LLVM's own summary view.

## Practical takeaway

The low function coverage was not caused by diff intersection rules in the gate engine. It was caused by parser normalization edge cases in LLVM callable records (covered-state derivation and duplicate span handling). After normalization fixes, function counts are materially more stable and better aligned with region-based expectations.

## Cross-tool policy decision: include anonymous/unnamed callable units

As of the Istanbul + Coverlet expansion, `covgate` intentionally keeps function-threshold semantics aligned with upstream tool outputs instead of trying to infer a narrower “named functions only” model.

Rationale:

- `cargo llvm-cov` function thresholds count LLVM function records, which can include callable units that are not source-level named functions.
- Coverlet method thresholds are based on reported method records; they are not restricted to only user-authored named methods in every compiler/toolchain scenario.
- Istanbul function metrics are based on `fnMap` entries and can include anonymous/inline callable units (for example arrow functions and callbacks).

Decision:

- `covgate` function gates (`--fail-under-functions`, `--fail-uncovered-functions`) should continue to count the callable opportunities reported by each native coverage format adapter, including anonymous/unnamed units where present.
- We prefer parity and predictability versus ecosystem-native thresholds over tool-specific filtering heuristics that might hide real coverage-tool behavior.

