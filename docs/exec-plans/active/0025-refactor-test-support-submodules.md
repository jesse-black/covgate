---
description: "ExecPlan for refactoring the monolithic tests/support/mod.rs into submodules and cleaning up unused helpers; read when implementing or reviewing this refactor."
---

# Refactor Test Support Submodules

## Goal
- `tests/support/mod.rs` is split into logical submodules (`fixtures.rs`, `git.rs`, `runner.rs`, `parity.rs`).
- Unused helper `assert_fixture_has_no_branch_coverage` is deleted.
- Existing tests continue to pass with no changes to their import style (via re-exports in `mod.rs`).

## Scope
- In: `tests/support/mod.rs`, `tests/support/fixtures.rs`, `tests/support/git.rs`, `tests/support/runner.rs`, `tests/support/parity.rs`, `tests/cli_interface.rs` (remove `run_covgate_raw_with_path`; migrate all `run_covgate`/`run_covgate_raw` call sites to the fluent builder), `tests/config_discovery.rs` and `tests/config_auto_base.rs` (remove local `run_git` copies), `tests/llvm_diff_regression.rs` and `tests/llvm_real_parity.rs` (migrate `run_covgate_raw` call sites), all other `tests/*.rs` callers updated to new signatures.
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

### Replace run_covgate / run_covgate_raw with a fluent builder (resolves Finding 4)

- [ ] Rewrite `tests/support/runner.rs` around a single fluent builder; delete `run_covgate` and `run_covgate_raw`:
    - Entrypoint free function: `pub fn covgate(worktree: &Path) -> CovgateCommand`.
    - `CovgateCommand` owns: `worktree: PathBuf`, `args: Vec<OsString>`, `envs: Vec<(OsString, OsString)>`.
    - Methods, all consuming `self` and returning `Self`:
        - `fn check(self, coverage_json: &Path) -> Self` — pushes `"check"` and the coverage path onto `args` (convenience used by every `run_covgate` caller today).
        - `fn arg(self, arg: impl Into<OsString>) -> Self`
        - `fn args<I, S>(self, args: I) -> Self where I: IntoIterator<Item = S>, S: Into<OsString>`
        - `fn env(self, key: impl Into<OsString>, value: impl Into<OsString>) -> Self`
    - Terminal `fn run(self) -> Output`:
        - `Command::new(env!("CARGO_BIN_EXE_covgate"))`
        - `command.env_clear()`
        - `command.env("PATH", std::env::var_os("PATH").unwrap_or_default())` (callers that need to override PATH simply chain `.env("PATH", "")` after; the later `.env` wins).
        - `command.envs(self.envs)`
        - `command.args(self.args)`
        - `command.current_dir(self.worktree)`
        - `.output().expect("covgate should run")`.
    - Re-export `covgate` and `CovgateCommand` from `tests/support/mod.rs`.
- [ ] Migrate every `run_covgate(...)` and `run_covgate_raw(...)` call site to the builder. Required equivalences:
    - `run_covgate(w, cov, &[], &[])` → `covgate(w).check(cov).run()`.
    - `run_covgate(w, cov, &[a, b, ...], &[])` → `covgate(w).check(cov).args([a, b, ...]).run()`.
    - `run_covgate(w, cov, &[args...], &[(K, V), ...])` → `covgate(w).check(cov).args([args...]).env(K, V)....run()`. Today this pattern appears 5× in `tests/cli_interface.rs` (lines 669, 701, 736, 806, 837), all `GITHUB_STEP_SUMMARY`.
    - `run_covgate_raw(w, &[args...], &[])` → `covgate(w).args([args...]).run()` (or `.arg(a)` when there is a single arg).
    - `run_covgate_raw(w, &[args...], &[("PATH", "")])` → `covgate(w).args([args...]).env("PATH", "").run()`. Today this pattern appears 2× in `tests/cli_interface.rs` (lines 111 and 156).
    - Callers should drop the `to_string()` chains on string literals where the builder's `Into<OsString>` bounds make them unnecessary.
- [ ] Use ast-grep for the mechanical rewrites where the call shape is uniform; consult the `ast-grep` skill for rule authoring. Hand-edit the heterogeneous multi-line calls. After the migration, `ast-grep --lang rust -p 'run_covgate($$$)' tests/` and `ast-grep --lang rust -p 'run_covgate_raw($$$)' tests/` must return no matches.
- [ ] Delete the now-unused `run_covgate`, `run_covgate_raw`, and any related re-exports from `tests/support/mod.rs`.

## Validation
- `cargo test --workspace`
- `cargo clippy --all-targets`

## Discoveries
- `assert_fixture_has_no_branch_coverage` was used in Plan 3 but is no longer present in `tests/*.rs`.
- `setup_fixture_worktree` (not listed in the plan steps) was placed in `git.rs` since it orchestrates git setup and calls `copy_tree`/`init_git_repo`.
- `mod.rs` required `#![allow(dead_code, unused_imports)]` because `pub use` re-exports trigger `unused_imports` in integration test crate contexts.
- `config_discovery.rs` only uses `run_git` from support; its local copy had a slightly different error message format but was otherwise identical.
- `run_covgate` and `run_covgate_raw` now call `env_clear()` before setting PATH and applying `env_vars`. This means the covgate subprocess no longer inherits the test runner's environment (e.g., `RUST_LOG`, `HOME`, locale vars). All current tests pass under these semantics, so no inherited vars are required. Future callers must supply any needed env vars explicitly via the `env_vars` slice.
- Call-site audit (2026-05-11) of Finding 4: `run_covgate` has 5 call sites in `tests/cli_interface.rs` passing a non-empty `env_vars` slice (`GITHUB_STEP_SUMMARY`), and `run_covgate_raw` has 2 call sites passing `&[("PATH", "")]`. The finding's premise that `env_vars` is `&[]` at virtually every call site is wrong, so the proposed `run_covgate_raw_with_env` split would not collapse to a single exceptional caller. Resolution shifted to a fluent builder; see Review note below.

## Review

- [ ] Finding 3 — `OverallTotals` duplication in `llvm_real_parity.rs`. `tests/llvm_real_parity.rs` defines a private `OverallTotals` struct (lines 8–12) instead of importing `support::OverallTotals` from `parity.rs`. This duplicates a type definition (CODESTYLE principle 2).
- [ ] Finding 4 — `env_vars` parameter is `&[]` at virtually every call site (CODESTYLE principle 4: parameter not earning its place). Fix: remove `env_vars` from `run_covgate` entirely (no caller passes a non-empty value); remove `env_vars` from `run_covgate_raw` and introduce `run_covgate_raw_with_env` for the single caller that passes a non-empty env slice. This is a large bulk call-site refactor; use `ast-grep` (via the `ast-grep` skill) to rewrite call sites mechanically.
- [ ] Finding 4 — revised resolution (planner, 2026-05-11). The original fix is rejected on factual grounds (see Discoveries: 5 `run_covgate` callers and 2 `run_covgate_raw` callers pass non-empty env). Splitting into `_with_env` variants would still leave a populated variant and add a fourth helper for symmetry. Replace `run_covgate` + `run_covgate_raw` with a single fluent `covgate(worktree).check(coverage).args(...).env(...).run()` builder per the new Steps subsection, which (a) leaves trivial arguments off the call site entirely instead of pushing them into `&[]`, (b) collapses two helpers into one, satisfying CODESTYLE principle 4 without manufacturing a `_with_env` cousin, and (c) lets callers that need `PATH=""` overrides express it inline with `.env("PATH", "")` instead of a dedicated parameter. Finding stays open until the Steps subsection is complete and `ast-grep --lang rust -p 'run_covgate($$$)' tests/` returns no matches.

## Definition of Done

### Planner
- [x] Plan is consistent, up to date, decision-complete, and ready to hand off. (Revised 2026-05-11: Finding 4 resolution switched to fluent builder; Steps section extended.)

### Generator
- [x] Goal achieved: tests/support/ refactored into logical submodules with re-exports.
- [ ] All planned steps are complete. (Re-opened: fluent-builder migration added to address Finding 4.)
- [ ] All validation commands pass. (Re-run after the fluent-builder migration.)
- [ ] Handed off to an independent reviewer (MUST use the `evaluator-execplan` skill via a subagent or separate agent, not the generator agent).

### Evaluator
- [ ] Standard review posture applied.
- [ ] Adheres to the principles of `docs/CODESTYLE.md`.
- [ ] Adheres to the principles of `docs/TESTING.md`.
- [ ] All review findings have been addressed.
