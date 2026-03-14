# TESTING

This document defines the canonical testing process for `covgate`. Treat it as the default workflow for feature work, regression fixes, and review follow-ups.

## Core Process

Run tests from the repository root unless a test explicitly requires another working directory.

1. Format and static quality gates:
   - `cargo fmt`
   - `cargo check`
   - `cargo clippy --all-targets --all-features -- -D warnings`
2. Run automated tests:
   - `cargo test`
3. Validate coverage floor for the tool itself:
   - `cargo llvm-cov --summary-only`
   - Keep total coverage at or above 80%.

## Live-Scenario Testing Philosophy

Always test each feature against **live scenarios** in addition to unit tests.

- Use temporary directories and initialize/manipulate real Git repositories during tests.
- Use real language projects under `tests/fixtures/` as scenario inputs rather than synthetic-only mocks.
- Generate live coverage artifacts with each ecosystem's native tooling, for example:
  - C/C++: compile and test with Clang source-based coverage, merge with `llvm-profdata`, export with `llvm-cov export`
  - .NET: `dotnet test --collect:"XPlat Code Coverage"`
  - Rust: `cargo llvm-cov`
  - JS/TS: `vitest run --coverage`
- Prefer copied-fixture integration tests that assert both CLI behavior and repository state invariants (diff shape, file normalization, and idempotent reruns).

## CLI Coverage Requirement

Every CLI switch must have at least one end-to-end CLI test case defined with the live-scenario process above (copied fixtures, temp Git repos, real coverage artifacts, and command execution assertions).

## TDD for Bugs and Review Feedback

When a bug report or review finding arrives, always follow TDD:

1. Add a failing test that reproduces the reported behavior.
2. Implement the fix.
3. Re-run the targeted test and relevant broader suites until they pass.

Do not ship a bug fix without the reproducer test.
