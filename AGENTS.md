# AGENTS

## Step 0 (required before keyword-searching for documentation)
Run `docgarden match <query>` before using `rg`, `grep`, `find`, or agents to locate Markdown documentation, plans, repository guidance, or repository-local skills.
Use this when your first instinct is to search docs or guidance by keyword.
Run `docgarden ls --active-plans` to list active ExecPlans when continuing or checking current plan-driven work.
Do not repeat this step when the relevant file is already named by the user, listed in this file, or still in active context.
Do not use this step for code-first work, code symbol searches, test names, compiler errors, or known file paths; inspect and search code directly.

## Development Process
### Must Follow
- Run `cargo xtask validate` before declaring Rust behavior changes complete, including parser, metric, gate, CLI behavior, fixture, or validation-policy changes. For documentation-only, CI-only, metadata-only, or lint/config-only changes, run the focused checks that exercise the touched surface instead.
- ALWAYS address bug reports and review findings with TDD: first reproduce the issue in a failing test, then fix the issue and rerun the relevant tests until they pass.
- NEVER lower repository gate defaults (for example in `covgate.toml`) without explicit maintainer instruction.

### Workflow
- During the edit-build-test loop, run the narrowest command that covers the changed surface, such as a focused `cargo test`, `cargo fmt`, or `cargo clippy`.
- Use `cargo xtask validate` when the change affects Rust behavior, fixtures, coverage semantics, gate defaults, release validation, or any path where the full coverage and dependency-audit sweep is the evidence needed.
- When skipping `cargo xtask validate`, state which focused checks were run and why they are sufficient for the change.

## Repository Map
### Start Here for Architecture and Implementation
- `ARCHITECTURE.md` – Top-level architecture codemap and invariants document. Read this first when you need the current system boundaries, code map, or architectural intent.
- `src/` – Rust code for the `covgate` linter.

### Start Here for Planning and Repository Guidance
- `docs/` – Repository knowledge system of record, including design docs, references, generated docs, product specs, and execution plans.
- `docs/PLANS.md` – Execution plan authoring and maintenance rules. Use this when creating, updating, or completing ExecPlans in `docs/exec-plans/`.
- `docs/TODO.md` - Small tasks and cleanups that came up during planning but aren't big enough or ready enough for a full exec plan.

### Start Here for Testing, Bugs, and Validation
- `docs/TESTING.md` – Canonical testing process and quality philosophy for unit, integration, CLI, and coverage validation.
- `tests/` – Integration tests, fixture-backed regression coverage, and shared test harness code. Start here for bug repros, CLI behavior, cross-language metric semantics, and real-world diff/coverage scenarios.
- `xtask/` – Repository-local automation for fast checks, full validation, and fixture coverage regeneration.
