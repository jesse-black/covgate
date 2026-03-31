# Purify Internal Tests And Move I/O Coverage To Integration

Save the canonical completed ExecPlan in `docs/exec-plans/completed/covgate-test-boundary-purification.md`.

This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds.

This document must be maintained in accordance with `docs/PLANS.md`.

## Purpose / Big Picture

After this change, the tests that live inside `src/` will better match the design boundary they are meant to protect, and the production code in `src/config.rs` and `src/coverage/mod.rs` will expose smaller pure seams that are easier to test directly. Pure decision logic will be tested with pure in-memory unit tests, while real filesystem behavior will be exercised from `tests/` as integration coverage. The result should be faster, clearer unit tests, a smaller amount of duplicated fixture setup inside library modules, and less need to prove logic indirectly through disk-backed setups. A reader should be able to see this working by inspecting `src/config.rs` and `src/coverage/mod.rs`, then running the targeted test commands in this plan and observing that pure tests remain in-module while file-backed discovery and parsing behavior lives under `tests/`.

## Progress

- [x] (2026-03-23 00:00Z) Created this ExecPlan to turn the current test-boundary classification into a concrete implementation plan.
- [x] (2026-03-25 00:00Z) Reproduced the Coverlet absolute-outside-repo path-normalization bug with a failing parser test before changing production code.
- [x] (2026-03-25 00:00Z) Ensured the existing Istanbul and LLVM path-normalization tests remained in place as regression guards while adjusting Coverlet normalization.
- [x] (2026-03-25 00:00Z) Addressed the Coverlet outside-repo absolute-path normalization bug without regressing in-repo relative matching.
- [x] (2026-03-31 21:22Z) Refactored `src/coverage/mod.rs` so `parse_str` is a thin repository-root lookup wrapper over `parse_with_repo_root`, preserving the direct git-required and git-repository-required error contract.
- [x] (2026-03-31 21:22Z) Added integration coverage in `tests/coverage_parse.rs` for repo-root lookup, missing-git behavior, empty repo-root output, and subdirectory invocation.
- [x] (2026-03-31 22:54Z) Refactored config discovery into pure candidate-selection and TOML-parsing helpers, then rewrote the in-module tests to stay in-memory.
- [x] (2026-03-31 22:54Z) Refactored `resolve_rules` through small helper functions so CLI-over-config precedence stays identical without eight repeated fallback blocks.
- [x] (2026-03-31 22:54Z) Extracted shared lexical-normalization and repo-root-relativization helpers into `src/coverage/path.rs`, preserving the format-specific prefix handling that still differs between Coverlet and Istanbul.
- [x] (2026-03-31 22:54Z) Renamed the top-level coverage entrypoint to `load_from_path`, removed the ambient string parser, aligned the format adapters on `parse_with_repo_root`, and moved the file-backed coverage behavior into `tests/coverage_parse.rs`.
- [x] (2026-03-31 22:54Z) Added `tests/config_discovery.rs` so real file discovery, repo-root stopping, unknown-repo-root fallback, and read/parse failures are covered at the integration boundary.
- [x] (2026-03-31 22:54Z) Ran the remaining targeted test commands during iteration: `cargo test config::`, `cargo test coverage::`, `cargo test --test coverage_parse`, `cargo test --test config_discovery`, `cargo test --test llvm_diff_regression`, and `cargo test --test llvm_real_parity`.
- [x] (2026-03-31 22:54Z) Ran `cargo xtask validate` as the final repository check. The sweep passed, including `fmt`, `clippy`, full tests, self-coverage gating, `cargo-machete`, and `cargo-deny`.
- [x] (2026-03-31 22:54Z) Updated this ExecPlan to reflect the finished state and prepared it to move from `docs/exec-plans/active/` to `docs/exec-plans/completed/`.

## Surprises & Discoveries

- Observation: The only remaining impure tests inside `src/` are filesystem-based, not subprocess-based.
  Evidence: `src/config.rs` uses `tempdir`, `fs::write`, and `fs::create_dir_all` in the config discovery tests, while `src/coverage/mod.rs` uses `tempdir`, `fs::write`, and git setup only for file-backed entrypoint coverage.

- Observation: `src/config.rs` still mixes three responsibilities in one path: candidate-directory selection, file I/O, and TOML parsing.
  Evidence: `load_file_config_from_with_repo_root` in `src/config.rs` both walks `dir.ancestors()` and calls `fs::read_to_string` and `toml::from_str`.

- Observation: `resolve_rules` in `src/config.rs` repeats the same precedence pattern for each metric and rule type.
  Evidence: the function contains eight nearly identical `if let Some(...)` / `else if let Some(...)` blocks that differ mainly by `MetricKind` and `GateRule` variant.

- Observation: The coverage half of the plan is only partially purified after the merge. `parse_str` now has a pure helper boundary plus integration coverage, but the file-backed `parse_path` tests still live in `src/coverage/mod.rs`.
  Evidence: `src/coverage/mod.rs` now contains private `parse_with_repo_root`, while `tests/coverage_parse.rs` exercises `parse_str` behavior and `src/coverage/mod.rs` still contains `parse_path_reads_file_and_dispatches`, `parse_path_prefers_git_repo_root_for_absolute_coverlet_paths`, and `parse_path_reads_istanbul_file_and_dispatches`.

- Observation: The Coverlet outside-repo normalization bug is already fixed at the parser layer, so the remaining work is mostly test-boundary cleanup rather than parser-correctness repair.
  Evidence: `src/coverage/coverlet_json.rs` contains `keeps_absolute_paths_outside_repo_as_absolute`, and `cargo test --test coverage_parse` passed on 2026-03-31 with repo-root and git-failure scenarios.

- Observation: The current top-level coverage entrypoint names hide important semantics. `parse_path` sounds like it parses a filesystem path string, but the function actually reads coverage JSON from disk, and `parse_str` currently mixes parsing with environment-driven repository-root discovery.
  Evidence: `src/coverage/mod.rs` reads file contents inside `parse_path`, and `parse_str` calls `git::resolve_repo_root` before dispatching.

- Observation: Repo-root stopping behavior in config discovery only becomes observable in integration tests when the test creates a real Git repository, not merely a `.git` directory.
  Evidence: `tests/config_discovery.rs` initially kept discovering the parent `covgate.toml` until the test switched from creating `repo/.git/` manually to running `git init`, after which `Config::try_from` stopped at the repository root as intended.

## Decision Log

- Decision: Expand this plan beyond pure test-file relocation to include small production refactors that create clearer pure seams in `src/config.rs` and `src/coverage/mod.rs`.
  Rationale: Moving tests alone would preserve some unnecessary indirectness. Extracting pure candidate-selection, TOML parsing, and explicit-repo-root parsing seams reduces test surface while keeping user-facing behavior unchanged.
  Date/Author: 2026-03-23 / Codex

- Decision: Handle the newly identified Coverlet outside-repo path-normalization bug as part of this same plan before finishing the remaining purification work.
  Rationale: The bug sits directly inside the coverage-path normalization boundary that this plan is already refactoring, and fixing it safely benefits from the same explicit-repo-root seams and stronger parser tests.
  Date/Author: 2026-03-25 / Codex

- Decision: Require TDD for the path-normalization fix and treat existing Istanbul and LLVM normalization coverage as regression guards during any consolidation.
  Rationale: The Coverlet bug was caused by subtle divergence in similar normalization paths. A failing reproducer test first, plus preserved cross-format regression coverage, reduces the risk of consolidating logic while changing behavior.
  Date/Author: 2026-03-25 / Codex

- Decision: Treat config discovery as a split-boundary problem that deserves both pure unit coverage and integration coverage.
  Rationale: The repo-root stopping rule and ancestor walk are pure selection logic, but the actual existence, readability, and parseability of `covgate.toml` are filesystem concerns.
  Date/Author: 2026-03-23 / Codex

- Decision: Treat `coverage::parse_path` as an integration entrypoint.
  Rationale: `parse_path` exists to read a file and then dispatch into the already-tested in-memory parsing path, so file-backed behavior belongs under `tests/`.
  Date/Author: 2026-03-23 / Codex

- Decision: Keep the refactors targeted and behavior-preserving rather than introducing new abstraction styles such as fluent wrappers.
  Rationale: `src/config.rs` and `src/coverage/mod.rs` each benefit more from extracting pure helper boundaries than from building reusable pipelines across multiple callsites.
  Date/Author: 2026-03-23 / Codex

- Decision: After the merge, treat the coverage parsing seam as partially complete and update this plan to focus the remaining work on config discovery and `parse_path` test relocation.
  Rationale: `src/coverage/mod.rs` now has the intended thin-wrapper shape for `parse_str`, and integration coverage already exists for repository-root lookup behavior. Marking that progress explicitly avoids sending a future contributor back into already-finished work.
  Date/Author: 2026-03-31 / Codex

- Decision: Keep the parser-normalization consolidation item open, but lower its priority behind the test-boundary moves.
  Rationale: The merge left three format-specific normalization helpers in place. That duplication is now a cleanup opportunity rather than a blocker because the bug fix and regression coverage already exist.
  Date/Author: 2026-03-31 / Codex

- Decision: Include a small `resolve_rules` refactor in this plan as part of the config-boundary cleanup.
  Rationale: The current repetition makes the config module harder to read and increases the cost of future rule additions. A small helper-based cleanup supports the plan’s overall goal of clearer pure seams without changing behavior.
  Date/Author: 2026-03-31 / Codex

- Decision: Simplify the top-level coverage entrypoints to two responsibilities: one file-loading wrapper and one pure explicit-repository-root parser.
  Rationale: There is no meaningful non-test consumer for a public “string plus ambient cwd/git lookup” API today. Removing that middle shape reduces duplicate parsing work and makes test purification line up with the real purity boundary.
  Date/Author: 2026-03-31 / Codex

- Decision: Revisit the top-level function names as part of the remaining coverage cleanup, favoring names that describe loading versus parsing rather than the raw argument type alone.
  Rationale: Names like `parse_path` are easy to misread as “parse a path string.” The remaining cleanup is a good point to choose names that remain clear even when read without full module context.
  Date/Author: 2026-03-31 / Codex

- Decision: Adopt `load_from_path` for the top-level impure wrapper, `parse_with_repo_root` for the top-level pure core, and `parse_with_repo_root` for each format adapter module.
  Rationale: These names express the actual boundary clearly, stay format-agnostic at the public API layer, and keep parallel naming across the format-specific parser modules.
  Date/Author: 2026-03-31 / Codex

- Decision: Remove the ambient string-based coverage entrypoint instead of keeping it solely for tests, and rewrite the integration coverage to exercise `load_from_path`.
  Rationale: There was no non-test caller for the ambient parser anymore, and keeping it would have preserved an API shape that no longer matched the intended purity boundary.
  Date/Author: 2026-03-31 / Codex

## Outcomes & Retrospective

The config and coverage cleanup landed cleanly and the final repository validation passed. `src/config.rs` now separates candidate-path selection, TOML parsing, and file I/O so the in-module tests stay pure, while `tests/config_discovery.rs` proves the real discovery behavior over directories and Git state. `src/coverage/mod.rs` now exposes `load_from_path` and `parse_with_repo_root`, the format adapters share the same parsing vocabulary, and the file-backed behavior moved into `tests/coverage_parse.rs`. The targeted regression runs and `cargo xtask validate` both passed, so this plan is ready to live under `docs/exec-plans/completed/`.

## Context and Orientation

`covgate` is a Rust CLI in `src/` with integration coverage in `tests/`. Tests inside `src/` are conventional Rust unit tests that can directly call private helpers in the same module. Tests in `tests/` are integration tests that exercise public behavior through the crate boundary, usually with real files, fixture trees, or CLI processes.

The current cleanup target is limited to two modules. `src/config.rs` builds runtime configuration by combining CLI arguments, `covgate.toml`, and Git-derived defaults. Its helper `load_file_config_from_with_repo_root` still walks ancestor directories, reads files from disk, and parses TOML in one function, and its filesystem-backed tests still live inside the module. `src/coverage/mod.rs` currently exposes a file-loading entrypoint and a string-based entrypoint, but those names do not yet clearly distinguish “load coverage JSON from disk” from “parse already-loaded coverage text with an explicit repository root.” The remaining cleanup should leave one impure file-loading wrapper named `load_from_path` and one pure explicit-repository-root parser named `parse_with_repo_root`. The format-specific modules should also expose `parse_with_repo_root` so the dispatch layer and the adapters share one vocabulary. The rest of the in-module tests in this repository are already pure and should stay where they are.

In this plan, “pure unit test” means a test that operates only on in-memory values and does not create files, directories, or subprocesses. “Integration test” means a test under `tests/` that intentionally exercises a public file-backed or CLI-backed entrypoint using the operating system.

## Plan of Work

The path-normalization bug fix already landed, so do not reopen that work unless a new regression appears. Instead, begin from the current merged state. Confirm the existing Coverlet, Istanbul, and LLVM normalization tests still describe the intended boundaries, then leave parser behavior alone unless a cleanup clearly reduces duplication without changing semantics.

Finish the coverage-boundary cleanup by collapsing the top-level API into two responsibilities. Keep one impure file-loading wrapper named `load_from_path` that accepts a filesystem path, reads coverage JSON, resolves the repository root, and delegates. Keep one pure parser named `parse_with_repo_root` that accepts coverage text plus an explicit repository root. Remove the ambient-environment string entrypoint if no non-test caller needs it. Rename the format-specific adapter entrypoints to `parse_with_repo_root` as well so the dispatch layer and the adapters line up naturally. Move the file-backed tests for the wrapper out of `src/coverage/mod.rs` and into `tests/`. Keep `detect_format` and explicit-repository-root parsing tests in-module, because those are pure and close to the parser implementation.

Then complete the original config-boundary cleanup. In `src/config.rs`, extract the ancestor-selection logic from `load_file_config_from_with_repo_root` into a helper that accepts a starting directory and an optional repository root and returns the ordered config candidates to consider, stopping at the repository root when one is supplied. Keep this helper pure so it can be tested with `Path` and `PathBuf` values alone. Extract TOML parsing into a small helper that accepts `&str` and returns `FileConfig`. Leave a thin file-I/O layer in `load_file_config_from_with_repo_root` that iterates those candidates, checks for existence, reads the first matching `covgate.toml`, and parses it through the new helper. While in the same module, refactor `resolve_rules` to remove the repeated “CLI value overrides config value” blocks through a small helper or data-driven pattern, but do not change rule precedence or error behavior.

As part of the coverage cleanup, extract only the truly shared path-normalization primitives from the three format adapters. Shared behavior includes lexical path normalization, repo-root-relative conversion for absolute paths that are actually inside the repository, and preservation of absolute paths outside the repository. Keep format-specific preprocessing, such as string-prefix handling that is unique to a native format, inside the individual adapter module.

Once that split exists, rewrite the current in-module config tests by intent. Keep pure tests in `src/config.rs` for candidate selection, CLI-versus-config precedence, and TOML parsing from strings. Move file-backed discovery tests into a new or existing integration test file under `tests/` so the actual disk behavior is still proven with temp directories. The ancestor-discovery cases should be covered twice: once as pure selection logic and once as integration behavior over a real directory tree. Read and parse failure behavior may remain integration coverage because those failures are specifically about filesystem interaction and path-context error reporting.

Do not broaden the scope into changing parser semantics, changing config lookup behavior, or moving already-pure tests out of their modules. The goal is to align tests with the boundaries that already exist.

## Concrete Steps

From the repository root, implement the remaining work in this order:

1. Confirm the existing normalization regression tests still cover the current Coverlet, Istanbul, and LLVM expectations, but do not reopen parser behavior unless a cleanup is clearly behavior-preserving.
2. On the coverage side, replace the current three-shape arrangement with two responsibilities: top-level `load_from_path` as the impure file-loading wrapper and top-level `parse_with_repo_root` as the pure explicit-repository-root parser. Remove the ambient-environment string entrypoint if no real caller needs it.
3. Rename the format-specific adapter entrypoints in `src/coverage/llvm_json.rs`, `src/coverage/coverlet_json.rs`, and `src/coverage/istanbul_json.rs` to `parse_with_repo_root` so the dispatch layer and the adapter modules share the same name.
4. Extract the shared coverage path-normalization primitives used by the adapters, but keep any format-specific prefix handling local to the adapter that needs it.
5. Update callers and test names to use `load_from_path` and `parse_with_repo_root`, then remove the file-backed coverage tests from `src/coverage/mod.rs` and keep only pure detection and explicit-repository-root parsing tests in-module.
6. Add or expand integration tests under `tests/` that write coverage JSON to disk and prove `coverage::load_from_path` still dispatches correctly for the supported parser families.
7. Edit `src/config.rs` to extract a pure helper for candidate config-path selection, a pure helper for parsing `FileConfig` from TOML text, and keep file reading in a thin wrapper.
8. Refactor `resolve_rules` to remove the repeated CLI-versus-config fallback pattern while preserving the current precedence and error behavior.
9. Update `src/config.rs` tests so only pure config logic remains in-module.
10. Add or expand integration tests under `tests/` for config file discovery, repo-root stopping behavior, unknown-repo-root fallback behavior, and config read/parse failures.
11. Make any further small refactors needed to simplify the logic and finish the test-boundary purification cleanly.
12. During iteration, run only the targeted tests that exercise the edited modules.
13. Once the remaining work is complete, run `cargo xtask validate` exactly once as the final repository check.

Expected targeted commands during implementation include:

    cargo test config::
    cargo test coverage::
    cargo test --test coverage_parse
    cargo test --test llvm_diff_regression
    cargo test --test llvm_real_parity
    cargo test --test <new-or-updated-config-integration-test>

Expected final validation command:

    cargo xtask validate

## Validation and Acceptance

Acceptance is behavior-based, not just structural. The work is complete when all of the following are true:

The in-module tests for `src/config.rs` and `src/coverage/mod.rs` no longer create temp directories or write files. The pure config tests prove ancestor-selection, precedence logic, TOML parsing using only in-memory paths and values, and the preserved CLI-over-config rule precedence after the `resolve_rules` cleanup. The pure coverage tests exercise format detection and `parse_with_repo_root` without relying on `env::current_dir()` or file I/O. The integration tests under `tests/` prove that `covgate.toml` is found in a parent directory, is not discovered past a known repository root, is still discovered when the repository root is unknown, and reports contextual read and parse failures when the discovered file cannot be read or parsed. The integration coverage for `coverage::load_from_path` proves that a real coverage file on disk is read and dispatched into the correct parser family.

The Coverlet regression remains fixed so that absolute paths outside the repository stay absolute instead of being converted into fake repo-relative paths. Any extracted shared path-normalization helper must preserve the existing intended Istanbul and LLVM behaviors, with tests that would fail if the shared logic regressed those formats.

Run the targeted test commands from the repository root while iterating and expect them to pass after the refactor. `cargo test --test coverage_parse` already passes in the merged state and should continue to pass while the remaining cleanup lands. Before closing the work, run `cargo xtask validate` from the repository root and expect the repository validation to pass. Per `AGENTS.md`, do not run `cargo xtask quick` in addition to that final validation step.

## Idempotence and Recovery

This work is safe to repeat because it only rearranges test boundaries and introduces helper functions; it does not require schema changes, fixture regeneration, or destructive commands. If an intermediate refactor leaves tests failing, restore progress by first making each new pure helper compile with a minimal unit test, then reintroduce integration coverage one behavior at a time. If a new integration test file proves awkward, it is acceptable to place the new tests into an existing related file in `tests/` as long as the behavior split remains clear.

## Artifacts and Notes

The current impure in-module test blocks that still need to be replaced or split are:

    src/config.rs:
      - loads_repo_config_file_when_present
      - loads_repo_config_file_from_parent_directory
      - does_not_walk_past_repo_root_when_config_is_missing_in_repo
      - still_walks_past_parent_boundaries_when_repo_root_is_unknown
      - reports_read_errors_for_parent_directory_config_candidates
      - reports_parse_errors_for_parent_directory_config_candidates

    src/coverage/mod.rs:
      - file-loading coverage entrypoint tests currently named with `parse_path_*`, to be renamed around `load_from_path`

The desired end state is:

    Pure in-module tests:
      - config candidate selection
      - CLI/config precedence
      - TOML parsing from strings
      - coverage parsing with an explicit repo root
      - coverage format detection
      - coverage string parsing thin-wrapper behavior only where needed

    Integration tests:
      - config discovery over real directory trees
      - config file read and parse failures with real paths
      - coverage file reading through `load_from_path`
      - repository-root lookup behavior for string parsing entrypoints

## Interfaces and Dependencies

The implementation should keep using the existing standard-library and crate dependencies already present in the repository: `std::path::{Path, PathBuf}`, `std::fs`, `tempfile`, `toml`, `serde_json`, and the existing public coverage and config interfaces. In `src/config.rs`, introduce a private helper that exposes the config-candidate selection boundary in a pure form. Its exact name may vary, but it must accept the same core inputs as `load_file_config_from_with_repo_root`: a starting directory and an optional repository root. Add a second private helper that parses `FileConfig` from TOML text. `load_file_config_from_with_repo_root` should remain the public-in-module orchestration point that performs the actual file read and parse work using those helpers.

In `src/coverage/mod.rs`, the desired end state is one pure parser named `parse_with_repo_root` that accepts both coverage text and an explicit repository root path, plus one impure wrapper named `load_from_path` that loads coverage data from a filesystem path and delegates. In `src/coverage/llvm_json.rs`, `src/coverage/coverlet_json.rs`, and `src/coverage/istanbul_json.rs`, the format-specific adapter entrypoints should also be named `parse_with_repo_root` so the dispatch layer and adapter modules share a consistent naming scheme.

Revision note: created this ExecPlan to turn the current verbal classification of impure `src/` tests into a concrete, repository-specific implementation plan with explicit file targets and validation expectations.

Revision note (2026-03-31): updated this ExecPlan after the merge to reflect that the Coverlet normalization fix and `parse_str`/repo-root integration coverage are already in place, while config-boundary cleanup and `parse_path` test relocation remain outstanding.

Revision note (2026-03-31, later): updated this ExecPlan again during planning to simplify the target coverage API to `load_from_path` plus `parse_with_repo_root`, and to align the format adapter modules on the same `parse_with_repo_root` naming.

Revision note (2026-03-31 22:54Z): updated Progress, Surprises & Discoveries, Decision Log, and Outcomes & Retrospective after landing the remaining config-boundary cleanup, coverage API rename, new integration tests, targeted regression runs, and the final successful `cargo xtask validate` pass, then prepared the document to move to `completed/`.
