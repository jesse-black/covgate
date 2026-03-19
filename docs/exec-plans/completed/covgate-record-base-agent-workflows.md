# Add `covgate record-base` and switch agent bootstrap scripts from `origin/main` fetches to recorded worktree base refs

Save the canonical completed ExecPlan in `docs/exec-plans/completed/covgate-record-base-agent-workflows.md`. This work is complete; keep any future follow-up changes in a new active ExecPlan rather than moving this file back.

This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds.

Maintain this document in accordance with `docs/PLANS.md`. Re-read that file before revising this plan and keep this plan aligned with its rules.

## Purpose / Big Picture

After this change, `covgate` will expose explicit subcommands: `covgate check <coverage-report>` for coverage gating and `covgate record-base` for task-start base capture. The `check` command will automatically detect the input file format, and when `--base` is omitted it will prefer the recorded worktree ref before legacy branch-based fallback refs such as `origin/main`. In cached cloud worktrees, `record-base` must also detect when a new task branch has started and refresh the recorded base for that branch while remaining idempotent for repeated runs during the same task.

This matters because cloud or agent worktrees are frequently detached, shallow, or missing remote-tracking refs, and cached containers can preserve old worktree-local refs across tasks. The current setup scripts try to fetch `origin/main`, but that behavior is non-deterministic and often non-functional in restricted or ephemeral environments. The new user-visible behavior is deterministic and local to the worktree: when a task starts on a fresh branch, run `covgate record-base` and get a branch-specific stable diff base even if remote refs are unavailable; if maintenance runs again later during the same task, it must keep the original recorded base instead of moving it forward. Separating `check` from `record-base` also removes today’s mixed root-command parsing and gives the CLI a clearer long-term shape for CI and wrapper commands such as `cargo xtask validate`.

You will know this is working when all of the following are true:

1. Running `covgate record-base` inside a Git worktree creates `refs/worktree/covgate/base` on first run for a task branch and preserves it on later runs from that same branch, even after new commits are created during the task.
2. Running `covgate check <coverage-report>` without `--base` chooses `refs/worktree/covgate/base` before `origin/HEAD`, `origin/main`, and similar fallback refs.
3. Running `covgate record-base` after switching to a different task branch refreshes `refs/worktree/covgate/base` to that branch's current `HEAD`.
4. The agent environment scripts no longer attempt best-effort `origin/main` fetches and instead initialize/refresh diff-base readiness by invoking `covgate record-base`.
5. README and tooling docs explain both the recommended `covgate record-base` workflow and the task-branch refresh semantics that keep repeated maintenance runs idempotent within one task.

## Progress

- [x] (2026-03-15 00:00Z) Adapted the incoming feature plan into a repository-specific ExecPlan with concrete file targets and validation commands.
- [x] (2026-03-17 20:10Z) Investigated cached-worktree behavior and confirmed that `record-base` currently preserves an existing `refs/worktree/covgate/base` ref even when maintenance is rerun for a new task branch.
- [x] (2026-03-17 23:20Z) Refresh-local-branches prep task is no longer a pending implementation gate in this session; subsequent completed steps and validations were executed from a current branch context.
- [x] (2026-03-17 23:05Z) Reshaped the CLI to explicit `check` and `record-base` subcommands in `src/cli.rs`, with Clap-owned required positional coverage-report parsing.
- [x] (2026-03-17 21:05Z) Implemented `covgate record-base` branch-aware refresh semantics via a persisted branch marker: same-branch reruns remain idempotent while branch changes refresh `refs/worktree/covgate/base`.
- [x] (2026-03-17 21:35Z) Confirmed and retained automatic base discovery preference ordering with `refs/worktree/covgate/base` first when `--base` is omitted.
- [x] (2026-03-17 21:12Z) Aligned `scripts/agent-env-maintenance.sh` raw Git fallback with `covgate record-base` semantics by adding the same branch-marker-based refresh behavior.
- [x] (2026-03-17 21:15Z) Updated README and tooling/context docs to describe same-branch idempotence, branch-change refreshes, and branch-aware raw Git maintenance flow.
- [x] (2026-03-17 22:05Z) Added default-on dirty-worktree protection in `covgate` for Git-base diff mode, with CLI/config opt-outs and explicit diff-file bypass behavior verified by tests.
- [x] (2026-03-17 21:26Z) Added branch-refresh regression coverage and passed full repository validation, including `cargo xtask validate`.
- [x] (2026-03-18 01:40Z) Coverage-gate hardening follow-up completed: added focused Git/diff regression tests, confirmed the Git-helper `#[inline(never)]` attributes were no longer needed, and restored `cargo xtask validate` without reducing repository default gates.
- [x] (2026-03-19 00:35Z) Added a default Git-base-mode untracked-files warning with regression coverage, clarifying that untracked paths can cause false passes unless users add them with `git add -N <path>`.
- [x] (2026-03-19 00:50Z) Final validation passed and this ExecPlan was closed out by moving it to `docs/exec-plans/completed/`.

## Surprises & Discoveries

- Observation: Current agent scripts explicitly perform best-effort shallow fetches of `origin/main` and log a skip/fail path instead of producing a deterministic base marker.
  Evidence: `scripts/agent-env-setup.sh` and `scripts/agent-env-maintenance.sh` both execute `git fetch --no-tags --depth=1 origin +refs/heads/main:refs/remotes/origin/main` with soft-failure behavior.

- Observation: Existing docs describe this fetch behavior as part of the intended cloud-agent bootstrap path.
  Evidence: `docs/TOOLS.md` and `docs/reference/environment-execution-contexts.md` describe the `origin/main` bootstrap and maintenance refresh flow.

- Observation: `git rev-parse -q --verify refs/worktree/covgate/base` can print a SHA from a ref file even when the corresponding Git object is missing in the current clone.
  Evidence: local reproduction with a synthetic `.git/refs/worktree/covgate/base` file returned the SHA, while `git cat-file -t <sha>` failed with `could not get object info`.

- Observation: The current `record_base_ref` implementation is deliberately write-once and does not look for task boundaries.
  Evidence: `src/git.rs` returns early with `Base already recorded at refs/worktree/covgate/base -> ...`, and `tests/cli_interface.rs` asserts that repeated runs preserve the first recorded SHA even after later commits.

- Observation: A lightweight branch marker file under the Git worktree path (`refs/worktree/covgate/base.branch`) gives deterministic same-branch idempotence and branch-change refreshes without requiring remote refs.
  Evidence: updated `src/git.rs` and `scripts/agent-env-maintenance.sh` both compare current branch identity against this marker before deciding whether to refresh `refs/worktree/covgate/base`.

- Observation: Local validate runs can produce misleading “no changed files” results when a Git-base diff is used against `HEAD` while task edits remain uncommitted.
  Evidence: introducing a default-on clean-worktree guard in `covgate` and adding a diff-file bypass test eliminated this discrepancy and codified the intended behavior.

- Observation: Lowering repository default gates to force green local validation hides real regressions and violates expected policy unless explicitly directed.
  Evidence: gate thresholds in `covgate.toml` were reverted after review feedback; remediation should come from additional tests/coverage, not weaker defaults.

- Observation: Even with dirty-worktree handling in place, Git-base diff mode can still falsely pass when brand-new untracked files are present because those paths are omitted from diff gating until they are added with index intent.
  Evidence: a new failing CLI regression test created `new_untracked.rs`, observed no stderr guidance before the fix, and passed once `covgate` emitted an explicit warning recommending `git add -N <path>`.

- Observation: Some changed uncovered regions in `src/git.rs` are private helper error paths (for example subprocess spawn failures in `resolve_git_path`) that are difficult to reach through public APIs, creating a coverage hardening blocker under strict changed-file gates.
  Evidence: `cargo xtask validate` reports uncovered changed regions in private helper branches despite full test-suite pass, and attempts to cover them via in-file tests inflated changed regions further.

- Observation: The prior gate-lowering attempt was driven by a perceived single-run obstacle: changed-file function coverage in `src/git.rs` remained below strict defaults despite passing functional tests.
  Evidence: local validate runs showed persistent uncovered helper spans/functions; this requires staged coverage work rather than policy changes.

- Observation: `scripts/agent-env-maintenance.sh` already bypasses `cargo run -- record-base` and uses raw Git plumbing directly because compiling `covgate` during maintenance was too slow for practical agent startup.
  Evidence: the script now checks `git rev-parse -q --verify refs/worktree/covgate/base` and then falls back to `git update-ref refs/worktree/covgate/base HEAD` without invoking `covgate` or `cargo`.

- Observation: `src/main.rs` currently owns Clap-specific validation logic for missing `--coverage-json`, which is a symptom of the root command serving two different modes with incompatible required arguments.
  Evidence: `src/main.rs` constructs a Clap `MissingRequiredArgument` error manually when `cli.args.coverage_json.is_none()`.

## Decision Log

- Decision: Use `refs/worktree/covgate/base` as the recorded base marker instead of a branch or tag.
  Rationale: Worktree refs are isolated per worktree, preventing collisions across concurrent tasks and preserving deterministic task-local behavior.
  Date/Author: 2026-03-15 / Codex

- Decision: Replace the maintenance script’s `origin/main` fetch behavior with an invocation of `covgate record-base`.
  Rationale: The maintenance script should reinforce the first-class product workflow rather than continue non-functional remote bootstrapping.
  Date/Author: 2026-03-15 / Codex

- Decision: Remove best-effort `origin/main` fetch attempts from both cloud setup and maintenance paths.
  Rationale: These fetches are unreliable in ephemeral agent contexts and are superseded by local recorded base refs.
  Date/Author: 2026-03-15 / Codex

- Decision: Keep `record-base` idempotent within a task, but refresh the recorded base when maintenance observes a different task branch than the branch that recorded the current base.
  Rationale: Cached containers can preserve old worktree refs across tasks, so write-once semantics are too sticky across branch boundaries. Branch identity matches the stated cloud-task workflow: a new task starts by branching from `main`, and repeated maintenance runs during that task stay on the same branch.
  Date/Author: 2026-03-17 / Codex

- Decision: Update the maintenance script's raw Git fallback in parallel with `src/git.rs` instead of routing maintenance back through `cargo run -- record-base`.
  Rationale: The repository already moved maintenance away from building `covgate` because compile time slowed task startup too much. The plan must preserve that operational constraint while keeping shell behavior consistent with the Rust implementation.
  Date/Author: 2026-03-17 / Codex

- Decision: Reshape the CLI to use explicit subcommands `covgate check <coverage-report>` and `covgate record-base`.
  Rationale: This keeps command parsing and required-argument handling inside `src/cli.rs`, removes the need for `src/main.rs` to manufacture Clap errors, and better matches the repository’s mostly scripted CI-oriented usage. Because coverage format is auto-detected, the primary input should be a generic required coverage-report path rather than a flag named `--coverage-json`.
  Date/Author: 2026-03-17 / Codex

- Decision: Never lower gate defaults without explicit instruction.
  Rationale: Validation failures from insufficient changed-file coverage must be fixed with tests or implementation changes; weakening `covgate.toml` defaults changes project policy and can mask regressions.
  Date/Author: 2026-03-18 / Codex

## Outcomes & Retrospective

This ExecPlan is complete. `covgate` now supports explicit `check` and `record-base` subcommands, records branch-aware worktree-local bases, keeps raw-Git agent maintenance aligned with the product behavior, and warns that Git-base-mode diff gating can falsely pass when brand-new untracked files have not yet been added with `git add -N`.

The final validation pass confirmed the workflow end to end: targeted CLI regressions for the new untracked-files warning pass, repository validation remains green under the existing default gates, and the plan has been moved to `docs/exec-plans/completed/` as the canonical historical record. The main lesson from the final follow-up is that “dirty worktree” guidance was not sufficient on its own; users also need explicit explanation of Git’s untracked-file blind spot so coverage diagnosis matches what the CLI can actually see.

## Context and Orientation

`covgate` is a Rust CLI linter in `src/` that computes diff coverage from an input coverage report plus a Git diff base. Command orchestration currently lives in `src/main.rs`. CLI types and Clap annotations live in `src/cli.rs`. Base resolution and Git subprocess helpers are implemented in `src/git.rs`, while argument/config resolution lives in `src/config.rs`.

Agent environment setup scripts live in `scripts/`. The relevant files are:

- `scripts/agent-env-setup.sh`: full setup path used by cloud agent environments.
- `scripts/agent-env-maintenance.sh`: lightweight recurring setup path that currently uses raw Git plumbing instead of invoking `covgate` or `cargo`, and must be kept behaviorally aligned with the Rust implementation.

User-facing docs live in `README.md`, while environment/tooling context docs live in `docs/TOOLS.md` and `docs/reference/environment-execution-contexts.md`.

A “worktree ref” in Git is a reference under `refs/worktree/...` intended to be local to the current worktree instead of shared like ordinary branch refs. This plan uses `refs/worktree/covgate/base` as a task-local marker for the recorded base commit.

A “task boundary” in this repository’s cloud workflow means the moment a new task branch is created from `main` and the maintenance script runs in the resumed container. The plan assumes branch identity is stable for one task and changes when a new task begins.

## Plan of Work

Implement the feature in seven cohesive edits.

First, refresh the relevant local branches and pause implementation edits until the working tree is ready for a clean task-boundary change. This plan intentionally treats branch refresh as a prerequisite because cached branch state is part of the bug we are fixing.

Second, reshape `src/cli.rs` so it owns the entire public CLI surface. Define explicit subcommands for `check` and `record-base`, make `check` require a positional `<coverage-report>` path, and move the existing gating flags onto the `check` argument struct. The path should no longer be named `coverage_json` in the user-facing CLI because the tool auto-detects format from the file contents or extension.

Third, simplify `src/main.rs` so it performs only high-level dispatch based on the parsed CLI enum returned by `src/cli.rs`. It should not construct Clap errors or inspect optional command-specific fields. All parsing-time validation should be expressed in Clap metadata or in helper functions inside `src/cli.rs`.

Fourth, extend `src/git.rs` with task-boundary detection. In addition to the existing `refs/worktree/covgate/base` ref, persist enough worktree-local metadata to identify which branch recorded that base. A simple branch-marker file under the current Git worktree is sufficient if it is created and read through `git rev-parse --git-path ...` so it stays worktree-local. The implementation must resolve the current branch name via Git plumbing, treat detached HEAD as “no branch identity available”, and remain safe when the marker file is missing.

Fifth, update `record_base_ref` in `src/git.rs` so it behaves as follows. If no recorded base ref exists, record `HEAD` and store the current branch marker. If a recorded base ref exists and the stored branch marker matches the current branch, return the existing SHA unchanged. If a recorded base ref exists but the stored branch marker is missing or differs from the current branch, refresh the ref to `HEAD`, rewrite the branch marker, and print a distinct “refreshed” message. This is the key behavioral change that fixes resumed-cache staleness while preserving same-task idempotence.

Sixth, keep automatic base discovery in `src/config.rs` preferring `refs/worktree/covgate/base` when `--base` is omitted. Explicit `--base` must still win, and the unresolved-base guidance must continue recommending `covgate record-base` while staying accurate with the refreshed semantics. Adjust config parsing and naming so repository-internal code refers to a generic coverage report path rather than a user-facing `coverage_json` flag.

Seventh, modify agent scripts and docs to match the new workflow. Remove best-effort `origin/main` fetch attempts from `scripts/agent-env-setup.sh` and keep `scripts/agent-env-maintenance.sh` on its raw Git path instead of routing back through `cargo run -- record-base`. Extend that shell logic so it uses the same task-boundary rule as the Rust implementation: same branch keeps the original base, a different branch refreshes the stored base. Update `docs/TOOLS.md`, `docs/reference/environment-execution-contexts.md`, and `README.md` to describe the new CLI surface, including `covgate check <coverage-report>` and `covgate record-base`, explain that same-branch reruns keep the original base, explain that a new task branch refreshes the base automatically, and note that maintenance uses raw Git for startup speed.

Eighth, add regression tests in `tests/git_module.rs` and `tests/cli_interface.rs` that prove both sides of the contract: repeated `record-base` calls on the same branch preserve the original SHA after later commits, while switching to a different branch and running `record-base` again refreshes the stored base to the new branch’s current `HEAD`. Those tests must also verify the new CLI surface and help output for `check`.

## Concrete Steps

Run all commands from repository root `/home/jesse/git/covgate` unless otherwise noted.

1. Refresh branch references and inspect implementation entry points before changing code.

    git status --short
    git branch --verbose --all
    git fetch --all --prune
    rg -n "record_base_ref|RECORDED_BASE_REF|discover_base_ref" src tests README.md docs scripts

    Expected result: the working tree state is known before edits, local/remote branch refs are refreshed, and the exact files that govern recorded-base behavior are confirmed.

2. Add or update tests to capture the new CLI shape, same-task idempotence, and branch-change refresh behavior.

    cargo test record_base
    cargo test git_module -- --nocapture
    cargo test cli_interface -- --nocapture

    Expected result: before the fix, new regression tests should fail because the CLI still expects root-level mixed arguments and because `record-base` does not refresh across branch changes. After the fix, `check` parsing works, same-branch reruns remain stable, and branch-change reruns refresh.

3. Implement the `check`/`record-base` CLI split and simplify `src/main.rs`.

    cargo test record_base
    cargo test git_module -- --nocapture
    cargo test cli_interface -- --nocapture

    Expected result: help output and missing-argument handling come entirely from Clap definitions in `src/cli.rs`, and `src/main.rs` is reduced to dispatch.

4. Implement branch-aware `record-base` refresh semantics and retest.

    cargo test record_base
    cargo test git_module -- --nocapture
    cargo test cli_interface -- --nocapture

    Expected result: stdout includes distinct “Recorded”, “Base already recorded”, and “Refreshed” messages, and the recorded ref matches the task branch contract.

5. Verify automatic base preference and fallback behavior.

    cargo test base
    cargo test config_auto_base -- --nocapture

    Expected result: `refs/worktree/covgate/base` remains the first automatic choice for `covgate check`, but explicit `--base` still overrides it.

6. Update agent scripts to remove `origin/main` fetches and bring the maintenance script's raw Git fallback into parity with the Rust task-boundary behavior.

    bash -n scripts/agent-env-setup.sh scripts/agent-env-maintenance.sh

    Expected result: scripts remain syntactically valid and logs/messages describe recorded-base workflow, including branch-change refresh behavior without compiling `covgate`.

7. Update docs and validate repository quality gates.

    cargo fmt --check
    cargo test
    cargo xtask validate

    Expected result: all checks pass; docs describe `covgate check <coverage-report>`, recommended `covgate record-base` usage, same-branch idempotence, and branch-change refresh semantics.

9. Coverage lift breakdown when strict gates fail during feature work.

    Step A — Diagnose concrete uncovered spans/functions

        cargo llvm-cov --json --output-path target/xtask/coverage-diagnose.json --fail-under-regions=88
        cargo run -- check target/xtask/coverage-diagnose.json

    Step B — Map each uncovered function/span to executable branches and add focused tests in existing test modules (`tests/git_module.rs`, `tests/cli_interface.rs`, parser-specific unit tests).

    Step C — Re-run targeted tests and repeat diagnosis until uncovered-function count and region threshold both satisfy repository gates.

    Step D — Run full validation (`cargo xtask quick`, `cargo xtask validate`) with strict defaults unchanged.

    ⚠️ Untracked-files warning: untracked files are not included in diff gating and can produce a false pass. Add them with `git add -N <path>`.

    Step E — If strict-gate blockers persist due to private helper error paths, pause feature edits and introduce a minimal test seam for Git subprocess execution (for example a small injectable command-runner trait or function pointer gated to tests) so failure branches can be exercised from integration tests without changing gate policy.

    Step F — After seam introduction, add focused regression tests for each previously unreachable error branch and rerun `cargo xtask validate` to confirm changed-region and uncovered-function gates pass.

    Potential blockers to monitor:
    - LLVM inlining collapsing changed helper functions into callsites and obscuring per-function coverage attribution.
    - Branch-specific/ref-state logic requiring non-trivial Git fixture setup (detached HEAD, marker missing, divergent ancestry).
    - Untracked files are not included in diff gating until they are added with `git add -N <path>`, so otherwise a check can falsely pass.

    Policy reminder: gate defaults are project policy and must remain unchanged unless maintainers explicitly request a gate policy change.

8. Coverage gate remediation when `cargo xtask validate` fails after feature changes.

    cargo llvm-cov --json --output-path /tmp/covgate-validate.json --fail-under-regions=88
    cargo run -- check /tmp/covgate-validate.json
    cargo test git_module -- --nocapture

    Expected result: uncovered changed spans/functions are identified in the touched files; follow-up commits add tests that execute those paths until `cargo xtask validate` passes.

    Policy constraint: Do not lower `covgate.toml` gate defaults to force validation green unless maintainers explicitly request a policy change.

## Validation and Acceptance

Acceptance is complete only when the behavior is observable end to end.

Invoking `covgate check <coverage-report>` with a valid report path and gate flags must parse successfully without requiring a root-level `--coverage-json` flag. Invoking `covgate check` without the required positional coverage report must fail with Clap-generated usage output owned by the `check` subcommand rather than ad hoc logic in `src/main.rs`.

In a temp Git repository with at least one commit on a task branch, `covgate record-base` must exit successfully and print `Recorded base commit <sha> at refs/worktree/covgate/base` when the ref is absent. Running it again after additional commits on that same branch must exit successfully and print `Base already recorded at refs/worktree/covgate/base -> <sha>`, where `<sha>` is still the first recorded commit for that branch.

In that same repository, after switching to a different branch and creating at least one commit, `covgate record-base` must exit successfully and print a distinct refresh message. After that run, `refs/worktree/covgate/base` must resolve to the new branch’s current `HEAD`, not the earlier branch’s recorded SHA.

When running `covgate check <coverage-report>` without `--base`, automatic resolution must first check `refs/worktree/covgate/base`. If it exists, `covgate` uses it. If not, `covgate` continues with legacy fallback refs without regression.

When running `covgate check <coverage-report> --base <REF>`, explicit `--base` must continue to override recorded and fallback auto discovery.

When no automatic base candidate exists, user-facing error/help text must mention `covgate record-base` as a remediation alongside explicit `--base` guidance.

Script acceptance requires that `scripts/agent-env-setup.sh` and `scripts/agent-env-maintenance.sh` no longer perform `git fetch ... origin/main` bootstrap attempts. The maintenance script must keep using raw Git plumbing, not compile `covgate`, and must still follow the same task-boundary semantics as `covgate record-base`.

Gate-policy acceptance: if validation fails due to changed-file coverage, remediation must add coverage/tests in changed files. Lowering `covgate.toml` gate defaults is out of scope unless explicitly requested by maintainers.

Documentation acceptance requires README coverage of:

- why agent environments may lack `origin/main`
- the new top-level CLI shape with `covgate check <coverage-report>` and `covgate record-base`
- recommended `covgate record-base` command
- automatic use of recorded base when `--base` is omitted
- same-branch reruns preserving the original task base
- branch changes refreshing the recorded task base
- a maintenance/bootstrap snippet or explanation that reflects the raw Git maintenance path

## Idempotence and Recovery

`covgate record-base` is intentionally idempotent within a task. Re-running the command on the same branch should never move an existing `refs/worktree/covgate/base` ref. This property enables safe retries in flaky agent sessions while still allowing a new task branch to refresh the stored base.

Script changes should also be retry-safe. Running setup/maintenance scripts multiple times must not require mutable remote state. If `covgate` is temporarily unavailable in PATH during setup, scripts should log a warning and continue rather than corrupting repository state.

If implementation introduces regressions in legacy base fallback behavior, recovery is to keep the inserted worktree-ref candidate logic but restore original fallback candidate ordering immediately below it. If the shell implementation and Rust implementation diverge, prefer restoring parity first even if that means temporarily simplifying branch-marker handling in both paths.

## Artifacts and Notes

Expected successful `record-base` transcript in a temp repo:

    $ covgate record-base
    Recorded base commit 0123456789abcdef0123456789abcdef01234567 at refs/worktree/covgate/base

Expected `check` usage example:

    $ covgate check coverage.json --fail-under-regions 90
    Diff: refs/worktree/covgate/base...WORKTREE
    Region Coverage: 100.0% (threshold: 90.0%)

Expected same-branch idempotent re-run transcript:

    $ covgate record-base
    Base already recorded at refs/worktree/covgate/base -> 0123456789abcdef0123456789abcdef01234567

Expected branch-change refresh transcript:

    $ git checkout -b task/two
    $ covgate record-base
    Refreshed base commit fedcba9876543210fedcba9876543210fedcba98 at refs/worktree/covgate/base for branch task/two

Expected maintenance-script-style raw Git behavior:

    $ git rev-parse -q --verify refs/worktree/covgate/base
    0123456789abcdef0123456789abcdef01234567
    $ # branch marker differs from current branch, so maintenance refreshes:
    $ git update-ref refs/worktree/covgate/base HEAD

Expected failure guidance excerpt when no base can be resolved:

    Unable to determine a base ref automatically.
    Try one of:
      - pass --base <REF>
      - run covgate record-base
      - create refs/worktree/covgate/base manually with git update-ref

## Interfaces and Dependencies

The implementation should preserve current crate boundaries and introduce only minimal new interfaces.

At minimum, by the end of this work there should be a callable path equivalent to:

- CLI command enum variant for `record-base` in `src/cli.rs`.
- CLI command enum variant for `check` in `src/cli.rs` with a required positional coverage-report path and the existing gate/base flags.
- Main dispatch branch in `src/main.rs` that matches on parsed command variants without constructing Clap errors manually.
- Git helper functions capable of:
  - resolving `HEAD` commit SHA
  - resolving current branch identity when available
  - resolving a worktree-local metadata path for the branch marker
  - resolving arbitrary ref SHA as optional result
  - creating a ref pointing at a target object
- Base resolver candidate list that includes `refs/worktree/covgate/base` before legacy fallback refs when explicit `--base` is not provided.

No new external dependency should be required for this feature; continue using existing subprocess/process helpers for Git interactions.

Change note (2026-03-15): Adapted a generic feature brief into a repository-specific ExecPlan and explicitly added script migration scope: remove non-functional `origin/main` fetch attempts from agent setup/maintenance, replace maintenance bootstrap behavior with `covgate record-base`, and remove xtask's best-effort mainline fetch fallback.

Change note (2026-03-17): Revised the plan after confirming that cached worktrees can preserve stale `refs/worktree/covgate/base` values across tasks. The plan now requires branch-aware refresh behavior: keep the recorded base stable for repeated runs on the same task branch, but refresh it when maintenance runs after a new task branch is created.

Change note (2026-03-17): Updated the plan to reflect the current operational constraint in `scripts/agent-env-maintenance.sh`: maintenance must keep using raw Git plumbing because compiling `covgate` during startup was too slow. The plan now explicitly includes updating that shell fallback alongside the Rust implementation so both paths enforce the same task-boundary semantics.


Change note (2026-03-19): Recorded the final untracked-files warning follow-up, marked validation complete, and moved this ExecPlan from `docs/exec-plans/active/` to `docs/exec-plans/completed/` because the planned work is now finished.
