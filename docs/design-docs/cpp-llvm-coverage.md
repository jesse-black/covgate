# C/C++ LLVM Coverage Support

**Status:** Draft (Migrated from abandoned ExecPlan `covgate-cpp-swift-llvm-fixture-confidence.md`)

## Overview

`covgate` supports C/C++ through LLVM JSON artifacts exported by `llvm-cov`. To ensure high confidence in this support, the repository requires realistic fixtures that exercise real-world language features and the resulting mangled symbols.

## Fixture Requirements

To validate function identity and normalization, C/C++ fixtures should include:
- **Namespaces:** Functions and classes nested in multiple namespaces.
- **Overloads:** Multiple functions with the same name but different signatures.
- **Templates:** Function and class template instantiations.
- **Lambdas:** Anonymous functions which generate unique mangled names.
- **Local Statics:** Functions with local static variables.
- **Out-of-line Methods:** Class methods defined outside the class declaration.

The goal is to produce a `coverage.json` artifact containing non-trivial mangled function names that would otherwise confuse a simple span-based deduplication strategy.

## Strategy

### Demangling
- **Primary Choice:** `cpp_demangle` crate. It is purpose-built for Itanium ABI mangling and narrower than multi-language demangling bundles.
- **Alternative:** `symbolic-demangle` if a unified LLVM demangling layer is pursued for all supported LLVM languages (Rust, C++, Swift).

### Normalization
- Function normalization logic lives in `src/coverage/llvm_json.rs`.
- Normalization should only be implemented after a failing test (backed by a realistic fixture) reproduces a function identity issue.

## Validation

- **Fixture Regeneration:** Must be supported via `cargo xtask regen-fixture-coverage cpp/<scenario>`.
- **Parity Tests:** Compare `covgate` metrics against `llvm-cov report` for the same artifact to ensure 100% parity.
- **Parser Tests:** Focused tests in `llvm_json.rs` to verify symbol normalization.
