# AGENTS

## Repository Map
### Start Here for Architecture and Implementation
- `ARCHITECTURE.md` – Top-level architecture codemap and invariants document. Read this first when you need the current system boundaries, code map, or architectural intent.
- `src/` – Rust code for the `covgate` linter.

### Start Here for Planning and Repository Guidance
- `docs/` – Repository knowledge system of record, including design docs, references, generated docs, product specs, and execution plans.
- `docs/PLANS.md` – Execution plan authoring and maintenance rules. Use this when creating, updating, or completing ExecPlans in `docs/exec-plans/`.

### Start Here for Testing, Bugs, and Validation
- `docs/TESTING.md` – Canonical testing process and quality philosophy for unit, integration, CLI, and coverage validation.
- `tests/` – Integration tests, fixture-backed regression coverage, and shared test harness code. Start here for bug repros, CLI behavior, cross-language metric semantics, and real-world diff/coverage scenarios.
- `xtask/` – Repository-local automation for fast checks, full validation, and fixture coverage regeneration.

## Rust Workflow
- Use `cargo xtask quick` for the test/check step of the edit-build-test loop during development.
- Run `cargo xtask validate` before considering work complete.
- Address bug reports and review findings with TDD: first reproduce the issue in a failing test, then fix the issue and rerun the relevant tests until they pass.
- Never lower repository gate defaults (for example in `covgate.toml`) without explicit maintainer instruction.
