# Purify Internal Tests And Move I/O Coverage To Integration

Save this in-progress ExecPlan in `docs/exec-plans/active/covgate-test-boundary-purification.md`. Move it to `docs/exec-plans/completed/covgate-test-boundary-purification.md` only after implementation, validation, and documentation updates are complete.

This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds.

This document must be maintained in accordance with `docs/PLANS.md`.

## Purpose / Big Picture

After this change, the tests that live inside `src/` will better match the design boundary they are meant to protect, and the production code in `src/config.rs` and `src/coverage/mod.rs` will expose smaller pure seams that are easier to test directly. Pure decision logic will be tested with pure in-memory unit tests, while real filesystem behavior will be exercised from `tests/` as integration coverage. The result should be faster, clearer unit tests, a smaller amount of duplicated fixture setup inside library modules, and less need to prove logic indirectly through disk-backed setups. A reader should be able to see this working by inspecting `src/config.rs` and `src/coverage/mod.rs`, then running the targeted test commands in this plan and observing that pure tests remain in-module while file-backed discovery and parsing behavior lives under `tests/`.

## Progress

- [x] (2026-03-23 00:00Z) Created this ExecPlan to turn the current test-boundary classification into a concrete implementation plan.
- [ ] Refactor config discovery so path-selection logic and TOML parsing can be tested without touching the filesystem.
- [ ] Refactor coverage parsing so `env::current_dir()` is isolated in a thin wrapper and the core parser dispatch can be tested with an explicit repository root.
- [ ] Move the file-backed `parse_path` coverage test out of `src/coverage/mod.rs` and into `tests/`.
- [ ] Split config discovery coverage into pure unit tests for ancestor-selection logic plus integration tests for actual file reads and parse failures.
- [ ] Run the relevant targeted tests during iteration, then run `cargo xtask validate` once before declaring the work complete.

## Surprises & Discoveries

- Observation: The only remaining impure tests inside `src/` are filesystem-based, not subprocess-based.
  Evidence: `src/config.rs` uses `tempdir`, `fs::write`, and `fs::create_dir_all` in the config discovery tests, while `src/coverage/mod.rs` uses `tempdir` and `fs::write` only for `parse_path_reads_file_and_dispatches`.

- Observation: `src/config.rs` currently mixes three responsibilities in one path: candidate-directory selection, file I/O, and TOML parsing.
  Evidence: `load_file_config_from_with_repo_root` in `src/config.rs` both walks `dir.ancestors()` and calls `fs::read_to_string` and `toml::from_str`.

## Decision Log

- Decision: Expand this plan beyond pure test-file relocation to include small production refactors that create clearer pure seams in `src/config.rs` and `src/coverage/mod.rs`.
  Rationale: Moving tests alone would preserve some unnecessary indirectness. Extracting pure candidate-selection, TOML parsing, and explicit-repo-root parsing seams reduces test surface while keeping user-facing behavior unchanged.
  Date/Author: 2026-03-23 / Codex

- Decision: Treat config discovery as a split-boundary problem that deserves both pure unit coverage and integration coverage.
  Rationale: The repo-root stopping rule and ancestor walk are pure selection logic, but the actual existence, readability, and parseability of `covgate.toml` are filesystem concerns.
  Date/Author: 2026-03-23 / Codex

- Decision: Treat `coverage::parse_path` as an integration entrypoint.
  Rationale: `parse_path` exists to read a file and then dispatch into the already-tested in-memory parsing path, so file-backed behavior belongs under `tests/`.
  Date/Author: 2026-03-23 / Codex

- Decision: Keep the refactors targeted and behavior-preserving rather than introducing new abstraction styles such as fluent wrappers.
  Rationale: `src/config.rs` and `src/coverage/mod.rs` each benefit more from extracting pure helper boundaries than from building reusable pipelines across multiple callsites.
  Date/Author: 2026-03-23 / Codex

## Outcomes & Retrospective

This section is intentionally incomplete because the work has not been implemented yet. At completion, record which pure helper seams were introduced in `src/config.rs` and `src/coverage/mod.rs`, which integration tests replaced the old in-module filesystem tests, and whether the final layout reduced duplication without weakening behavior coverage.

## Context and Orientation

`covgate` is a Rust CLI in `src/` with integration coverage in `tests/`. Tests inside `src/` are conventional Rust unit tests that can directly call private helpers in the same module. Tests in `tests/` are integration tests that exercise public behavior through the crate boundary, usually with real files, fixture trees, or CLI processes.

The current cleanup target is limited to two modules. `src/config.rs` builds runtime configuration by combining CLI arguments, `covgate.toml`, and Git-derived defaults. Its helper `load_file_config_from_with_repo_root` currently walks ancestor directories, reads files from disk, and parses TOML in one function. `src/coverage/mod.rs` provides two entrypoints: `parse_str`, which parses a JSON string already in memory, and `parse_path`, which reads a JSON file from disk before delegating to the same parser family. `parse_str` currently also resolves the current working directory before dispatching to the format-specific parser modules. The rest of the in-module tests in this repository are already pure and should stay where they are.

In this plan, “pure unit test” means a test that operates only on in-memory values and does not create files, directories, or subprocesses. “Integration test” means a test under `tests/` that intentionally exercises a public file-backed or CLI-backed entrypoint using the operating system.

## Plan of Work

Start in `src/config.rs`. Extract the ancestor-selection logic from `load_file_config_from_with_repo_root` into a helper that accepts a starting directory and an optional repository root and returns the ordered config candidates to consider, stopping at the repository root when one is supplied. Keep this helper pure so it can be tested with `Path` and `PathBuf` values alone. Extract TOML parsing into a small helper that accepts `&str` and returns `FileConfig`. Leave a thin file-I/O layer in `load_file_config_from_with_repo_root` that iterates those candidates, checks for existence, reads the first matching `covgate.toml`, and parses it through the new helper.

Once that split exists, rewrite the current in-module config tests by intent. Keep pure tests in `src/config.rs` for candidate selection, CLI-versus-config precedence, and TOML parsing from strings. Move file-backed discovery tests into a new or existing integration test file under `tests/` so the actual disk behavior is still proven with temp directories. The ancestor-discovery cases should be covered twice: once as pure selection logic and once as integration behavior over a real directory tree. Read and parse failure behavior may remain integration coverage because those failures are specifically about filesystem interaction and path-context error reporting.

Then clean up `src/coverage/mod.rs`. Extract a `parse_str_with_repo_root`-style helper that accepts the input string and an explicit repository root path, and make the current `parse_str` implementation a thin wrapper that looks up `env::current_dir()` and delegates. This keeps format detection and parser dispatch testable without environment lookup. Remove the file-backed `parse_path_reads_file_and_dispatches` unit test from the module and recreate it as an integration test under `tests/`. Keep the current in-memory `detect_format` tests in `src/coverage/mod.rs`, and expand the pure parsing coverage around the explicit-repo-root entrypoint instead of relying on filesystem-backed tests for dispatch behavior.

Do not broaden the scope into changing parser semantics, changing config lookup behavior, or moving already-pure tests out of their modules. The goal is to align tests with the boundaries that already exist.

## Concrete Steps

From the repository root, implement the work in this order:

1. Edit `src/config.rs` to extract a pure helper for candidate config-path selection, a pure helper for parsing `FileConfig` from TOML text, and keep file reading in a thin wrapper.
2. Update `src/config.rs` tests so only pure logic remains in-module.
3. Add integration tests under `tests/` for config file discovery, repo-root stopping behavior, unknown-repo-root fallback behavior, and config read/parse failures.
4. Edit `src/coverage/mod.rs` to introduce an explicit-repo-root parsing entrypoint and make `parse_str` a thin environment-reading wrapper.
5. Remove the file-backed `parse_path` unit test from `src/coverage/mod.rs` and keep only pure detection and parsing tests in-module.
6. Add an integration test under `tests/` that writes a coverage JSON file to disk and proves `coverage::parse_path` still dispatches correctly.
7. During iteration, run only the targeted tests that exercise the edited modules.
8. Once the work is complete, run `cargo xtask validate` exactly once as the final repository check.

Expected targeted commands during implementation include:

    cargo test config::
    cargo test coverage::
    cargo test --test <new-config-integration-test>
    cargo test --test <new-coverage-integration-test>

Expected final validation command:

    cargo xtask validate

## Validation and Acceptance

Acceptance is behavior-based, not just structural. The work is complete when all of the following are true:

The in-module tests for `src/config.rs` and `src/coverage/mod.rs` no longer create temp directories or write files. The pure config tests prove ancestor-selection, precedence logic, and TOML parsing using only in-memory paths and values. The pure coverage tests exercise format detection and parsing through an explicit-repository-root entrypoint rather than relying on `env::current_dir()`. The integration tests under `tests/` prove that `covgate.toml` is found in a parent directory, is not discovered past a known repository root, is still discovered when the repository root is unknown, and reports contextual read and parse failures when the discovered file cannot be read or parsed. The integration coverage for `coverage::parse_path` proves that a real JSON file on disk is read and dispatched into the correct parser family.

Run the targeted test commands from the repository root while iterating and expect them to pass after the refactor. Before closing the work, run `cargo xtask validate` from the repository root and expect the repository validation to pass. Per `AGENTS.md`, do not run `cargo xtask quick` in addition to that final validation step.

## Idempotence and Recovery

This work is safe to repeat because it only rearranges test boundaries and introduces helper functions; it does not require schema changes, fixture regeneration, or destructive commands. If an intermediate refactor leaves tests failing, restore progress by first making each new pure helper compile with a minimal unit test, then reintroduce integration coverage one behavior at a time. If a new integration test file proves awkward, it is acceptable to place the new tests into an existing related file in `tests/` as long as the behavior split remains clear.

## Artifacts and Notes

The current impure in-module test blocks that this plan is expected to replace or split are:

    src/config.rs:
      - loads_repo_config_file_when_present
      - loads_repo_config_file_from_parent_directory
      - does_not_walk_past_repo_root_when_config_is_missing_in_repo
      - still_walks_past_parent_boundaries_when_repo_root_is_unknown
      - reports_read_errors_for_parent_directory_config_candidates
      - reports_parse_errors_for_parent_directory_config_candidates

    src/coverage/mod.rs:
      - parse_path_reads_file_and_dispatches

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
      - coverage file reading through parse_path

## Interfaces and Dependencies

The implementation should keep using the existing standard-library and crate dependencies already present in the repository: `std::path::{Path, PathBuf}`, `std::fs`, `tempfile`, `toml`, `serde_json`, and the existing public coverage and config interfaces. In `src/config.rs`, introduce a private helper that exposes the config-candidate selection boundary in a pure form. Its exact name may vary, but it must accept the same core inputs as `load_file_config_from_with_repo_root`: a starting directory and an optional repository root. Add a second private helper that parses `FileConfig` from TOML text. `load_file_config_from_with_repo_root` should remain the public-in-module orchestration point that performs the actual file read and parse work using those helpers.

In `src/coverage/mod.rs`, introduce a parsing entrypoint that accepts both the coverage text and an explicit repository root path, then make the existing `parse_str` function a thin wrapper that resolves the current directory and delegates. `parse_path` should remain the file-reading entrypoint that reads text from disk and delegates into the string-based parser path.

Revision note: created this ExecPlan to turn the current verbal classification of impure `src/` tests into a concrete, repository-specific implementation plan with explicit file targets and validation expectations.
