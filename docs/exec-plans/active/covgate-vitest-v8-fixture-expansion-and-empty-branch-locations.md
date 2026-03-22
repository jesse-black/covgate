# Expand the Vitest v8 repro fixture until it reproduces empty branch locations, then make the Istanbul parser tolerate them

Save this in-progress ExecPlan in `docs/exec-plans/active/covgate-vitest-v8-fixture-expansion-and-empty-branch-locations.md`. Move it to `docs/exec-plans/completed/covgate-vitest-v8-fixture-expansion-and-empty-branch-locations.md` only after implementation, validation, and documentation updates are complete.

This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds.

Maintain this document in accordance with `docs/PLANS.md`. Re-read that file before revising this plan and keep this plan aligned with its rules.

## Purpose / Big Picture

`covgate` currently parses Istanbul JSON from Vitest’s default v8 coverage provider, but the checked-in Vitest fixtures are still tiny one-file examples. A real user report showed that a larger project can produce Istanbul `branchMap.locations` entries whose `start` and `end` positions are empty objects instead of line-bearing positions. Right now `covgate` rejects that report during JSON deserialization with `missing field 'line'`, which means users cannot gate coverage for some otherwise ordinary Vitest v8 projects.

After this work, the repository will contain an open, project-like Vitest fixture that is much closer to a real application than the current toy fixture, and that fixture will either reproduce the empty-location shape directly or be refined until it does. Once that reproduction exists, `covgate` will accept the resulting Istanbul JSON and compute metrics without crashing. A novice should be able to regenerate the fixture, inspect the coverage artifact for the empty branch locations, run the parser tests, and see that the parser now handles the case.

## Progress

- [x] (2026-03-22 00:00Z) Re-read `docs/PLANS.md`, `ARCHITECTURE.md`, and `docs/TESTING.md` to confirm plan format, parser boundaries, and the repository rule that fixture coverage artifacts must come from native toolchains rather than hand-authored JSON.
- [x] (2026-03-22 00:05Z) Inspected `src/coverage/istanbul_json.rs` and confirmed the current deserializer requires every `IstanbulPosition` to contain a `line`, including every entry in `branchMap.locations`.
- [x] (2026-03-22 00:10Z) Compared the reported failing payload against the parser shape and confirmed that the observed `missing field 'line'` error is consistent with empty `branchMap.locations` entries such as `{"start":{},"end":{}}`.
- [x] (2026-03-22 00:15Z) Inspected the existing Vitest fixtures and confirmed they are still toy-sized single-file projects under `tests/fixtures/vitest/basic-pass`, `tests/fixtures/vitest/basic-fail`, and `tests/fixtures/vitest/statement-line-divergence`.
- [x] (2026-03-22 00:20Z) Confirmed the checked-in Vitest fixture config uses the same broad setup as the reported project: `vitest run --coverage` with `provider: "v8"` and JSON reporters in `tests/fixtures/vitest/*/repo/vitest.config.mjs`.
- [x] (2026-03-22 00:25Z) Confirmed the current fixture artifacts only contain `"type": "branch"` entries and do not currently exercise `"type": "if"` branches with empty alternate locations.
- [x] Verify the exact local command path for `npm` and `npx` in the shell environment used for fixture regeneration so `cargo xtask regen-fixture-coverage vitest/...` can run reliably during implementation.
- [x] Expand `tests/fixtures/vitest/statement-line-divergence/` from a one-file `src/math.js` toy into a small multi-file application-like fixture with several source files, helper modules, and tests, while preserving its role as the repository’s main Vitest native-summary repro fixture.
- [x] Regenerate `tests/fixtures/vitest/statement-line-divergence/coverage.json` and `tests/fixtures/vitest/statement-line-divergence/native-summary.json` after each meaningful fixture expansion, inspect the generated `branchMap` entries, and keep iterating on the source shape until the fixture reproduces empty `start`/`end` branch locations or until the plan records a justified stopping point.
- [x] If the expanded `statement-line-divergence` fixture still does not reproduce the issue, add one new Vitest scenario under `tests/fixtures/vitest/` dedicated to empty branch locations rather than overloading the line-summary fixture past readability.
- [x] Add a Rust-side parser regression test in `src/coverage/istanbul_json.rs` using a checked-in open fixture artifact. The test failed before the parser change and now passes.
- [x] Implement the parser fix in `src/coverage/istanbul_json.rs` so empty branch-location objects no longer abort deserialization, while preserving correct line attribution for branch opportunities.
- [x] Update any fixture harness assumptions that still hard-code the old single-file Vitest layout, especially path normalization helpers in `tests/support/mod.rs` and fixture spec metadata in `xtask/src/main.rs`.
- [x] Run targeted Rust tests during development, then `cargo xtask quick`, then `cargo xtask validate` before considering the work complete.

## Surprises & Discoveries

- Observation: The current parse failure happens before any branch-processing logic runs, because `serde_json::from_str` deserializes directly into `IstanbulPosition { line: u32 }`.
  Evidence: `src/coverage/istanbul_json.rs` deserializes the entire report at the top of `parse_str_with_repo_root`, and `IstanbulBranchMap.locations` is currently `Vec<IstanbulSpan>` whose `start` and `end` both require `line`.

- Observation: The reported failing payload and the checked-in fixtures both come from Vitest’s v8 coverage provider, so the provider choice alone is not enough to characterize the JSON shape.
  Evidence: the current fixtures use `@vitest/coverage-v8` with `provider: "v8"`, while the reported payload still contains shapes the fixtures do not reproduce.

- Observation: The current Vitest fixtures are too small to prove much about real-world branch-location behavior because they exercise only a single changed file and produce only `"type": "branch"` entries.
  Evidence: `tests/fixtures/vitest/basic-pass/repo/src/math.js`, `tests/fixtures/vitest/basic-fail/repo/src/math.js`, and `tests/fixtures/vitest/statement-line-divergence/repo/src/math.js` are all tiny single-module examples, and the committed `coverage.json` files only show `"type": "branch"`.

- Observation: `xtask` already has the correct high-level Vitest regeneration flow, so this plan should expand fixture source projects first and only change xtask where the richer fixture layout requires it.
  Evidence: `xtask/src/main.rs` already copies `repo/` plus `overlay/`, runs `npm install`, runs `npx vitest run --coverage --coverage.reporter=json --coverage.reporter=json-summary`, normalizes `coverage-final.json`, and writes `native-summary.json`.

- Observation: The environment used during initial investigation did not resolve `npm` on PATH even though the user reports `npm -v` works in their shell.
  Evidence: a first attempt to run `cargo xtask regen-fixture-coverage vitest/statement-line-divergence` failed with `failed to execute 'npm'` and `No such file or directory (os error 2)`.

- Observation: The first multi-file TS expansion under Vitest 3 still emitted only one-location generic branch entries, so source shape alone was not enough to reproduce the empty-location branch form.
  Evidence: the regenerated `tests/fixtures/vitest/statement-line-divergence/coverage.json` under `vitest@3.2.4` contained only single-location branch records and no `start: {}` / `end: {}` entries.

- Observation: Updating the expanded fixture to Vitest 4 produced the desired empty branch-location shape, but it also changed the old `statement-line-divergence` fixture’s line-summary semantics enough to break `tests/overall_summary.rs`.
  Evidence: the Vitest 4 regeneration produced multiple `"type": "if"` entries with empty alternate locations in `authService.ts` and `msalConfig.ts`, while `overall_summary_line_totals_match_native_summary_for_all_line_capable_fixtures` failed until the fixture was split into a new dedicated scenario.

## Decision Log

- Decision: Use `tests/fixtures/vitest/statement-line-divergence` as the primary expansion target instead of rewriting the `basic-pass` or `basic-fail` fixtures first.
  Rationale: `statement-line-divergence` already exists as the Vitest native-summary repro fixture and is not part of the broad branch/function matrix. That makes it the safest place to grow a more realistic project without turning the smoke-test fixtures into hard-to-maintain mini-apps.
  Date/Author: 2026-03-22 / Codex

- Decision: Do not hand-author a coverage artifact from the closed-source report or from a guessed JSON shape.
  Rationale: `docs/TESTING.md` explicitly requires native fixture artifacts generated through xtask. The open reproduction must come from an open fixture project in this repository.
  Date/Author: 2026-03-22 / Codex

- Decision: Add the parser regression test only after the expanded open fixture produces the problematic shape, and distill the test input from that fixture artifact.
  Rationale: The bug already appears understood, but the point of this plan is to replace anecdotal evidence with an open, native, repository-owned reproduction. The unit test should be derived from that reproduction, not invented ahead of it.
  Date/Author: 2026-03-22 / Codex

- Decision: If the expanded `statement-line-divergence` fixture becomes too contorted while chasing the reproduction, split the work into a second dedicated Vitest scenario instead of making one fixture serve unrelated purposes.
  Rationale: The repository should keep a readable line-summary repro and a readable parser-shape repro rather than one overly clever fixture that is hard for a novice to understand.
  Date/Author: 2026-03-22 / Codex

## Outcomes & Retrospective

Implementation finished with a slight variation on the original fixture plan. The repository now has a dedicated open Vitest fixture at `tests/fixtures/vitest/empty-branch-locations/` that uses a small multi-file TS project and a Vitest 4 V8 coverage stack to reproduce the empty `branchMap.locations` alternate shape. The original `statement-line-divergence` fixture was restored to its previous role so the line-summary parity tests continue to describe the old behavior they were written for.

The parser bug is now covered end to end with TDD. A checked-in fixture-backed parser test in `src/coverage/istanbul_json.rs` first failed against the reproduced artifact with `missing field 'line'`, then passed after the parser was updated to treat branch-location lines as optional and fall back to the enclosing branch line when Vitest leaves an alternate location empty.

The final lesson is that “same provider” was not enough, but “same provider plus a realistic enough project and the newer Vitest export path” was. Splitting the new repro into its own scenario kept the existing line-summary fixture stable while still giving the repository an open, native-generated parser regression artifact.

## Context and Orientation

`covgate` is a Rust command-line tool in `src/` that reads native coverage reports, normalizes them into a shared internal model, intersects that model with changed lines from a Git diff, and applies coverage gates. The Istanbul parser lives in `src/coverage/istanbul_json.rs`. That file currently deserializes line, branch, and function map positions into a strict `IstanbulPosition` type with a required `line` field. Because deserialization happens before coverage normalization, a single empty branch-location object can reject the entire report.

Vitest fixture generation lives in `xtask/src/main.rs`. The Vitest path copies a fixture’s `repo/` and `overlay/` directories into a temporary directory, runs `npm install`, then runs `npx vitest run --coverage --coverage.reporter=json --coverage.reporter=json-summary`, then normalizes `coverage-final.json` into the checked-in `coverage.json` and normalizes `coverage-summary.json` into `native-summary.json`.

The current Vitest fixtures live under `tests/fixtures/vitest/`. The basic fixtures are smoke tests for line, branch, and function threshold behavior. The `statement-line-divergence` fixture is the repository’s current Vitest repro for overall line totals that differ from raw statement count. Today all three are tiny one-file projects. That is the central limitation this plan addresses.

`tests/support/mod.rs` is the reusable integration-test harness. It knows how to copy fixture repositories into temporary worktrees, build diffs, invoke `covgate`, and compute native totals from the committed coverage artifacts. It also contains a hard-coded relative source path for Vitest coverage when writing absolute-path coverage fixtures. Any fixture expansion that changes the main changed-file path must keep this helper in sync.

In this plan, “empty branch location” means an Istanbul `branchMap.locations` entry whose `start` and `end` are empty objects rather than objects with line numbers. A representative shape is:

    {
      "locations": [
        {"start": {"line": 9, "column": 2}, "end": {"line": 12, "column": null}},
        {"start": {}, "end": {}}
      ]
    }

In this plan, “project-like fixture” means an open source fixture with several source files and tests that resemble a small application or library rather than a one-function example. It does not need to be large. It does need enough structure to exercise more realistic branch and source-map behavior.

## Plan of Work

Start by making the existing `tests/fixtures/vitest/statement-line-divergence` fixture look like a miniature application. Replace the single `src/math.js` module with a small multi-file tree. Keep one changed file that the overlay edits, but add neighboring modules so the coverage run includes realistic imports, multiple functions, branch-heavy code, and at least one or two files that are executed only indirectly through the changed file’s test path. Prefer ordinary language constructs that are common in user code: `if` statements without `else`, `if/else` chains, small helper functions, object construction, callbacks, and multiple files imported from a central entry point.

Keep the fixture within Vitest’s default v8 path unless the fixture source itself proves that another ingredient is required. The first expansion should stay within plain JavaScript or TypeScript that Vitest can run without adding heavy framework dependencies. If TypeScript source is introduced, update the fixture package and config only as much as necessary to keep regeneration native and deterministic. If a JSX or TSX step becomes necessary to reproduce the issue, add only the minimum dependencies needed and document the reason in this plan.

After each fixture expansion, regenerate the fixture through `cargo xtask regen-fixture-coverage vitest/statement-line-divergence` and inspect the generated `tests/fixtures/vitest/statement-line-divergence/coverage.json`. The inspection goal is specific: find whether any `branchMap` entries now have `"type": "if"` and whether any of their alternate `locations` use empty `start` or `end` objects. Keep the fixture readable. The right way to iterate is a series of small, meaningful source changes with regeneration after each, not one giant rewrite.

If the expanded line-summary fixture still does not reproduce the empty-location case, stop stretching it and add a second Vitest scenario dedicated to parser-shape reproduction. That new scenario should live under `tests/fixtures/vitest/<new-scenario>/` with the same `repo/`, `overlay/`, `coverage.json`, and optional `native-summary.json` layout. Reuse the same xtask regeneration flow and add the new fixture id to the Vitest section of `xtask/src/main.rs`. The parser-shape repro does not need to participate in the overall-summary parity tests unless it also has meaningful native-summary behavior.

Once an open fixture produces the problematic JSON shape, distill the smallest representative excerpt into a unit test in `src/coverage/istanbul_json.rs`. That test should assert that `parse_str_with_repo_root` accepts the input and produces sensible branch opportunities rather than failing in `serde`. Keep the unit-test input small enough to read, but make sure it still reflects the exact structural reason the fixture failed.

Then update `src/coverage/istanbul_json.rs` so it no longer requires every branch-location object to contain a line number. The fix should preserve correct line attribution whenever the report provides one. When a specific branch location omits a line, the parser should fall back to another stable branch line source if Istanbul provides one at the branch level; if no line is available at all for that branch alternative, the parser should skip only that unusable alternative rather than rejecting the entire file. Record the final fallback policy in this plan and in the unit test expectations.

Finally, update any path assumptions broken by the richer fixture layout, rerun the relevant Rust tests, rerun the xtask validation flow, and keep the checked-in fixture artifacts native-generated. Do not ship the parser fix without both the open fixture repro work and the distilled regression test.

## Concrete Steps

Run all commands from the repository root, the directory containing `Cargo.toml`, unless a step explicitly says otherwise.

1. Verify the shell environment used for fixture regeneration and make `npm`/`npx` invocation reliable inside the same environment that runs `cargo xtask`.

    which npm
    which npx
    npm -v
    npx --version
    cargo xtask regen-fixture-coverage vitest/statement-line-divergence

    Expected result: `which` prints usable command paths, version commands print actual versions, and xtask reaches the Vitest execution phase instead of failing with `No such file or directory`.

2. Inspect the existing Vitest repro fixture before changing it.

    find tests/fixtures/vitest/statement-line-divergence -type f | sort
    sed -n '1,200p' tests/fixtures/vitest/statement-line-divergence/repo/src/math.js
    sed -n '1,200p' tests/fixtures/vitest/statement-line-divergence/overlay/src/math.js

    Expected result: the current fixture is visibly a one-file toy, confirming why expansion is necessary.

3. Expand the fixture source tree, regenerate coverage, and inspect the generated shape after each meaningful change.

    cargo xtask regen-fixture-coverage vitest/statement-line-divergence
    rg -n '"type":|"locations"|"start": \{\}|"end": \{\}' tests/fixtures/vitest/statement-line-divergence/coverage.json

    Expected result: at first the artifact may still contain only simple branch shapes. Continue iterating until the `branchMap` includes realistic `if` branches and ideally empty alternate locations.

4. If the expansion reproduces the bug, add a focused parser test and prove it fails before the parser change.

    cargo test istanbul_json -- --nocapture

    Expected result before the fix: the new regression test fails with a parse error or another branch-location handling failure.

5. Implement the parser fix and rerun focused tests.

    cargo test istanbul_json -- --nocapture
    cargo test overall_summary -- --nocapture
    cargo test cli_metrics -- --nocapture

    Expected result after the fix: the new regression test passes, existing Istanbul tests stay green, and fixture-backed integrations keep their expected behavior.

6. Run the repository’s normal development and validation loops.

    cargo xtask quick
    cargo xtask validate

    Expected result: all checks pass. If any fail, update this plan’s `Progress`, `Surprises & Discoveries`, and `Decision Log` with the blocker and next corrective step.

## Validation and Acceptance

The work is accepted only when all of the following are true:

An open Vitest fixture in `tests/fixtures/vitest/` is no longer a one-file toy and now resembles a small project with multiple modules and realistic control flow.

That expanded fixture, or a closely related new Vitest fixture added because the first one stayed readable but did not reproduce the bug, produces a native-generated `coverage.json` artifact with the empty branch-location shape that originally broke the parser.

`src/coverage/istanbul_json.rs` contains a regression test distilled from the open fixture artifact, and that test demonstrates the failure before the parser change and success after it.

`covgate` no longer rejects the reproduced Istanbul JSON with `missing field 'line'`. Instead, it parses successfully and computes line, branch, and function totals as far as the report meaningfully allows.

`cargo xtask quick` and `cargo xtask validate` pass after the fixture, parser, and test changes land.

## Idempotence and Recovery

Fixture regeneration must remain safe to rerun. Re-running `cargo xtask regen-fixture-coverage vitest/statement-line-divergence` should only refresh that fixture’s checked-in coverage artifacts. If a new dedicated scenario is added, its xtask regeneration command must likewise be safe to rerun without hand-editing the JSON.

If a fixture expansion path becomes too framework-heavy or unstable, recover by backing out only the risky fixture-source changes while keeping any useful intermediate notes in this plan. Then try a smaller, more focused expansion or split the work into a dedicated parser-shape scenario. The repository should prefer two understandable fixtures over one opaque fixture.

If the parser fallback policy turns out to undercount or overcount branches, recover by tightening the fallback logic and extending the regression tests. Never recover by relaxing the parser so far that it silently invents branch lines without a documented policy.

If `npm` or `npx` resolution remains unreliable in the environment that runs xtask, solve that command-resolution issue before trusting any fixture-regeneration results. Do not check in manually edited coverage artifacts as a workaround.

## Artifacts and Notes

Representative current parser shape, which explains the failure:

    #[derive(Debug, Deserialize)]
    struct IstanbulBranchMap {
        locations: Vec<IstanbulSpan>,
    }

    #[derive(Debug, Deserialize)]
    struct IstanbulSpan {
        start: IstanbulPosition,
        end: IstanbulPosition,
    }

    #[derive(Debug, Deserialize)]
    struct IstanbulPosition {
        line: u32,
    }

Representative current xtask Vitest regeneration flow:

    copy repo/
    copy overlay/
    npm install
    npx vitest run --coverage --coverage.reporter=json --coverage.reporter=json-summary
    normalize coverage/coverage-final.json -> coverage.json
    normalize coverage/coverage-summary.json -> native-summary.json

Representative expected evidence once the open fixture reproduces the problem:

    $ rg -n '"start": \{\}|"end": \{\}' tests/fixtures/vitest/<scenario>/coverage.json
    123:                "start": {}
    124:                "end": {}

Representative expected evidence after the parser fix:

    $ cargo test istanbul_json -- --nocapture
    test coverage::istanbul_json::tests::parses_empty_branch_alternate_locations_from_vitest_v8_fixture ... ok

## Interfaces and Dependencies

The main implementation files in this plan are:

- `tests/fixtures/vitest/statement-line-divergence/repo/...` and `tests/fixtures/vitest/statement-line-divergence/overlay/...` for the expanded open fixture source tree.
- `xtask/src/main.rs` for any fixture-spec or regeneration changes needed by a richer Vitest fixture layout or by an added dedicated scenario.
- `tests/support/mod.rs` for fixture-path assumptions that currently treat every Vitest fixture as `src/math.js`.
- `src/coverage/istanbul_json.rs` for both the distilled regression test and the parser change.
- `tests/fixtures/vitest/README.md` if regeneration guidance changes materially.

Use only the repository’s existing native fixture pipeline. Do not introduce a second way to build Vitest fixtures. Keep the Vitest provider on the default v8 path unless a reproduced open fixture demonstrates that another provider is strictly necessary.

The parser end state should be explicit:

- statements still use real statement start and end lines;
- functions still use real function `loc` start and end lines;
- branches may now accept missing location lines without aborting the whole report;
- branch alternatives with usable line information still become `OpportunityKind::BranchOutcome` records;
- branch alternatives with no usable line information at all are skipped individually rather than making the whole file unparseable.

Plan revision note: created this ExecPlan to replace a closed-source Vitest v8 parse failure with an open, native-generated fixture reproduction, then use that reproduction to drive a parser fix for empty branch-location objects.
