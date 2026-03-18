# LLVM function normalization in `covgate`

This document records the LLVM-specific function discrepancy uncovered during the summary parity investigation, why it happened, and how the current fix works.

## Big picture

This work is about validating `covgate`'s own calculations, not teaching `covgate` to print the same summary numbers as LLVM by passing native summary data through unchanged.

That distinction matters because summary parity alone does not prove diff coverage is correct. If `covgate` cannot derive the same underlying opportunities and covered-state from native data, matching rendered totals would create false confidence.

The function fix documented here is acceptable because it improves `covgate`'s own normalization logic. It does **not** rely on LLVM summary pass-through.

## Why the discrepancy happened

`covgate` originally deduplicated LLVM functions by source span:

- `(path, start_line, end_line)`

That works for many fixtures, but it undercounts real Rust LLVM exports.

In the failing repro, LLVM emitted multiple raw function records that shared the same source span but represented different symbol identities. Those Rust symbols were LLVM-exported mangled names, and crate-hash disambiguators were part of that mangled form.

That created a mismatch:

- raw LLVM records were not all unique by span
- native LLVM totals still counted more callable records than `covgate`
- `covgate` collapsed too aggressively and reported one fewer function than LLVM

On the real repro fixture, one representative file had:

- 14 raw function records
- 7 unique source spans
- 8 native LLVM functions

So span-only identity was too coarse, but treating every raw record as distinct would have overcounted.

For `src/metrics.rs`, the raw LLVM function record names were:

1. `_RNCNCNvNtCs6ZlX2b1lC0o_7covgate7metrics22compute_changed_metric00B7_`
2. `_RNCNCNvNtCs6ZlX2b1lC0o_7covgate7metrics22compute_changed_metrics0_00B7_`
3. `_RNCNvNtCs6ZlX2b1lC0o_7covgate7metrics22compute_changed_metric0B5_`
4. `_RNCNvNtCs6ZlX2b1lC0o_7covgate7metrics22compute_changed_metrics0_0B5_`
5. `_RNCNvNtCs6ZlX2b1lC0o_7covgate7metrics22compute_changed_metrics_0B5_`
6. `_RNvNtCs6ZlX2b1lC0o_7covgate7metrics22compute_changed_metric`
7. `_RNvNtCsiqc4wHYDJq1_7covgate7metrics22compute_changed_metric`
8. `_RNvNtNtCsiqc4wHYDJq1_7covgate7metrics5testss_30computes_changed_region_metric`
9. `_RNvNtNtCsiqc4wHYDJq1_7covgate7metrics5testss_54metric_with_only_zero_totals_is_treated_as_unavailable`
10. `_RNCNCNvNtCsiqc4wHYDJq1_7covgate7metrics22compute_changed_metric00B7_`
11. `_RNCNCNvNtCsiqc4wHYDJq1_7covgate7metrics22compute_changed_metrics0_00B7_`
12. `_RNCNvNtCsiqc4wHYDJq1_7covgate7metrics22compute_changed_metric0B5_`
13. `_RNCNvNtCsiqc4wHYDJq1_7covgate7metrics22compute_changed_metrics0_0B5_`
14. `_RNCNvNtCsiqc4wHYDJq1_7covgate7metrics22compute_changed_metrics_0B5_`

If `covgate` collapses those records by source span alone, they reduce to 7 span groups:

1. `5..72`
   `_RNvNtCs6ZlX2b1lC0o_7covgate7metrics22compute_changed_metric`
   `_RNvNtCsiqc4wHYDJq1_7covgate7metrics22compute_changed_metric`
2. `13..13`
   `_RNCNCNvNtCs6ZlX2b1lC0o_7covgate7metrics22compute_changed_metric00B7_`
   `_RNCNvNtCs6ZlX2b1lC0o_7covgate7metrics22compute_changed_metric0B5_`
   `_RNCNCNvNtCsiqc4wHYDJq1_7covgate7metrics22compute_changed_metric00B7_`
   `_RNCNvNtCsiqc4wHYDJq1_7covgate7metrics22compute_changed_metric0B5_`
3. `14..19`
   `_RNCNvNtCs6ZlX2b1lC0o_7covgate7metrics22compute_changed_metrics_0B5_`
   `_RNCNvNtCsiqc4wHYDJq1_7covgate7metrics22compute_changed_metrics_0B5_`
4. `31..37`
   `_RNCNvNtCs6ZlX2b1lC0o_7covgate7metrics22compute_changed_metrics0_0B5_`
   `_RNCNvNtCsiqc4wHYDJq1_7covgate7metrics22compute_changed_metrics0_0B5_`
5. `36..36`
   `_RNCNCNvNtCs6ZlX2b1lC0o_7covgate7metrics22compute_changed_metrics0_00B7_`
   `_RNCNCNvNtCsiqc4wHYDJq1_7covgate7metrics22compute_changed_metrics0_00B7_`
6. `86..135`
   `_RNvNtNtCsiqc4wHYDJq1_7covgate7metrics5testss_30computes_changed_region_metric`
7. `138..160`
   `_RNvNtNtCsiqc4wHYDJq1_7covgate7metrics5testss_54metric_with_only_zero_totals_is_treated_as_unavailable`

After demangling those Rust symbols with hash-stripping formatting, the native LLVM-aligned function identities become:

1. `covgate::metrics::compute_changed_metric::{closure#0}::{closure#0}`
2. `covgate::metrics::compute_changed_metrics::{closure#0}::{closure#0}`
3. `covgate::metrics::compute_changed_metric::{closure#0}`
4. `covgate::metrics::compute_changed_metrics::{closure#0}`
5. `covgate::metrics::compute_changed_metrics`
6. `covgate::metrics::compute_changed_metric`
7. `covgate::metrics::tests::computes_changed_region_metric`
8. `covgate::metrics::tests::metric_with_only_zero_totals_is_treated_as_unavailable`

## Earlier function debugging that still applies

Before the normalization fix below, two earlier function issues had already been corrected:

- covered-state uses `function.count > 0` or any executed function-region count, so functions with zero top-level count but executed regions are still considered covered
- path matching prefers the longest valid suffix when diff paths and LLVM paths disagree in prefix shape

Those fixes remain valid. They were not the cause of the final one-function summary drift.

## How the fix works

`covgate` now keeps the native LLVM function name when parsing each function record and uses a normalized name as the primary deduplication key when a name is available.

Normalization now uses `rustc-demangle` instead of a hand-rolled mangled-name rewrite:

- input shape is the LLVM-exported Rust mangled symbol
- demangling converts it into a stable Rust path-style name
- alternate demangle formatting strips the crate hash, so hash-only variants collapse together
- non-Rust or undecodable names fall back to the original string

In practice:

1. Parse the optional LLVM function name into `LlvmFunction.name`.
2. Demangle Rust LLVM symbols with `rustc-demangle` and strip the crate hash via alternate formatting.
3. Deduplicate functions by:
   - normalized name + span when a function name exists
   - span only when no name exists
4. Keep the source span on the emitted opportunity so diff filtering continues to work through the shared metric engine.

This lands in `covgate`'s calculation path, not in summary rendering.

## Why this fixes the function mismatch

The normalized-name key preserves distinctions that span-only deduplication erased, while still merging raw LLVM records that differ only by crate hash.

That gives `covgate` a function identity closer to LLVM's own semantics:

- different callable records that happen to share a span can remain distinct
- hash-only variants of the same callable record still collapse together

With that change, the real LLVM repro no longer has a function discrepancy.

## What this does not prove

This fix resolves only the function portion of the LLVM parity bug.

The real repro still has region and line mismatches. That means the broader investigation remains open: we still do not fully understand LLVM's line/region opportunity semantics well enough to claim `covgate`'s derived calculations are correct end-to-end.

So the current state is:

- function calculation is improved and matches the native repro
- region and line calculation are still under investigation
- summary pass-through is still explicitly rejected as a false fix

## Validation

The function fix is covered by:

- parser regression test:
  - `keeps_rust_functions_with_different_crate_hashes_as_one_name_based_record`
- parser suite:
  - `cargo test llvm_json -- --nocapture`

The real LLVM parity repro now fails only on non-function metrics:

- region: native `3285/3408`, `covgate` `3252/3355`
- line: native `2890/2957`, `covgate` `2865/2910`

## Source pointers

Implementation lives in:

- `src/coverage/llvm_json.rs`

Related investigation artifacts:

- `tests/llvm_real_parity.rs`
- `tests/fixtures/llvm-real/covgate-self-full.json`
- `docs/exec-plans/completed/covgate-llvm-summary-parity.md`
