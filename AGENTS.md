# AGENTS

## Step 0: First command routing

Before reaching for `rg`/`grep`/`find`, agents, `sed`, one-off scripts, or regex bulk edits, route by target type:

- **Active plans** → `docgarden ls --active-plans`.
- **Docs, repo guidance, repo-local skills, or topic routing** → `docgarden match <query>`.
- **Code symbols or structure** → `ast-grep --lang rust -p '<pattern>'`; use the `ast-grep` skill for patterns beyond simple syntax.
- **Structural bulk edits** → `ast-grep --lang rust -p '<pattern>' --rewrite '<replacement>'` or an `ast-grep scan` rule with `fix:`; use the `ast-grep` skill for rule authoring.

Skip routing only when the exact file is already named by the user or in active context. If a named symbol needs callers, definitions, signatures, or rewrites, still use `ast-grep`. Use plain-text tools directly only for literal non-structural text such as log lines, error messages, comments, fixture strings, or "does this exact text occur?" checks.

## Development Process
### Workflow
- During the edit-build-test loop, run the narrowest command that covers the changed surface, such as a focused `cargo test`, `cargo fmt`, or `cargo clippy`.
- For behavioral bug reports and code review findings, follow the TDD workflow in `docs/TESTING.md`: reproduce with a failing test, fix, then rerun focused checks.
- For Rust behavior changes, follow `docs/TESTING.md`: use focused checks during iteration and run `cargo xtask validate` before declaring the change complete.

## Repository Map
### Architecture
- `ARCHITECTURE.md` – Top-level architecture codemap and invariants document. Read this first when you need the current system boundaries, code map, or architectural intent.
- `src/` – Rust code for the `covgate` linter.

### Planning
- `docs/` – Repository knowledge system of record, including design docs, references, generated docs, product specs, and execution plans.
- `docs/PLANS.md` – Execution plan authoring and maintenance rules. Use this when creating, updating, or completing ExecPlans in `docs/exec-plans/`.
- `docs/TODO.md` - Small tasks and cleanups that came up during planning but aren't big enough or ready enough for a full exec plan.

### Testing
- `docs/TESTING.md` – Canonical testing process and quality philosophy for unit, integration, CLI, and coverage validation.
- `tests/` – Integration tests, fixture-backed regression coverage, and shared test harness code. Start here for bug repros, CLI behavior, cross-language metric semantics, and real-world diff/coverage scenarios.
- `xtask/` – Repository-local automation for fast checks, full validation, and fixture coverage regeneration.
