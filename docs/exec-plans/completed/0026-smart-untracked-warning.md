---
description: "ExecPlan for filtering the untracked-files check to only coverage-relevant files and promoting it to a fail-fast error; read when implementing or reviewing this change."
---

# Smart Untracked-Files Check: Intersect with Coverage Files and Fail Fast

## Goal
`emit_untracked_files_warning` is replaced by a fail-fast error that only triggers when at least one untracked file is also present in the coverage report. Untracked files with no coverage entry are silently ignored. When coverage-relevant untracked files are found, covgate exits non-zero with a clear error message and the exact `git add -N` command to fix it.

## Scope
- In: `src/lib.rs` — `run`, `load_changed_lines_with_warnings`, `emit_untracked_files_warning` (renamed), and their private helpers.
- In: `tests/cli_interface.rs` — existing untracked-warning tests updated to assert error exit; new intersection tests added.
- In: `tests/lib_run.rs` — existing untracked-warning tests updated to assert `Err`; new intersection tests added.
- Out: `src/git.rs` — `list_untracked_files` is unchanged.
- Out: `src/diff.rs`, `src/config.rs`, `src/model.rs`, renderers, parsers.

## Decisions

### Warning vs. error promotion: promote to fail-fast error
Once intersection filtering eliminates false positives, every remaining hit is a genuine problem: an untracked file that covgate tracks but cannot see in the diff, which produces a silent false pass. The fix is always the same trivially-copy-paste command (`git add -N <path>`) that the error message provides. Keeping it as a warning leaves users unknowingly running on bad data after an expensive coverage run; erroring stops them immediately. Truly new files that will never be tracked are rare, so the blast radius of a hard error is low. No suppression flag is needed because the intersection filter already acts as the suppression mechanism for irrelevant files.

## Relevant Areas
- `src/lib.rs:20-34` — `run` loads `report` before calling `load_changed_lines_with_warnings`; the report is already available to thread through.
- `src/lib.rs:236-246` — `supported_files(report)` already computes the `BTreeSet<PathBuf>` of repo-relative covered file paths.
- `src/lib.rs:248-268` — `load_changed_lines_with_warnings` and `emit_untracked_files_warning` — the two functions to change.
- `src/git.rs:129-142` — `list_untracked_files()` returns repo-relative `Vec<String>`; unchanged.
- `tests/cli_interface.rs:397-466` — existing untracked-warning CLI tests; `git_base_mode_warns_about_untracked_files` and `diff_file_mode_skips_untracked_files_warning`.
- `tests/lib_run.rs:58-125` — existing lib-level untracked-warning tests; `run_with_git_base_checks_untracked_files_before_loading_diff`, `run_with_git_base_quotes_paths_in_add_command_when_needed`, `run_with_git_base_skips_warning_when_no_untracked_files_exist`.

## Open Questions
None.

## Steps

### src/lib.rs — production changes
- [x] In `run`: call `supported_files(&report)` before `load_changed_lines_with_warnings` and bind to `coverage_files`; pass `&coverage_files` as a second argument to `load_changed_lines_with_warnings`.
- [x] Change `load_changed_lines_with_warnings` signature to accept `coverage_files: &BTreeSet<std::path::PathBuf>`; pass `coverage_files` through to the check function.
- [x] Rename `emit_untracked_files_warning` to `check_untracked_coverage_files`; update its signature to accept `coverage_files: &BTreeSet<std::path::PathBuf>`.
- [x] Inside `check_untracked_coverage_files`: after obtaining `untracked_files`, filter to only those whose `PathBuf` appears in `coverage_files`; bind the filtered list as `relevant`; if `relevant.is_empty()` return `Ok(())`.
- [x] When `relevant` is non-empty, return `Err(...)` (using the existing error type) with a message that names the coverage-relevance criterion and includes the exact fix command, e.g.: `"untracked files appear in the coverage report and are excluded from diff gating, which produces a false pass — add them with: \`{add_command}\`"`. Do not print to stderr and continue; the error propagates and causes a non-zero exit.

### tests/cli_interface.rs — test updates
- [x] Rename and update `git_base_mode_warns_about_untracked_files` → `git_base_mode_errors_on_coverage_untracked_files`: uses `git rm --cached src/lib.rs` to make the coverage-present file untracked. Asserts non-zero exit and stderr contains fix command.
- [x] Add new test `git_base_mode_passes_for_uncovered_untracked_file`: create an untracked file at a path not in coverage (e.g., `new_untracked.rs`). Assert the command exits zero and stderr does not contain the error text.
- [x] Rename `diff_file_mode_skips_untracked_files_warning` → `diff_file_mode_skips_untracked_files_check`; assertion updated from "Untracked-files warning" to "false pass".

### tests/lib_run.rs — test updates
- [x] Rename `run_with_git_base_checks_untracked_files_before_loading_diff` → `run_with_git_base_errors_on_coverage_untracked_files`: uses `git rm --cached src/lib.rs`. Asserts `Err` with fix command.
- [x] Rename `run_with_git_base_quotes_paths_in_add_command_when_needed` → `run_with_git_base_passes_for_uncovered_untracked_file_with_spaces`: now asserts `Ok` (non-coverage untracked file). Added new test `run_with_git_base_quotes_coverage_paths_with_spaces_in_error_command` using a custom coverage JSON with `src/my lib.rs`; asserts error message contains properly shell-quoted path.
- [x] Rename `run_with_git_base_skips_warning_when_no_untracked_files_exist` → `run_with_git_base_passes_when_no_untracked_files_exist`; intent unchanged.

## Validation
- `cargo test git_base_mode_errors_on_coverage_untracked_files`
- `cargo test git_base_mode_passes_for_uncovered_untracked_file`
- `cargo test diff_file_mode_skips_untracked_files_check`
- `cargo test --test lib_run run_with_git_base_errors_on_coverage_untracked_files`
- `cargo test --test lib_run run_with_git_base_passes_for_uncovered_untracked_file_with_spaces`
- `cargo test --test lib_run run_with_git_base_quotes_coverage_paths_with_spaces_in_error_command`
- `cargo test --test lib_run run_with_git_base_passes_when_no_untracked_files_exist`
- `cargo test --workspace`
- `cargo xtask validate`

## Discoveries
- `run` in `src/lib.rs` already loads `report` (line 32) before the warning is emitted (line 33 via `load_changed_lines_with_warnings`), so threading `supported_files(&report)` through requires no reordering of pipeline steps.
- `supported_files` returns `BTreeSet<PathBuf>` with repo-relative paths. `list_untracked_files` returns `Vec<String>` of repo-relative paths. Converting each untracked string to `PathBuf` and checking set membership is a one-liner per path.
- The rust basic-pass fixture's coverage JSON references exactly one file: `src/lib.rs`. Tests that need a "coverage-present" untracked file should use this path.
- `run_with_git_base_quotes_paths_in_add_command_when_needed` currently relies on a non-coverage file to trigger the warning path; it will silently pass after this change. The generator must resolve this per the guidance in Steps.

## Review

### Finding 1 — CODESTYLE: `list_untracked_files` private wrapper does not earn its layer

**File:** `src/lib.rs:281-283`

```rust
fn list_untracked_files() -> Result<Vec<String>> {
    crate::git::list_untracked_files()
}
```

This is a pure single-line alias. It adds no logic, no error transformation, no testability seam. `check_untracked_coverage_files` should call `crate::git::list_untracked_files()` directly. CODESTYLE Principle 4: "Earn every layer."

---

### Finding 2 — PLAN: Validation section lists two stale test names

**File:** `docs/exec-plans/active/0026-smart-untracked-warning.md`, Validation section

The Validation section still lists the old pre-rename test names that no longer exist in the codebase:

- `cargo test run_with_git_base_checks_untracked_files_before_loading_diff` — renamed to `run_with_git_base_errors_on_coverage_untracked_files`
- `cargo test run_with_git_base_quotes_paths_in_add_command_when_needed` — renamed to `run_with_git_base_passes_for_uncovered_untracked_file_with_spaces`

Running these commands produces 0 tests filtered in (they silently pass without exercising anything). The three new test names introduced by the generator are also absent from the Validation list:

- `run_with_git_base_errors_on_coverage_untracked_files`
- `run_with_git_base_passes_for_uncovered_untracked_file_with_spaces`
- `run_with_git_base_quotes_coverage_paths_with_spaces_in_error_command`

The Validation section must be updated to reflect the actual test names.

---

### Finding 3 — TESTING: `diff_file_mode_skips_untracked_files_check` does not assert the intended behavior

**File:** `tests/cli_interface.rs:443-473`

The test creates `new_untracked.rs`, a file that is not present in the coverage report. Using a non-coverage untracked file means the test cannot distinguish the "diff mode skips the check entirely" behavior from the "git-base mode silently ignores non-coverage untracked files" behavior — both exit zero and emit no error text for a non-coverage file.

To assert that diff mode actually bypasses the check, the untracked file must be one that is present in the coverage report (e.g., `src/lib.rs` via `git rm --cached`). With a coverage-tracked untracked file: git-base mode errors, diff-file mode passes. The current test cannot falsify a broken implementation that skips diff-mode suppression but still passes because the file has no coverage entry.

TESTING.md Principle 5: "A test's value is what it can falsify."

---

### Finding 4 — TESTING: `git_base_mode_errors_on_coverage_untracked_files` uses `assert_ne` where `assert_eq!(Some(1))` is the codebase norm

**File:** `tests/cli_interface.rs:413`

```rust
assert_ne!(output.status.code(), Some(0));
```

Every other error-exit assertion in `cli_interface.rs` uses `assert_eq!(output.status.code(), Some(1))`. The `assert_ne` form passes for any non-zero exit code including 2 (clap usage error) and `None` (killed process). `run()` errors propagate through `main` via `anyhow::Result<()>` and exit with code 1; the assertion should be `assert_eq!(output.status.code(), Some(1))` to match that contract and the surrounding test file convention.

### Re-review — 2026-05-11

No open findings. The current branch addresses the prior review notes:

- `src/lib.rs` now calls `crate::git::list_untracked_files()` directly rather than using an unearned private wrapper.
- `docs/exec-plans/active/0026-smart-untracked-warning.md` lists the current validation commands.
- `tests/cli_interface.rs::diff_file_mode_skips_untracked_files_check` now uses `git rm --cached src/lib.rs`, so diff-file mode is tested with a coverage-present untracked file.
- `tests/cli_interface.rs::git_base_mode_errors_on_coverage_untracked_files` now asserts `Some(1)`.

Validation evidence: `cargo xtask validate` passed on 2026-05-11.

## Definition of Done

### Planner
- [x] Plan is consistent, up to date, decision-complete, and ready to hand off.

### Generator
- [x] Goal achieved: check errors (not warns) when untracked files intersect with coverage-tracked files; passes silently otherwise.
- [x] All planned steps are complete.
- [x] All validation commands pass.
- [x] Handed off to an independent reviewer (MUST use the `evaluator-execplan` skill via a subagent or separate agent, not the generator agent).

### Evaluator
- [x] Standard review posture applied.
- [x] Adheres to the principles of `docs/CODESTYLE.md`.
- [x] Adheres to the principles of `docs/TESTING.md`.
- [x] All review findings have been addressed.
