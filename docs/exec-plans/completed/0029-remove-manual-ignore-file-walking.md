---
description: "Remove the manual recursive .gitignore loading logic from src/config.rs; the repo-ignore walk leaks nested rules globally and is redundant since Git already filters changed files; read when debugging path-scoped gate mismatches in mono-repo setups."
---

# ExecPlan: Remove Manual Ignore File Walking

## Background & Motivation
A user reported that a path-scoped gate (`**/*.tsx`) failed to match a file (`web/src/.../Chat.tsx`) in their mono-repo setup when `covgate` was run from the `web/` subdirectory. 

Investigation confirmed this was caused by a rule leakage bug in `covgate`'s manual `.gitignore` loading logic (`src/config.rs`). Because `GitignoreBuilder::add` was rooted at the repository root, rules in nested `.gitignore` files (like a sibling `.gitignore` containing `*`) were incorrectly applied to the entire repository, rather than being scoped to their respective subdirectories. This caused the changed files to be unintentionally ignored by `covgate`'s `PathMatcher`.

Furthermore, manual ignore file loading is fundamentally redundant for `covgate`. `covgate` relies on `git diff` (which respects ignore rules for tracked files) and `git ls-files --exclude-standard` (which respects ignore rules for untracked files). Therefore, `covgate` should trust the file list provided by Git. Removing this manual logic fixes the mono-repo bug, eliminates redundancy, and improves performance.

## Objective
- Remove the redundant and buggy manual recursive `.gitignore` loading logic from `src/config.rs`.
- Ensure `PathMatcher` no longer filters based on a synthesized repository-wide `Gitignore`.
- Add a minimal, compliant regression test using an existing native fixture to demonstrate that nested ignore files no longer leak rules globally.

## Key Files & Context
- `src/config.rs`: Contains the buggy `build_repo_ignores` and `add_gitignore_files` functions.
- `tests/cli_interface.rs`: Needs a regression test to prevent recurrence.

## Implementation Steps

### 1. Remove Manual Ignore Logic [DONE]
- In `src/config.rs`, delete the `build_repo_ignores` and `add_gitignore_files` functions. [DONE]
- Remove the `repo_ignores: Arc<Gitignore>` field from the `PathMatcher` struct. [DONE]
- Update `PathMatcher::new` to only take `root`, `include`, and `exclude` as arguments. [DONE]
- Update `PathMatcher::matches` to remove the `repo_ignore_match` check. [DONE]
- Update the instantiation of `PathMatcher` in `resolve_gates` to no longer pass `repo_ignores`. [DONE]
- Ensure no unresolved imports or test compilation errors remain in `src/config.rs`. [DONE]

### 2. Add Compliant Regression Test [DONE]
- In `tests/cli_interface.rs`, replace the bloated integration test with a new minimal test: `path_scoped_gates_ignore_files_in_siblings_do_not_leak_globally`. [DONE]
- **Constraint:** Do not construct synthetic `coverage.json` strings (Violates `TESTING.md` Philosophy #4). Use `vitest_path_scoped_gates_fixture()`. [DONE]
- The test will:
  1. Call `setup_path_scoped_fixture()` which provides the worktree and diff. [DONE]
  2. Write a `covgate.toml` configuration with path-scoped gates (e.g., `include = ["src/**/*.ts"]`). [DONE]
  3. Create a sibling directory (e.g., `sibling/`) and write a `.gitignore` containing `*` to simulate the user's environment. [DONE]
  4. Run `covgate` with `--diff-file` pointing to the generated diff and using the fixture's coverage JSON. [DONE]
  5. Assert that the process evaluates the gates properly and does not ignore files from the sibling `.gitignore` rule, proving the fix. [DONE]

## Verification & Testing [DONE]
- Run `cargo test` to ensure the new regression test passes and no existing tests break. [DONE]
- Run `cargo xtask validate` to ensure all linters, formatting, and coverage validations pass for the rust codebase. [DONE]

## Definition of Done

### Planner
- [x] Plan is decision-complete and self-contained.
- [x] Acceptance criteria are clear and falsifiable.
- [x] Key files and context are identified.

### Generator
- [x] Implementation matches the approved plan.
- [x] Living document (this plan) is updated with progress.
- [x] All tests and validations pass.
- [x] Regression test added and verified.

## Review
- [x] Added `description` frontmatter.
- Missing template sections (`Scope`, `Open Questions`, `Discoveries`) noted for future planning; acceptable for this completed plan.
- Investigation doc stale test name is acceptable — investigations are historical records.
- Implementation clean: all planned steps completed, no remaining references to removed code, 231/231 tests pass, `cargo xtask validate` passes.
- Inline tests in `src/config.rs` correctly call private items; new integration test in `tests/cli_interface.rs` exercises only the public API. Placement follows TESTING.md.

### Evaluator
- [x] Code review completed.
- [x] No regressions identified.
- [x] Acceptance criteria met.

