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
- [ ] Add a failing integration test in `tests/cli_interface.rs` that reproduces the architectural regression: verify that 'Overall Coverage' does not have its scope narrowed inappropriately and includes all files from the coverage report even when scoped gates are configured.
- [ ] Refactor `src/lib.rs` and `src/metrics.rs` to decouple informational overall coverage from gating logic. Compute repository-wide global metrics once per run, independent of gate scoping. Ensure `GateResult` carries both the scoped gate results and the global repository totals.
- [ ] Refactor renderers to show a single global "Overall Coverage" section at the end of output. Remove gate-specific scoping or labeling from overall totals in both console and Markdown output.
- [ ] Address review findings: O(gates * repo_size) config walk, variable shadowing, exhaustive destructuring, and moving `istanbul_json.rs` inline tests.
- [ ] Run focused checks during iteration, then `cargo xtask validate` before completion.

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

## Review

### Findings

- **[MAJOR] Architectural Regression in Overall Coverage:** Informational overall coverage is currently coupled to gate-specific partitioning. This causes unchanged files to be omitted from the summary and fragments the repository-wide view. `ARCHITECTURE.md` has been updated to explicitly forbid this; overall coverage must be computed globally once per run.
- **[PERF] Inefficient Config Loading:** `src/config.rs::PathMatcher::new` calls `build_repo_ignores` which recursively walks the entire repository to find `.gitignore` files. This walk is currently performed once for every `[[gates]]` entry in the config file. For a repository with many gates or a large file tree, this is O(gates * repo_size). Repository ignores should be built once and shared across matchers.
- **[STYLE] Missing Variable Shadowing:** `src/lib.rs::run` and `src/config.rs` functions do not shadow mutable initialization variables to become immutable (`let x = x;`) after the initialization phase is complete, violating `CODESTYLE.md` ("Defensive Rust").
- **[STYLE] Missing Exhaustive Destructuring:** Core logic in `src/lib.rs::run` and `src/config.rs` uses property access on `Args` and `Config` instead of exhaustive destructuring, violating `CODESTYLE.md` ("Defensive Rust"). This prevents the compiler from forcing updates when new metrics or flags are added.
- **[TEST] Existing Debt:** `src/coverage/istanbul_json.rs` still contains inline tests that exercise only public APIs, which `docs/TESTING.md` explicitly calls out as incorrect. While existing debt, the branch successfully moved similar tests for `src/gate.rs`, so `istanbul_json.rs` should ideally be addressed as well to bring the codebase into compliance with the new standards.

### Evidence

- **Overall Coverage regression:** `src/lib.rs::assign_changed_files` only adds `changed_file.path` to `GateRunInput::paths`. These paths are then used in `src/metrics.rs::compute_changed_metric` to filter `totals_by_file`.
- **Inefficient Walk:** `src/config.rs:480` `PathMatcher::new` calls `build_repo_ignores` directly.
- **Shadowing/Destructuring:** Visible in `src/lib.rs:18` (`run` function) and `src/config.rs:218` (`resolve_gates`).

## Definition of Done

### Planner
- [x] Plan is consistent, up to date, decision-complete, and ready to hand off.

### Generator
- [x] Goal achieved: `covgate` supports `[[gates]]` path-scoped policies, fallback-only CLI merging, and scoped console/Markdown output as defined in `docs/design-docs/path-scoped-gates.md`.
- [x] All planned steps are complete.
- [x] All validation commands pass.
- [x] Added or updated fixture-backed tests for scoped gating behavior and output shape.
- [ ] Handed off to an independent reviewer using the `evaluator-execplan` skill.

### Evaluator
- [ ] Standard review posture applied.
- [ ] Adheres to `docs/CODESTYLE.md`.
- [ ] Adheres to `docs/TESTING.md`.
- [ ] All review findings have been addressed.

## Assumptions and Defaults
- This change is intentionally breaking: legacy top-level `[gates]` is removed rather than supported in parallel with `[[gates]]`.
- There is at most one fallback gate: the single `[[gates]]` entry without `include`.
- Overlap validation is based on actual changed files in the current run, not on static proof that two pattern sets can never overlap.
- No new CLI targeting syntax is added in this slice; scoped gates are configured only in `covgate.toml`.
