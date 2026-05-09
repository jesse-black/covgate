# Design Doc: Named Function Metric

## Context & Problem Statement

`covgate` currently exposes a `functions` metric that counts all function-level coverage opportunities regardless of whether they are named, anonymous, or compiler-generated. This means closures, lambdas, async state machines, and other synthetic callables inflate the function count and can cause gates to fail for coverage that is impractical to test directly.

The goal is a `named-functions` metric that applies the same diff-gating semantics as `functions` but restricted to functions with genuine source-level names — excluding closures, anonymous function expressions, and compiler-generated synthetic methods.

---

## Goals

- Add a `named-functions` metric to `covgate`.
- Add two gate rules: `fail-under-named-functions` (percent) and `fail-uncovered-named-functions` (count).
- Support the metric across all three currently-supported coverage formats: LLVM JSON (Rust), Coverlet JSON (.NET), and Istanbul JSON (JS/TS).
- Keep the existing `functions` metric unchanged; `named-functions` is additive.

---

## Format Support Analysis

All three formats expose enough information to classify functions as named vs. anonymous. However, each format requires different classification logic.

### LLVM JSON (Rust)

**Where names come from:** `covgate` already demangles LLVM function names via `rustc-demangle`. The demangled output makes anonymous-function classification straightforward:

- Named function: `covgate::metrics::compute_changed_metric`
- Closure: `covgate::metrics::compute_changed_metric::{closure#0}`
- Async fn body: `covgate::metrics::compute_changed_metric::{async_fn#0}`
- Coroutine: `covgate::metrics::compute_changed_metric::{coroutine#0}`

**Classification rule:** A function is **anonymous** if any path segment of its demangled name is surrounded by `{...}`. Named functions have no such segment.

**Current parser state:** Names are parsed and used for deduplication via `FunctionKey::NormalizedName`. The classification would reuse the same demangled string already computed in `normalize_llvm_function_name()`. No new parsing is required.

**LLVM C/C++ and Swift:** C++ lambdas demangle to `{lambda()#N}::operator()` via `cpp_demangle`. The same `{...}` detection rule covers them. Swift closures follow a similar Itanium-derived pattern. Because C/C++ and Swift LLVM support is still in draft ([cpp-llvm-coverage.md](cpp-llvm-coverage.md), [swift-llvm-coverage.md](swift-llvm-coverage.md)), named-function classification for those sub-formats is not a blocker but should use the same rule when those parsers land.

**Verdict: fully supported.**

---

### Istanbul JSON (JS/TS)

**Where names come from:** Istanbul's `fnMap` contains a `name` field for each function entry. The current `IstanbulFunctionMap` struct does not deserialize this field — it only reads `loc`. The field needs to be added.

Example Istanbul fnMap entry:
```json
"0": {
  "name": "compute",
  "loc": { "start": { "line": 1, "column": 0 }, "end": { "line": 3, "column": 1 } }
}
```

Anonymous functions typically appear as:
- Empty string `""` (arrow function with no inferred name)
- `"(anonymous_N)"` (nyc / istanbul-instrumenter-loader pattern)
- `"<anonymous>"` (some V8 contexts)

Named functions are any non-empty name that does not match an anonymous sentinel pattern.

**Classification rule:** A function is **named** if `name` is non-empty, does not match `(anonymous_N)` (where N is digits), and is not `"<anonymous>"`. Arrow functions and function expressions assigned to a variable receive their inferred name from the JavaScript engine, so `const foo = () => {}` produces `"foo"` and is counted as named.

**Instrumenter variance:** The exact sentinel pattern for anonymous functions varies by instrumenter (nyc, c8, Babel). The rule above targets the most common patterns. A fixture from each known instrumenter should anchor the behavior.

**Verdict: supported; requires parser change to deserialize `name` field.**

---

### Coverlet JSON (.NET)

**Where names come from:** Coverlet identifies methods by their full .NET signature, used as the key in the JSON method map. The method name is embedded in the key string.

Examples:
```
System.Int32 Demo.MathOps::Add(System.Int32,System.Int32)          ← named
System.Void Demo.MathOps::<Add>b__0_0()                             ← lambda / anonymous
System.Void Demo.MathOps::<HandleAsync>d__0::MoveNext()             ← async state machine
System.Void Demo.MathOps::<>c::.ctor()                              ← compiler-generated closure class
```

C# compiler-generated methods always include angle brackets (`<` and `>`) in the method name segment — the portion between `::` and `(`.

**Classification rule:** Extract the method name segment (between `::` and the first following `(`). A method is **anonymous** if that segment contains `<` or `>`. Named methods have a plain identifier (letters, digits, underscore only).

**Current parser state:** The Coverlet parser iterates `methods_value` entries but does not have access to the method key string (the loop calls `methods.values()` discarding keys). The loop would need to change to `methods.iter()` to capture keys alongside values.

**Verdict: supported; requires parser change to capture method key.**

---

## Classification Rules Summary

| Format       | Named                                         | Anonymous / Excluded                                                  |
|--------------|-----------------------------------------------|-----------------------------------------------------------------------|
| LLVM (Rust)  | Demangled name has no `{...}` path segment    | Any `{closure#N}`, `{async_fn#N}`, `{coroutine#N}`, etc. segment     |
| LLVM (C/C++) | Same `{...}` rule via `cpp_demangle` output   | `{lambda()#N}::operator()` and similar                               |
| Istanbul     | `name` non-empty, not `(anonymous_N)` / `<anonymous>` | Empty name, `(anonymous_N)`, `<anonymous>`                   |
| Coverlet     | Method name segment has no `<` or `>`         | `<Add>b__0_0`, `<HandleAsync>d__0::MoveNext`, `<>c`, `<>c__DisplayClass` |

---

## Resolved Decisions

- **Istanbul inferred names:** Arrow functions assigned to variables (`const foo = () => {}`) receive an inferred name `"foo"` from V8. These are counted as named because they can be tested directly via the identifier they are bound to.

- **LLVM records with no name field:** No name means not classifiable as named, so excluded. This is not a special case — a function without a name is by definition not a named function.

- **Coverlet static constructors:** `.cctor` and `.ctor` are plain identifiers (no `<>`), so they are classified as named. Initial behavior: include them.

- **Metrics display:** `named-functions` appears in output only when a named-function gate is configured, matching the behavior of the existing function metric.

---

## Implementation Plan

1. **`src/model.rs`:** Add `MetricKind::NamedFunction`. Add `is_named_function: Option<bool>` to `CoverageOpportunity`. Update all `MetricKind` match arms throughout the codebase.

2. **`src/coverage/llvm_json.rs`:** Add `is_named` classification using the demangled name. Emit `is_named_function` on function opportunities. Accumulate `named_function_totals_by_file` and insert under `MetricKind::NamedFunction`.

3. **`src/coverage/istanbul_json.rs`:** Add `name: Option<String>` to `IstanbulFunctionMap`. Classify and emit `is_named_function`. Accumulate named function totals.

4. **`src/coverage/coverlet_json.rs`:** Switch to `methods.iter()`. Classify method key. Emit `is_named_function`. Accumulate named function totals.

5. **`src/metrics.rs`:** Add filter branch for `MetricKind::NamedFunction` in `compute_changed_metric`.

6. **`src/cli.rs`:** Add `--fail-under-named-functions` and `--fail-uncovered-named-functions` arguments.

7. **`src/config.rs`:** Map new TOML keys and CLI args to `GateRule::Percent` / `GateRule::UncoveredCount` with `MetricKind::NamedFunction`.

8. **`src/gate.rs` / `src/render/`:** Update label and display for the new metric kind.

9. **Tests:** Add unit tests for each parser's named-function classification. Add integration fixtures that include anonymous functions (closures, lambdas, compiler-generated methods) and verify they are excluded from the named-function count. Add gate integration tests for both new gate rules.
