# AGENTS

## Development Process
### Must Follow
- ALWAYS run `cargo xtask validate` before declaring code changes complete unless the user explicitly tells you not to or the command is blocked.
- NEVER declare work complete after `cargo xtask quick` alone.
- NEVER run `cargo xtask quick` and `cargo xtask validate` at the same time for the same check pass.
- ALWAYS address bug reports and review findings with TDD: first reproduce the issue in a failing test, then fix the issue and rerun the relevant tests until they pass.
- NEVER lower repository gate defaults (for example in `covgate.toml`) without explicit maintainer instruction.

### Workflow
- Use `cargo xtask quick` for the fast test/check step during the edit-build-test loop. It is the iteration command and intentionally skips the slower coverage-oriented validation work.
- Use `cargo xtask validate` as the final pre-completion check before declaring work complete. It is the full validation command, including coverage validation.
- Treat `cargo xtask quick` and `cargo xtask validate` as alternatives for a given check pass: use `quick` during iteration and `validate` at the end.

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
