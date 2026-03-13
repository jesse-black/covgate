# Covgate fail-uncovered gates

This ExecPlan is complete and archived in `docs/exec-plans/completed/covgate-fail-uncovered.md` for historical traceability.

This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds.

Maintain this document in accordance with `docs/PLANS.md`. Re-read that file before revising this plan and keep this plan aligned with its rules.

This plan builds on the current `covgate` implementation in `docs/exec-plans/active/covgate.md`, but it repeats the needed context here so a reader can execute this plan without opening any other file.

## Purpose / Big Picture

After this change, a repository will be able to ask a second kind of diff-coverage question in `covgate`: not only “is changed coverage at least N percent,” but also “are uncovered changed opportunities at most N?” The new user-visible behavior is a family of metric-specific maximum-count gates such as `--fail-uncovered-regions 3`, with matching repository-local TOML defaults. A user should be able to run one `covgate` invocation and see whether the changed diff both clears percentage thresholds and stays below an uncovered-count budget.

This matters because percent thresholds alone can be awkward on small diffs. A one-line uncovered change can fail a strict percent rule even when the team only wants to allow, for example, up to one uncovered changed region. The observable outcome is a CLI run that prints the diff coverage summary, includes the configured uncovered-count rule in the result, and exits nonzero when the number of uncovered changed opportunities exceeds the configured maximum.

You will know this is working when a contributor can run a command like the example below from a repository root and see `covgate` fail only because the diff has too many uncovered changed regions:

    cargo run -- --coverage-json coverage.json --base origin/main --fail-uncovered-regions 1

You will also know this is working when a repository can keep defaults in `covgate.toml` such as:

    base = "origin/main"

    [gates]
    fail_under_regions = 90
    fail_uncovered_regions = 1

and `covgate` applies both rules in the same run with explicit CLI flags still taking precedence.

## Progress

- [x] (2026-03-11 20:20Z) Create a separate ExecPlan for `--fail-uncovered-*` gates instead of folding that work into the current fail-under implementation.
- [x] (2026-03-13 13:24Z) Generalized the threshold model so one run evaluates multiple gate rules, including minimum-percent and maximum-uncovered-count rules.
- [x] (2026-03-13 13:24Z) Extended CLI parsing and repository-local TOML loading to accept pluralized `--fail-uncovered-*` flags and matching config keys with per-rule precedence.
- [x] (2026-03-13 13:24Z) Updated gate evaluation, console rendering, Markdown rendering, and integration tests so uncovered-count gates are visible, enforced, and covered in end-to-end scenarios.
- [x] (2026-03-13 13:24Z) Recorded completion evidence and synchronized living sections with the implemented feature set.

## Surprises & Discoveries

- Observation: The merged implementation now evaluates both percentage and uncovered-count gates in one run.
  Evidence: `src/model.rs` defines `GateRule` plus `RuleOutcome`, and `src/gate.rs` evaluates every configured rule into a unified `GateResult`.

- Observation: CLI/TOML precedence is now independent per rule family.
  Evidence: `src/config.rs` resolves each fail-under and fail-uncovered field separately, and `tests/cli.rs` verifies a CLI override for one rule keeps the other rule from TOML active.

## Decision Log

- Decision: `--fail-uncovered-*` must be implemented as a separate threshold family rather than a special case inside percentage thresholds.
  Rationale: “At least N percent covered” and “at most N uncovered opportunities” are different gate shapes. Treating them as the same thing would make configuration, help text, rendering, and later multi-metric support harder to reason about.
  Date/Author: 2026-03-11 / Codex

- Decision: The uncovered-count CLI should use pluralized metric-specific flags such as `--fail-uncovered-regions`, `--fail-uncovered-lines`, and `--fail-uncovered-branches`.
  Rationale: The existing fail-under CLI already uses pluralized metric-specific flags, and the uncovered-count family should follow the same naming scheme for predictability in help text, CI configuration, and TOML keys.
  Date/Author: 2026-03-11 / Codex

- Decision: One `covgate` run should be able to enforce both percent and uncovered-count gates at the same time.
  Rationale: The motivating use case is not “pick one style of threshold.” Teams often want both a floor and a budget, for example “changed region coverage at least 90% and no more than one uncovered changed region.” Requiring separate runs would duplicate work and fragment reporting.
  Date/Author: 2026-03-11 / Codex

- Decision: Explicit CLI values must override repository-local TOML defaults independently for each configured rule.
  Rationale: Users should be able to keep a checked-in default budget while tightening or loosening one rule in a CI job or local run without redefining every other threshold in the config file.
  Date/Author: 2026-03-11 / Codex

## Outcomes & Retrospective

The `--fail-uncovered-*` feature is now fully implemented and exercised across model, configuration, gate evaluation, renderers, and CLI integration tests. `covgate` can evaluate percent and uncovered-count gates in one run, render each rule outcome explicitly, and merge CLI values with `covgate.toml` defaults independently per rule family.

The remaining risk profile is operational rather than design-oriented: future rule families should continue to route through the normalized `GateRule` and `RuleOutcome` model so renderer output remains consistent as support expands to additional metric types.

## Context and Orientation

`covgate` is a Rust CLI in this repository. The current code paths relevant to this feature are:

- `src/cli.rs`, which defines the public command-line interface with `clap`
- `src/config.rs`, which merges CLI values with repository-local defaults from `./covgate.toml`
- `src/model.rs`, which defines `MetricKind`, `Threshold`, `ComputedMetric`, and `GateResult`
- `src/metrics.rs`, which computes changed coverage totals and collects uncovered changed opportunities
- `src/gate.rs`, which currently evaluates one threshold
- `src/render/console.rs` and `src/render/markdown.rs`, which print the gate result
- `tests/cli.rs`, which contains copied-fixture CLI integration tests

In this plan, an “uncovered-count gate” means a rule that fails when the number of uncovered changed opportunities for a metric is greater than a configured maximum. For the currently implemented LLVM path, the relevant opportunity kind is a coverage region, so `--fail-uncovered-regions 1` means “fail if there are more than one uncovered changed region in the diff.” A “gate rule” means one independently configured condition, such as a minimum percentage or a maximum uncovered count. An “effective rule set” means the full set of rules after merging CLI overrides with TOML defaults.

The current implementation already computes the raw data needed for uncovered-count gates. `src/metrics.rs` returns the total number of covered changed opportunities, the total number of changed opportunities, and a vector of uncovered changed opportunities. What is missing is a model that can hold more than one rule at a time, gate evaluation that can check different rule types, and renderers that can explain which rule or rules failed.

The current CLI and TOML surfaces are also intentionally narrow. `src/cli.rs` exposes `--fail-under-regions`, `--fail-under-lines`, and `--fail-under-branches`. `src/config.rs` resolves those into exactly one `Threshold`, and the TOML loader treats `[gates]` as a place where exactly one threshold may be set in v1. This plan must relax that one-threshold assumption without breaking the existing behavior for users who only set a fail-under threshold.

## Plan of Work

Start in `src/model.rs` by replacing the current single-threshold representation with a small gate-rule model that can represent both kinds of checks cleanly. Keep `MetricKind`, but introduce a dedicated rule type that distinguishes between a minimum-percent rule and a maximum-uncovered rule. The exact names are up to the implementation, but the model must make it obvious whether a rule compares a percentage or a count. Add a result shape that can report every evaluated rule, whether each one passed, and which ones caused the overall failure. Do not hide count gates behind overloaded percentage fields.

Once the model can represent multiple rules, update `src/config.rs` so it resolves an ordered collection of effective rules instead of one `Threshold`. Preserve the current repository-local config discovery from `./covgate.toml`. Extend the CLI layer in `src/cli.rs` with pluralized flags `--fail-uncovered-regions`, `--fail-uncovered-lines`, and `--fail-uncovered-branches`. Extend the TOML format with matching keys inside `[gates]`, such as `fail_uncovered_regions = 1`. Keep the existing fail-under names and allow both families to appear together. CLI precedence must stay per field: if TOML sets both a percent rule and an uncovered-count rule, and the CLI overrides only one of them, the unoverridden rule should still come from TOML.

After configuration resolution, update `src/gate.rs` to evaluate every effective rule against the computed metric. The overall run should pass only if every configured rule passes. A percent rule uses the existing percentage value. An uncovered-count rule compares the length of `uncovered_changed_opportunities` against the configured maximum. Make the evaluation order deterministic and preserve enough detail for renderers to report individual pass/fail status. If a rule refers to a metric that the loaded report does not support, return a clear configuration error instead of silently ignoring it.

Then update `src/render/console.rs` and `src/render/markdown.rs` so they show both rule families clearly. The console output should keep the current diff summary but add a short rules section or totals block that lists each configured rule and whether it passed. The Markdown output should do the same in a small table near the top of the diff summary. The output must make it obvious whether a failure came from percent coverage, uncovered-count budget, or both.

Finally, extend `tests/cli.rs` and any focused unit tests in `src/config.rs` and `src/gate.rs` to cover four concrete situations: a run that passes only because the uncovered-count budget is generous enough, a run that fails only the uncovered-count budget, a run that fails both the percent rule and the uncovered-count rule, and a run where CLI values override one TOML rule while the other rule still comes from the config file. Reuse the copied-fixture strategy that the repository already uses so the tests stay realistic.

## Concrete Steps

Run the following commands from the `covgate` repository root.

1. Generalize the model and configuration resolution.

    Working directory: the `covgate` repository root

    Edit `src/model.rs` to add a first-class gate-rule representation for minimum-percent and maximum-uncovered-count rules. Edit `src/config.rs` to resolve a collection of effective rules from CLI flags plus `covgate.toml` defaults. Edit `src/cli.rs` to add `--fail-uncovered-regions`, `--fail-uncovered-lines`, and `--fail-uncovered-branches`.

    Example commands:

        cargo test config
        cargo test gate

    Expected outcome: unit tests prove that the program can resolve more than one effective rule at a time, that CLI values override TOML defaults per field, and that both rule families can coexist in one run.

2. Implement multi-rule gate evaluation.

    Working directory: the `covgate` repository root

    Edit `src/gate.rs` so it evaluates every effective rule and returns one aggregate result containing both the overall pass/fail status and per-rule outcomes.

    Example commands:

        cargo test gate
        cargo test metrics

    Expected outcome: gate tests prove that uncovered-count rules compare against the number of uncovered changed opportunities, that the overall run fails when any configured rule fails, and that unsupported metric rules still produce actionable errors.

3. Update console and Markdown rendering.

    Working directory: the `covgate` repository root

    Edit `src/render/console.rs` and `src/render/markdown.rs` so they surface every configured rule and its outcome without hiding the existing diff summary.

    Example commands:

        cargo test render

    Expected outcome: renderer tests prove that percent gates and uncovered-count gates are both visible in the output and that a reader can tell which rule caused a failure.

4. Add end-to-end copied-fixture CLI coverage.

    Working directory: the `covgate` repository root

    Extend `tests/cli.rs` with copied-fixture scenarios that exercise CLI-only uncovered-count gates, TOML-only uncovered-count defaults, and mixed CLI-over-TOML precedence across both rule families.

    Example commands:

        cargo test cli
        cargo test

    Expected outcome: integration tests prove that one invocation can enforce both rule families, that config defaults work, and that CLI overrides are applied per field rather than all-or-nothing.

5. Run the repository validation workflow.

    Working directory: the `covgate` repository root

    Example commands:

        cargo fmt --check
        cargo check
        cargo clippy --all-targets --all-features -- -D warnings
        cargo test
        cargo llvm-cov --summary-only

    Expected outcome: the full repository validation stack passes, and total coverage remains at or above the repository target after the new rule family and tests are added.

## Validation and Acceptance

Acceptance is complete only when all of the following are true.

Running `covgate` with `--fail-uncovered-regions <MAX>` evaluates the number of uncovered changed regions in the diff and exits with status 1 only when that number is greater than `MAX`. At minimum, tests must prove that `MAX = 1` passes when exactly one changed region is uncovered and fails when two changed regions are uncovered.

Running `covgate` with both `--fail-under-regions <MIN>` and `--fail-uncovered-regions <MAX>` in the same invocation evaluates both rules and fails if either one fails. The console output and Markdown output must both show the configured percent rule and the configured uncovered-count rule distinctly enough that a novice can tell which rule failed.

Running `covgate` in a repository that contains `covgate.toml` with both `fail_under_regions` and `fail_uncovered_regions` defaults must apply both defaults when the CLI omits them. Tests must also prove that a CLI override for one rule family does not erase the other rule family’s TOML default.

If the user configures an uncovered-count rule for a metric that the loaded report cannot provide, `covgate` must fail clearly with a configuration or unsupported-metric error. It must not silently skip that rule.

CLI integration tests must cover at least:

- a passing run with only `--fail-uncovered-regions`
- a failing run with only `--fail-uncovered-regions`
- a run where percent coverage passes but the uncovered-count budget fails
- a run where the uncovered-count budget passes but the percent rule fails
- a run where both rules are supplied via TOML defaults
- a run where one rule comes from TOML and the other is overridden by the CLI

Acceptance is not complete if the output only reports the existing percent threshold and leaves the uncovered-count rule implicit. A user must be able to see the uncovered-count rule and the measured uncovered count directly in the output.

Acceptance is not complete if the implementation keeps the current one-threshold-per-run assumption. This feature exists to support gating multiple rules at once, so a design that forces separate invocations for percent and uncovered-count rules does not satisfy the plan.

## Idempotence and Recovery

This feature should be implemented additively. Re-running the same `covgate` command against the same coverage JSON and the same diff should produce the same pass/fail result and the same rendered output. The new CLI flags and TOML keys should only affect how gate rules are configured; they must not mutate repository files under test.

If configuration resolution fails midway through implementation, keep the old one-threshold behavior behind tests until the new multi-rule model is complete. Do not partially wire `--fail-uncovered-*` flags into the public CLI while the renderer and gate model still assume only one rule, because that would produce misleading output.

If a copied-fixture integration test needs new fixture content, copy it into a temporary worktree and keep the checked-in fixture baseline immutable. Follow the existing repository pattern rather than editing fixture repositories in place during a test run.

## Artifacts and Notes

Expected console output excerpt for a run that fails only the uncovered-count rule:

    -------------
    Diff Coverage: FAIL
    Diff: origin/main...HEAD
    Metric: region
    -------------
    src/lib.rs (50.00%): uncovered changed spans 3-3, 5-5
    -------------
    Changed regions: 4
    Covered regions: 2
    Coverage: 50.00%
    Rule fail-under-regions: PASS (50.00% >= 40.00%)
    Rule fail-uncovered-regions: FAIL (2 > 1)
    -------------

Expected TOML excerpt:

    base = "origin/main"

    [gates]
    fail_under_regions = 40
    fail_uncovered_regions = 1

These snippets are examples of user-visible behavior, not required final formatting. The exact wording may change, but the output must still surface both rule families explicitly.

## Interfaces and Dependencies

Stay on Rust stable. Reuse the existing repository dependencies unless implementation evidence proves a new crate is necessary.

In `src/model.rs`, replace the single-threshold assumption with stable, explicit concepts. At the end of this work, the code should have a type that distinguishes rule families and a type that records per-rule evaluation results. One acceptable direction is a `GateRule` enum with variants for minimum percent and maximum uncovered count, plus a `RuleOutcome` struct that captures the configured value, the observed value, and whether the rule passed. The exact names may differ, but the responsibilities may not.

In `src/config.rs`, configuration loading should continue to discover `./covgate.toml`. The effective config type should expose a collection of gate rules rather than one threshold. CLI flags should remain metric-specific and pluralized. TOML keys for uncovered-count gates should follow the same pattern with underscores, for example `fail_uncovered_regions`.

In `src/gate.rs`, define one entrypoint that accepts a computed metric plus the effective rule set and returns an aggregate result that renderers can consume without needing to re-derive rule failures themselves.

In `src/render/console.rs` and `src/render/markdown.rs`, renderers must consume normalized rule outcomes from the gate layer rather than reimplementing gate logic. This keeps later additions, such as line- and branch-based uncovered-count gates, additive.

At the bottom of this plan, append a revision note every time the plan changes materially, describing what changed and why.

Revision note: Initial standalone plan created for the `--fail-uncovered-*` feature so uncovered-count gates can be designed and implemented as a separate threshold family without overloading the current fail-under work.

Revision note: Updated the intended TOML section name from `[thresholds]` to `[gates]` so the follow-up plan matches the current repository configuration vocabulary.

Revision note: Marked this ExecPlan complete, updated progress and retrospective sections to reflect shipped behavior, and prepared the document for archive under `docs/exec-plans/completed/`.
