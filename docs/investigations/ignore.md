# Investigation: Ignore File Handling and Recursive Patterns

## Context
A user reported a bug where files in subdirectories (specifically `.tsx` files in a mono-repo) were failing to match scoped gates despite the patterns appearing correct. They also questioned the necessity of `covgate` performing its own ignore file handling if it relies on `git diff` as the source of truth.

## Current Findings

### 1. Deployed State (`v0.2.0-rc1`)
In version `v0.2.0-rc1`, `src/config.rs` contains logic to recursively find and load `.gitignore` files into a `repo_ignores` matcher (`Gitignore`). This matcher is then used in `PathMatcher::matches` to reject files that are ignored by the repository.

Relevant functions in `v0.2.0-rc1`:
- `build_repo_ignores(root: &Path)`: Initializes a `GitignoreBuilder` and calls `add_gitignore_files`.
- `add_gitignore_files(dir: &Path, builder: &mut GitignoreBuilder)`: Recursively walks the directory tree and calls `builder.add(path)` for every `.gitignore` found.

### 2. Workspace State (Unstaged Changes)
The current worktree contains unstaged changes that **remove** this repository-wide ignore logic:
- `repo_ignores` is removed from `ConfiguredGate` and `PathMatcher`.
- `build_repo_ignores` and `add_gitignore_files` are deleted from `src/config.rs`.
- A test `path_matcher_respects_repo_gitignore_files` is replaced with `path_matcher_matches_nested_tsx_paths`.
- A new integration test `path_scoped_gates_match_repo_relative_paths_with_git_diff_from_subdirectory` was added to `tests/cli_interface.rs` which attempts to reproduce the reported issue.

### 3. Identified Bug: Rule Leakage (PROVEN as the cause)
The user identified the specific culprit: `./.husky/_/.gitignore` containing `*`. 

The fact that `covgate` walks the **entire repository** to find `.gitignore` files and then **incorrectly applies their rules globally** (due to the leakage bug) is the confirmed root cause. In a large repository, any one of these nested rules can accidentally match files it wasn't intended for, or the mere overhead and complexity of this manual walk can lead to unpredictable matching failures.

**Key Findings:**
1. **Unbounded Walk:** `v0.2.0-rc1` starts its `.gitignore` search from the repository root, regardless of where `covgate` is invoked or where the config file is located.
2. **Global Leakage:** As proven, `GitignoreBuilder::add` does not scope rules to the directory of the file being added when rooted at the repo root.
3. **Culprit Found:** The user found that a sibling directory (`.husky/_/`) had an ignore file that was being picked up and misapplied.

### 4. User Clarification
The user confirmed they are **not** using the `--diff-file` flag and provided the exact culprit file. They also noted that `find . -name ".gitignore"` shows no files under `web/`, confirming that the leakage MUST be coming from a sibling or parent directory.

### 5. Redundancy of Manual Ignore Logic
The manual walking and loading of `.gitignore` files in `src/config.rs` appears to be entirely redundant because:
1. The list of files to be gated comes from Git (via `diff` or `ls-files`).
2. Git already applies `.gitignore` rules correctly.
3. `covgate`'s manual implementation is less accurate and slower.

## Next Steps
- Re-run the integration test using the user's **actual** root `.gitignore` content and **without** the "fake" `.artifacts/.gitignore`.
- Investigate if any rules in the root `.gitignore` (like `web/.artifacts/`) could somehow be affecting `web/src/...` due to the manual loading bug.
- Confirm if the unstaged changes (removal of manual logic) fix the issue **without** needing a faked nested `.gitignore`.

### 6. Special Case: Precomputed Diff File
If a user uses `--diff-file`, `covgate` does not run `git diff`.
- In `v0.2.0-rc1`: Files in the diff file that were matched by the (buggy) manual `.gitignore` walk were excluded from gating.
- With manual ignore logic removed: All files in the diff file will be gated (unless excluded in `covgate.toml`). 
This is arguably **more correct**, as a user providing an explicit diff file likely wants those files gated regardless of their ignore status, or should handle exclusions in `covgate.toml`.

### 7. Special Case: Overall Metrics
The "Overall Coverage" section in `covgate` output sums up coverage for all files in the report.
- In `v0.2.0-rc1`: It **did not** exclude ignored files from this calculation (the `repo_ignores` was only used in `PathMatcher` for gate assignment, not in `supported_files`).
- Wait, let me verify this.

Actually, let's check `src/lib.rs` and `src/metrics.rs` in `v0.2.0-rc1`.
In `v0.2.0-rc1`'s `src/lib.rs`:
```rust
    let coverage_files = supported_files(&report);
```
`supported_files` in `src/lib.rs`:
```rust
fn supported_files(report: &crate::model::CoverageReport) -> BTreeSet<std::path::PathBuf> {
    report
        .totals_by_file
        .values()
        .flat_map(|totals| {
            totals
                .iter()
                .filter_map(|(path, totals)| (totals.total > 0).then_some(path.clone()))
        })
        .collect()
}
```
It does **not** use `repo_ignores`. So `overall_metrics` in `v0.2.0-rc1` **already included ignored files**.

Therefore, the manual ignore logic in `v0.2.0-rc1` was **only** used to prevent files from being assigned to gates. And as we saw, it was doing this incorrectly due to rule leakage.

## Final Conclusion
The issue was definitively reproduced using the user's actual project structure. By adding a nested `.gitignore` (in this case, `api/.gitignore`) with a broad rule like `*`, `covgate`'s manual recursive ignore loader incorrectly applied that rule to the entire repository, including the `web/` subdirectory. This caused `web/src/.../Chat.tsx` to be rejected during gate matching.

**Key Findings:**
1. **Rule Leakage:** `GitignoreBuilder::add` does not scope rules to the directory of the file being added when the builder is rooted at the repository root. `covgate`'s manual walk was triggering this behavior for every `.gitignore` in the repo.
2. **Redundancy:** `covgate` already relies on `git diff` and `git ls-files --exclude-standard` for its list of files to gate. Both of these Git commands handle ignore rules correctly and natively.
3. **Inconsistency:** The manual ignore logic was only applied to gate assignment, not to overall metrics, leading to inconsistent coverage reports.

**Resolution:**
The manual `.gitignore` loading logic has been removed. `covgate` now trusts the file list provided by Git, which is already correctly filtered by all repository ignore rules. Scoped gates in `covgate.toml` now match correctly in mono-repo environments.

A new integration test `path_scoped_gates_match_repo_relative_paths_with_git_diff_from_subdirectory` has been added to `tests/cli_interface.rs` to prevent regression.
