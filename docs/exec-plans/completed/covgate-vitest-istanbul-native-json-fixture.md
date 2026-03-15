# Covgate Istanbul native JSON support with a Vite/Vitest v8 fixture

This ExecPlan is complete and archived in `docs/exec-plans/completed/covgate-vitest-istanbul-native-json-fixture.md` for historical traceability.

This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds.

Maintain this document in accordance with `docs/PLANS.md`. Re-read that file before revising this plan and keep this plan aligned with its rules.

This plan follows the just-completed .NET Coverlet milestone and the testing policy in `docs/TESTING.md`: add native ecosystem fixture generation, parse the ecosystem’s native JSON format directly, and validate behavior through end-to-end copied-fixture CLI tests.

## Purpose / Big Picture

After this change, users running JS/TS test suites with Vitest’s default v8 coverage engine will be able to feed the resulting Istanbul JSON output directly to `covgate --coverage-json` and receive the same changed-line and changed-branch gating decisions already available for LLVM and Coverlet inputs.

Today, `covgate` supports LLVM JSON and Coverlet JSON parsing. The next ecosystem milestone is Istanbul-native JSON ingestion, with a real checked-in Vite/Vitest fixture that demonstrates pass/fail threshold behavior in diff-focused CLI tests. The user-visible success condition is simple: run Vitest coverage in a JS/TS project, pass the Istanbul JSON report to `covgate`, and get correct gate results without conversion to another report format.

## Progress

- [x] (2026-03-15 00:30Z) Re-read `docs/TESTING.md` and the completed .NET Coverlet ExecPlan to carry forward fixture-generation and parser-autodetect expectations.
- [x] Define fixture layout under `tests/fixtures/vitest/` with at least one pass and one fail scenario (`repo/`, `overlay/`, checked-in Istanbul JSON artifact).
  Completed: 2026-03-15 / Codex.
- [x] Implement/extend xtask fixture regeneration for `vitest/*` scenarios using native `vitest run --coverage` with the default v8 coverage provider and deterministic artifact normalization.
  Completed: 2026-03-15 / Codex.
- [x] Implement `src/coverage/istanbul_json.rs` parser support that maps Istanbul line and branch data into `covgate` internal metric structures.
  Completed: 2026-03-15 / Codex.
- [x] Extend coverage-format autodetection to identify Istanbul JSON and dispatch it without requiring a new primary CLI input switch.
  Completed: 2026-03-15 / Codex.
- [x] Add/expand integration tests in `tests/cli_metrics.rs` and `tests/cli_interface.rs` to validate pass/fail, metric availability semantics, and unknown/ambiguous JSON format errors.
  Completed: 2026-03-15 / Codex.
- [x] Add Istanbul function normalization + fixture-matrix validation for `--fail-under-functions` / `--fail-uncovered-functions` so Istanbul follows the same public `functions` vocabulary as LLVM/Coverlet.
  Completed: 2026-03-15 / Codex.
- [x] Run full validation (`cargo xtask validate`) and complete this plan’s retrospective before moving it to `docs/exec-plans/completed/`.
  Completed: 2026-03-15 / Codex.

## Surprises & Discoveries

- Observation: `tests/fixtures/vitest/README.md` currently states the directory is reserved for an Istanbul-based fast-follow plan, so there is no existing runnable fixture baseline yet.
  Evidence: Current README content is a placeholder sentence only.

- Observation: `docs/TESTING.md` explicitly expects JS/TS live artifacts from native tooling (`vitest run --coverage`) and expects fixture coverage regeneration through xtask, not hand-authored JSON.
  Evidence: The live-scenario section lists JS/TS native generation and the xtask regeneration policy.

## Decision Log

- Decision: Target Istanbul-native JSON output from Vitest’s default v8 coverage runner instead of introducing nyc instrumentation-first setup.
  Rationale: The request explicitly asks for the default v8 runner path, and this keeps fixture setup close to out-of-the-box Vitest behavior.
  Date/Author: 2026-03-15 / Codex

- Decision: Keep parser-selection UX consistent with existing coverage ingestion by extending autodetect rather than adding a separate primary CLI argument.
  Rationale: Users should continue to rely on `--coverage-json` regardless of ecosystem, with format dispatch handled internally.
  Date/Author: 2026-03-15 / Codex

- Decision: Treat Istanbul branch coverage as first-class when present, but preserve explicit metric-unavailable semantics where fixture payloads omit required branch structures.
  Rationale: Cross-ecosystem metric behavior should be deterministic and transparent in CLI output.
  Date/Author: 2026-03-15 / Codex

## Outcomes & Retrospective

- Decision: Istanbul function-threshold validation now lives in this active Istanbul plan rather than the completed function-threshold plan.
  Rationale: Function gating is already shipped for supported parser families; Istanbul remains the active parser milestone and should own Istanbul-specific function normalization and fixture-matrix acceptance criteria.
  Date/Author: 2026-03-16 / Codex

Implementation is complete. Outcomes:

1. `covgate` now parses Istanbul-native JSON and computes changed line/branch/function metrics using the same internal opportunity model as existing parsers.
2. `tests/fixtures/vitest/` now contains `basic-pass` and `basic-fail` fixture projects (`repo/` + `overlay/`) with checked-in Istanbul `coverage.json` artifacts generated via xtask.
3. Metric CLI integration suites now include Vitest fixtures in line, branch, and function fixture matrices.
4. Coverage format autodetect now supports LLVM, Coverlet, and Istanbul; unknown payload diagnostics enumerate all three families.
5. `cargo xtask validate` passes with the Istanbul milestone included.

## Context and Orientation

The relevant code paths are under `src/coverage/`, where format adapters parse incoming coverage JSON into a common internal representation used by gating logic. `src/coverage/mod.rs` currently contains format detection and dispatch logic for LLVM and Coverlet.

Integration behavior is exercised from Rust integration tests under `tests/`, with reusable fixture setup helpers in `tests/support/mod.rs`. Fixture inputs are stored under `tests/fixtures/<language>/<scenario>/` with checked-in coverage artifacts and `repo/` + `overlay/` trees used to construct diff scenarios in temporary directories.

Fixture artifact generation and normalization is centralized in `xtask/src/main.rs`. This is the required path for regenerating committed fixture coverage artifacts across ecosystems according to `docs/TESTING.md`.

In this plan, “Istanbul JSON” means the JSON schema keyed by source-file paths where each file entry includes fields such as statement maps, function maps, branch maps, and corresponding execution counters. “Vitest default v8 runner” means running Vitest coverage with the v8 engine (the default coverage provider in modern Vitest) and producing Istanbul-format JSON output artifacts from that run.

## Plan of Work

First, create concrete Vitest fixture projects in `tests/fixtures/vitest/basic-pass/` and `tests/fixtures/vitest/basic-fail/` with minimal Vite/Vitest setup, deterministic tests, and changed-file overlays that exercise diff-only gates. Capture generated Istanbul JSON artifacts in each fixture directory.

Second, extend xtask fixture regeneration so maintainers can regenerate each Vitest fixture artifact via `cargo xtask regen-fixture-coverage vitest/<scenario>`. The xtask flow must run the fixture’s native test command (`vitest run --coverage`) and then copy/normalize the desired JSON output into the checked-in artifact path.

Third, add an Istanbul parser module in `src/coverage/istanbul_json.rs` and wire it into `src/coverage/mod.rs`. The parser must:

- ingest per-file Istanbul counters and maps,
- convert line coverage into `covgate`’s changed-line opportunities/hits,
- convert branch coverage into branch opportunities/hits where branch arrays exist,
- normalize source paths to align with existing diff path matching,
- report explicit metric unavailability when required structures are missing.

Fourth, expand coverage format autodetection. The detector should probe known JSON shapes for LLVM, Coverlet, and Istanbul; dispatch automatically when exactly one format matches; and return actionable errors for unknown or ambiguous payloads.

Fifth, expand integration coverage tests. Keep metric semantics in `tests/cli_metrics.rs` (including cross-language fixture matrices where appropriate) and parser/interface behaviors in `tests/cli_interface.rs` (autodetect behavior and error messaging). Add fixture capability declarations in shared support helpers so tests can skip or assert unavailable metrics intentionally.

Finally, update fixture README/doc notes for Vitest regeneration and run the full validation gate from the repository root.

## Concrete Steps

Working directory for every command below: repository root unless noted.

1. Create or update fixture repositories and overlays.

       mkdir -p tests/fixtures/vitest/basic-pass tests/fixtures/vitest/basic-fail

2. Generate/update native coverage artifacts through xtask once support is wired.

       cargo xtask regen-fixture-coverage vitest/basic-pass
       cargo xtask regen-fixture-coverage vitest/basic-fail

   Expected transcript excerpt should show Vitest executing with coverage enabled and a checked-in Istanbul JSON artifact being written/normalized.

3. Run targeted tests while implementing parser + harness changes.

       cargo test istanbul
       cargo test cli_metrics
       cargo test cli_interface

4. Run full repository validation before completion.

       cargo xtask validate

## Validation and Acceptance

Acceptance is complete only when all conditions below are true:

- Vitest fixture scenarios (pass and fail) exist with committed native Istanbul JSON artifacts generated by xtask.
- `covgate --coverage-json <vitest-istanbul-json>` parses successfully and computes changed-line thresholds correctly in integration tests.
- Branch threshold behavior is covered for fixtures where Istanbul branch data exists; if branch data is absent for a scenario, tests assert explicit metric-unavailable behavior.
- Coverage JSON autodetection identifies LLVM, Coverlet, and Istanbul payloads and rejects unknown payloads with actionable diagnostics.
- `cargo xtask validate` passes at closeout.

## Idempotence and Recovery

Re-running `cargo xtask regen-fixture-coverage vitest/<scenario>` should only update that scenario’s coverage artifact. Integration tests must continue to run against copied fixtures in temporary repositories so checked-in fixture trees remain unchanged during routine test runs.

If Vitest output paths or JSON shape vary across tool versions, update xtask normalization first, then lock behavior with fixture tests before changing parser heuristics. If autodetect becomes ambiguous for future payloads, add explicit disambiguation logic and test coverage before widening accepted shape probes.

## Artifacts and Notes

Representative commands after implementation:

    vitest run --coverage
    covgate --coverage-json tests/fixtures/vitest/basic-fail/coverage-final.json --base origin/main --fail-under-lines 80

Representative expected user behavior:

    covgate --coverage-json coverage-final.json

where `covgate` determines whether the JSON payload is LLVM, Coverlet, or Istanbul and parses it accordingly.

## Interfaces and Dependencies

- Add `src/coverage/istanbul_json.rs` with serde models + conversion helpers into existing internal coverage metrics.
- Extend `src/coverage/mod.rs` format detection and parser dispatch.
- Extend `xtask/src/main.rs` fixture spec table and regeneration logic for `vitest/*` entries.
- Update `tests/support/mod.rs` fixture descriptors and capability declarations for the Vitest fixtures.
- Add or update integration tests in `tests/cli_metrics.rs` and `tests/cli_interface.rs` for Istanbul behavior.

At the bottom of this plan, append a revision note every time the plan changes materially, describing what changed and why.

Revision note: Initial plan authored to add native Istanbul JSON support, create Vite/Vitest fixtures using the default v8 runner, and integrate parser + autodetect behavior into the existing coverage pipeline.

Revision note: Added explicit Istanbul ownership of function-threshold follow-up (normalization + fixture matrix validation) after closing out the dedicated function-threshold plan for currently supported parser families.

Revision note: Closed the Istanbul ExecPlan after implementing parser support, Vitest fixtures + xtask regeneration, fixture-matrix CLI coverage, and full repository validation.
