# Covgate function and method thresholds

Save this in-progress ExecPlan at `docs/exec-plans/active/covgate-function-thresholds.md` while the work is being designed or implemented in this repository.

This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds.

Maintain this document in accordance with `docs/PLANS.md`. Re-read that file before revising this plan and keep this plan aligned with its rules.

This plan is intentionally separate from `docs/exec-plans/active/covgate.md` and `docs/exec-plans/active/covgate-fail-uncovered.md`. The current `covgate` plan is about the base diff gate and the uncovered-count follow-up. This plan is a draft for adding function or method-based thresholds as a first-class metric family once Istanbul and Coverlet support lands.

## Purpose / Big Picture

After this change, `covgate` will be able to gate changed code using function-like opportunities in addition to lines, branches, and regions. The public behavior should let a repository say “all changed functions or methods must be exercised” with a rule such as `--fail-under-functions 100` or `--fail-uncovered-functions 0`, and see that rule enforced in the same diff-focused workflow that `covgate` already uses for other metrics.

This matters because some teams care less about the exact line or branch percentage and more about whether every changed callable unit was exercised at least once. In plain language, a “function” here means a named callable unit as reported by the underlying coverage tool. In Istanbul that is a function record. In Coverlet that is typically a method-level record. The user-visible outcome is a `covgate` run that can fail specifically because a changed function or method was not exercised, even when line or branch thresholds still pass.

You will know this is working when a contributor can run a command like the example below from a repository root and see `covgate` fail because one changed function remains uncovered:

    covgate --coverage-json coverage.json --base origin/main --fail-under-functions 100

You will also know this is working when a repository can keep defaults in `covgate.toml` such as:

    [gates]
    fail_under_functions = 100
    fail_uncovered_functions = 0

and `covgate` explains those function-oriented rules directly in console output and Markdown output.

## Progress

- [x] (2026-03-11 20:35Z) Create a separate draft ExecPlan for function or method thresholds so the idea is captured without prematurely expanding the current implementation milestone.
- [x] (2026-03-16 01:05Z) Define and implement shared internal callable-unit normalization for currently supported parser paths (LLVM functions and Coverlet methods) into `MetricKind::Function` / `OpportunityKind::Function`.
- [x] (2026-03-16 00:15Z) Implement shared internal callable-unit normalization for currently supported parsers (LLVM functions and Coverlet methods) into `MetricKind::Function` / `OpportunityKind::Function`, including parser edge-case deduplication and covered-state fixes discovered during dogfooding.
- [x] (2026-03-16 00:15Z) Define the public CLI and TOML surface for function-based fail-under and fail-uncovered gates (`--fail-under-functions`, `--fail-uncovered-functions`, `fail_under_functions`, `fail_uncovered_functions`).
- [x] (2026-03-16 00:15Z) Specify and implement diff intersection rules for changed functions: callable spans are normalized to source line ranges and selected with the same line-overlap logic used by other metrics in `compute_changed_metric`.
- [x] (2026-03-16 01:05Z) Move Istanbul-specific validation expectations to the active Istanbul ExecPlan (`docs/exec-plans/active/covgate-vitest-istanbul-native-json-fixture.md`) and keep this plan scoped to shipped function-gating behavior in currently supported formats.
- [x] (2026-03-15 16:05Z) Align function-threshold integration-test expectations with `docs/TESTING.md`: metric semantics belong in `tests/cli_metrics.rs`, should execute across a compatible fixture list, and should stay separate from CLI interface-only coverage in `tests/cli_interface.rs`.

## Current evaluation after .NET Coverlet landing

- Status: .NET coverage support is now present, but only for line and branch metrics.
  Evidence: `src/coverage/coverlet_json.rs` deserializes method-shaped Coverlet entries but emits only `OpportunityKind::Line` and `OpportunityKind::BranchOutcome` opportunities and only `MetricKind::Line`/`MetricKind::Branch` totals.

- Status: Coverlet output does include method-level records that are viable inputs for callable-unit gating.
  Evidence: fixture artifacts under `tests/fixtures/dotnet/**/coverage.json` use method-signature keys (for example `System.Int32 CovgateDemo.MathOps::Add(System.Int32,System.Int32)`) beneath class nodes.

- Product implication: public naming should remain `functions` as the canonical gate vocabulary, while documentation explicitly states that Coverlet methods are normalized into that shared function metric when function-threshold support is implemented.
  Rationale: one stable cross-format term in CLI/config keeps UX predictable; adding duplicate user-facing gate names (`functions` and `methods`) would increase config/API surface without adding distinct behavior.

## Surprises & Discoveries

- Observation: “Function coverage” is not one universal raw concept across the planned formats, but both near-term target ecosystems do expose a callable-unit signal that users can reason about.
  Evidence: Istanbul reports function coverage directly, while Coverlet exposes method-level coverage data that can serve the same gating purpose for this tool.

- Observation: The current `covgate` model is already close to supporting this feature because it distinguishes metric families from raw parser details.
  Evidence: `src/model.rs` already separates `MetricKind` from parser-specific structures, and the active plans already reserve room for additional opportunity kinds beyond regions.

## Decision Log

- Decision: Function or method thresholds should be treated as a real future metric family, not only as a parser-specific renderer detail.
  Rationale: The user need here is a gate decision, not merely extra report context. If the tool is going to support “all changed functions must be exercised,” that must live in the same gate model as other thresholds so it can participate in pass or fail behavior.
  Date/Author: 2026-03-11 / Codex

- Decision: The public CLI should use the name `functions` even when the underlying parser source is a Coverlet method record.
  Rationale: Users need one stable cross-format vocabulary at the CLI and config layer. “Functions” is the simpler public term, while parser adapters can translate Istanbul functions and Coverlet methods into that shared model.
  Date/Author: 2026-03-11 / Codex

- Decision: This feature should remain draft-only until at least one real parser path for Istanbul or Coverlet is active in the codebase.
  Rationale: The product direction is clear, but the exact normalization details depend on how those formats expose callable units and source spans. Capturing the design now is useful, but implementation should follow parser availability rather than inventing a fake abstraction in isolation.
  Date/Author: 2026-03-11 / Codex

## Outcomes & Retrospective

Function gating is now implemented and shipping for currently supported coverage formats. Repositories can configure `--fail-under-functions` and `--fail-uncovered-functions` (or TOML equivalents) and get diff-focused function gate results through the same shared gate engine used for regions/lines/branches.

Dogfooding uncovered parser-level normalization issues in LLVM callable records (covered-state derivation, duplicate span handling, and suffix-path mapping). Those issues were fixed with TDD regressions, which stabilized uncovered-function counts and aligned function totals with expected execution behavior.

Istanbul-specific function validation concerns have been moved to the active Istanbul ExecPlan so this completed plan remains scoped to the delivered function metric foundation and currently supported parser families.

## Context and Orientation

`covgate` is a diff-focused Rust CLI in this repository. The current codebase centers on LLVM region coverage and already has active plans for broader metrics and uncovered-count thresholds. The key files that will eventually matter for function thresholds are:

- `src/model.rs`, which defines metric families, opportunity kinds, and gate-facing result types
- `src/config.rs`, which merges CLI flags and `covgate.toml` defaults
- `src/cli.rs`, which defines the public command-line interface
- `src/metrics.rs`, which computes changed coverage metrics from normalized opportunities
- `src/gate.rs`, which evaluates threshold rules
- `src/render/console.rs` and `src/render/markdown.rs`, which explain gate outcomes
- future parser modules for Istanbul and Coverlet that do not exist yet in the current codebase

In this plan, a “function opportunity” means one named callable unit that the coverage report can mark as covered or uncovered. In Istanbul this is naturally a function. In Coverlet this is a method. A “changed function” means a function opportunity whose source span overlaps the changed lines in the diff. A “function threshold” means either a minimum function coverage percent such as `--fail-under-functions 100` or a maximum uncovered changed function count such as `--fail-uncovered-functions 0`.

The main design constraint is that functions are only interesting here if they remain diff-focused. A repository-wide function total is not the product goal. The tool should count only changed callable units or callable units whose source span overlaps the diff, just as it currently counts only changed regions.

## Plan of Work

Start by extending the internal model in `src/model.rs` so it has a stable place for function-oriented opportunity kinds and a metric family for functions. Do not tie the shared model to Istanbul-specific field names or Coverlet-specific method terminology. The normalized model should let a parser adapter say “this callable unit lives in this file, spans these lines, and is covered or uncovered.” That is the minimum information needed for diff intersection and threshold evaluation.

Once the model can represent function opportunities, the future Istanbul and Coverlet adapters should map their native records into that shape. Istanbul will likely provide a more natural function record. Coverlet may require translating methods into the same shared kind. This plan deliberately does not prescribe the exact parser code because those parser plans are separate, but it does require that both adapters end up producing one shared function metric instead of two public concepts.

After normalization exists, the CLI and TOML layers should grow function-specific threshold flags and config keys. The intended public shape is:

    --fail-under-functions <MIN>
    --fail-uncovered-functions <MAX>

and matching TOML keys:

    [gates]
    fail_under_functions = 100
    fail_uncovered_functions = 0

Those names should sit alongside the existing pluralized threshold families such as `fail_under_regions` and `fail_uncovered_regions`.

Then define the diff intersection rule explicitly. For v1 of this feature, a function should count as changed when its normalized source span overlaps at least one added or modified line in the diff. That means a single-line change inside a function body can make that function eligible for the gate. This is the simplest rule, and it matches the existing region intersection logic. If later evidence suggests that declaration-only or signature-only changes need special treatment, that can be a refinement after the basic function metric exists.

Finally, renderers and tests must treat function thresholds as first-class gates. Console output and Markdown output should show function rules using the public word `functions`, not parser-specific vocabulary like `methods`. Tests should include realistic Istanbul and Coverlet fixtures where a changed function or method is left uncovered, causing the function gate to fail even when another metric threshold passes.

## Concrete Steps

This is a draft plan, so the commands below are the intended implementation path once Istanbul or Coverlet parsing exists in the repository.

1. Add function-oriented model support.

    Working directory: the `covgate` repository root

    Edit `src/model.rs` to add a function metric family and a function opportunity kind. Update any shared types that currently assume only regions, lines, or branches.

    Example commands:

        cargo test model

    Expected outcome: model tests prove that the shared types can represent function opportunities without parser-specific terminology leaking into the core model.

2. Implement parser normalization for callable units.

    Working directory: the `covgate` repository root

    In the future Istanbul and Coverlet parser modules, map native function or method records into the shared function opportunity model with repository-relative paths and line spans.

    Example commands:

        cargo test istanbul
        cargo test coverlet

    Expected outcome: parser tests prove that both formats can emit normalized function opportunities and that uncovered callable units retain enough source-span detail for diff intersection and reporting.

3. Add CLI and TOML thresholds for functions.

    Working directory: the `covgate` repository root

    Edit `src/cli.rs` and `src/config.rs` so they accept `--fail-under-functions`, `--fail-uncovered-functions`, `fail_under_functions`, and `fail_uncovered_functions`.

    Example commands:

        cargo test config

    Expected outcome: config tests prove that function thresholds can come from CLI or TOML and follow the same precedence rules as other metric-specific thresholds.

4. Compute and gate changed function metrics.

    Working directory: the `covgate` repository root

    Edit `src/metrics.rs` and `src/gate.rs` so changed function coverage percent and uncovered changed function counts can be evaluated by the same gate engine used for other metrics.

    Example commands:

        cargo test metrics
        cargo test gate

    Expected outcome: metric and gate tests prove that changed functions are selected by diff overlap and that function thresholds can pass or fail independently of line, branch, or region thresholds.

5. Render and validate the new gates.

    Working directory: the `covgate` repository root

    Edit `src/render/console.rs`, `src/render/markdown.rs`, and integration tests under `tests/`.

    The function-threshold CLI assertions in this step must follow `docs/TESTING.md`: add them to `tests/cli_metrics.rs` (not `tests/cli_interface.rs`), define fixture lists for shared threshold semantics, and run each scenario across every compatible fixture unless the scenario is intentionally proving format-specific metric availability behavior.

    Example commands:

        cargo test render
        cargo test cli_metrics
        cargo test cli_interface
        cargo fmt --check
        cargo check
        cargo clippy --all-targets --all-features -- -D warnings
        cargo test
        cargo xtask validate
        cargo llvm-cov --summary-only

    Expected outcome: output and integration tests prove that function gates are visible in both text and Markdown summaries and that the repository validation stack still passes.

## Validation and Acceptance

Acceptance is complete only when all of the following are true.

Running `covgate` with `--fail-under-functions 100` against a fixture where one changed function or method is uncovered fails with a clear explanation that the function threshold did not pass.

Running `covgate` with `--fail-uncovered-functions 0` against the same fixture also fails, but for the uncovered-count reason rather than the percent reason. Output must make that distinction visible.

Running `covgate` against an Istanbul-backed fixture and a Coverlet-backed fixture must use the same public function threshold names and produce the same style of gate explanation, even if the parser adapters use different internal details to compute the metric.

If the function parser path is unavailable for the chosen coverage report format, `covgate` must fail clearly when the user asks for a function threshold. It must not silently ignore the rule.

CLI integration tests must eventually include at least, and these cases must live in `tests/cli_metrics.rs` as metric-semantics tests that iterate across compatible fixture lists when semantics are shared:

- an Istanbul scenario where a changed function is covered and `--fail-under-functions 100` passes
- an Istanbul scenario where a changed function is uncovered and `--fail-under-functions 100` fails
- a Coverlet scenario where a changed method is covered and `--fail-under-functions 100` passes
- a Coverlet scenario where a changed method is uncovered and `--fail-under-functions 100` fails
- at least one scenario using `--fail-uncovered-functions 0`
- at least one scenario where a function threshold and a line, branch, or region threshold are both configured and produce different outcomes

Acceptance is not complete if the CLI says `functions` but the output exposes raw parser terminology such as `methods` inconsistently. The public product vocabulary should stay stable even when parsers differ.

Acceptance is not complete if function thresholds are implemented only as repository-wide totals. The gate must stay diff-focused and only count changed callable units.

## Idempotence and Recovery

This draft feature should be implemented additively once the supporting parsers exist. Re-running the same `covgate` command against the same coverage report and the same diff should produce the same function-threshold result and the same output.

Do not expose `--fail-under-functions` or `--fail-uncovered-functions` publicly before at least one parser path can produce normalized function opportunities. A partially wired public flag with no real parser support would make the CLI misleading.

If fixture work requires new Istanbul or Coverlet repositories, follow the repository’s copied-fixture strategy. Keep checked-in fixture baselines immutable and perform any scenario-specific edits only in temporary working directories during tests.

## Artifacts and Notes

Expected CLI examples for the eventual feature:

    covgate --coverage-json coverage.json --base origin/main --fail-under-functions 100

    covgate --coverage-json coverage.json --base origin/main --fail-uncovered-functions 0

Expected TOML excerpt:

    [gates]
    fail_under_functions = 100
    fail_uncovered_functions = 0

These are draft examples. The final formatting may change, but the public naming and diff-focused behavior described in this plan should remain stable unless the plan itself is revised.

## Interfaces and Dependencies

Stay on Rust stable. Reuse the repository’s existing dependency strategy unless real parser work proves that a new crate is needed.

In `src/model.rs`, the eventual implementation should expose one shared public metric family for functions and one shared opportunity kind for callable units. Parser adapters must translate Istanbul functions and Coverlet methods into those shared concepts.

In `src/config.rs` and `src/cli.rs`, the eventual implementation should follow the same metric-specific threshold pattern already used elsewhere in `covgate`. The function gate keys should be named `fail_under_functions` and `fail_uncovered_functions` in TOML and `--fail-under-functions` and `--fail-uncovered-functions` on the CLI.

In `src/metrics.rs`, changed-function selection should reuse the same diff-overlap principle already used for changed regions: a callable unit counts as changed when its normalized span overlaps added or modified lines in the diff.

In `src/gate.rs`, function thresholds should be evaluated by the same general gate engine as other metrics. Do not add a parser-specific special case that bypasses the shared gate model.

At the bottom of this plan, append a revision note every time the plan changes materially, describing what changed and why.

Revision note: Initial draft plan created for future function or method thresholds because Istanbul and Coverlet are the next intended coverage formats and both expose a callable-unit signal worth gating.

Revision note: Updated the intended TOML section name from `[thresholds]` to `[gates]` so this draft matches the current repository configuration vocabulary.

Revision note: Re-evaluated this plan after native .NET Coverlet support landed; documented that current parser support is line/branch only, confirmed method-shaped data is available in Coverlet fixtures, and reaffirmed `functions` as the single public term with parser-level method normalization.

Revision note: Updated test-planning guidance to match `docs/TESTING.md`: function-threshold scenarios are metric tests in `tests/cli_metrics.rs`, should use compatible fixture matrices for shared semantics, and should be validated with `cargo xtask validate` as part of completion criteria.

Revision note: Documented completed milestones (CLI/TOML surface and diff-overlap behavior), clarified that Istanbul validation remains pending parser support, and captured that function normalization is implemented for currently supported parsers with additional dogfooding-driven parser fixes.

Revision note: Closed out this plan after function-gating implementation shipped for supported parser families (LLVM/Coverlet), moved Istanbul-specific follow-up concerns into the active Istanbul plan, and relocated this file from active to completed.
