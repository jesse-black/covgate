# Architecture

This document describes the high-level architecture of `covgate`.

`covgate` is a local diff-coverage gate. It does not run tests itself and it does not own a language-specific coverage engine. Instead, it reads a native coverage report, identifies the lines changed in a Git diff, maps those changed lines onto a normalized internal coverage model, and then applies coverage rules only to the changed opportunities.

This file is intentionally short. It is a codemap and a set of architectural invariants, not a line-by-line implementation guide. For deeper investigation, use the files and documents named here as starting points.

## Bird's-Eye View

At the highest level, `covgate` is a pipeline:

1. Parse CLI input and repository-local defaults.
2. Load a diff, either from a Git base ref or a precomputed unified diff file.
3. Parse a native coverage artifact into `covgate`'s internal model.
4. Compute changed coverage metrics by intersecting normalized coverage opportunities with changed line ranges.
5. Evaluate configured gate rules against those changed metrics.
6. Render human-readable output to the console and optionally to Markdown.

The important design choice is that the gate operates on changed coverage opportunities, not on global project totals. That is the core product behavior.

## Code Map

### Entry points

`src/main.rs` is the CLI entry point. It supports two commands:

- `check`, which runs the diff gate
- `record-base`, which records a stable per-worktree Git base ref for agent-style workflows

`src/lib.rs` is the execution pipeline for `check`. The `run` function is the best single place to read first if you want the whole flow in one screen.

### CLI and configuration

`src/cli.rs` defines the clap surface.

`src/config.rs` resolves the effective runtime configuration from CLI flags plus `covgate.toml`. It is responsible for:

- choosing the diff source
- merging repository-local defaults with CLI overrides
- building the ordered list of gate rules

### Git and diff handling

`src/git.rs` contains Git-specific helper behavior, especially the recorded-base workflow built around `refs/worktree/covgate/base`.

`src/diff.rs` turns either a Git base ref or a unified diff file into a list of changed files plus changed line ranges. From the rest of the system's perspective, this is the only shape a diff needs to have.

### Coverage parsing

`src/coverage/mod.rs` auto-detects the report format and dispatches to a native-format adapter.

The format adapters are:

- `src/coverage/llvm_json.rs`
- `src/coverage/coverlet_json.rs`
- `src/coverage/istanbul_json.rs`

Each adapter translates native report data into a shared `CoverageReport`. That shared model lives in `src/model.rs`.

### Shared internal model

`src/model.rs` defines the types that connect the whole program:

- `MetricKind`
- `GateRule`
- `CoverageOpportunity`
- `CoverageReport`
- `ComputedMetric`
- `GateResult`

If you are trying to understand what `covgate` fundamentally "means" by line, region, branch, or function coverage in a diff gate, this file defines the vocabulary.

### Metric calculation and gating

`src/metrics.rs` computes changed metrics from a `CoverageReport` plus changed diff lines. This is where normalized opportunities become changed covered/total counts and uncovered changed opportunity lists.

`src/gate.rs` evaluates configured rules against those computed metrics. It does not know how coverage reports are parsed and it does not know how diffs are loaded. It only knows how to decide pass or fail from already-computed metrics.

### Rendering

`src/render/console.rs` renders the terminal summary.

`src/render/markdown.rs` renders the Markdown summary.

These renderers are downstream of the calculation model. They are presentation layers, not alternate coverage engines.

### Tests and fixture generation

`tests/` contains integration tests. The most important current boundaries are:

- `tests/cli_interface.rs` for command behavior and workflow semantics
- `tests/cli_metrics.rs` for shared gate semantics across fixture families
- `tests/llvm_diff_regression.rs` for exact changed-opportunity and CLI-gate regressions on LLVM-backed fixtures

`tests/support/mod.rs` is the shared fixture harness.

`xtask/src/main.rs` contains repository-local build and validation tasks, including fixture coverage regeneration.

## Architecture Invariants

### `covgate` owns the gating model

Native tools produce input artifacts. `covgate` parses those artifacts into its own internal model and computes gate results from that model. The core pass/fail behavior must remain explainable in terms of `CoverageOpportunity`, changed line overlap, and `GateRule`.

### The diff gate is opportunity-first

The decisive question is not "what is the project's global coverage?" The decisive question is "which coverage opportunities overlap the changed lines, and are those opportunities covered?"

This is why `src/metrics.rs` is a core architectural boundary. That module expresses the product's real contract.

### Parsers normalize into one shared model

LLVM, Coverlet, and Istanbul all have different native shapes. They must converge into one shared internal model before gating or rendering happens. Feature-specific logic in renderers or gate evaluation is a design smell unless it is purely presentational.

### Renderers do not define semantics

Console and Markdown output are views over computed results. They must not silently substitute another source of truth for totals or gate outcomes.

In particular, normal `covgate` summaries are calculation-backed, not pass-through views of native upstream summary fields. Native summaries are useful for comparison, investigation, and regression tests, but they are not the production source of truth for `covgate` output.

This invariant matters because some native tools, especially LLVM-based ones, can expose competing report semantics for the same run. `covgate` must not hide that ambiguity by printing upstream summary numbers as if they were equivalent to its own calculation model.

### Path normalization is part of correctness

Coverage reports, Git diffs, and repository working directories often disagree about path shape. Each parser normalizes paths relative to the current repository root so diff matching can work across absolute and repo-relative inputs. A broken path-normalization change can make coverage look perfect by matching nothing.

### The recorded-base workflow is a first-class feature

`record-base` is not a side utility. It is part of the architecture for agent and sandbox workflows where standard remote refs are unavailable. The `refs/worktree/covgate/base` path is a stable architectural boundary between Git state discovery and diff gating.

## Important Boundaries

### Native coverage tool -> `covgate`

The parser boundary is where lossy or tool-specific concepts are translated into `covgate`'s shared model. When correctness questions arise, first ask whether the native report actually exposes the needed information.

### Changed line ranges -> changed opportunities

The overlap check in `SourceSpan::overlaps_line_range` is simple, but it is the boundary that turns generic coverage data into diff coverage. Many user-visible gate results reduce to whether this mapping is correct.

### Calculation -> presentation

`ComputedMetric` and `GateResult` are the boundary objects between "what the tool believes" and "how the tool explains it." This is why summary rows and gate messages should stay downstream of the calculation model.

## Cross-Cutting Concerns

### Lossless-first coverage support

`covgate` is intentionally selective about accepted formats. The project prefers native, structural coverage formats over flattened interchange formats because diff gating depends on preserving executable structure.

### Confidence comes from fixture-backed behavior

This repository treats test fixtures as architectural assets, not just convenience data. Parser unit tests, CLI integration tests, and real-export diff regressions are the main way we prove that the normalization and gating model still matches reality closely enough to trust.

### Validation workflow

`cargo xtask quick` is the fast development loop.

`cargo xtask validate` is the broad validation sweep. It intentionally runs multiple validation commands and reports all pass/fail outcomes before returning a final exit code.

## Where To Start

If you are new to the codebase, read in this order:

1. `README.md` for product intent
2. `src/lib.rs` for the end-to-end execution path
3. `src/model.rs` for shared vocabulary
4. `src/metrics.rs` and `src/gate.rs` for the diff-gating core
5. one parser module in `src/coverage/`
6. the relevant integration test in `tests/`

If you are investigating LLVM summary mismatches specifically, start with:

- `docs/reference/llvm-export-semantics-investigation.md`
- `docs/reference/function-coverage-debugging.md`
- `tests/llvm_real_parity.rs`
- `tests/llvm_diff_regression.rs`
