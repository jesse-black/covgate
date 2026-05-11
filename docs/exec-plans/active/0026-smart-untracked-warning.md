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
- [ ] In `run`: call `supported_files(&report)` before `load_changed_lines_with_warnings` and bind to `coverage_files`; pass `&coverage_files` as a second argument to `load_changed_lines_with_warnings`.
- [ ] Change `load_changed_lines_with_warnings` signature to accept `coverage_files: &BTreeSet<std::path::PathBuf>`; pass `coverage_files` through to the check function.
- [ ] Rename `emit_untracked_files_warning` to `check_untracked_coverage_files`; update its signature to accept `coverage_files: &BTreeSet<std::path::PathBuf>`.
- [ ] Inside `check_untracked_coverage_files`: after obtaining `untracked_files`, filter to only those whose `PathBuf` appears in `coverage_files`; bind the filtered list as `relevant`; if `relevant.is_empty()` return `Ok(())`.
- [ ] When `relevant` is non-empty, return `Err(...)` (using the existing error type) with a message that names the coverage-relevance criterion and includes the exact fix command, e.g.: `"untracked files appear in the coverage report and are excluded from diff gating, which produces a false pass — add them with: \`{add_command}\`"`. Do not print to stderr and continue; the error propagates and causes a non-zero exit.

### tests/cli_interface.rs — test updates
- [ ] Rename and update `git_base_mode_warns_about_untracked_files` → `git_base_mode_errors_on_coverage_untracked_files`: replace the `new_untracked.rs` untracked file with a file at a path present in the fixture's coverage report (e.g., `src/lib.rs` removed from index). Assert the command exits non-zero and stderr contains the fix command naming that path.
- [ ] Add new test `git_base_mode_passes_for_uncovered_untracked_file`: create an untracked file at a path not in coverage (e.g., `new_untracked.rs`). Assert the command exits zero and stderr does not contain the error text.
- [ ] Rename `diff_file_mode_skips_untracked_files_warning` → `diff_file_mode_skips_untracked_files_check` (diff-file mode never errors regardless of coverage membership); update assertions to match error-vs-warning framing if needed.

### tests/lib_run.rs — test updates
- [ ] Rename and update `run_with_git_base_checks_untracked_files_before_loading_diff`: replace `new_untracked.rs` with a coverage-present file removed from index. Assert the result is `Err` and the error message contains the fix command.
- [ ] Update `run_with_git_base_quotes_paths_in_add_command_when_needed`: this test uses `"space name.rs"` which is not in coverage — it will no longer trigger an error. Replace with a coverage-present file whose path contains a space, or if that is impractical in the harness, add a separate test for quoting using a coverage-present path with a space and convert the original to assert `Ok` behavior for an irrelevant untracked file. The generator should choose the simplest option that preserves quoting coverage.
- [ ] Update `run_with_git_base_skips_warning_when_no_untracked_files_exist`: rename to `run_with_git_base_passes_when_no_untracked_files_exist`; intent unchanged (no untracked files → `Ok`).

## Validation
- `cargo test git_base_mode_errors_on_coverage_untracked_files`
- `cargo test git_base_mode_passes_for_uncovered_untracked_file`
- `cargo test diff_file_mode_skips_untracked_files_check`
- `cargo test run_with_git_base_checks_untracked_files_before_loading_diff`
- `cargo test run_with_git_base_quotes_paths_in_add_command_when_needed`
- `cargo test run_with_git_base_passes_when_no_untracked_files_exist`
- `cargo test --workspace`
- `cargo xtask validate`

## Discoveries
- `run` in `src/lib.rs` already loads `report` (line 32) before the warning is emitted (line 33 via `load_changed_lines_with_warnings`), so threading `supported_files(&report)` through requires no reordering of pipeline steps.
- `supported_files` returns `BTreeSet<PathBuf>` with repo-relative paths. `list_untracked_files` returns `Vec<String>` of repo-relative paths. Converting each untracked string to `PathBuf` and checking set membership is a one-liner per path.
- The rust basic-pass fixture's coverage JSON references exactly one file: `src/lib.rs`. Tests that need a "coverage-present" untracked file should use this path.
- `run_with_git_base_quotes_paths_in_add_command_when_needed` currently relies on a non-coverage file to trigger the warning path; it will silently pass after this change. The generator must resolve this per the guidance in Steps.

## Review
None yet.

## Definition of Done

### Planner
- [x] Plan is consistent, up to date, decision-complete, and ready to hand off.

### Generator
- [ ] Goal achieved: check errors (not warns) when untracked files intersect with coverage-tracked files; passes silently otherwise.
- [ ] All planned steps are complete.
- [ ] All validation commands pass.
- [ ] Handed off to an independent reviewer (MUST use the `evaluator-execplan` skill via a subagent or separate agent, not the generator agent).

### Evaluator
- [ ] Standard review posture applied.
- [ ] Adheres to the principles of `docs/CODESTYLE.md`.
- [ ] Adheres to the principles of `docs/TESTING.md`.
- [ ] All review findings have been addressed.
