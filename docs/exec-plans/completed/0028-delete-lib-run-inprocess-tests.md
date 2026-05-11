---
description: "ExecPlan to eliminate tests/lib_run.rs in-process run() tests: migrate two unique scenarios to subprocess tests in cli_interface.rs and delete the four that are already covered; read when implementing or reviewing this cleanup."
---

# Delete `lib_run.rs` In-Process Tests

## Goal
- `tests/lib_run.rs` is deleted.
- The two unique behavioral scenarios it contained are preserved as subprocess tests in `tests/cli_interface.rs`.
- Actual `$GITHUB_STEP_SUMMARY` set by GitHub Actions runner is no longer written by the test suite during `cargo llvm-cov`, because all covgate invocations in tests go through the `CovgateCommand` builder which calls `env_clear()`.

## Scope
- In: `tests/lib_run.rs` (deleted), `tests/cli_interface.rs` (two new tests added).
- Out: `src/` (production code), `tests/support/` (no harness changes needed).

## Relevant Areas
- `tests/lib_run.rs` — six in-process `covgate::run(config)` tests; all call only public API so none belong inline per `docs/TESTING.md`.
- `tests/cli_interface.rs` — target for the two migrated tests; uses the `covgate(worktree).check(...).run()` subprocess builder that already calls `env_clear()`.
- `tests/support/runner.rs` — `CovgateCommand` builder; no changes needed.

## Open Questions
- None yet

## Steps

### Audit and delete redundant tests

- [x] Delete `run_with_diff_file_executes_without_untracked_warning_lookup` — covered by `diff_file_mode_skips_untracked_files_check` in `cli_interface.rs` (which is strictly more thorough: it stages an untracked file that would fail git-base mode).
- [x] Delete `run_with_git_base_errors_on_coverage_untracked_files` — covered by `git_base_mode_errors_on_coverage_untracked_files` in `cli_interface.rs`.
- [x] Delete `run_with_git_base_passes_when_no_untracked_files_exist` — the clean-worktree git-base pass is implicit in many existing `cli_interface.rs` tests.
- [x] Delete `run_with_git_base_requires_git_repo_for_coverage_path_normalization` — covered by `failure_text_requires_git_repo_when_run_outside_repository` in `cli_interface.rs`.

### Migrate unique tests to subprocess equivalents

- [x] Add `git_base_mode_passes_for_uncovered_untracked_file_with_spaces` to `tests/cli_interface.rs`:
    - Set up the rust basic-pass fixture worktree (with a committed `covgate.toml` gating `fail-under-regions = 90`).
    - Write an untracked file named `space name.rs` (not present in the fixture coverage JSON) to the worktree root.
    - Run `covgate(&worktree).check(&fixture.coverage_json()).run()`.
    - Assert `status.code() == Some(0)`.
    - Assert stderr does not contain "false pass".

- [x] Add `git_base_mode_quotes_coverage_paths_with_spaces_in_error_command` to `tests/cli_interface.rs`:
    - Set up the rust basic-pass fixture worktree (with a committed `covgate.toml` gating `fail-under-regions = 90`).
    - Write a modified coverage JSON replacing `"src/lib.rs"` with `"src/my lib.rs"` in the coverage data; save to a temp path.
    - Write `src/my lib.rs` into the worktree as an untracked file (use `fs::write`; no `git add`).
    - Run `covgate(&worktree).check(&coverage_path).run()`.
    - Assert `status.code() == Some(1)`.
    - Assert stderr contains `"git add -N 'src/my lib.rs'"` (single-quoted path due to space).

### Delete the file

- [x] Delete `tests/lib_run.rs`.

## Validation
- `cargo test --test cli_interface git_base_mode_passes_for_uncovered_untracked_file_with_spaces`
- `cargo test --test cli_interface git_base_mode_quotes_coverage_paths_with_spaces_in_error_command`
- `cargo xtask validate`

## Discoveries
- All four redundant tests were removed by deleting the entire file rather than incrementally editing it.
- Both new subprocess tests passed on first run. `cargo xtask validate` passed with 229 tests (229 previously, 2 added and 6 removed in lib_run.rs; net count held because nextest aggregates across binaries).

## Review
- All six deleted tests called only the public `covgate::run()`, `Args`, and `Config` API — deletion is correct per TESTING.md placement rule.
- Both new subprocess tests have discriminating assertions that match or strengthen the originals: status code checked and stderr content checked for each scenario.
- `env_clear()` in `CovgateCommand::run()` (runner.rs:52) confirms `GITHUB_STEP_SUMMARY` is stripped from all subprocess test invocations — Goal 3 satisfied.
- New tests follow the established setup pattern of adjacent tests in `cli_interface.rs`; no CODESTYLE violations found.
- `cargo fmt --check` and `cargo clippy` both pass on the modified file.
- Minor: Discoveries test-count arithmetic is inconsistent (229 + 2 − 6 = 225, not 229), but this is in an append-only section and does not affect code correctness; no action required.
- No unresolved findings.

## Definition of Done

### Planner
- [x] Plan is consistent, up to date, decision-complete, and ready to hand off.

### Generator
- [x] Goal achieved: `tests/lib_run.rs` deleted; two unique scenarios preserved as subprocess tests.
- [x] All planned steps are complete.
- [x] All validation commands pass; `cargo xtask validate` passes before handoff.
- [x] Handed off to an independent reviewer (MUST use the `evaluator-execplan` skill via a subagent or separate agent, not the generator agent).

### Evaluator
- [x] Standard review posture applied.
- [x] Adheres to the principles of `docs/CODESTYLE.md`.
- [x] Adheres to the principles of `docs/TESTING.md`.
- [x] Validation evidence is present and sufficient, including `cargo xtask validate` for Rust behavior changes.
- [x] All review findings have been addressed.
