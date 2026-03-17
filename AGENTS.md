# AGENTS

## Repository Map
- `docs/` – Repository knowledge system of record, including design docs, references, generated docs, product specs, and execution plans.
- `docs/IDEAS.md` – Backlog of future `covgate` ideas that are intentionally not committed ExecPlan scope yet.
- `docs/PLANS.md` – Execution plan authoring and maintenance rules. Use this when creating, updating, or completing ExecPlans in `docs/exec-plans/`.
- `docs/TESTING.md` – Canonical testing process and quality philosophy for unit, integration, CLI, and coverage validation.
- `src/` – Rust code for the `covgate` linter.

## Rust Workflow
- Run `cargo xtask validate` before considering work complete.
- Address bug reports and review findings with TDD: first reproduce the issue in a failing test, then fix the issue and rerun the relevant tests until they pass.
