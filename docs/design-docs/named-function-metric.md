# Design Doc: Named Function Metric

## Context & Problem Statement

`covgate` currently exposes a `functions` metric that counts all function-level coverage opportunities regardless of whether they are named, anonymous, or compiler-generated. This means closures, lambdas, async state machines, and other synthetic callables inflate the function count and can cause gates to fail for coverage that is impractical to test directly.

The goal is a `named-functions` metric that applies the same diff-gating semantics as `functions` but restricted to review-relevant source-level functions. The broader `functions` metric remains the place to enforce every function-like unit reported by a coverage backend. `named-functions` is intentionally more opinionated: it should exclude anonymous, synthetic, and generated-looking entries where possible, and it should avoid making users chase backend artifacts such as generic/template monomorphizations.

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

## Classification Hierarchy

`named-functions` should classify function coverage records in this order:

1. **Exclude synthetic, anonymous, and generated-looking entries.** Closure bodies, anonymous function expressions, async/coroutine state-machine entries, compiler-generated methods, and nameless records do not count as named functions. These entries remain part of the broader `functions` metric.
2. **Collapse generic/template instantiations to their authored base function identity.** A generic or templated function may appear in coverage as multiple backend function records, one per concrete instantiation. For `named-functions`, those records should represent one authored function. The collapsed function is covered if any reported instantiation is covered, and uncovered only if all reported instantiations for that authored function are uncovered.
3. **Count ordinary named source functions normally.** After synthetic/generated exclusions and generic/template collapsing, the remaining named records form the `named-functions` opportunities and totals.

This hierarchy keeps `named-functions` focused on the code review question: "Did each meaningful function the author wrote get exercised?" Users who want raw backend function accounting can use `functions`.

### Generic / Template Instantiations

Generic and template instantiations are not synthetic in the same sense as closures or compiler-generated state machines: they often come from user-authored source functions. However, counting each concrete instantiation separately makes named-function gates depend on compiler and coverage-backend mechanics rather than on authored behavior units.

For example, these LLVM-demangled records should collapse to one `my_crate::parse` named-function opportunity:

```text
my_crate::parse::<toml::de::Deserializer>
my_crate::parse::<serde_json::de::Deserializer>
```

The collapsed opportunity is covered if either instantiation is covered. This enforces that at least one concrete variant of a user-authored generic function ran, without requiring every monomorphized variant to run. It also avoids treating framework or deserialization adapter callbacks as multiple independent named functions merely because they were instantiated through generic machinery.

The collapse rule must not rely on framework-specific names such as `serde`, `toml`, or `clap`. It should be based on generic/template syntax in the normalized backend name, such as Rust/C++ demangled `<...>` instantiation suffixes, when the format exposes enough information to do so.

Generic/template collapsing is a format-specific concern, not a universal parser requirement. It applies where a coverage backend can report multiple function records for one authored generic/template function. Today that means LLVM-backed parsers, especially Rust and future C/C++ support. Istanbul does not need this rule because TypeScript generics are erased before runtime and Istanbul reports instrumented source function locations. Coverlet does not currently need this rule because its method keys are already source-method oriented for this metric; compiler-generated C# artifacts remain handled by the `<...>` exclusion rule.

## Classification Rules Summary

| Format       | Named                                         | Anonymous / Excluded                                                  | Generic / template handling |
|--------------|-----------------------------------------------|-----------------------------------------------------------------------|-----------------------------|
| LLVM (Rust)  | Demangled non-generic base name has no `{...}` path segment | Any `{closure#N}`, `{async_fn#N}`, `{coroutine#N}`, etc. segment | Collapse `<...>` instantiations to the base function identity |
| LLVM (C/C++) | Same `{...}` rule via `cpp_demangle` output   | `{lambda()#N}::operator()` and similar                               | Collapse template instantiations when available through demangling |
| Istanbul     | `name` non-empty, not `(anonymous_N)` / `<anonymous>` | Empty name, `(anonymous_N)`, `<anonymous>`                   | Not applicable; TypeScript generics are erased and Istanbul reports source function locations |
| Coverlet     | Method name segment has no `<` or `>`         | `<Add>b__0_0`, `<HandleAsync>d__0::MoveNext`, `<>c`, `<>c__DisplayClass` | Not currently applied; method keys are source-method oriented for this metric |

---

## Resolved Decisions

- **Istanbul inferred names:** Arrow functions assigned to variables (`const foo = () => {}`) receive an inferred name `"foo"` from V8. These are counted as named because they can be tested directly via the identifier they are bound to.

- **LLVM records with no name field:** No name means not classifiable as named, so excluded. This is not a special case — a function without a name is by definition not a named function.

- **LLVM generic instantiations:** Generic/template instantiations should collapse to the authored base function identity rather than being excluded outright or counted once per instantiation. This preserves enforcement for user-authored generic functions while avoiding monomorphization-shaped gate failures.

- **Coverlet static constructors:** `.cctor` and `.ctor` are plain identifiers (no `<>`), so they are classified as named. Initial behavior: include them.

- **Metrics display:** `named-functions` appears in output only when a named-function gate is configured, matching the behavior of the existing function metric.
