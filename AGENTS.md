# AGENTS

## Repository Map
- `docs/` – Repository knowledge system of record, including design docs, references, generated docs, product specs, and execution plans.
- `docs/IDEAS.md` – Backlog of future `covgate` ideas that are intentionally not committed ExecPlan scope yet.
- `docs/PLANS.md` – Execution plan authoring and maintenance rules. Use this when creating, updating, or completing ExecPlans in `docs/exec-plans/`.
- `src/` – Rust code for the `covgate` linter.

## Rust Workflow
- Format Rust code with `cargo fmt`.
- Run `cargo check` as the fast baseline compiler verification step.
- Lint Rust code with `cargo clippy --all-targets --all-features -- -D warnings`.
- Run `cargo test` for the automated test suite.
- Run `cargo llvm-cov --summary-only` and keep coverage at or above 80% across the codebase before considering work complete.
- For small lexical filters and token-shape checks, prefer direct string predicates such as `contains()` plus a single `chars().any(...)` pass over regexes. Reserve regexes for genuinely pattern-oriented parsing or matching.
