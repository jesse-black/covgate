# Covgate

Save this in-progress ExecPlan at `docs/exec-plans/active/covgate.md` while the work is being designed or implemented inside this repository. If the tool is moved into its own repository, move this plan into that repository and update every repository-relative path in this document so the plan remains self-contained in its new home.

This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds.

Maintain this document in accordance with `docs/PLANS.md`. If that file exists in the target repository, re-read it before revising this plan and keep this plan aligned with its rules.

## Purpose / Big Picture

After this change, a repository will be able to run a small Rust command-line tool named `covgate` in continuous integration and ask one focused question about a pull request: "Does the changed Rust code meet the required region coverage threshold?" The first implementation will read LLVM coverage JSON exported from `llvm-cov` or `cargo llvm-cov`, compute changed code from Git diff input, intersect uncovered regions with the changed lines, print a human-readable console report, optionally emit a Markdown report suitable for `$GITHUB_STEP_SUMMARY`, and fail with a nonzero exit code when changed-region coverage falls below the configured threshold.

The user-visible behavior is intentionally narrow in v1. `covgate` is not a general coverage dashboard, not a repository-wide gate, and not yet a multi-format parser. It is a diff-focused gate for Rust coverage data that happens to be architected so additional metrics such as line, branch, or combined coverage, plus additional coverage-report formats such as Cobertura XML or other native JSON outputs, can be added later without rewriting the core model.

You will know this is working when a contributor can run a command like the example below from a repository root, point it at LLVM JSON coverage output plus a Git base reference, and see both a console summary and a pass or fail result that matches the changed-region coverage in the diff.

    cargo run -- --coverage-json coverage.json --base origin/main --fail-under region=90

You will also know this is working when GitHub Actions can write a Markdown summary to `$GITHUB_STEP_SUMMARY` showing the changed files, the changed-region coverage percentage, the threshold, and the uncovered changed regions that caused the failure.

## Progress

- [x] (2026-03-07 00:00Z) Create a standalone ExecPlan for `covgate` that assumes an eventual separate repository and defines a future-proof architecture around metrics, formats, and outputs while keeping the first implementation intentionally narrow.
- [ ] Define the initial crate layout, command-line contract, and core data model for diff coverage opportunities, metrics, and report formats.
- [ ] Implement the LLVM JSON parser, Git diff reader, region-to-diff intersection logic, console reporting, Markdown summary reporting, and threshold evaluation for changed-region coverage.
- [ ] Add unit tests, fixture tests, and copied-fixture CLI integration tests modeled on the current repository’s `dglint` strategy.
- [ ] Capture final validation evidence and move this plan to `docs/exec-plans/completed/` when the first usable `covgate` release exists.

## Surprises & Discoveries

- Observation: None yet.
  Evidence: This plan is being created before implementation work begins.

## Decision Log

- Decision: The first implementation will gate only changed-region coverage from LLVM JSON, even though the architecture must anticipate line, branch, combined, and future metric types.
  Rationale: Region coverage is the strongest standalone metric currently available from the Rust and LLVM coverage stack for this use case. Narrowing the initial behavior keeps the tool shippable while still forcing a metric model that will not collapse when additional metric types are introduced later.
  Date/Author: 2026-03-07 / Codex

- Decision: The first implementation will parse native LLVM JSON coverage export rather than Cobertura XML or LCOV.
  Rationale: LLVM JSON preserves the coverage model needed for region-aware gating. Cobertura and LCOV flatten coverage down to line and optional branch records, which would discard the main signal this tool is meant to enforce.
  Date/Author: 2026-03-07 / Codex

- Decision: `covgate` will support both console output and Markdown summary output in v1.
  Rationale: Console output is needed for local use and CI logs, while Markdown output is the lowest-friction way to produce readable pull-request-adjacent reporting through `$GITHUB_STEP_SUMMARY` without committing to a broader GitHub API integration surface.
  Date/Author: 2026-03-07 / Codex

- Decision: The architecture will separate coverage parsing, diff parsing, metric computation, gating, and output rendering into distinct modules.
  Rationale: Future support for other metrics and report formats should be additive. A monolithic implementation would make later support for line, branch, combined, Cobertura, or other native JSON formats much harder to add safely.
  Date/Author: 2026-03-07 / Codex

- Decision: The testing strategy will mirror `dglint` by combining focused unit tests with copied-fixture CLI integration tests.
  Rationale: The most important failure modes are end-to-end: coverage file parsing, diff selection, threshold evaluation, and output rendering. Copied fixtures in temporary working directories keep those tests realistic without mutating checked-in source fixtures.
  Date/Author: 2026-03-07 / Codex

## Outcomes & Retrospective

This plan exists before implementation, so the current outcome is a scoped specification rather than working behavior. The main design result so far is clarity about what belongs in v1 and what does not. V1 must do one thing well: fail a diff-based region coverage gate from LLVM JSON and explain the result clearly in CI and local output. Future-proofing matters, but only at the architecture layer, not as extra parser or metric work in the first delivery.

The main risk to watch during implementation is over-generalizing too early. If the code tries to fully solve cross-language coverage normalization in the first pass, the tool will likely become slow to build and hard to validate. The intended balance is a narrow first parser and metric with a clean internal model that makes later formats and metrics additions straightforward.

## Context and Orientation

`covgate` is intended to become a small standalone Rust CLI. It may be prototyped inside the current repository first, but the design should assume its long-term home is a dedicated repository with its own `Cargo.toml`, `src/`, `tests/`, fixture directories, and workflow files. The problem it solves is a narrow but common one: many teams want a pull-request coverage gate that only judges the code changed in the diff, not the entire repository, and they want that gate to use a stronger metric than line coverage where the language and tooling support it.

In this plan, a "coverage opportunity" means a measurable unit of executable code that may be covered or uncovered. In v1, the relevant opportunity type is a coverage region exported by LLVM. A "changed opportunity" means a coverage opportunity whose source span overlaps the lines changed in the pull request diff. A "metric" means a specific coverage ratio computed over a set of opportunities, such as line coverage, branch coverage, region coverage, or a combined metric. A "format parser" means code that reads one coverage report format and converts it into the internal coverage model used by the rest of the tool. A "gate" means the pass or fail decision produced by comparing a computed metric against a configured threshold.

The expected external inputs are:

- a coverage report generated from the repository under test, initially LLVM JSON from `cargo llvm-cov --json --output-path coverage.json`
- a Git diff range, usually expressed as a base reference such as `origin/main`, a base and head pair, or a precomputed unified diff file
- one or more threshold settings, initially only a changed-region threshold
- optional output configuration controlling Markdown summary emission

The core architectural challenge is not parsing one format. It is preserving enough semantic structure that the code can later support other formats and metrics without breaking the v1 region gate. That means the internal data model must distinguish between:

1. raw parser-specific coverage records
2. normalized source spans and coverage opportunities
3. changed-span selection from Git diff
4. metric computation
5. threshold evaluation
6. output rendering

The plan assumes Git is available in the execution environment for local runs and CI, and that repositories using the tool already know how to generate LLVM JSON coverage before invoking `covgate`.

## Plan of Work

Start by creating a new binary crate for `covgate` with a library-first layout. Keep the main binary entrypoint thin so it only parses arguments, calls a library function, prints the selected output, and exits with a status code derived from the gate result. Put the real implementation behind the library crate and split the code into modules with responsibilities that are stable even as new metrics and formats are added later.

The initial crate layout should include modules conceptually equivalent to the list below, although exact filenames may be adjusted if the implementation reveals a better split:

    src/main.rs
    src/lib.rs
    src/cli.rs
    src/config.rs or src/options.rs
    src/diff.rs
    src/coverage/mod.rs
    src/coverage/llvm_json.rs
    src/model.rs
    src/metrics.rs
    src/gate.rs
    src/render/console.rs
    src/render/markdown.rs

Define the internal model first. The model must not hard-code region coverage as the only possible metric even though v1 only computes changed-region coverage. It should include a `MetricKind` enum or equivalent that can already name `Region`, `Line`, `Branch`, and `Combined`, plus a threshold structure that can pair a metric kind with a percentage. It should also define a parser-neutral source span type that can represent file path plus start and end line information, because diff intersection and report rendering both need that information even when future formats provide richer column-level data.

For coverage opportunities, define an internal shape that can support multiple opportunity kinds later. In v1, it may only materialize region opportunities, but the type should still encode the kind explicitly. One workable design is an enum such as the example below, plus a wrapper that stores covered versus total counts and optional parser provenance:

    enum OpportunityKind {
        Region,
        Line,
        BranchOutcome,
    }

The LLVM JSON parser module should do two things only: parse the JSON safely into Rust types and convert the relevant file-level region records into the normalized model used by the metrics layer. Do not let parser-specific JSON details leak into the metric, gate, or rendering modules. If the JSON contains more detail than v1 needs, preserve only what is necessary for changed-region computation and useful failure reporting. The parser should normalize file paths consistently and reject malformed or incomplete reports with actionable error messages.

Implement Git diff handling separately from coverage parsing. The diff module should accept either a base reference or an explicit diff file path if that keeps CI and local use simpler. In both cases, the module should normalize the result to repository-relative changed-file entries with changed line ranges. The intersection algorithm should then compare changed line ranges against normalized region spans and decide which coverage regions count toward the changed-region metric. Make the intersection rule explicit in code and tests: a region counts as changed when its source span overlaps at least one added or modified line in the diff. Deleted-only lines should not create coverage obligations because there is no remaining executable code to measure.

With parsing and diff selection in place, implement the metric layer. The first required metric is changed-region coverage:

    changed_region_coverage = covered_changed_regions / total_changed_regions

The metric layer should return both the aggregate ratio and the underlying uncovered changed regions so the renderers can explain failures. Even though combined, line, and branch gates are out of scope for implementation, the metric layer must already be structured so future metric calculators can be added without changing the diff or parser modules.

Implement threshold evaluation in a dedicated gate module as a separate step after metric computation. The gate should accept one or more threshold definitions even though v1 only supports a region threshold in the CLI. That keeps the code ready for later additions without making the current user-facing interface confusing. The gate result should capture pass or fail, the computed percentage, the configured threshold, and the uncovered changed opportunities that caused failure.

Renderers should consume the gate result, not raw parser output. The console renderer should produce a concise but informative summary suitable for local development and CI logs. It should show the metric name, computed percentage, threshold, pass or fail state, and a compact list of uncovered changed regions grouped by file. The Markdown renderer should produce a GitHub-friendly summary with headings, percentages, and short code blocks or tables only if they remain readable in GitHub’s summary UI. Keep Markdown output deterministic and plain; this is a machine-written CI artifact, not a rich report site.

The CLI contract should be explicit and stable in v1. A novice should be able to run `covgate --help` and understand:

- where to point the tool at LLVM JSON coverage data
- how to specify the Git base reference or diff file
- how to set a changed-region threshold
- how to write Markdown output to a file or directly to `$GITHUB_STEP_SUMMARY`
- what exit code behavior to expect on pass, fail, and configuration or parse errors

Do not overreach into repository-specific coverage generation in v1. `covgate` should assume the coverage JSON already exists and should not try to invoke `cargo llvm-cov` internally. That keeps the tool focused on gating rather than test orchestration.

## Concrete Steps

Run the following commands from the repository root that will contain `covgate`.

1. Create the Rust crate and verify the stub command works.

    Working directory: repository root

    Example commands:

        cargo new --bin covgate
        cargo run --manifest-path covgate/Cargo.toml -- --help

    Expected outcome: The repository now contains a compilable CLI skeleton and a help screen for the new binary.

2. Define the internal types before implementing parsers.

    Working directory: the `covgate` crate root

    Example commands:

        cargo test model
        cargo test metrics

    Expected outcome: Unit tests prove that source spans, opportunity kinds, metric kinds, and threshold definitions can represent the future line, branch, region, and combined metric families even though only regions are actually computed in v1.

3. Implement LLVM JSON parsing and normalized coverage opportunities.

    Working directory: the `covgate` crate root

    Example commands:

        cargo test llvm_json
        cargo run -- --coverage-json tests/fixtures/coverage/basic.json --base HEAD~1 --fail-under region=80

    Expected outcome: Checked-in LLVM JSON fixtures parse successfully into normalized region opportunities, malformed fixtures fail clearly, and parser details do not leak outside the coverage module.

4. Implement Git diff parsing and changed-line normalization.

    Working directory: the `covgate` crate root

    Example commands:

        cargo test diff
        cargo test intersection

    Expected outcome: Tests prove that changed lines are selected correctly from fixture diffs or temporary Git repositories, deleted-only hunks are ignored for gating, and changed lines intersect correctly with region spans.

5. Implement changed-region metric computation, threshold evaluation, and rendering.

    Working directory: the `covgate` crate root

    Example commands:

        cargo test gate
        cargo test render
        cargo run -- --coverage-json tests/fixtures/coverage/basic.json --base origin/main --fail-under region=90 --markdown-output /tmp/covgate-summary.md

    Expected outcome: The CLI prints a readable console report, writes Markdown output when requested, and exits with success or failure according to the configured changed-region threshold.

6. Add end-to-end CLI integration tests using copied fixtures.

    Working directory: the `covgate` crate root

    Example commands:

        cargo test cli
        cargo test --quiet

    Expected outcome: Integration tests copy fixture repositories into temporary working directories, run the compiled CLI against fixture coverage JSON and Git histories, assert exit codes and outputs, and confirm that the checked-in fixtures remain unchanged.

7. Add CI workflow coverage for the tool itself.

    Working directory: repository root for the `covgate` repository

    Example commands:

        cargo fmt --check
        cargo check
        cargo clippy --all-targets --all-features -- -D warnings
        cargo test
        cargo llvm-cov --summary-only

    Expected outcome: The repository has a repeatable baseline quality workflow, and the coverage gate tool can dogfood its own implementation tests later if desired.

## Validation and Acceptance

Acceptance is complete only when all of the following are true.

Running `covgate` against a repository with LLVM JSON input and a known diff computes changed-region coverage only from the changed lines in the diff, not from the entire repository. At minimum, tests must prove the tool distinguishes between changed and unchanged covered regions in the same file.

The LLVM JSON parser must be validated with checked-in fixtures representing at least:

- a clean small report with covered and uncovered regions
- a report containing multiple files
- malformed or incomplete JSON that should fail clearly
- file paths that need normalization relative to the repository root

The diff parser must be validated with checked-in fixtures or temporary Git repositories representing at least:

- a file with added lines that overlap uncovered regions
- a file with changed lines that overlap covered regions
- deleted-only hunks that must not count toward changed coverage
- multiple files in one diff
- a file present in the coverage report but absent from the diff, which must not count toward changed coverage

CLI integration tests must use copied fixtures in temporary working directories, mirroring the testing strategy used by `dglint`. The checked-in fixtures should include miniature Git repositories plus coverage JSON files or reproducible fixture data that the tests copy before invoking the compiled `covgate` binary. Acceptance is not complete until those integration tests assert:

- success exit code when changed-region coverage meets the threshold
- failure exit code when changed-region coverage falls below the threshold
- console output includes the metric name, computed percentage, threshold, and uncovered changed regions
- Markdown output can be written to a file that matches expected content closely enough to detect regressions
- rerunning the CLI on the same fixture is idempotent

The architecture must be demonstrably future-proof in code shape even though future metrics and formats are out of scope for implementation. Acceptance is not complete until the code has a stable place for:

- additional metric kinds such as line, branch, and combined
- additional opportunity kinds beyond regions
- additional coverage parsers such as Cobertura XML or other native JSON formats

This does not require implementing those formats or metrics, but it does require proving via tests or type-level structure that adding them later will be additive rather than requiring a redesign of the core model.

The console report should be concise enough for local use. A passing run should show a short summary with the changed-region percentage and threshold. A failing run should additionally list the uncovered changed regions grouped by file. The Markdown report should be suitable for direct use in `$GITHUB_STEP_SUMMARY` without additional processing.

## Idempotence and Recovery

The implementation steps in this plan are additive and should be safe to repeat. The CLI itself is read-only with respect to source code under test. Re-running `covgate` on the same coverage JSON and the same diff should produce the same result and should not modify repository files unless the user explicitly chooses a Markdown output path that overwrites an existing file.

If a parser or diff-handling step is only partially implemented and tests fail midway through development, the safe recovery path is to keep the incomplete logic behind non-exported functions or feature-complete module boundaries until the associated tests pass. Avoid partially wiring unfinished parser details into the main CLI path, because that will make end-to-end failures harder to diagnose.

If the tool is moved to a new repository during implementation, copy this plan with it before changing paths or build commands, then update the repository-relative references throughout the document so a novice can still execute it from top to bottom without access to the original repository.

## Artifacts and Notes

Record concise evidence here as implementation proceeds. Replace the placeholders below with real transcripts and examples.

Expected passing console excerpt:

    covgate: PASS
    Metric: region
    Changed coverage: 92.31%
    Threshold: 90.00%
    Covered changed regions: 12 / 13

Expected failing console excerpt:

    covgate: FAIL
    Metric: region
    Changed coverage: 66.67%
    Threshold: 90.00%
    Covered changed regions: 4 / 6

    Uncovered changed regions:
    - src/metrics.rs:41-47
    - src/metrics.rs:73-79

Expected Markdown summary excerpt:

    ## Covgate

    - Result: FAIL
    - Metric: region
    - Changed coverage: 66.67%
    - Threshold: 90.00%

    ### Uncovered changed regions

    - `src/metrics.rs:41-47`
    - `src/metrics.rs:73-79`

At the bottom of this plan, append a revision note every time the plan changes materially, describing what changed and why.

Revision note: Initial standalone plan created for `covgate`, scoped to LLVM JSON plus changed-region gating in v1 while requiring a code architecture that can later absorb additional metrics, thresholds, and report formats.

## Interfaces and Dependencies

Use Rust stable unless implementation evidence proves a nightly-only feature is required. Prefer small, well-scoped dependencies.

The CLI should use `clap` for argument parsing. Error handling may use `anyhow` at the top-level command boundary, but internal modules should prefer typed structures where that improves clarity. Git interaction may begin by shelling out to `git diff --unified=0` if that keeps the first implementation small and deterministic, but the diff module must hide that choice behind a stable internal interface so a pure-Rust diff reader can replace it later if needed.

In the coverage layer, define a parser trait or parser entrypoint that can support multiple formats later. One acceptable shape is:

    pub trait CoverageParser {
        fn parse(&self, input: &str) -> anyhow::Result<CoverageReport>;
    }

In the model layer, define stable types for spans, opportunities, metrics, and thresholds. One acceptable target shape is:

    pub enum MetricKind {
        Region,
        Line,
        Branch,
        Combined,
    }

    pub struct Threshold {
        pub metric: MetricKind,
        pub minimum_percent: f64,
    }

    pub struct SourceSpan {
        pub path: std::path::PathBuf,
        pub start_line: u32,
        pub end_line: u32,
    }

    pub enum OpportunityKind {
        Region,
        Line,
        BranchOutcome,
    }

    pub struct CoverageOpportunity {
        pub kind: OpportunityKind,
        pub span: SourceSpan,
        pub covered: bool,
    }

    pub struct GateResult {
        pub metric: MetricKind,
        pub covered: usize,
        pub total: usize,
        pub percent: f64,
        pub threshold: Threshold,
        pub passed: bool,
        pub uncovered_changed_opportunities: Vec<CoverageOpportunity>,
    }

The renderer layer should consume `GateResult` plus lightweight metadata rather than parser-specific coverage records. That boundary is important because future report outputs, such as SARIF, GitHub Checks API payloads, or richer Markdown summaries, should not require any changes to the LLVM parser or diff logic.
