# LLVM export semantics investigation in `covgate`

This document records the current investigation into why `covgate` still disagrees with LLVM on line and region totals even after the Rust function-identity fix landed.

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

## Practical guidance

Until this is resolved:

- do not "fix" summary parity by passing LLVM summary data through production code
- do not assume LLVM text view and LLVM summary totals are interchangeable or derived from the same exposed detail
- do use live side-by-side comparisons between exported detail, rendered text, and summary totals when investigating any new LLVM line or region mismatch

## Source pointers

- `src/coverage/llvm_json.rs`
- `tests/llvm_real_parity.rs`
- `tests/fixtures/llvm-real/covgate-self-full.json`
- `docs/exec-plans/active/covgate-llvm-summary-parity.md`
- [`cargo-llvm-cov/src/json.rs#L645-L662`](https://github.com/taiki-e/cargo-llvm-cov/blob/main/src/json.rs#L645-L662)

Upstream references:

- [llvm/llvm-project#126307](https://github.com/llvm/llvm-project/issues/126307)
