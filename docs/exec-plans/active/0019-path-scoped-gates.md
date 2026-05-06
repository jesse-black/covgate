---
description: "ExecPlan for implementing path-scoped gates with `[[gates]]` config, fallback-gate CLI merging, and gate-labeled console/Markdown output; read when implementing or continuing this feature."
---

# Path-scoped gates

## Goal
- `covgate` supports path-scoped gate policies from `covgate.toml` via `[[gates]]`, evaluates each gate against its matching changed files, merges CLI threshold flags into the fallback gate only, and reports scoped results in console and Markdown output without changing the parser metric model.

## Scope
- In: replace `[gates]` config with `[[gates]]`, add path matching with the `ignore` crate, assign changed files to scoped or fallback gates, evaluate gates independently, preserve existing CLI flags with fallback-only merge behavior, update console and Markdown renderers, add fixture-backed integration coverage, and run full validation.
- Out: new CLI syntax for targeting named gates, parser-format changes, AST-based UI/logic classification, compatibility shim for legacy `[gates]`, and changes to metric definitions.

## Relevant Areas
- `src/config.rs`, `src/cli.rs`, `src/model.rs` — config schema, CLI merge rules, gate result shape
- `src/metrics.rs`, `src/gate.rs`, `src/render/console.rs`, `src/render/markdown.rs` — scoped evaluation and output
- `tests/cli_interface.rs`, `tests/render_console.rs`, `tests/render_markdown.rs`, `tests/support/mod.rs`, `tests/fixtures/vitest/` — config/output integration tests and new mixed TS/TSX fixture
- `Cargo.toml`, `docs/design-docs/path-scoped-gates.md` — dependency and design reference

## Open Questions
- None

## Steps
- [x] Add failing config tests for the new `[[gates]]` schema before implementation. Cover: valid scoped gates, valid fallback gate, duplicate explicit names, `exclude` without `include`, multiple fallback gates, fallback synthesized from CLI flags when config has only scoped gates, and legacy `[gates]` rejection with an actionable error.
- [x] Replace the current flat config model with a gate-entry model in `src/config.rs`. Each entry should carry optional `name`, optional include/exclude patterns, and resolved threshold rules. Add `ignore` as a dependency and compile gate matchers with gitignore-style semantics. Keep top-level `base`, `markdown-output`, and `verbose` behavior unchanged.
- [x] Implement CLI/config merge rules in `src/config.rs`: CLI threshold flags override only the fallback gate on a per-rule basis; if config has no fallback gate and CLI thresholds are present, synthesize one; scoped gates come only from config; `--base`, `--diff-file`, `--markdown-output`, and `--verbose` keep current precedence.
- [x] Extend the runtime model and evaluation flow to handle multiple participating gates without introducing parallel collections. Add a gate-scoped result type that keeps gate label, computed metrics, and rule outcomes together. Keep one-gate runs cheap and straightforward.
- [x] Implement changed-file gate assignment using the `ignore` crate. Match normalized repo-relative paths, respect repository ignore rules, and detect overlaps on actual changed files: if a changed file matches more than one scoped gate, fail with an error naming the file and the conflicting gates. If a changed file with supported opportunities matches no scoped gate and no fallback gate exists, fail with an actionable error.
- [x] Update metric computation so each gate evaluates only its assigned files and reports filtered `changed_totals_by_file`. Keep current zero-total behavior for percent and uncovered-count rules.
- [x] Update console output. Minimal output should show gate labels only when needed: named or multiple participating gates get labels, an unnamed fallback gate renders as `default` when other gates also participate, and a lone unnamed fallback gate keeps the current unlabeled shape. Verbose output should group by gate.
- [x] Update Markdown output without changing its table-based format. Add a `Gate` column only when multiple gates participate; render an unnamed fallback gate as `default` in that case; keep the current table shape unchanged when a lone unnamed fallback gate is the only participant.
- [x] Add fixture-backed CLI tests with a dedicated mixed Vitest fixture containing both `*.ts` and `*.tsx` changed files. Use copied-fixture integration tests, not synthetic JSON. Cover: scoped gate pass/fail, fallback handling, overlap failure, unmatched-file failure without fallback, CLI override of fallback only, console minimal output labels, and Markdown `Gate` column behavior.
- [x] Run focused checks during iteration, then `cargo xtask validate` before completion.
- [x] Add a failing integration test in `tests/cli_interface.rs` that reproduces the architectural regression: verify that 'Overall Coverage' does not have its scope narrowed inappropriately and includes all files from the coverage report even when scoped gates are configured.
- [x] Refactor `src/lib.rs` and `src/metrics.rs` to decouple informational overall coverage from gating logic. Compute repository-wide global metrics once per run, independent of gate scoping. Ensure `GateResult` carries both the scoped gate results and the global repository totals.
- [x] Refactor renderers to show a single global "Overall Coverage" section at the end of output. Remove gate-specific scoping or labeling from overall totals in both console and Markdown output.
- [x] Address review findings: O(gates * repo_size) config walk, variable shadowing, exhaustive destructuring, and moving `istanbul_json.rs` inline tests.
- [x] Run focused checks during iteration, then `cargo xtask validate` before completion.

## Validation
- `cargo test config`
- `cargo test render_console`
- `cargo test render_markdown`
- `cargo test cli_interface`
- `cargo test`
- `cargo xtask validate`

## Discoveries
- Current config resolution in `src/config.rs` builds a single flat `Vec<GateRule>`; scoped gates will require a grouped runtime model rather than adding more parallel vectors.
- Current Markdown rendering in `src/render/markdown.rs` is entirely table-based; the scoped-gate change should extend that shape with an optional `Gate` column instead of switching to section-per-gate prose.
- The fixture harness in `tests/support/mod.rs` already supports copied `repo/` plus `overlay/` integration scenarios, so path-scoped behavior should be proven with a dedicated Vitest fixture rather than synthetic parser-only tests.
- The fallback gate still needs its assigned path set even when the diff is empty; otherwise single-gate Markdown overall totals collapse to `0/0` because `totals_by_file` was filtered through changed paths instead of gate paths.
- Manual fixture runs surfaced a CLI merge bug where scoped `[[gates]]` inherited `--fail-under-*` overrides. The fix was to gate CLI threshold precedence on `is_fallback`, plus an explicit config test for fallback-only override behavior.
- Integration tests uncovered that `ConfiguredGate::fallback` was missing after refactoring, which broke existing tests. Restored it as a public constructor for test compatibility.
- The residual exhaustive-destructuring finding became stale by the time follow-up work resumed: `src/config.rs` and `src/gate.rs` no longer contain `..` struct destructures in the cited critical logic, so the remaining work was test coverage rather than production cleanup.

## Review

### Findings

- [x] **Markdown overall-coverage regression is now pinned down for multi-scope rendering.** `tests/render_markdown.rs` now includes `renders_global_unlabeled_overall_coverage_for_multi_scope_results`, which constructs multiple gate scopes plus independent `overall_metrics` and proves the overall section remains global, unlabeled, and free of fallback-gate rows while the diff-coverage section still shows gate labels.
- [x] **Residual `CODESTYLE.md` debt review note was stale.** Follow-up verification on `src/config.rs` and `src/gate.rs` found no remaining `..` struct destructures in the cited critical logic, so no production cleanup remained for the exhaustive-destructuring finding.
- [x] **Architectural Regression in Overall Coverage:** Informational overall coverage has been successfully decoupled from gate-specific partitioning. It is computed once globally and included in a separate "Overall Coverage" section in both console (verbose mode) and Markdown output. Integration tests in `tests/cli_interface.rs` verify that it remains global and includes all files even when scoped gates are used.
- [x] **Inefficient Config Loading:** Config loading has been optimized. `build_repo_ignores` is now called once in `resolve_gates` and shared across all `PathMatcher` instances via `Arc`, resolving the O(gates * repo_size) walk issue.
- [x] **Variable Shadowing:** `src/lib.rs::run` and `src/config.rs::resolve_gates` now correctly use variable shadowing (`let x = x;`) to transition from mutable initialization to immutable usage, adhering to `CODESTYLE.md` ("Defensive Rust").
- [x] **Test Debt:** Inline tests in `src/coverage/istanbul_json.rs` have been removed, and coverage parsing is now exercised through integration tests in `tests/coverage_parse.rs` and other integration suites, adhering to `docs/TESTING.md`.
- [x] **Console Minimal Output:** "Overall Coverage" is correctly omitted from console minimal output to maintain token efficiency, while being present in verbose and Markdown output. This strikes a good balance for CLI usage.

### Evidence

- **Markdown multi-scope overall coverage pinned:** Verified in `tests/render_markdown.rs::renders_global_unlabeled_overall_coverage_for_multi_scope_results`.
- **Exhaustive destructuring note stale:** Verified by inspection of `src/config.rs` and `src/gate.rs`; no remaining `..` struct destructures are present in the cited logic.
- **Overall Coverage fixed:** Verified in `src/lib.rs:run` and `tests/cli_interface.rs::overall_coverage_remains_global_when_scoped_gates_are_configured`.
- **Inefficient Walk fixed:** Verified in `src/config.rs:218` (single call to `build_repo_ignores` with `Arc` sharing).
- **Shadowing fixed:** Verified in `src/lib.rs:25-28` and `src/config.rs:253`.
- **Validation:** `cargo xtask validate` passed successfully.

## Definition of Done

### Planner
- [x] Plan is consistent, up to date, decision-complete, and ready to hand off.

### Generator
- [x] Goal achieved: `covgate` supports `[[gates]]` path-scoped policies, fallback-only CLI merging, and scoped console/Markdown output as defined in `docs/design-docs/path-scoped-gates.md`.
- [x] All planned steps are complete.
- [x] All validation commands pass.
- [x] Added or updated fixture-backed tests for scoped gating behavior and output shape.
- [x] Handed off to an independent reviewer using the `evaluator-execplan` skill.

### Evaluator
- [x] Standard review posture applied.
- [x] Adheres to `docs/CODESTYLE.md`.
- [ ] Adheres to `docs/TESTING.md`.
- [ ] All review findings have been addressed.

## Assumptions and Defaults
- This change is intentionally breaking: legacy top-level `[gates]` is removed rather than supported in parallel with `[[gates]]`.
- There is at most one fallback gate: the single `[[gates]]` entry without `include`.
- Overlap validation is based on actual changed files in the current run, not on static proof that two pattern sets can never overlap.
- No new CLI targeting syntax is added in this slice; scoped gates are configured only in `covgate.toml`.
