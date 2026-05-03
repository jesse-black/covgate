---
description: "ExecPlan for adding the named-functions metric, which filters function coverage to exclude closures, anonymous functions, and compiler-generated callables; read when implementing or continuing this feature."
---

# Named function metric

## Goal
- `covgate` exposes a `named-functions` metric and two gate rules (`fail-under-named-functions`, `fail-uncovered-named-functions`) that apply the same diff-gating logic as the existing `functions` metric but restricted to functions with genuine source-level names.

## Scope
- In: `MetricKind::NamedFunction`, `is_named_function` field on `CoverageOpportunity`, classification helpers in all three parsers, CLI flags, TOML config keys, metric engine filter, unit and integration tests.
- Out: changes to existing `functions` metric behavior, new fixture repos, render changes (render is already generic over MetricKind).

## Relevant Areas
- `src/model.rs` — MetricKind enum, CoverageOpportunity struct
- `src/coverage/llvm_json.rs` — LLVM parser; demangled names already available in FunctionKey
- `src/coverage/istanbul_json.rs` — Istanbul parser; `name` field in fnMap not yet deserialized
- `src/coverage/coverlet_json.rs` — Coverlet parser; method key discarded in inner loop
- `src/metrics.rs` — `compute_changed_metric`; needs secondary filter for NamedFunction
- `src/cli.rs` — Args struct; add two new fields
- `src/config.rs` — GateConfig struct and `resolve_rules()`; add two fields and two push calls
- `tests/cli_metrics.rs` — integration tests for gate rules
- `docs/design-docs/named-function-metric.md` — design reference

## Open Questions
- None

## Steps
- [ ] `src/model.rs`: add `NamedFunction` to `MetricKind`; add match arms in `as_str()` (`"named-function"`), `label()` (`"named-functions"`), `to_opportunity_kind()` (`OpportunityKind::Function`); add `is_named_function: Option<bool>` to `CoverageOpportunity`; update all construction sites in parsers to set `is_named_function: None` for non-function opportunities.
- [ ] `src/coverage/llvm_json.rs`: add `is_llvm_function_named(key: &FunctionKey) -> bool` (returns false for `Span` variant; for `NormalizedName` returns true unless any `::` segment is `{…}`); set `is_named_function: Some(is_named)` on emitted function opportunities; accumulate `named_function_totals_by_file` alongside `function_totals_by_file`; insert under `MetricKind::NamedFunction`.
- [ ] `src/coverage/istanbul_json.rs`: add `name: Option<String>` to `IstanbulFunctionMap`; add `is_istanbul_function_named(name: Option<&str>) -> bool` (false for None, `""`, `"<anonymous>"`, or string starting with `"(anonymous"` and ending with `")"`); add `is_named: bool` to private `FunctionRecord`; set `is_named_function`; accumulate named totals.
- [ ] `src/coverage/coverlet_json.rs`: change innermost `methods.values()` to `methods.iter()` to capture key; add `is_coverlet_method_named(key: &str) -> bool` (extracts segment between `"::"` and first `"("`; named if that segment contains neither `<` nor `>`); add `is_named: bool` to private `FunctionRecord`; set `is_named_function`; accumulate named totals.
- [ ] `src/metrics.rs`: in `compute_changed_metric`, add `let named_only = matches!(metric, MetricKind::NamedFunction);` and skip opportunities where `named_only && !opportunity.is_named_function.unwrap_or(false)`.
- [ ] `src/cli.rs`: add `fail_under_named_functions: Option<f64>` and `fail_uncovered_named_functions: Option<usize>` to `Args`, following the existing function-arg pattern.
- [ ] `src/config.rs`: add `fail_under_named_functions: Option<f64>` and `fail_uncovered_named_functions: Option<usize>` to `GateConfig`; add `push_percent_rule` and `push_uncovered_rule` calls with `MetricKind::NamedFunction` in `resolve_rules()`.
- [ ] `src/coverage/llvm_json.rs` tests: add unit test verifying closure-named symbols are excluded from named-function totals and plain functions are included; inline JSON using symbols from existing `demangles_real_repro_rust_symbol_set_into_eight_identities` test.
- [ ] `src/coverage/istanbul_json.rs` tests: add unit test with `fnMap` entries named `"(anonymous_0)"`, `""`, `"<anonymous>"`, and `"compute"`; assert only `"compute"` contributes to named totals.
- [ ] `src/coverage/coverlet_json.rs` tests: add unit test with method keys `"System.Void Demo.MathOps::<Add>b__0_0()"` and `"System.Int32 Demo.MathOps::Add(System.Int32)"`, assert only `Add` is named.
- [ ] `tests/cli_metrics.rs`: add integration tests for `--fail-under-named-functions` and `--fail-uncovered-named-functions` mirroring `function_threshold_fails_when_below_threshold` and `uncovered_function_budget_fails_when_exceeded`; use the same compatible fixture lists (`function_capable_fail_fixtures`, `function_capable_pass_fixtures`).

## Validation
- `cargo check` — use non-exhaustive match errors to verify all MetricKind sites are covered
- `cargo test model`
- `cargo test llvm_json`
- `cargo test istanbul`
- `cargo test coverlet`
- `cargo test metrics`
- `cargo test config`
- `cargo test cli_metrics`
- `cargo test`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo fmt --check`

## Discoveries
- None yet

## Review
- [ ] None yet
