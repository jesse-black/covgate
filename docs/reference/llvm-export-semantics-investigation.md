# LLVM export semantics investigation in `covgate`

This document records the current investigation into why `covgate` still disagrees with LLVM on line and region totals even after the Rust function-identity fix landed.

The most important update since the original investigation is that line and region discrepancies should no longer be treated as the same class of problem.

- The region discrepancy currently looks like a narrow summary-rule gap inside LLVM's segment semantics.
- The line discrepancy currently looks broader and more structurally ambiguous because LLVM exposes multiple externally visible line views for the same run.

That distinction matters for both confidence claims and test design.

The important takeaway is that there may be more than one "LLVM truth" visible to downstream tools:

- exported JSON detail such as `files[].segments` and `functions[].regions`
- rendered human-facing views such as `llvm-cov report --text`
- per-file and top-level summary totals such as `files[].summary` and `data[0].totals`

Those views do not always line up exactly.

## Why this matters

The active parity work is not trying to make `covgate` print the same numbers by copying LLVM summaries. It is trying to prove that `covgate`'s own calculations are correct.

That only works if the exported detail we parse actually contains enough information to derive the same semantics LLVM uses in its summaries. If it does not, then "summary parity" and "correct diff coverage calculation" may be related but not identical goals.

## Current state

On the checked-in real LLVM repro fixture in `tests/fixtures/llvm-real/covgate-self-full.json`, `covgate` now matches LLVM for functions but still misses lines and regions:

- native summary: regions `3285/3408`, lines `2890/2957`, functions `160/165`
- `covgate`: regions `3252/3355`, lines `2865/2910`, functions `160/165`

That is the red test in `tests/llvm_real_parity.rs`.

The repository should read that red test carefully:

- it is strong evidence that `covgate` does not match LLVM summary totals for lines and regions on the real fixture
- it is not, by itself, proof that the line and region discrepancies have the same root cause

## Live investigation finding: text view and summary view can disagree

For a live repository coverage run, we generated both:

- `cargo llvm-cov report --text --output-dir /tmp/llvmtext-covgate`
- `cargo llvm-cov report --json --output-path /tmp/liveexport.json`

When comparing LLVM's text-rendered executable lines to the same file's JSON `summary.lines`, they did not always match.

Examples:

- `src/config.rs`
  - text view: covered `322`, total `337`
  - file summary: covered `309`, total `342`
- `src/coverage/llvm_json.rs`
  - text view: covered `786`, total `788`
  - file summary: covered `832`, total `834`
- `src/metrics.rs`
  - text view: covered `127`, total `127`
  - file summary: covered `133`, total `133`

This means a downstream parser can match LLVM's visible text rendering for a file and still fail summary parity for that same file.

This is the key reason the line discrepancy should not be bucketed together with the region discrepancy. For regions, we have evidence of a remaining small summary-rule gap. For lines, we have evidence that LLVM itself exposes multiple line semantics through different outputs.

## What `covgate` currently matches

For some files, `covgate`'s current line derivation already matches LLVM's text-rendered executable-line view exactly.

One concrete example from the live run:

- `src/config.rs`
  - `covgate`-style line derivation from exported detail: `322/337`
  - LLVM text view: `322/337`
  - LLVM file summary: `309/342`

So the remaining mismatch is not always "our parser disagrees with LLVM's visible line rendering." In at least some files, it is "LLVM summary counts something different from the visible line rendering."

## Upstream evidence from `cargo-llvm-cov`

The `cargo-llvm-cov` project documents the same problem space in its JSON handling code.

In [`src/json.rs` from `cargo-llvm-cov`](https://github.com/taiki-e/cargo-llvm-cov/blob/main/src/json.rs#L645-L662), the uncovered-line logic includes tests and comments saying that counting line coverage from file segments can be wrong and that function-region-based inference matches `llvm-cov report` better for some cases.

The relevant comment says, in paraphrase:

- counting line coverage based on file segments led to incorrect results
- using regions inside functions matched the `llvm-cov report` behavior for that case

That is useful evidence for `covgate` because it shows another LLVM JSON consumer already had to work around export-detail ambiguity instead of assuming the semantics were obvious.

## `cargo-llvm-cov` global gates vs `covgate` diff gates

Inspecting `cargo-llvm-cov`'s source is useful because it shows a different trust model from `covgate`.

In [`src/report.rs`](https://github.com/taiki-e/cargo-llvm-cov/blob/main/src/report.rs), `cargo-llvm-cov` handles its global gate flags by loading a JSON export and then:

- `--fail-under-functions`, `--fail-under-lines`, and `--fail-under-regions`
  use `get_coverage_percent(...)`
- `--fail-uncovered-functions` and `--fail-uncovered-regions`
  use `count_uncovered_functions()` and `count_uncovered_regions()`
- `--fail-uncovered-lines`
  uses `get_uncovered_lines(...)`

In [`src/json.rs`](https://github.com/taiki-e/cargo-llvm-cov/blob/main/src/json.rs), those helpers split into two styles:

- summary-driven helpers:
  - `get_coverage_percent(...)`
  - `count_uncovered_functions()`
  - `count_uncovered_lines()`
  - `count_uncovered_regions()`
  These read `data[*].totals[...]` directly.
- concrete-line helper:
  - `get_uncovered_lines(...)`
  This derives uncovered lines from `functions[].regions` instead of reading a summary line set.

So `cargo-llvm-cov` is not using a single unified model for every coverage question. It is already comfortable doing both:

- trusting LLVM summary totals for global percentage and uncovered-count gates
- deriving concrete uncovered lines from function-region detail for line listing behavior

That matters when comparing it to `covgate`.

`covgate`'s diff gate path is different:

- `src/coverage/*.rs` adapters build `CoverageOpportunity` records
- `src/metrics.rs::compute_changed_metric` filters those opportunities by changed diff lines using `SourceSpan::overlaps_line_range`
- `src/gate.rs::evaluate` decides pass or fail from the resulting changed covered/total/uncovered values

So the comparison is:

- `cargo-llvm-cov` global gates are summary-first
- `covgate` diff gates are opportunity-first

This is why exact LLVM summary parity is informative but not sufficient as the only confidence target for `covgate`. A global gate tool can safely trust LLVM summary totals because it is enforcing global thresholds. A diff gate tool must primarily trust the correctness of the changed opportunity set it constructs from the native export.

## Function-region union is not a complete answer for this repro

One obvious follow-up hypothesis was that `covgate` should derive lines from the union of function regions instead of from file segments.

That idea is worth taking seriously because `cargo-llvm-cov` already uses function-region-based inference for uncovered-line reporting in some cases. But on the checked-in real repro fixture, that change by itself does not explain the remaining gap.

For the highest-drift files in the real repro:

- `src/coverage/llvm_json.rs`
  - summary lines: `681/684`
  - segment-derived lines: `673/675`
  - function-region union lines: `673/675`
- `src/config.rs`
  - summary lines: `309/342`
  - segment-derived lines: `322/337`
  - function-region union lines: `322/337`
- `src/metrics.rs`
  - summary lines: `132/133`
  - segment-derived lines: `126/127`
  - function-region union lines: `126/127`

So for this repro, switching line derivation from file segments to simple function-region union would leave the mismatch unchanged for the files we inspected.

That does not make function-region information useless. It does mean the remaining discrepancy is not solved by that one substitution alone.

## Additional findings from the live mismatch files

For the highest-drift live files we inspected (`src/metrics.rs`, `src/config.rs`, and `src/coverage/llvm_json.rs`):

- `expansions` were empty
- `branches` were empty
- summary line totals still exceeded both the segment-derived line union and the function-region line union

Examples from the live export:

- `src/metrics.rs`
  - summary lines: `133`
  - union of all function-region lines: `127`
  - delta: `+6`
- `src/config.rs`
  - summary lines: `342`
  - union of all function-region lines: `337`
  - delta: `+5`
- `src/coverage/llvm_json.rs`
  - summary lines: `834`
  - union of all function-region lines: `822`
  - delta: `+12`

So the remaining line difference is not explained by:

- macro-expansion records
- branch records
- switching from file segments to simple function-region union

## LCOV shows both line views at once

Adding `cargo llvm-cov report --lcov` to the comparison produced a very useful result.

For the same live coverage run:

- LCOV `LF` and `LH` matched the JSON summary line totals exactly
- LCOV `DA:` entries matched the lower executable-line view instead

Examples:

- `src/metrics.rs`
  - JSON summary lines: `133/133`
  - LCOV `LF/LH`: `133/133`
  - LCOV `DA:` line count: `127`
  - LLVM text executable lines: `127/127`
- `src/config.rs`
  - JSON summary lines: `342/309`
  - LCOV `LF/LH`: `342/309`
  - LCOV `DA:` line count: `337`
  - LLVM text executable lines: `337/322`
- `src/coverage/llvm_json.rs`
  - JSON summary lines: `834/832`
  - LCOV `LF/LH`: `834/832`
  - LCOV `DA:` line count: `822`
  - LLVM text output in the earlier captured run was lower still at `788/786`

The important point is not the exact live numbers for one file. The important point is the shape:

- summary-oriented counts live in `LF/LH`
- concrete listed lines live in `DA:`
- those are not always the same set cardinality
- LLVM text output can differ from both of them

This strongly suggests LLVM is exporting two different notions of line accounting:

- a summary line metric
- a concrete per-line listing metric

So a downstream parser that works from concrete line listings or derivable executable lines should not assume it can always reproduce the summary `LF/LH` line totals exactly from those concrete entries alone.

At this point the strongest current line-specific conclusion is not "LLVM line summaries are internally inconsistent in exactly the same way as regions." The stronger conclusion is:

- LLVM line summaries, concrete LCOV `DA:` line listings, and text-rendered executable lines are competing visible views
- `covgate` may already match one of those views for some files while still disagreeing with another
- the remaining line work must identify which external line oracle should define confidence for diff gating before treating summary disagreement as a parser defect

## Region-side finding: gap regions are not the cause in the live drift files

For the high-drift live files we inspected, the file-segment data had:

- no gap regions
- many non-entry segments
- only a small summary-region delta over `covgate`'s current "entry and not gap" region count

Examples:

- `src/config.rs`
  - summary regions: `436`
  - current `covgate` region count: `425`
  - delta: `+11`
  - non-entry segments present: `113`
  - gap segments present: `0`
- `src/coverage/llvm_json.rs`
  - summary regions: `838`
  - current `covgate` region count: `827`
  - delta: `+11`
  - non-entry segments present: `86`
  - gap segments present: `0`
- `src/coverage/coverlet_json.rs`
  - summary regions: `412`
  - current `covgate` region count: `403`
  - delta: `+9`
  - non-entry segments present: `36`
  - gap segments present: `0`

That means "count all non-entry segments too" would overshoot badly, but "ignore all non-entry segments" is still missing a small number of regions in some files.

So the remaining region mismatch also appears to depend on a narrower LLVM rule than the simple heuristics tested so far.

This is an important contrast with the line story. The region evidence points toward a still-unresolved summary-rule detail inside one semantic family. The line evidence points toward multiple semantic families that LLVM exposes simultaneously.

## Current hypotheses that remain plausible

Based on the evidence collected so far, the remaining possibilities include:

- `covgate` still has a real parser bug in line or region derivation for some LLVM segment patterns
- LLVM summary counts include semantics that are not exposed directly enough in exported detail to reproduce exactly
- both of those are true at once

The investigation should keep treating those as separate possibilities until a smaller failing pattern is isolated.

## Upstream LLVM evidence

LLVM users have also reported that exact covered-line sets are not exposed directly in JSON export detail today.

The clearest current example is [llvm/llvm-project#126307](https://github.com/llvm/llvm-project/issues/126307), which asks LLVM to include covered lines explicitly because tools currently have to infer them from exported JSON.

That does not prove every remaining `covgate` mismatch is impossible to fix. It does prove we should not assume the export already contains an obvious one-to-one encoding of LLVM summary semantics.

## What this means for the parity investigation

The remaining line and region work needs to answer two separate questions:

1. Which semantics are actually recoverable from LLVM export detail and therefore fair for `covgate` to compute itself?
2. Does `covgate` currently compute those recoverable semantics correctly?

Only after answering those questions should we decide whether the red parity test indicates:

- a real parser bug in `covgate`
- an upstream LLVM export-detail limitation
- or a mixture of both

For lines and regions, those answers may now legitimately diverge. The current repository evidence does not justify assuming one shared explanation.

## What should give `covgate` confidence for diff gating

The code path in `covgate` makes an important distinction:

- overall summaries come from `totals_by_file`
- changed-code gate decisions come from `CoverageOpportunity` records plus diff-line overlap in `src/metrics.rs`

That means summary parity is useful evidence, but it is not the only thing that matters for trusting diff gating.

For changed-code gating, the confidence question is narrower and more concrete:

1. Did the parser produce the right changed opportunity set for the metric?
2. Did each opportunity get the right covered or uncovered state?
3. Did the shared overlap logic include exactly the opportunities touched by the diff?

Those are the conditions that directly drive:

- changed covered count
- changed total count
- changed percent
- uncovered changed opportunity count

in `compute_changed_metric()`, and therefore the actual pass/fail result in `evaluate()`.

## Confidence model for each metric

### Line gating

For line gating, the strongest confidence evidence should come from LLVM's concrete listed lines, not just from summary totals.

The most useful current external oracles are:

- LLVM text output for visible executable lines
- LCOV `DA:` entries for concrete listed lines

Those are closer to the actual gating question than summary `LF/LH` totals because a diff gate needs to know which changed lines are concrete opportunities, not just how many summary lines LLVM reports overall.

So line-gating confidence should come from fixture-backed tests that:

- generate real diffs against checked-in files
- assert that changed lines matching LLVM concrete line listings are gated
- assert that changed lines absent from LLVM concrete line listings are not treated as line opportunities

Summary parity remains useful as a secondary signal, but it should not be the only acceptance bar for line gating.

The repository's current line-focused tests do not yet fully reach that bar. They prove `covgate`'s changed-line behavior is internally consistent on real LLVM exports, but they do not yet compare changed line opportunities against LLVM text output or LCOV `DA:` entries directly.

### Region gating

For region gating, the strongest confidence evidence should come from targeted LLVM segment-pattern fixtures plus diff overlap behavior.

Region totals are useful smoke checks, but diff gating cares more about:

- whether a changed span creates a region opportunity
- whether that opportunity is covered
- whether the changed line range overlaps the intended region span

So region-gating confidence should come from small, explicit fixtures and parser tests that isolate:

- entry vs non-entry segment windows
- same-line vs multi-line region spans
- covered vs uncovered region transitions
- changed-line overlap against those spans

### Function gating

For function gating, the strongest confidence evidence is already closer to hand:

- normalized function identity tests for mangled names
- covered-state tests for top-level count vs executed function regions
- diff overlap tests for function spans

That is why function confidence improved meaningfully once the Rust mangled-name repro was added.

## Practical acceptance shift

If LLVM summary line or region totals remain partially irreducible from export detail, `covgate` can still earn strong confidence for diff gating by proving the changed opportunity calculations directly.

In other words:

- summary parity is a useful overall regression signal
- changed-opportunity correctness is the direct proof for gate correctness

That should shape the next tests we add.

More concretely:

- the line follow-up should be an external-oracle confidence plan
- the region follow-up should stay a parser- and segment-semantics investigation

## Current test-suite gap

The existing repository tests already prove some important pieces:

- parser unit tests for selected LLVM segment and function cases
- diff parsing tests in `src/diff.rs`
- end-to-end CLI pass/fail behavior across the checked-in fixture matrix in `tests/cli_metrics.rs`

But they do not yet give strong direct evidence for the exact changed opportunity set in the hard LLVM cases.

In practice, most current integration coverage tests assert that a whole fixture passes or fails under a threshold. That is valuable smoke coverage, but it does not directly assert:

- which exact line opportunities should be present for changed lines
- which exact region opportunities should be present for changed lines
- which exact function opportunities should be present for changed lines
- which changed opportunities should appear in `uncovered_changed_opportunities`

So the next high-value tests should be diff-focused assertions over explicit changed-line scenarios, not only more whole-fixture threshold checks.

The first step in that direction now exists:

- `tests/llvm_diff_regression.rs` parses real LLVM fixture coverage for Rust, C++, and Swift basic fixtures, loads the actual unified diff for each fixture worktree, and asserts exact changed line, region, and function opportunities through `compute_changed_metric()`
- the C++ fixture regression also asserts exact changed branch opportunities
- `tests/llvm_diff_regression.rs` now also uses the checked-in real multi-file LLVM export with synthetic diffs for `src/config.rs` and `src/coverage/llvm_json.rs`, asserting exact changed line, region, and function outcomes on higher-complexity report shapes
- `tests/llvm_diff_regression.rs` also runs end-to-end `covgate check` assertions on real-export diff slices for `src/config.rs`, `src/coverage/coverlet_json.rs`, and `src/render/markdown.rs`, proving the actual rule pass/fail outcomes users see for percent and uncovered-count gates

That improves confidence meaningfully, but it is still important to describe the limit precisely:

- these tests prove `covgate`'s changed-line, changed-region, and changed-function behavior on real LLVM exports is stable and explicit
- they do not yet prove that the changed line opportunities match LLVM's best external concrete-line oracle
- they are therefore stronger evidence for diff-gating consistency than for LLVM line-oracle parity

That limit matters much more for lines than for regions, because the line investigation has already shown multiple competing external views.

## Current repository state

The repository is in a stronger position than it was when the original parity work closed, but the confidence picture is asymmetric:

- `tests/llvm_real_parity.rs` still documents real-summary disagreement for lines and regions on `tests/fixtures/llvm-real/covgate-self-full.json`
- `tests/llvm_diff_regression.rs` now proves exact changed opportunities and CLI gate outcomes on small LLVM fixtures and on several higher-complexity slices of the real LLVM export
- function parity is on much firmer ground because the normalized-name fix closed the real fixture mismatch
- region confidence is improved by the explicit real-export diff regressions, but the remaining summary mismatch still looks like a segment-rule investigation
- line confidence is improved for diff gating, but not yet for external-oracle parity, because no test currently compares changed line opportunities to LLVM text output or LCOV `DA:` entries

In other words, the repository now has good evidence that `covgate`'s LLVM line gating is coherent on the exercised real diffs. It does not yet have enough evidence to say that the remaining LLVM line summary mismatch is "the same kind of internal inconsistency" as the region mismatch.

That does not resolve the harder live-summary ambiguity yet, but it is the right shape of proof for trusting diff gating.

## Practical guidance

Until this is resolved:

- do not "fix" summary parity by passing LLVM summary data through production code
- do not assume LLVM text view and LLVM summary totals are interchangeable or derived from the same exposed detail
- do use live side-by-side comparisons between exported detail, rendered text, and summary totals when investigating any new LLVM line or region mismatch
- do treat new line discrepancies and new region discrepancies as potentially different problems until a concrete repro proves otherwise

## Source pointers

- `src/coverage/llvm_json.rs`
- `tests/llvm_real_parity.rs`
- `tests/fixtures/llvm-real/covgate-self-full.json`
- `docs/exec-plans/completed/covgate-llvm-summary-parity.md`
- [`cargo-llvm-cov/src/json.rs#L645-L662`](https://github.com/taiki-e/cargo-llvm-cov/blob/main/src/json.rs#L645-L662)

Upstream references:

- [llvm/llvm-project#126307](https://github.com/llvm/llvm-project/issues/126307)
