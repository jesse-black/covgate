---
description: "Implementation plan for token-efficient console output; read when implementing, reviewing, or continuing this task."
---

# Token-Efficient Console Output

## Goal
- Implement token-efficient console output for `covgate` to minimize AI agent context usage while providing actionable column-level precision.

## Scope
- In: Updating `SourceSpan` model, LLVM/Istanbul parsers to extract columns, implementing `PASS` minimal and `FAIL` focused console output, adding `--verbose` CLI flag.
- Out: Coverlet parser column support (format does not provide source columns), other output formats (e.g. JSON, XML).

## Relevant Areas
- `src/model.rs` — Core types (`SourceSpan`) needing column support.
- `src/coverage/llvm_json.rs` — LLVM parser needing column extraction.
- `src/coverage/istanbul_json.rs` — Istanbul parser needing column extraction.
- `src/render/console.rs` — Output formatter needing token-efficient logic and grouping.
- `src/cli.rs` — CLI argument parsing for `--verbose`.

## Open Questions
- None yet

## Steps
- [x] Add `start_col: Option<u32>` and `end_col: Option<u32>` to `SourceSpan` in `src/model.rs`.
- [x] Update `SourceSpan::display()` to support `line:col` formatting and line-only fallback.
- [x] Extract `start_col`/`end_col` in LLVM parser (`src/coverage/llvm_json.rs`) for regions, lines, and branches.
- [x] Update function key parsing in LLVM to carry column information.
- [x] Extract `column` from `IstanbulPosition` in Istanbul parser (`src/coverage/istanbul_json.rs`) for lines, branches, and functions.
- [x] Add `verbose: bool` parameter to `render` function in `src/render/console.rs`.
- [x] Implement Minimal Success Output (PASS) in console renderer.
- [x] Implement Focused Failure Output (FAIL) with grouping by file in console renderer.
- [x] Update `group_spans` in `console.rs` to be column-aware for accurate grouping.
- [x] Add `--verbose` flag to CLI (`src/cli.rs`) and pass it down.
- [x] Update/add tests in `src/coverage/*_json.rs` to verify column parsing.
- [x] Update/add tests in `src/render/console.rs` to verify new PASS/FAIL formats and column handling.
- [x] Update integration tests to handle format changes (using `--verbose` or updating assertions).
- [x] Fix compile error in `src/render/console.rs` (type annotations for `BTreeMap`).
- [x] De-duplicate missed span formatting logic by moving it to `SourceSpan` in `src/model.rs`.
- [x] Refactor `render_minimal` in `src/render/console.rs` to reduce nesting and extract helpers.
- [x] Ensure `src/render/markdown.rs` is fully updated with column support (use `SourceSpan` methods).
- [x] Verify `verbose` configuration works via `covgate.toml`.
- [x] Audit for any remaining handwritten string matchers (e.g. `MetricKind::parse`) and replace with toolchain-provided mechanisms.

## Validation
- [x] `cargo check` and `cargo clippy`
- [x] `cargo test`
- [x] `cargo run -- check tests/fixtures/llvm-real/covgate-self-full.json`
- [x] `cargo run -- check tests/fixtures/llvm-real/covgate-self-full.json --verbose`

## Discoveries
- Istanbul formats sometimes provide `null` for columns; `IstanbulPosition::column` must be `Option<u32>` with `#[serde(default)]`.
- Integration tests in `tests/cli_interface.rs` and `tests/cli_metrics.rs` rely heavily on verbose output (headers, rulers, rule labels); many were updated to use `--verbose`.

## Review
- [x] **Principle 2 (One fact, one place):**
    - [x] Logic Duplication: `SourceSpan::display` (model.rs) vs `format_span` (console.rs).
    - [x] Missing Fact: `src/render/markdown.rs` has not been updated to include column information.
    - [x] Function Duplication: `title_case` is defined identically in both `console.rs` and `markdown.rs`.
    - [x] Config Inconsistency: `verbose` is missing from `FileConfig` (TOML), meaning it cannot be set in `covgate.toml`.
- [x] **Principle 1 (Trust the toolchain):**
    - [x] `MetricKind::parse` is a handwritten string matcher; should use `serde` or `clap::ValueEnum`.
- [x] **Principle 5 (Happy path reads top to bottom):**
    - [x] `render_minimal` in `console.rs` has high nesting; extract file-grouping and stats logic into helpers.
