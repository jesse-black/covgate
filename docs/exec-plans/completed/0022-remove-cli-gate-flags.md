---
description: "ExecPlan for removing the ten CLI gate threshold flags from covgate, leaving only workflow flags; read when implementing or reviewing this removal."
---

# Remove CLI Gate Threshold Flags

## Goal
- The ten CLI gate threshold flags (`--fail-under-{regions,lines,branches,functions,named-functions}` and `--fail-uncovered-{regions,lines,branches,functions,named-functions}`) are removed from the CLI. `covgate.toml` is the sole gate configuration surface. Workflow flags (`--base`, `--diff-file`, `--markdown-output`) are unchanged.

## Scope
- In: `src/cli.rs`, `src/config.rs`, `tests/cli_metrics.rs` (deleted entirely), specific tests in `tests/cli_interface.rs`, `README.md`.
- Out: workflow flags (`--base`, `--diff-file`, `--markdown-output`), `covgate.toml` schema, gate evaluation logic, renderers, parsers, `src/model.rs`, `src/gate.rs`, `src/metrics.rs`.

## Relevant Areas
- `src/cli.rs` — defines all ten flag fields on `Args`; all ten must be removed.
- `src/config.rs` — `resolve_gate_rules`, `has_cli_rules`, the `TryFrom<Args>` destructuring pattern, and the `no-rules-configured` error message all reference the flags; all must be updated.
- `tests/cli_metrics.rs` — 467 lines, 18 tests; every test exercises CLI flag-to-gate wiring. Delete the file entirely.
- `tests/cli_interface.rs` — 13 tests pass CLI gate flags as arguments; these need to be converted to config-file equivalents or deleted (see Steps for each decision). 4 tests exercise the `path_scoped_gates_*` config-only path and are untouched.
- `README.md` — the "CLI Reference" section line `Every threshold in \`covgate.toml\` has a corresponding CLI flag for one-off use. Run \`covgate --help\` for the full list.` must be removed.
- `docs/exec-plans/active/0023-minimal-console-rule-display.md` — plan 23 includes steps to update double-space assertions in `tests/cli_metrics.rs` and `tests/cli_interface.rs`. With plan 22 applied first: `tests/cli_metrics.rs` no longer exists, so plan 23's step 4 becomes "no-op — file deleted by plan 22." Plan 23's step 5 still applies to `tests/cli_interface.rs` assertions that survive plan 22.

## Open Questions
- None.

## Steps

### src/cli.rs

- [x] Remove the ten gate threshold fields from `Args`:
  - `fail_under_regions: Option<f64>`
  - `fail_under_lines: Option<f64>`
  - `fail_under_branches: Option<f64>`
  - `fail_under_functions: Option<f64>`
  - `fail_under_named_functions: Option<f64>`
  - `fail_uncovered_regions: Option<usize>`
  - `fail_uncovered_lines: Option<usize>`
  - `fail_uncovered_branches: Option<usize>`
  - `fail_uncovered_functions: Option<usize>`
  - `fail_uncovered_named_functions: Option<usize>`
  - Retain `coverage_report`, `base`, `diff_file`, and `markdown_output`.

### src/config.rs

- [x] In `TryFrom<Args> for Config`: remove the ten threshold field bindings from the exhaustive `Args { ... }` destructuring pattern (lines 96–111). The pattern currently binds them as `_`; remove all ten `fail_under_*/fail_uncovered_*: _` entries.
- [x] Delete `resolve_gate_rules` (lines 316–521) in its entirety: the function's only job is merging CLI thresholds with config values, a concept that no longer exists. Gate rules now come from config only.
- [x] Delete `has_cli_rules` (lines 553–568) in its entirety.
- [x] Simplify `push_percent_rule` (lines 523–536) and `push_uncovered_rule` (lines 538–551): strip the three-parameter CLI-override signature down to a two-parameter config-only form and retain both as helpers for `gate_rules_from_config` (10 call sites justify keeping them rather than inlining the conditional-push pattern).
- [x] Rewrite `resolve_gates` (lines 244–314) to build `Vec<ConfiguredGate>` from `file_config` only:
  - Remove the `args: &Args` parameter entirely (callers in `TryFrom<Args>` pass `&args` today).
  - Iterate `config.gates`, convert each `GateEntryConfig` directly to a `ConfiguredGate` using a new local helper `gate_rules_from_config(c: &GateRuleConfig) -> Vec<GateRule>` that reads the ten config fields directly.
  - Remove the `!has_fallback && has_cli_rules(args)` branch that synthesized a fallback gate from CLI flags.
  - Keep the validation: empty rules per gate → bail, `configured.is_empty()` → bail (update the error message to omit the mention of CLI flags: `"at least one rule is required; configure a [[gates]] entry in covgate.toml"`).
- [x] In the `#[cfg(test)]` block (lines 686–1259):
  - Delete tests that exist solely to test CLI-flag-to-rule wiring:
    - `parses_region_cli_rules` (line 707) — delete.
    - `prefers_cli_over_config_defaults` (line 743) — delete.
    - `cli_function_rules_override_repo_config_defaults` (line 875) — delete.
    - `cli_thresholds_override_only_the_fallback_gate` (line 922) — delete.
    - `cli_thresholds_synthesize_a_fallback_gate_when_config_is_scoped_only` (line 971) — delete.
  - Keep and update the remaining tests to construct `Args` without the ten threshold fields. Every `Args { ... }` literal in the surviving tests must drop the ten fields.
  - Surviving tests that use `Args` with all fields None for the ten flags: `loads_defaults_from_repo_config`, `loads_function_rules_from_repo_config`, `resolve_gates_rejects_named_gate_without_rules`, `file_config_defaults_empty_gates` — update each `Args` literal to remove the ten fields.

### tests/cli_metrics.rs

- [x] Delete `tests/cli_metrics.rs` entirely.

### tests/cli_interface.rs

For each test that passes CLI gate flags, decide: **convert** (the behavior under test is the config path, flags are just scaffolding) or **delete** (the behavior under test is the CLI flag itself). Summary follows:

- [x] `covgate_includes_dirty_worktree_changes_by_default` (line 343) — **convert**: the behavior is dirty-worktree inclusion, not the flag. Replace `&["--fail-under-regions".to_string(), "90".to_string()]` with a `covgate.toml` file containing `[[gates]]\nfail-under-regions = 90\n` written to the worktree before the `run_covgate` call.
- [x] `diff_file_mode_skips_dirty_worktree_guard` (line 370) — **convert**: behavior is `--diff-file` mode skipping the dirty guard. Replace `--fail-under-regions` args with a `covgate.toml` containing `[[gates]]\nfail-under-regions = 90\n`. Keep `--diff-file` arg.
- [x] `git_base_mode_warns_about_untracked_files` (line 396) — **convert**: behavior is untracked-files warning. Replace `--fail-under-regions` with a `covgate.toml`. The `run_covgate` call does not pass `--diff-file`, so automatic base discovery applies; covgate.toml provides the gate.
- [x] `diff_file_mode_skips_untracked_files_warning` (line 429) — **convert**: behavior is `--diff-file` suppressing the untracked warning. Replace `--fail-under-regions` with a `covgate.toml`. Keep `--diff-file` arg.
- [x] `automatic_base_prefers_standard_branch_ref_over_recorded_worktree_ref` (line 462) — **convert**: behavior is base-ref precedence. Replace `--fail-under-regions` with a `covgate.toml` written before the commit that adds it.
- [x] `explicit_base_overrides_recorded_worktree_ref` (line 491) — **convert**: behavior is `--base` override. Replace `--fail-under-regions` with a `covgate.toml`. Keep `--base main` arg.
- [x] `failure_text_requires_git_repo_when_run_outside_repository` (line 525) — **convert**: behavior is git-repo-required error. Replace `--fail-under-regions` with a `covgate.toml` in the temp directory (no git repo, so the error fires before config is read; a `covgate.toml` is not required for the error to fire, but removing the flag means zero rules unless config is provided — since the error fires first, simply removing the flag from args is sufficient; verify the error still fires before config resolution).
- [x] `markdown_summary_rust_fixture` (line 544) — **convert**: behavior is Markdown output. Replace `--fail-under-regions 90` with a `covgate.toml` containing `[[gates]]\nfail-under-regions = 90\n`. Keep `--diff-file` and `--markdown-output` args.
- [x] `path_scoped_gates_cli_thresholds_override_only_the_fallback_gate` (line 725) — **delete**: behavior under test is the CLI flag overriding the fallback gate threshold. This feature is being removed; delete the test.
- [x] `absolute_llvm_paths_match_diff_fixture` (line 816) — **convert**: behavior is path normalization. Replace `--fail-under-regions 90` with a `covgate.toml`. Keep `--diff-file` arg.
- [x] `pr_branch_against_main_fixture` (line 842) — **convert**: behavior is PR branch diff against main. Replace `--fail-under-regions 90` with a `covgate.toml`. Keep `--base main` arg.
- [x] `mixed_cli_over_toml_precedence` (line 941) — **delete**: behavior under test is CLI flag overriding TOML threshold. This feature is being removed; delete the test.
- [x] `cli_threshold_overrides_repo_config_default` (line 980) — **delete**: behavior under test is CLI flag overriding repo config default. This feature is being removed; delete the test.
- [x] `unknown_coverage_json_shape_reports_supported_formats` (line 1016) — **convert**: behavior is unsupported-format error. Replace `--fail-under-lines 90` with a `covgate.toml`. Keep `--diff-file` arg.
- [x] `minimal_pass_output_is_token_efficient` (line 1048) — **convert**: behavior is minimal console output on PASS. Replace `--fail-under-regions 90` with a `covgate.toml` containing `[[gates]]\nfail-under-regions = 90\n` written to the worktree.
- [x] `minimal_fail_output_is_focused` (line 1079) — **convert**: behavior is minimal console output on FAIL. Replace `--fail-under-regions 100` with a `covgate.toml` containing `[[gates]]\nfail-under-regions = 100\n` written to the worktree.

  For converted tests: use `fs::write(worktree.join("covgate.toml"), "[[gates]]\nfail-under-<metric> = <value>\n")` immediately before calling `run_covgate` (or before the commit that adds it, where needed for the diff to include the toml).

- [x] `check_help_describes_arguments_and_options` (line 210) — update the assertion `stdout.contains("Minimum changed-region coverage percentage required to pass")` to instead assert the flag is NOT present (since the flags are removed, the help should not list them). Replace with `assert!(!stdout.contains("--fail-under-regions"), ...)` or convert to assert the remaining option descriptions (`--base`, `--diff-file`, `--markdown-output`).

### README.md

- [x] Remove the entire "CLI Reference" subsection (line 153–155):
  ```
  ### CLI Reference

  Every threshold in `covgate.toml` has a corresponding CLI flag for one-off use. Run `covgate --help` for the full list.
  ```

## Validation
- `cargo fmt --check`
- `cargo clippy -- -D warnings`
- `cargo test --test cli_interface`
- `cargo test -p covgate --lib`
- `cargo xtask validate`

## Discoveries
- `Args` struct literals in `tests/config_discovery.rs`, `tests/config_auto_base.rs`, and `tests/lib_run.rs` also need threshold fields removed after `src/cli.rs` changes.
- `tests/llvm_diff_regression.rs`, `tests/llvm_real_parity.rs`, and `tests/support/mod.rs` also used CLI gate flags as test scaffolding and were converted to write temporary config gates.

## Review
- Clean evaluator pass on 2026-05-10. Reviewed the current worktree against this ExecPlan, `docs/CODESTYLE.md`, and `docs/TESTING.md`; no implementation findings.
- Validation inspected during review: `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test --test cli_interface`, `cargo test -p covgate --lib`, and `cargo xtask validate` all passed.
- Second independent evaluator pass on 2026-05-10. No findings after review discussion; step 47 description corrected to reflect simplification rather than deletion of `push_percent_rule` and `push_uncovered_rule`.

## Definition of Done

### Planner
- [x] Plan is consistent, up to date, decision-complete, and ready to hand off.

### Generator
- [x] Goal achieved: all ten CLI gate threshold flags removed; `covgate.toml` is the sole gate configuration surface; workflow flags unchanged.
- [x] All planned steps are complete.
- [x] All validation commands pass.
- [x] Handed off to an independent reviewer (MUST use the `evaluator-execplan` skill via a subagent or separate agent, not the generator agent).

### Evaluator
- [x] Standard review posture applied.
- [x] Adheres to the principles of `docs/CODESTYLE.md`.
- [x] Adheres to the principles of `docs/TESTING.md`.
- [x] All review findings have been addressed.
