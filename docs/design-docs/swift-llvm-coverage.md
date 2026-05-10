---
description: "Draft design doc for Swift LLVM coverage support in covgate; read when adding Swift coverage parsing, investigating Swift LLVM fixture behavior, or planning Swift-specific LLVM normalization work."
---

# Swift LLVM Coverage Support

**Status:** Draft (Migrated from abandoned ExecPlan `covgate-cpp-swift-llvm-fixture-confidence.md`)

## Overview

`covgate` supports Swift through LLVM JSON artifacts exported by `llvm-cov export` (often following `swift test --enable-code-coverage`). High confidence support requires fixtures that reflect the distinctive mangling and structural patterns of Swift.

## Fixture Requirements

To validate function identity and normalization, Swift fixtures should include:
- **Structs and Classes:** Methods and initializers.
- **Generics:** Generic functions and types.
- **Protocol Conformances:** Methods implementing protocol requirements.
- **Nested Functions:** Functions defined within other functions.
- **Closures:** Anonymous functions and escaping closures.
- **Test Targets:** Interaction between test code and library code.

The goal is to produce a `coverage.json` artifact whose function records look like real Swift compiler output, exercising the mangled symbol names that Swift produces.

## Strategy

### Demangling
- **Options:** 
    - `swift-demangle`: Lighter, Swift-specific.
    - `symbolic-demangle`: Broader, more mature, but heavier.
- **Consolidation:** If `symbolic-demangle` is chosen, evaluate whether it should replace existing language-specific demanglers (like `rustc-demangle`) to provide a unified LLVM parsing layer.

### Normalization
- Function normalization logic lives in `src/coverage/llvm_json.rs`.
- Normalization should only be implemented after a failing test (backed by a realistic fixture) reproduces a function identity issue.

## Validation

- **Fixture Regeneration:** Must be supported via `cargo xtask regen-fixture-coverage swift/<scenario>`.
- **Parity Tests:** Compare `covgate` metrics against `llvm-cov report` for the same artifact to ensure 100% parity.
- **Parser Tests:** Focused tests in `llvm_json.rs` to verify symbol normalization.
