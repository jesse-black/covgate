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
  - Swift: `swift test --enable-code-coverage`, then export with `llvm-cov export`
  - JS/TS: `vitest run --coverage`
- Prefer copied-fixture integration tests that assert both CLI behavior and repository state invariants (diff shape, file normalization, and idempotent reruns).
- Prefer checking generated coverage artifacts such as `coverage.json` into the fixture directory for normal test runs. Regenerate them deliberately when the fixture project or expected report shape changes, but do not require every `cargo test` run to rebuild every language fixture from scratch.
- Regenerate fixture coverage artifacts through xtasks so fixture JSON shape stays consistent across languages. These xtasks must invoke native language toolchains (`rustc`/LLVM tools, `clang++`, `swiftc`, and `dotnet test --collect:"XPlat Code Coverage;Format=json"`) plus format-native export steps (`llvm-cov export` for LLVM fixtures); they must not hand-author coverage JSON payloads.
  - Individual fixture: `cargo xtask regen-fixture-coverage <language>/<scenario>` (examples: `rust/basic-fail`, `cpp/basic-pass`, `swift/basic-fail`)
  - All fixtures: `cargo xtask regen-fixture-coverage-all`
  - After regenerating fixture artifacts, rerun the affected integration test file(s) and then `cargo xtask validate`.

## CLI Coverage Requirement

Every CLI switch must have at least one end-to-end CLI test case defined with the live-scenario process above (copied fixtures, temp Git repos, real coverage artifacts, and command execution assertions).

When a CLI test case is fundamentally about metric semantics, such as whether a threshold passes, fails, or reports that a metric is unavailable, the test should define the list of fixtures it exercises and run the same scenario against every compatible fixture whenever possible. “Whenever possible” is deliberate: some scenarios are supposed to be fixture-specific, such as proving that a Rust LLVM fixture without branch data returns “metric not available” for branch thresholds. In those cases, limit the fixture list to the fixtures that actually express the intended capability or lack of capability, and make that reason obvious in the test code.

Keep metric-oriented CLI tests separate from CLI interface tests. Metric tests are expected to iterate across multiple language fixtures when the scenario semantics are shared. Interface tests, such as config precedence, base-ref defaults, diff-source selection, Markdown output, or absolute-path normalization, usually do not need a full fixture matrix because they validate command-surface behavior rather than cross-language metric parity.

The preferred integration-test layout is:

- `tests/cli_metrics.rs` for metric thresholds, uncovered budgets, and metric-availability behavior across compatible fixtures
- `tests/cli_interface.rs` for CLI/config/output behavior that can usually use a single representative fixture

Shared helper code for fixture setup, Git initialization, diff generation, and `covgate` invocation should live in a reusable test module so new LLVM-producing fixture families can be added without duplicating the harness.

## TDD for Bugs and Review Feedback

When a bug report or review finding arrives, always follow TDD:

1. Add a failing test that reproduces the reported behavior.
2. Implement the fix.
3. Re-run the targeted test and relevant broader suites until they pass.

Do not ship a bug fix without the reproducer test.
