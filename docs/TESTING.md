# TESTING

This document defines the canonical testing process for `covgate`. Treat it as the default workflow for feature work, regression fixes, and review follow-ups.

## Philosophy

Four principles, ordered by how often they bite at review. The process sections below derive from them; when a process rule and a principle conflict, the principle wins.

1. **Fixture sets are contracts.** Membership in a fixture set such as `function_capable_fail_fixtures` is a declaration that every member can express the scenario being tested. If a fixture cannot yet demonstrate the behavior, it must not be in the set — not even temporarily, not even with a compensating conditional. The set name is load-bearing documentation that reviewers and future authors will read and trust.

2. **Matrix tests assert uniformly.** A test that iterates over a fixture set must reach the same assertion branch for every member. Any conditional inside the loop that branches on fixture identity — language, scenario name, or any other property — is evidence that the fixture set is wrong, not that the test needs a special case. Differentiation belongs in the fixture set definition, not in the test body.

3. **Gaps are surfaced, not papered over.** When a parser or fixture does not yet support a capability, the correct response is to stop and ask questions to clarify the gap in the plan and how to address it. A test that appears to exercise a capability while silently accepting a wrong outcome is worse than no test at all — it creates false confidence and hides the debt.

4. **Fixtures are grounded in real toolchain output.** Fixture coverage JSON must come from native toolchains via `cargo xtask regen-fixture-coverage`. A fixture that was hand-edited to make a test pass no longer represents a real-world scenario; it represents the author's assumption about what the toolchain would produce, which may be wrong in exactly the ways that matter.

## Rules in practice

### Place tests where their access needs dictate

*Principles 1, 2.*

Use `#[cfg(test)]` inline when the test must reach private items — private functions, private helpers, or internal types that are not exposed through the public API. The inline module is a sibling to the production code and can access private items directly; this is the primary reason the feature exists. If a test calls a private function, it belongs inline, and only there.

Use `tests/*.rs` when the test exercises only the public crate API. Tests in `tests/` compile as a separate crate and cannot see private or `pub(crate)` items, which makes the boundary explicit and prevents the test from depending on internals.

The deciding question is: **does this test call a private or `pub(crate)` item?** If yes, inline. If no, `tests/`.

Inline test modules that do not call any private function are wrong placement, regardless of how they got there. A large `#[cfg(test)]` block at the bottom of a parser module that only exercises behavior reachable through the public API (like `coverage::parse_with_repo_root`) is not earning its place inline — it is making the production code harder to read without gaining anything from the access.

Correct uses of inline tests in this codebase:
- `src/coverage/llvm_json.rs` — tests for private `normalize_path` and `normalize_llvm_function_name` must be inline.
- `src/coverage/coverlet_json.rs` — tests for private `normalize_path` must be inline.

Incorrect uses (tests that exercise only public APIs but live inline anyway):
- `src/coverage/istanbul_json.rs` — all tests exercise parser behavior reachable through the public coverage API; they belong in `tests/coverage_parse.rs`.
- `src/gate.rs` — all tests call `pub fn evaluate`; they belong in `tests/`.

When a file has a mix — some tests calling private functions, some calling only public ones — keep only the private-access tests inline. Extract the rest.

### Never introduce fixture-specific conditionals inside a matrix test

*Principles 1, 2.*

The following pattern is always wrong, regardless of how temporary it feels:

```rust
// WRONG: conditional inside a fixture loop
for fixture in function_capable_fail_fixtures() {
    let expected_status = if fixture.language == "dotnet" { 0 } else { 1 };
    assert_eq!(output.status.code(), Some(expected_status));
}
```

This pattern emerged when a dotnet fixture was added to `function_capable_fail_fixtures` before the coverlet parser supported function-level metrics. The fixture could not fail a function threshold rule, so the test was patched to expect it to pass. The result: a fixture named "fail" that the test asserts will succeed, with the contradiction hidden inside the loop body rather than visible at the fixture set definition.

The correct fix at the time would have been one of:
- Update the fixture to contain an uncovered function before including it in the set.
- Exclude the fixture from the set and record the parser work needed for inclusion in an issue, exec plan, or `docs/TODO.md`.
- Create a separate `function_threshold_not_supported_fixtures()` set for the "metric unavailable" scenario and test that path explicitly.

If you find yourself writing `if fixture.language == …` inside a test loop, stop. The fixture set is wrong; fix the set, not the assertion.

## Core Process

Use focused checks as the default inner-loop while developing. Prefer the narrowest command that exercises the changed behavior: a specific `cargo test` target or test name for behavior, `cargo fmt` for formatting edits, and `cargo clippy` for lint-policy or Rust-shape edits. The broader Clippy flags live in `cargo xtask validate`.

Run `cargo xtask validate` from the repository root before considering Rust behavior changes complete. It performs all format checks, linting, test execution, coverage validation, dependency checks, and self-coverage analysis. Documentation-only, CI-only, metadata-only, and lint/config-only changes may close with focused checks instead when those checks cover the touched surface.

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
3. Re-run the targeted test and relevant broader suites until they pass. During active iteration, prefer the narrowest command that exercises the changed area. Run `cargo xtask validate` before shipping Rust behavior changes.

Do not ship a bug fix without the reproducer test.
