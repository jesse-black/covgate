---
description: "ExecPlan for refactoring the monolithic tests/support/mod.rs into submodules and cleaning up unused helpers; read when implementing or reviewing this refactor."
---

# Refactor Test Support Submodules

## Goal
- `tests/support/mod.rs` is split into logical submodules (`fixtures.rs`, `git.rs`, `runner.rs`, `parity.rs`).
- Unused helper `assert_fixture_has_no_branch_coverage` is deleted.
- Existing tests continue to pass with no changes to their import style (via re-exports in `mod.rs`).

## Scope
- In: `tests/support/mod.rs`, `tests/support/fixtures.rs`, `tests/support/git.rs`, `tests/support/runner.rs`, `tests/support/parity.rs`, `tests/cli_interface.rs` (remove `run_covgate_raw_with_path`), `tests/config_discovery.rs` and `tests/config_auto_base.rs` (remove local `run_git` copies), all other `tests/*.rs` callers updated to new signatures.
- Out: `src/` (production code), `tests/fixtures/` (data).

## Relevant Areas
- `tests/support/mod.rs` — current ~640 line monolithic helper.
- `tests/*.rs` — integration tests that rely on `mod support;`.

## Open Questions
- None yet

## Steps

### tests/support/
- [x] Create `tests/support/fixtures.rs`:
    - Add `#![allow(dead_code)]` at top.
    - Move `Fixture` struct and impl.
    - Move language-specific fixture constructors (`rust_basic_fail_fixture`, etc.).
    - Move fixture matrix functions (`fail_fixtures_with_regions`, etc.).
- [x] Create `tests/support/git.rs`:
    - Add `#![allow(dead_code)]` at top.
    - Move `init_git_repo`, `run_git`, `copy_tree`, `write_worktree_diff`.
    - Delete the local `run_git` copies in `tests/config_discovery.rs` and `tests/config_auto_base.rs` (identical to `support::run_git`); add `mod support;` and import `run_git` from support in both files.
- [x] Create `tests/support/runner.rs`:
    - Add `#![allow(dead_code)]` at top.
    - Consolidate the five `run_covgate_*` helpers (currently spread across `tests/support/mod.rs` and `tests/cli_interface.rs`) down to two:
        - `run_covgate(worktree, coverage_json, extra_args, env_vars)` — prepends `check` and the coverage path; accepts `&[(&str, &str)]` for env overrides.
        - `run_covgate_raw(worktree, args, env_vars)` — full arg control; accepts `&[(&str, &str)]` for env overrides.
    - Both helpers use `Command::env_clear()` followed by `command.env("PATH", std::env::var_os("PATH").unwrap_or_default())` then `command.envs(env_vars)`, so tests start from an empty environment rather than inheriting and selectively removing vars.
    - Delete `run_covgate_with_coverage` (pure passthrough to `run_covgate_with_env` with `&[]` — adds nothing).
    - Delete `run_covgate_with_env` (replaced by `run_covgate`).
    - Lift `run_covgate_raw_with_path` out of `tests/cli_interface.rs` and collapse it into `run_covgate_raw`: callers pass `("PATH", custom_path)` in the env_vars slice instead of a dedicated parameter.
    - Update all callers in `tests/*.rs` to match the new signatures.
- [x] Create `tests/support/parity.rs`:
    - Add `#![allow(dead_code)]` at top.
    - Move `OverallTotals`, `MetricFixtureCase`, and `native_summary_overall_totals` family of parsers.
    - Move `parse_markdown_overall_totals` and `write_absolute_path_coverage_fixture`.
    - Move `write_rebased_real_llvm_fixture`.
- [x] Update `tests/support/mod.rs`:
    - Declare `pub mod fixtures;`, `pub mod git;`, `pub mod runner;`, `pub mod parity;`.
    - Re-export all moved items at the root level so `tests/*.rs` (using `use support::*;` or `use support::Foo;`) remain unchanged.
    - Ensure `#![allow(dead_code)]` is retained.
    - Delete `assert_fixture_has_no_branch_coverage` (unused).

## Validation
- `cargo test --workspace`
- `cargo clippy --all-targets`

## Discoveries
- `assert_fixture_has_no_branch_coverage` was used in Plan 3 but is no longer present in `tests/*.rs`.
- `setup_fixture_worktree` (not listed in the plan steps) was placed in `git.rs` since it orchestrates git setup and calls `copy_tree`/`init_git_repo`.
- `mod.rs` required `#![allow(dead_code, unused_imports)]` because `pub use` re-exports trigger `unused_imports` in integration test crate contexts.
- `config_discovery.rs` only uses `run_git` from support; its local copy had a slightly different error message format but was otherwise identical.
- `run_covgate` and `run_covgate_raw` now call `env_clear()` before setting PATH and applying `env_vars`. This means the covgate subprocess no longer inherits the test runner's environment (e.g., `RUST_LOG`, `HOME`, locale vars). All current tests pass under these semantics, so no inherited vars are required. Future callers must supply any needed env vars explicitly via the `env_vars` slice.

## Review

### Finding 1 — Implementation is not committed (blocker)

The submodule files (`tests/support/fixtures.rs`, `tests/support/git.rs`, `tests/support/parity.rs`, `tests/support/runner.rs`) exist in the working tree as **untracked** files. The changes to `tests/support/mod.rs`, `tests/cli_interface.rs`, `tests/config_discovery.rs`, `tests/config_auto_base.rs`, `tests/llvm_diff_regression.rs`, and `tests/llvm_real_parity.rs` are **unstaged modifications**. Nothing from this plan has been committed.

`git status --short` output at review time:
```
 M docs/exec-plans/active/0025-refactor-test-support-submodules.md
 M tests/cli_interface.rs
 M tests/config_auto_base.rs
 M tests/config_discovery.rs
 M tests/llvm_diff_regression.rs
 M tests/llvm_real_parity.rs
 M tests/support/mod.rs
?? tests/support/fixtures.rs
?? tests/support/git.rs
?? tests/support/parity.rs
?? tests/support/runner.rs
```

The generator checked off "All validation commands pass" without staging or committing. `cargo test --workspace` and `cargo clippy --all-targets` may well pass against the working tree, but the branch does not carry the change. The plan cannot close until the work is committed.

**Required action**: Stage and commit all changed and new files for this plan.

### Finding 2 — `env_clear()` semantics change is unannounced (minor)

The old `run_covgate_with_env` (and `run_covgate_raw`) ran the binary inheriting the full test-runner environment plus the explicit `env_vars`. The new `run_covgate` and `run_covgate_raw` in `runner.rs` call `env_clear()` first, then restore only PATH, then apply `env_vars`. This is a semantics change: any env var the test process happened to have (e.g., `RUST_LOG`, `HOME`, locale vars) is no longer forwarded to the covgate subprocess.

The change is intentional and aligns with the plan spec, and the reviewed tests do not appear to depend on inherited vars. The Discoveries section should note this behavioral change explicitly so future callers of `run_covgate` know they must supply any required env vars explicitly.

**Required action**: Add a note to Discoveries documenting the env_clear semantics and why it is safe for the current test suite.

### Finding 3 — `OverallTotals` duplication in `llvm_real_parity.rs` pre-exists, not introduced here (observation)

`tests/llvm_real_parity.rs` defines a private `OverallTotals` struct (lines 8–12) instead of importing `support::OverallTotals` from `parity.rs`. This duplicates a type definition (CODESTYLE principle 2). This pre-dates this plan and was not introduced by it, so it is not a blocker for this plan. However, it should be cleaned up in a follow-on task or noted in `docs/TODO.md`.

**Required action**: Add to `docs/TODO.md` that `llvm_real_parity.rs` should use `support::OverallTotals` and delete its local copy.

## Definition of Done

### Planner
- [x] Plan is consistent, up to date, decision-complete, and ready to hand off.

### Generator
- [x] Goal achieved: tests/support/ refactored into logical submodules with re-exports.
- [x] All planned steps are complete.
- [x] All validation commands pass.
- [x] Handed off to an independent reviewer (MUST use the `evaluator-execplan` skill via a subagent or separate agent, not the generator agent).

### Evaluator
- [ ] Standard review posture applied.
- [ ] Adheres to the principles of `docs/CODESTYLE.md`.
- [ ] Adheres to the principles of `docs/TESTING.md`.
- [ ] All review findings have been addressed.
