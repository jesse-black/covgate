# Add `covgate record-base` and switch agent bootstrap scripts from `origin/main` fetches to recorded worktree base refs

Save this in-progress ExecPlan in `docs/exec-plans/active/covgate-record-base-agent-workflows.md`. Move it to `docs/exec-plans/completed/covgate-record-base-agent-workflows.md` only after implementation, validation, and documentation updates are complete.

This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds.

Maintain this document in accordance with `docs/PLANS.md`. Re-read that file before revising this plan and keep this plan aligned with its rules.

## Purpose / Big Picture

After this change, `covgate` will support an explicit `covgate record-base` command that captures a stable per-worktree Git base commit at task start. When later `covgate` commands run without `--base`, they will automatically prefer that recorded worktree ref before legacy branch-based fallback refs such as `origin/main`.

This matters because cloud or agent worktrees are frequently detached, shallow, or missing remote-tracking refs. The current setup scripts try to fetch `origin/main`, but that behavior is non-deterministic and often non-functional in restricted or ephemeral environments. The new user-visible behavior is deterministic and local to the worktree: run `covgate record-base` once at task start, then run `covgate` normally and get a stable diff base even if remote refs are unavailable.

You will know this is working when all of the following are true:

1. Running `covgate record-base` inside a Git worktree creates `refs/worktree/covgate/base` on first run and preserves it on later runs.
2. Running `covgate` without `--base` chooses `refs/worktree/covgate/base` before `origin/HEAD`, `origin/main`, and similar fallback refs.
3. The agent environment scripts no longer attempt best-effort `origin/main` fetches and instead initialize/refresh diff-base readiness by invoking `covgate record-base`.
4. README and tooling docs explain both the recommended `covgate record-base` workflow and the raw Git plumbing equivalent.

## Progress

- [x] (2026-03-15 00:00Z) Adapted the incoming feature plan into a repository-specific ExecPlan with concrete file targets and validation commands.
- [ ] Implement `covgate record-base` CLI plumbing and Git helper logic.
- [ ] Extend automatic base discovery to prefer `refs/worktree/covgate/base` when `--base` is omitted.
- [ ] Remove non-functional `origin/main` fetch attempts from agent setup and maintenance scripts and replace maintenance behavior with `covgate record-base`.
- [ ] Update README and tooling/context docs for agent workflows and recorded base usage.
- [ ] Add and run tests plus full repository validation (`cargo xtask validate`).

## Surprises & Discoveries

- Observation: Current agent scripts explicitly perform best-effort shallow fetches of `origin/main` and log a skip/fail path instead of producing a deterministic base marker.
  Evidence: `scripts/agent-env-setup.sh` and `scripts/agent-env-maintenance.sh` both execute `git fetch --no-tags --depth=1 origin +refs/heads/main:refs/remotes/origin/main` with soft-failure behavior.

- Observation: Existing docs describe this fetch behavior as part of the intended cloud-agent bootstrap path.
  Evidence: `docs/TOOLS.md` and `docs/reference/environment-execution-contexts.md` describe the `origin/main` bootstrap and maintenance refresh flow.

## Decision Log

- Decision: Use `refs/worktree/covgate/base` as the recorded base marker instead of a branch or tag.
  Rationale: Worktree refs are isolated per worktree, preventing collisions across concurrent tasks and preserving deterministic task-local behavior.
  Date/Author: 2026-03-15 / Codex

- Decision: Keep `record-base` idempotent and write-once by default.
  Rationale: Task-start baselines must remain stable; moving the ref on every run would invalidate deterministic diffs.
  Date/Author: 2026-03-15 / Codex

- Decision: Replace the maintenance script’s `origin/main` fetch behavior with an invocation of `covgate record-base`.
  Rationale: The maintenance script should reinforce the first-class product workflow rather than continue non-functional remote bootstrapping.
  Date/Author: 2026-03-15 / Codex

- Decision: Remove best-effort `origin/main` fetch attempts from both cloud setup and maintenance paths.
  Rationale: These fetches are unreliable in ephemeral agent contexts and are superseded by local recorded base refs.
  Date/Author: 2026-03-15 / Codex

## Outcomes & Retrospective

This plan adaptation is complete, but feature implementation is not yet started. The outcome of this planning pass is a concrete, repo-scoped implementation path that includes CLI behavior, base-resolution behavior, script workflow changes, and documentation changes aligned with this repository’s current file layout.

The main risk to monitor during implementation is preserving existing local developer behavior while changing agent scripts and fallback messaging. Validation must prove both backward compatibility (`--base` still wins; legacy fallback refs still work) and the new deterministic agent flow.

## Context and Orientation

`covgate` is a Rust CLI linter in `src/` that computes diff coverage from coverage JSON plus a Git diff base. CLI parsing currently lives in `src/cli.rs` and command orchestration in `src/main.rs`. Base resolution and Git diff behavior are implemented in the Git/diff modules under `src/` (exact function names should be confirmed before editing).

Agent environment setup scripts live in `scripts/`. The relevant files are:

- `scripts/agent-env-setup.sh`: full setup path used by cloud agent environments.
- `scripts/agent-env-maintenance.sh`: lightweight recurring setup path that will call `covgate record-base`.

User-facing docs live in `README.md`, while environment/tooling context docs live in `docs/TOOLS.md` and `docs/reference/environment-execution-contexts.md`.

A “worktree ref” in Git is a reference under `refs/worktree/...` intended to be local to the current worktree instead of shared like ordinary branch refs. This plan uses `refs/worktree/covgate/base` as a task-local marker for the recorded base commit.

## Plan of Work

Implement the feature in six cohesive edits.

First, extend CLI shape to include a `record-base` subcommand. Update the command enum/parser in `src/cli.rs` and dispatch in `src/main.rs` so `covgate record-base` executes a dedicated handler without requiring coverage input arguments.

Second, add or extend Git helper functions (in whichever module currently owns Git subprocess calls) so the command can validate repository context, resolve `HEAD`, resolve ref existence, and create refs deterministically. The implementation should use Git plumbing commands equivalent to `git rev-parse -q --verify` and `git update-ref` while keeping subprocess handling aligned with existing repository patterns.

Third, implement `record-base` behavior: validate Git repo, resolve `HEAD` commit SHA, check `refs/worktree/covgate/base`, create only if absent, and print deterministic stdout messages for “recorded” vs “already recorded”. Return nonzero on true failures with clear stderr.

Fourth, update automatic base discovery logic used when `--base` is omitted. Insert `refs/worktree/covgate/base` at the top of auto-candidate refs before existing fallbacks such as `origin/HEAD`, `origin/main`, `origin/master`, `main`, and `master`. Keep explicit `--base` precedence unchanged.

Fifth, revise failure guidance text for unresolved base refs so it explicitly recommends `covgate record-base`, while still documenting `--base <REF>` and the raw Git fallback option.

Sixth, modify agent scripts and docs to match the new workflow. Remove best-effort `origin/main` fetch attempts from `scripts/agent-env-setup.sh` and `scripts/agent-env-maintenance.sh`. Replace maintenance script behavior with a call to `covgate record-base` (guarded with clear messaging if `covgate` is unavailable), and remove xtask fallback attempts to fetch mainline refs. Update `docs/TOOLS.md`, `docs/reference/environment-execution-contexts.md`, and `README.md` to describe the recorded-base workflow and include the raw Git equivalent.

## Concrete Steps

Run all commands from repository root `/workspace/covgate` unless otherwise noted.

1. Inspect implementation entry points and tests.

    rg -n "enum|Subcommand|record|base" src/cli.rs src/main.rs src
    rg -n "origin/main|base" tests README.md docs/TOOLS.md docs/reference/environment-execution-contexts.md

    Expected result: identify exact files and functions to edit for CLI wiring, base resolution, and user messaging.

2. Implement and unit/integration test `record-base` behavior.

    cargo test record_base
    cargo test cli -- --nocapture

    Expected result: tests prove create-if-missing and idempotent-if-present behavior in real temporary Git repositories.

3. Implement automatic base preference for `refs/worktree/covgate/base` and explicit-override behavior.

    cargo test base
    cargo test cli -- --nocapture

    Expected result: tests prove `--base` still wins while auto mode prefers recorded worktree ref.

4. Update agent scripts to remove `origin/main` fetches and call `covgate record-base` in maintenance flow.

    bash -n scripts/agent-env-setup.sh scripts/agent-env-maintenance.sh

    Expected result: scripts remain syntactically valid and logs/messages describe recorded-base workflow.

5. Update docs and validate repository quality gates.

    cargo fmt --check
    cargo test
    cargo xtask validate

    Expected result: all checks pass; docs describe recommended `covgate record-base` usage plus raw Git equivalent.

## Validation and Acceptance

Acceptance is complete only when the behavior is observable end to end.

In a temp Git repository with at least one commit, `covgate record-base` must exit successfully and print `Recorded base commit <sha> at refs/worktree/covgate/base` when the ref is absent. Running it again after additional commits must exit successfully and print `Base already recorded at refs/worktree/covgate/base -> <sha>`, where `<sha>` is still the first recorded commit.

When running `covgate` without `--base`, automatic resolution must first check `refs/worktree/covgate/base`. If it exists, `covgate` uses it. If not, `covgate` continues with legacy fallback refs without regression.

When running `covgate --base <REF>`, explicit `--base` must continue to override recorded and fallback auto discovery.

When no automatic base candidate exists, user-facing error/help text must mention `covgate record-base` as a remediation alongside explicit `--base` guidance.

Script acceptance requires that `scripts/agent-env-setup.sh` and `scripts/agent-env-maintenance.sh` no longer perform `git fetch ... origin/main` bootstrap attempts. The maintenance script must invoke `covgate record-base` (or emit a clear warning if `covgate` is unavailable).

Documentation acceptance requires README coverage of:

- why agent environments may lack `origin/main`
- recommended `covgate record-base` command
- automatic use of recorded base when `--base` is omitted
- a maintenance/bootstrap snippet that uses `covgate record-base`
- raw Git equivalent:

    git rev-parse -q --verify refs/worktree/covgate/base >/dev/null || \
      git update-ref refs/worktree/covgate/base HEAD

## Idempotence and Recovery

`covgate record-base` is intentionally idempotent. Re-running the command should never move an existing `refs/worktree/covgate/base` ref. This property enables safe retries in flaky agent sessions.

Script changes should also be retry-safe. Running setup/maintenance scripts multiple times must not require mutable remote state. If `covgate` is temporarily unavailable in PATH during setup, scripts should log a warning and continue rather than corrupting repository state.

If implementation introduces regressions in legacy base fallback behavior, recovery is to keep the inserted worktree-ref candidate logic but restore original fallback candidate ordering immediately below it.

## Artifacts and Notes

Expected successful `record-base` transcript in a temp repo:

    $ covgate record-base
    Recorded base commit 0123456789abcdef0123456789abcdef01234567 at refs/worktree/covgate/base

Expected idempotent re-run transcript:

    $ covgate record-base
    Base already recorded at refs/worktree/covgate/base -> 0123456789abcdef0123456789abcdef01234567

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
- Main dispatch branch in `src/main.rs` that invokes a new function such as `record_base()`.
- Git helper functions capable of:
  - resolving `HEAD` commit SHA
  - resolving arbitrary ref SHA as optional result
  - creating a ref pointing at a target object
- Base resolver candidate list that includes `refs/worktree/covgate/base` before legacy fallback refs when explicit `--base` is not provided.

No new external dependency should be required for this feature; continue using existing subprocess/process helpers for Git interactions.

Change note (2026-03-15): Adapted a generic feature brief into a repository-specific ExecPlan and explicitly added script migration scope: remove non-functional `origin/main` fetch attempts from agent setup/maintenance, replace maintenance bootstrap behavior with `covgate record-base`, and remove xtask's best-effort mainline fetch fallback.
