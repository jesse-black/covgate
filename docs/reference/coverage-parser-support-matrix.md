# Coverage parser support matrix in `covgate`

This document records parser-specific normalization concerns across the native coverage formats that `covgate` currently supports.

## Support matrix

| Concern | LLVM (Rust) | LLVM (C/C++) | LLVM (Swift) | Coverlet | Istanbul |
| --- | --- | --- | --- | --- | --- |
| Mangled function names | Yes | Yes | Yes | No | No |

## Discussion

### Mangled function names

LLVM exports function records from compiler-produced symbol data. That means mangled names are part of the raw input shape across LLVM languages, even though the exact mangling scheme differs by language and toolchain.

For `covgate`, the most important current example is Rust:

- real LLVM exports can include multiple function records that share a source span but differ in mangled symbol identity
- deduplicating by span alone can undercount functions
- demangling provides a more stable callable identity than source span by itself

That is why `covgate` now uses `rustc-demangle` for Rust LLVM symbol normalization before deduplicating function records.

For C/C++ and Swift, the big-picture parser concern is the same:

- raw LLVM function names are compiler/toolchain symbols, not guaranteed source-level display names
- parser logic should assume names may be mangled
- any future language-specific parity bug in LLVM function totals may require language-aware normalization

At the moment, the concrete fix we have implemented is Rust-specific because that is the discrepancy we reproduced and verified.

### Why Coverlet is different

Coverlet reports coverage at .NET method granularity. `covgate` normalizes those methods into the public `functions` metric, but it does not currently need symbol demangling for that step.

The main Coverlet parser concern is:

- method-to-function normalization

That work is documented separately in [coverlet-method-to-function-normalization.md](/home/jesse/git/covgate/docs/reference/coverlet-method-to-function-normalization.md).

### Why Istanbul is different

Istanbul reports coverage from JavaScript or TypeScript source-oriented structures. Its function data is already expressed in terms of source files and source spans, so mangled compiler symbol handling is not part of the parser problem.

That means Istanbul function normalization is mostly about:

- preserving source spans correctly
- translating native file/function data into shared `covgate` opportunities

not about reversing compiler symbol encodings.

The main current Istanbul parser concern is:

- line-summary semantics

Vitest v8 writes Istanbul `coverage-final.json`, but it also writes a separate `coverage-summary.json` with the native overall line totals users actually see. Those line totals are not the same as "every source line touched by a statement span." In the checked-in Vitest fixtures that currently anchor parser behavior, the native line totals match unique statement start lines instead:

- each statement contributes only `statement.start.line` as a line opportunity
- duplicate statement starts on the same file line are merged
- a merged line is covered if any statement beginning on that line has hits

This behavior is exercised by the checked-in parity fixtures under `tests/fixtures/vitest/`, including both `.ts` and `.tsx` source shapes. If a future Istanbul or Vitest artifact disagrees, the repository policy is to capture that native-generated fixture first and then update the parser against the checked-in `native-summary.json` evidence.

### Practical takeaway

The matrix above should guide where we expect symbol-identity bugs to show up:

- LLVM: assume mangled function names are part of the native format and may matter for parity
- Coverlet: expect method-shape normalization issues instead
- Istanbul: expect source-span and line-summary-semantic issues instead

This does not mean every LLVM language needs the same demangler. It means LLVM should be treated as the family where symbol normalization may be required, while the non-LLVM formats should be debugged from different native assumptions.
