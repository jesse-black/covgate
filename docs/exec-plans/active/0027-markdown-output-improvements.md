---
description: "ExecPlan for adding stdout support (`-` sentinel) and auto-detected GitHub Step Summary output to covgate's markdown rendering; read when implementing or reviewing markdown output destination changes."
---

# Markdown Output: Stdout and GitHub Step Summary

## Goal
- `--markdown-output -` writes the markdown report to stdout.
- When the `GITHUB_STEP_SUMMARY` environment variable is set, covgate automatically appends the markdown report to the file it points to (the GitHub Actions step summary).
- Both destinations can be active simultaneously (explicit `--markdown-output` plus auto-detected `GITHUB_STEP_SUMMARY`).
- `--no-github-summary` suppresses the auto-detection.

## Scope
- In: `src/cli.rs`, `src/config.rs`, `src/lib.rs`, `xtask/src/main.rs`, `tests/cli_interface.rs`, `tests/support/mod.rs`.
- Out: markdown render logic (`src/render/markdown.rs`), gate semantics, all other CLI flags.

## Relevant Areas
- `src/cli.rs` — `Args` struct; `--markdown-output` and new `--no-github-summary` flag.
- `src/config.rs` — `Config`, `FileConfig`, and `TryFrom<Args>`; add `OutputSink` enum and `no_github_summary: bool`.
- `src/lib.rs:98-101` — current markdown write site; extend to handle sink enum and GITHUB_STEP_SUMMARY.
- `xtask/src/main.rs:983-1042` — `run_llvm_cov`, `llvm_cov_task`, `covgate_task`, and their call sites; remove freshness cache, remove `--force`, forward extra args.
- `tests/cli_interface.rs` — CLI interface tests; add cases for all three destinations.
- `tests/support/mod.rs` — add `run_covgate_with_env` helper to inject env vars.

## Open Questions
- None yet

## Steps

### Production code

- [x] Add `OutputSink` enum to `src/config.rs`:
  ```rust
  pub enum OutputSink {
      File(PathBuf),
      Stdout,
  }
  ```
- [x] Change `Config.markdown_output: Option<PathBuf>` → `markdown_output: Option<OutputSink>`.
- [x] Add `Config.no_github_summary: bool`.
- [x] Update `TryFrom<Args> for Config` in `src/config.rs`:
  - Convert `--markdown-output -` → `OutputSink::Stdout`.
  - Convert any other path → `OutputSink::File(path)`.
  - Apply the same `-`-to-`Stdout` conversion when resolving the value from `FileConfig.markdown_output`.
  - Propagate `args.no_github_summary` to `Config.no_github_summary`.
- [x] Add `--no-github-summary` boolean flag to `Args` in `src/cli.rs`.
- [x] Update `src/lib.rs::run`:
  - Destructure `no_github_summary` from `Config` alongside `markdown_output`.
  - Render the markdown string once (reuse for all sinks).
  - Match on `markdown_output`:
    - `Some(OutputSink::File(path))` → `std::fs::write(path, &markdown)?`
    - `Some(OutputSink::Stdout)` → `print!("{markdown}")`
    - `None` → no-op.
  - After explicit sink: if `!no_github_summary`, read `std::env::var("GITHUB_STEP_SUMMARY")`; if present, open the file in append mode and write the markdown.

### xtask

No tests required for xtask changes — it is an internal dev tool.

- [x] Migrate xtask arg parsing to clap: replace the manual `args.next()` dispatch in `main` with a clap `Cli` / `Subcommand` derived struct. Use `trailing_var_arg = true` on the `LlvmCov` and `Covgate` subcommands to capture forwarded args natively. Check whether clap is already in `xtask/Cargo.toml`; add it if not.
- [x] Remove the freshness cache: delete `coverage_is_fresh` and `most_recent_rs_mtime`; remove their call sites in `llvm_cov_task` and `covgate_task`.
- [x] Remove `--force` from `llvm_cov_task` (parameter and call site in `main`); `cargo xtask llvm-cov` always reruns.
- [x] Update `run_llvm_cov` to accept `extra_args: &[String]` and append them at the end of the `cargo llvm-cov` invocation.
- [x] Update `llvm_cov_task` and `covgate_task` to accept `extra_args: &[String]` and forward to `run_llvm_cov` / `cargo run` respectively.
- [x] Update the `"llvm-cov"` and `"covgate"` arms in `main` to collect remaining args (skipping a leading `--` separator if present) and pass them to their task functions.
- [x] Update the usage string in `main`: `llvm-cov [-- <llvm-cov-args>...]`, `covgate [-- <covgate-args>...]`; remove `[--force]`.

### Tests

- [x] Add `run_covgate_with_env(worktree, coverage_json, extra_args, env_vars)` helper to `tests/support/mod.rs` that accepts `&[(&str, &str)]` and sets them on the `Command` before running.
- [x] In `tests/cli_interface.rs`, add:
  - `markdown_output_to_stdout`: pass `--markdown-output -`; assert stdout contains markdown table content.
  - `markdown_output_to_file_regression`: pass `--markdown-output <tempfile>`; assert file is written with markdown content (regression guard).
  - `github_step_summary_auto_detected`: set `GITHUB_STEP_SUMMARY` to a temp file path; assert that file is created and contains markdown content after run.
  - `github_step_summary_suppressed_by_flag`: set `GITHUB_STEP_SUMMARY` and pass `--no-github-summary`; assert the summary file is not written.
  - `github_step_summary_and_explicit_output_both_write`: set `GITHUB_STEP_SUMMARY` and also pass `--markdown-output <file>`; assert both destinations are written.

## Validation
- `cargo test --test cli_interface markdown_output`
- `cargo test --test cli_interface github_step_summary`
- `cargo xtask validate`

## Discoveries
- Added failing CLI regressions first: `markdown_output_to_stdout` failed because `-` was treated as a file path, and GitHub summary tests failed because auto-detection and `--no-github-summary` did not exist yet.
- `cargo xtask validate` initially failed covgate's own changed-region gate on new markdown filesystem error branches in `src/lib.rs`; added CLI error-path tests for explicit file write errors and GitHub summary write errors instead of lowering gate defaults.
- Evaluator findings were addressed by adding config-file `markdown-output = "-"` coverage, destination-specific IO error context, stronger stderr assertions, and a GitHub summary open-error regression.

## Review
- [x] Evaluator finding: add direct coverage for config-file `markdown-output = "-"` resolving to `OutputSink::Stdout`.
- [x] Evaluator finding: strengthen markdown write-error tests so they assert stderr context, not only exit code.
- [x] Fresh evaluator pass completed cleanly after fixes; no remaining findings reported.
- [x] Evaluator finding (independent pass): `src/lib.rs:105` — `std::fs::write(path.as_path(), &markdown)` uses redundant `.as_path()`. Fixed to `std::fs::write(path, &markdown)`.
- [x] Evaluator observation (non-blocking): `xtask/src/main.rs` — `run_with_args` generic existed solely to bridge the `&[&str]` vs `&[String]` type mismatch. Flattened to two functions: `run` contains the implementation; `run_owned` converts via `args.iter().map(String::as_str).collect()` and delegates to `run`.
- [x] Evaluator finding: `xtask/src/main.rs:31-48` — replaced `trailing_var_arg = true` + `allow_hyphen_values = true` + `skip_arg_separator` with `#[arg(last = true)]` on both `LlvmCov.args` and `Covgate.args`; deleted `skip_arg_separator`. Clap now handles `--` stripping natively.
- [ ] Evaluator finding (post-merge regression): `tests/support/mod.rs` (`run_covgate_with_env`, `run_covgate_raw`) and `tests/cli_interface.rs` (`run_covgate_raw_with_path`) — test helpers inherited `GITHUB_STEP_SUMMARY` from the parent process environment via `Command::envs` / implicit inheritance. When the test suite runs in GitHub Actions, every `covgate check` invocation (not just the step-summary tests) appended its fixture output to the real step summary file, producing a massively duplicated report. Fixed by adding `command.env_remove("GITHUB_STEP_SUMMARY")` to each helper before the `envs()` call; step-summary tests re-add the variable explicitly so they still pass.
- [ ] Evaluator finding: no regression test covers `--markdown-output <path>` and `GITHUB_STEP_SUMMARY` pointing to the same file. In that scenario `std::fs::write` (explicit sink) overwrites the file, then the append-mode open writes again, producing a duplicated report. A failing test should be added first (TDD), then the production code deduplicated (e.g. skip the `GITHUB_STEP_SUMMARY` write when its resolved path equals the explicit output path).

## Definition of Done

### Planner
- [x] Plan is consistent, up to date, decision-complete, and ready to hand off.

### Generator
- [x] Goal achieved: stdout sink via `-`, auto-detected GITHUB_STEP_SUMMARY, `--no-github-summary` suppression, `--` arg forwarding for both xtask tasks, and freshness cache removed.
- [x] All planned steps are complete.
- [x] All validation commands pass.
- [x] Handed off to an independent reviewer (MUST use the `evaluator-execplan` skill via a subagent or separate agent, not the generator agent).

### Evaluator
- [ ] Standard review posture applied.
- [ ] Adheres to the principles of `docs/CODESTYLE.md`.
- [ ] Adheres to the principles of `docs/TESTING.md`.
- [ ] All review findings have been addressed.
