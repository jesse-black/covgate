# Clarify when `covgate record-base` is required and make the normal checkout workflow the default guidance

Save this in-progress ExecPlan in `docs/exec-plans/active/covgate-clarify-record-base-guidance.md`. Move it to `docs/exec-plans/completed/covgate-clarify-record-base-guidance.md` only after the README, CLI help text, the defensive `record-base` behavior, and any supporting docs all describe the same environment split and the validation steps below pass.

This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds.

Maintain this document in accordance with `docs/PLANS.md`. Re-read that file before revising this plan and keep this plan aligned with its rules.

## Purpose / Big Picture

`covgate` already supports two different base-resolution paths, but the current docs and `covgate --help` do not explain clearly when each path should be used. In an ordinary local checkout or devcontainer, the normal workflow should usually be `covgate check <coverage-report>` and automatic base discovery should handle `origin/main`, `origin/HEAD`, or `main` without any extra setup. In a constrained cloud agent sandbox, those refs may be missing or unstable, and `covgate record-base` is the right escape hatch.

This plan makes that split obvious and adds one small guardrail in the product itself. After this work, a novice should be able to read the README or CLI help text and immediately understand three things: `record-base` is conditional rather than universal, the common local workflow does not need it, and if they do need it they must run it at task start before the agent makes Git changes. The command should also be more defensive in standard checkouts by detecting when a normal branch ref already resolves and telling the user that `record-base` is unnecessary instead of silently reinforcing overuse. The work is complete when the docs and help text present the same recommendation, include a side-by-side local-versus-cloud example, avoid calling normal automatic base discovery a mere “fallback,” and the `record-base` command emits a clear “unnecessary here” message when standard base refs are already available.

## Progress

- [x] (2026-03-23 16:05Z) Created this repository-specific active ExecPlan with concrete file targets, validation commands, and a tightly scoped goal: clarify when `record-base` is needed and add one small defensive behavior.
- [x] (2026-03-23 16:10Z) Reviewed the current wording in `README.md` and `src/cli.rs` to identify where `record-base` is presented too broadly.
- [x] (2026-03-23 16:18Z) Expanded the plan scope to include one small behavior change: make `record-base` detect when a standard base ref is already available and explain that recording is unnecessary in that environment.
- [x] (2026-03-23 16:40Z) Refined the CLI-help direction: remove config guidance from top-level `--help`, avoid examples blocks at the root command, move the cloud-agent workflow guidance into `record-base --help`, and ensure `check --help` includes descriptive argument and option text.
- [ ] Rewrite `README.md` so the standard local/devcontainer workflow appears before the cloud-agent workflow and explicitly states that `record-base` is not the default in ordinary clones.
- [x] (2026-03-23 16:45Z) Updated `src/cli.rs` so top-level `--help` no longer includes config prose or roadmap wording, `record-base --help` uses user-focused cloud-agent guidance, and `check --help` now describes its argument and options.
- [x] (2026-03-23 17:35Z) Implemented a defensive `record-base` preflight in `src/git.rs` so standard local checkouts now emit an explanatory no-op when a normal base ref already resolves.
- [x] (2026-03-23 17:35Z) Updated Git, CLI, and config tests to cover both sides of the contract: ordinary local checkouts no-op, while constrained task-branch repos still create and refresh `refs/worktree/covgate/base`.
- [x] (2026-03-23 18:05Z) Narrowed the remaining documentation scope to `README.md` only; no update to `docs/reference/environment-execution-contexts.md` is needed for this plan.
- [x] (2026-03-23 17:40Z) Revalidated the implementation with focused `record-base` test slices plus `cargo xtask quick`; remaining work is documentation alignment, not code behavior.

## Surprises & Discoveries

- Observation: The current CLI help text still labels `record-base` as the generic “Agent workflow” even though the implementation already supports normal automatic base discovery.
  Evidence: `src/cli.rs` `after_help` currently ends with `Agent workflow:` followed by `covgate record-base` and `covgate check <coverage-report>`.

- Observation: The README already explains the automatic base-resolution order, but it still gives the cloud-agent path more emphasis than the ordinary local path.
  Evidence: the `README.md` usage section has a dedicated “Autonomous Agent Workflows” heading, while the normal no-`record-base` case appears only indirectly through `--base` examples and config examples.

- Observation: The existing product behavior already matches the intended narrower recommendation.
  Evidence: `src/config.rs` error text and the completed plan in `docs/exec-plans/completed/covgate-record-base-agent-workflows.md` both describe `record-base` as a remediation when automatic base selection cannot succeed, not as a universal prerequisite.

- Observation: A minimal defensive no-op fits this plan without reopening the broader command design.
  Evidence: the desired behavior is narrow and user-facing: when a normal base ref already resolves, `record-base` should report that it is unnecessary in this environment instead of recording an extra worktree-local base ref.

- Observation: The current `after_help` text has drifted from current behavior and mixes evergreen help with implementation-history wording.
  Evidence: `src/cli.rs` says `Repository-local defaults may be read from ./covgate.toml.` even though config discovery now traverses ancestor directories, and it also says `Supported defaults in v1:` even though `--help` should describe current behavior rather than roadmap/version framing.

- Observation: Once `record-base` becomes defensive in ordinary checkouts, many existing tests must move onto nonstandard task-branch names to keep exercising the constrained-environment path.
  Evidence: tests that stayed on `main` or `master` began no-oping before they could create branch markers or recorded refs; renaming those repos to task-style branches restored the intended cloud-agent setup for the test fixtures.

## Decision Log

- Decision: Treat this as a guidance plan plus one deliberately small behavior change in `record-base`.
  Rationale: The main issue is still user guidance, but the proposed defensive no-op directly reinforces the intended model without turning this into a broader command redesign.
  Date/Author: 2026-03-23 / Codex

- Decision: Keep the plan tightly scoped to the highest-signal user surfaces: `README.md`, `src/cli.rs`, and the small `src/git.rs` behavior change.
  Rationale: The confusion is happening at first-contact surfaces. Narrowing the scope keeps this follow-up concise and focused instead of reopening the broader completed `record-base` implementation plan.
  Date/Author: 2026-03-23 / Codex

- Decision: Standard local/devcontainer guidance must be presented before cloud-agent guidance.
  Rationale: That ordering matches the common case and makes `record-base` read as an exception path instead of the default path.
  Date/Author: 2026-03-23 / Codex

- Decision: Keep `--help` focused on current, immediately actionable behavior and remove roadmap-style or overly detailed config prose unless it clearly helps first-time invocation.
  Rationale: Command help is most useful when it is accurate, short, and about the current command. Path assumptions like `./covgate.toml` and phrases like `v1` add noise and can be wrong.
  Date/Author: 2026-03-23 / Codex

- Decision: Do not add a top-level “Examples” section to `covgate --help`; put the cloud-agent workflow guidance on the `record-base` subcommand instead.
  Rationale: Comparable Rust CLIs keep root help compact and attach concrete examples to the relevant subcommand or flag. That keeps the root help skimmable while still giving `record-base` enough context to explain when to use it.
  Date/Author: 2026-03-23 / Codex

- Decision: Reuse the same standard branch candidates for the defensive `record-base` preflight that automatic discovery already treats as ordinary checkout bases, excluding only the recorded worktree ref itself.
  Rationale: This keeps the guidance and product behavior aligned: if `covgate check` would ordinarily succeed from a normal branch ref, `record-base` should explain that it is unnecessary instead of creating a redundant worktree-local marker.
  Date/Author: 2026-03-23 / Codex

## Outcomes & Retrospective

Implementation is partially complete. The CLI help work and the defensive `record-base` preflight are now in place, along with regression coverage that distinguishes ordinary local checkouts from constrained task-branch repos. The remaining work is to align the README; no extra environment-reference-doc update is needed for this plan.

The main lesson so far is that environment-specific guidance needs both wording and behavioral reinforcement. Once the command itself says “this is unnecessary here” in ordinary checkouts, the intended workflow becomes much harder to misread.

## Context and Orientation

`covgate` is a Rust CLI in `src/` that computes diff coverage from a coverage report and a Git diff base. The main user-facing docs live in `README.md`. The top-level CLI help text is defined in `src/cli.rs` through Clap metadata for the root command and subcommands. Git-base recording and base-ref discovery logic live in the Rust Git helpers under `src/git.rs` and config resolution under `src/config.rs`.

In this repository, “automatic base discovery” means the path `covgate` uses when the user does not pass `--base`. It looks for an already recorded worktree-local ref first and then checks standard branch refs such as `origin/HEAD`, `origin/main`, and `main`. In a normal local clone or devcontainer, one of those standard refs is often available, so `covgate check <coverage-report>` should work without `record-base`.

In this plan, a “cloud-agent environment” means a constrained sandbox or cached worktree where normal branch refs may be unavailable or unreliable. `covgate record-base` exists for that case. It records the task’s starting commit in `refs/worktree/covgate/base`, which is a Git ref stored per worktree rather than as a shared branch.

This plan carries five concrete goals. First, narrow the recommendation for `record-base` so it is clearly conditional rather than universal. Second, make the normal local or devcontainer workflow more prominent than the cloud-agent escape hatch. Third, stop describing automatic branch discovery as a “fallback” when it is the normal path in ordinary clones. Fourth, explain that `record-base` belongs at task start before the agent mutates Git state. Fifth, add one small defensive behavior change: when a standard base ref already resolves, `record-base` should no-op with an explanatory message instead of recording an unnecessary worktree base ref.

## Plan of Work

Start in `README.md`. Rewrite the usage guidance so a novice sees the standard local/devcontainer workflow first. The text should plainly say that in a standard checkout the normal command is `covgate check <coverage-report>`, that no `record-base` step is needed when `origin/main`, `origin/HEAD`, or `main` is available, and that `--base` remains available for teams that want a non-default base. Keep the cloud-agent section, but rewrite it as a conditional workflow for environments where those refs are unavailable. Add one short side-by-side example that contrasts the two environments.

Next, update `src/cli.rs` so `covgate --help` stays compact and current. Remove the old root-level `after_help` prose instead of replacing it with another top-level examples block. Put the cloud-agent usage guidance on the `record-base` subcommand help instead, and make that wording explicitly say it applies when normal base branches such as `main` or `origin/main` are unavailable. The `record-base` help text should also make timing obvious by saying to run it once at task start before making Git changes and then use `covgate check <coverage-report>` without `--base`.

While editing `src/cli.rs`, remove config guidance from `--help` entirely. Do not keep text that assumes the config file lives specifically at `./covgate.toml`, because the product now walks ancestor directories. Do not keep versioned phrasing such as `Supported defaults in v1:` because `--help` should describe current behavior, not roadmap or release-era context. Instead, make the subcommand help itself more useful by ensuring `check --help` includes descriptive text for every argument and option, and `record-base --help` explains the user problem it solves rather than Git internals.

Then add the small defensive behavior in the Git path. Update `src/git.rs` so `covgate record-base` first checks the same standard branch refs that a normal local checkout would rely on. If one resolves cleanly, return without writing `refs/worktree/covgate/base` and print an explanatory message that `record-base` is unnecessary in this environment because a normal base ref is already available. Keep the command’s existing behavior for constrained environments where those standard refs do not resolve. This change must be intentionally narrow: it should reinforce the guidance, not redesign the command.

Finally, validate both behavior and wording as rendered output rather than only as source edits. Run `cargo run -- --help` to inspect the real help output, use focused tests to confirm the defensive no-op in normal local clones and the original recording behavior in constrained repos, and re-read the updated README sections to confirm that a novice would encounter the common local workflow before the exception path. If the wording still makes “agent” sound equivalent to “always record-base,” or if the command still records a worktree base in an ordinary clone where a standard base ref is available, keep revising until that ambiguity is gone.

## Concrete Steps

Run all commands from the repository root `/home/jesse/git/covgate`.

1. Inspect the current wording before editing.

    sed -n '70,150p' README.md
    sed -n '1,120p' src/cli.rs
    rg -n "record-base|origin/main|Standard workflow|Agent workflow|cloud agent" README.md docs src

    Expected result: the exact places that overemphasize `record-base` are visible before changes begin.

2. Update the docs and help text to match the intended environment split.

    After editing, the README should show a normal local/devcontainer example before the cloud-agent example. The CLI help text should remove the old root footer, keep top-level help compact, place cloud-agent workflow guidance on `record-base --help`, and avoid `./covgate.toml` path claims and roadmap wording like `v1`.

3. Add or update focused tests for the new defensive `record-base` behavior.

    cargo test record_base -- --nocapture
    cargo test git_module -- --nocapture
    cargo test cli_interface -- --nocapture

    Expected result: a normal local-style repo with `main`, `origin/main`, or `origin/HEAD` available now receives an explanatory no-op, while a constrained repo that lacks those refs still records `refs/worktree/covgate/base` successfully.

4. Inspect the rendered help output.

    cargo run -- --help

    Expected result: the help output shows the standard `covgate check <coverage-report>` path first and describes `record-base` as conditional on unavailable branch refs.

5. Re-read the changed prose and rerun repository validation.

    cargo fmt --check
    cargo xtask quick

    Expected result: formatting checks stay green, the development validation loop still passes, the wording across README/help/docs is consistent about when to use `record-base`, and the new defensive behavior remains covered by tests.

## Validation and Acceptance

This work is accepted only when a novice can verify the guidance directly from the repository’s public surfaces.

Reading `README.md` must make it obvious that the standard local or devcontainer workflow is `covgate check <coverage-report>` without a prior `record-base` step when normal base refs exist. The README must also state clearly that `record-base` is for constrained cloud-agent environments where `main`, `origin/main`, or similar refs are unavailable or unreliable.

Running `cargo run -- --help` must keep the top-level help concise. It should list the commands and omit the stale config footer entirely. `record-base --help` must carry the cloud-agent guidance, and `check --help` must include descriptive argument and option text.

In a repository where a standard branch ref already resolves, running `covgate record-base` must exit successfully without recording a new worktree base and must print a clear explanation that the command is unnecessary in this environment because a normal base ref is already available.

In a constrained repository that lacks `origin/HEAD`, `origin/main`, and `main`, running `covgate record-base` must continue to record `refs/worktree/covgate/base` successfully so the intended cloud-agent escape hatch still works.

The final wording must also explain timing. A reader should be able to infer that `record-base` belongs at the beginning of a task before Git changes are made, not immediately before `covgate check`.

## Idempotence and Recovery

Most of this plan changes prose, and the one product change is intentionally narrow. If an edit makes the guidance more confusing, recover by restoring the simpler version and checking it against the plan’s core goals: narrow the scope, prioritize the normal workflow, describe `record-base` as an escape hatch, make task-start timing explicit, and keep the defensive behavior limited to an explanatory no-op when normal base refs already work.

If the defensive behavior change causes regressions in constrained repos, recover by reverting only that narrow preflight logic while keeping the improved docs and tests that expose the intended environment split. If a broader redesign becomes necessary, stop and open a new follow-up plan rather than expanding this focused plan midstream.

## Artifacts and Notes

Representative rendered help text after this plan should look roughly like this:

    $ covgate --help
    Diff-focused coverage gate

    Usage: covgate <COMMAND>

    Commands:
      check
      record-base

Representative `record-base` help text should carry the workflow guidance:

    $ covgate record-base --help
    Use this when a cloud agent or sandboxed worktree cannot rely on normal base branches such as main or origin/main.
    Run it once at the start of a task before making Git changes, then run `covgate check <coverage-report>` without `--base`.

Representative README wording should make the same contrast in prose:

    In a standard local checkout or devcontainer, run `covgate check <coverage-report>`.
    Use `covgate record-base` only in constrained cloud-agent environments where normal base refs are unavailable.

Representative defensive command behavior in a normal checkout should look roughly like this:

    $ covgate record-base
    Base ref `origin/main` is available; `record-base` is unnecessary in this environment.

## Interfaces and Dependencies

This plan should not add any dependency. It changes one narrow Rust behavior and the related user-facing text surfaces:

- `README.md` for first-contact product guidance.
- `src/cli.rs` for the top-level `covgate --help` text.
- `src/git.rs` for the defensive `record-base` preflight and explanatory no-op.
- related tests in `tests/git_module.rs` and `tests/cli_interface.rs` to preserve both the normal-checkout and constrained-environment behaviors.

The implementation must preserve the existing command surface: `covgate check <coverage-report>` for gating and `covgate record-base` for recording a worktree-local base. The intended change is explanatory: make it clear which command sequence belongs to which environment.

Plan revision note: created on 2026-03-23 as a focused active ExecPlan with explicit file targets, validation commands, and an intentionally narrow scope: clarify when `record-base` is needed, improve CLI help and docs, and add one defensive no-op for standard checkouts.
