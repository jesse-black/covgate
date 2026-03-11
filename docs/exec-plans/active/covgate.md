# Covgate

Save this in-progress ExecPlan at `docs/exec-plans/active/covgate.md` while the work is being designed or implemented in this repository.

This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds.

Maintain this document in accordance with `docs/PLANS.md`. Re-read that file before revising this plan and keep this plan aligned with its rules.

## Purpose / Big Picture

After this change, a repository will be able to run a small Rust command-line tool named `covgate` in continuous integration and ask one focused question about a pull request: "Does the changed code meet the configured coverage threshold for the selected metric?" The first implementation will read LLVM coverage JSON exported from `llvm-cov` or `cargo llvm-cov`, compute changed code from Git diff input, intersect changed lines with normalized coverage opportunities, print a human-readable plain-text summary to standard output, optionally emit a Markdown report suitable for `$GITHUB_STEP_SUMMARY` in the same invocation, and fail with a nonzero exit code when the configured diff metric falls below the threshold. In v1, the only implemented metric is changed-region coverage.

The user-visible behavior is intentionally narrow in v1. `covgate` is not a general coverage dashboard, not a repository-wide gate, and not yet a multi-format parser. It is a diff-focused gate for Rust coverage data that happens to be architected so additional metrics such as line, branch, or combined coverage, plus additional coverage-report formats such as Coverlet, Istanbul, Cobertura XML, or other native JSON outputs, can be added later without rewriting the core model. The one deliberate exception is reporting: when the source coverage data already contains repository-wide totals for the active metric family, the Markdown summary may include those overall totals as extra context, but only the diff-based metric controls pass or fail in v1.

You will know this is working when a contributor can run a command like the example below from a repository root, point it at LLVM JSON coverage output plus a Git base reference, and see both a console summary and a pass or fail result that matches the changed-region coverage in the diff.

    cargo run -- --coverage-json coverage.json --base origin/main --fail-under-regions 90

You will also know this is working when GitHub Actions can run one `covgate` command that writes a Markdown summary to `$GITHUB_STEP_SUMMARY`, prints a plain-text summary to standard output for the job log, and returns the pass or fail exit code directly without needing a wrapper shell script to capture and replay status.

## Progress

- [x] (2026-03-07 00:00Z) Create a standalone ExecPlan for `covgate` that defines a future-proof architecture around metrics, formats, and outputs while keeping the first implementation intentionally narrow.
- [x] (2026-03-10 00:00Z) Evaluate whether diff collection in v1 should use `git2-rs`, `gitoxide` (`gix`), or the installed `git` CLI.
- [x] (2026-03-11 00:00Z) Define the initial crate layout, command-line contract, and core data model for diff coverage opportunities, metrics, and report formats.
- [x] (2026-03-11 18:20Z) Clarify that v1 includes repository-local TOML configuration and that configuration values may provide defaults for `--base` and `--fail-under-<METRIC>` while remaining overridable from the CLI.
- [x] (2026-03-11 19:05Z) Implement repository-local TOML configuration loading from `covgate.toml` and merge it with CLI values so `base` and gate defaults can come from config while explicit CLI flags still win.
- [x] (2026-03-11 20:05Z) Replace the generic `--fail-under METRIC=PERCENT` CLI shape with pluralized metric-specific `--fail-under-*` flags and align TOML gate keys to the same naming scheme while keeping v1 limited to one effective threshold per run.
- [x] (2026-03-11 20:55Z) Expand copied-fixture Rust CLI coverage with a passing diff-file scenario, a Markdown-output scenario, and an explicit branch-versus-main Git-base scenario.
- [x] (2026-03-11 22:10Z) Normalize real LLVM JSON file paths to repository-relative paths so branch-versus-main dogfooding can match coverage records to Git diff entries instead of falsely reporting zero changed regions.
- [x] (2026-03-11 22:25Z) Add an end-to-end CLI regression test that rewrites fixture coverage JSON to use an absolute temp-worktree path, proving that real-path LLVM normalization survives full diff matching and does not regress back to `Changed regions: 0`.
- [x] (2026-03-11 22:40Z) Fix deleted-file unified diff handling via TDD by first reproducing the `/dev/null` header failure in a diff parser test and then teaching the parser to skip deleted-file hunks without poisoning the next file’s path state.
- [ ] Implement the LLVM JSON parser, Git diff reader, region-to-diff intersection logic, console reporting, Markdown summary reporting, and threshold evaluation for changed-region coverage. Completed: initial end-to-end vertical slice with LLVM JSON parsing, unified diff parsing, changed-region metrics, gate evaluation, console/Markdown renderers, explicit pass/fail fixture coverage for both diff-file and branch-versus-main paths, and repository-relative normalization for absolute LLVM JSON file paths. Remaining: tighten renderer/report details and broaden parser-oriented fixture variety beyond the current small Rust scenarios.
- [ ] Add unit tests, fixture tests, and copied-fixture CLI integration tests using temporary working directories and immutable checked-in fixtures. Completed: focused unit tests plus copied-fixture Rust CLI integration tests covering diff-file failure, diff-file success, Markdown emission, PR-branch-against-main behavior, config-provided `base` and gate defaults, and CLI-over-config threshold precedence. Remaining: expand parser-oriented fixture variety and wire fuller follow-up setup for Dotnet and Vitest.
- [ ] Capture final validation evidence and move this plan to `docs/exec-plans/completed/` when the first usable `covgate` release exists.

## Surprises & Discoveries

- Observation: None yet.
  Evidence: This plan is being created before implementation work begins.

- Observation: For v1, the critical Git behavior is matching the changed-line output users already inspect with `git diff`, not broad repository API coverage.
  Evidence: The current plan only needs repository-relative changed files and added or modified line ranges to drive region intersection and reporting.

- Observation: A thin vertical slice is practical if the first CLI integration test uses a copied miniature Rust repository, an overlay file to create the diff, and a checked-in LLVM JSON report rather than full fixture-side coverage generation.
  Evidence: The current implementation already compiles, passes unit tests, and passes one copied-fixture Rust CLI integration test with that pattern.

- Observation: Metric-specific threshold flags are clearer for this product than a generic `METRIC=PERCENT` value because the likely long-term metric set is small and stable, but the current gate path still evaluates only one metric per run.
  Evidence: `cargo test` now covers pluralized CLI flags and TOML keys, CLI-over-config precedence, and rejects multiple effective thresholds in `src/config.rs`.

- Observation: The copied-fixture test harness is sufficient to model both direct diff-file inputs and realistic branch-versus-main Git-base scenarios without a second fixture convention.
  Evidence: `cargo test` now passes `basic_pass_rust_fixture`, `markdown_summary_rust_fixture`, and `pr_branch_against_main_fixture` in `tests/cli.rs` using the same helper flow of copied worktree plus nested Git repository setup.

- Observation: The first CI dogfooding attempt exposed a real parser fidelity gap that fixture-only JSON had not covered: real `cargo llvm-cov --json` output can use absolute file paths, which must be normalized to repository-relative paths before diff intersection.
  Evidence: The initial dogfood run reported `Changed regions: 0` on a branch with significant changes. After adding absolute-path normalization plus a dedicated parser test, the Rust validation stack remains green and the code now has explicit coverage for that path case.

- Observation: A parser-only regression test is not enough for this path-handling bug; the failure mode needs a CLI-level assertion because the user-visible symptom is a false zero-change pass after diff matching.
  Evidence: `cargo test` and `cargo llvm-cov --summary-only` now both pass `absolute_llvm_paths_match_diff_fixture` in `tests/cli.rs`, which rewrites the copied fixture coverage file to use the temporary worktree’s absolute `src/lib.rs` path and asserts that `Changed regions: 2` appears instead of `Changed regions: 0`.

- Observation: Deleted-file parsing was brittle until the parser tracked `diff --git` headers explicitly instead of relying only on `+++ b/...` lines to seed file state.
  Evidence: A new failing test first reproduced `encountered hunk before file header` for a diff that began with a deleted file using `+++ /dev/null`. After the parser was updated to track `diff --git` headers and skip deleted-file hunks, the reproducer passes in both `cargo test diff` and the full validation stack.

## Decision Log

- Decision: The first implementation will gate only changed-region coverage from LLVM JSON, even though the architecture must anticipate line, branch, combined, and future metric types.
  Rationale: Region coverage is the strongest standalone metric currently available from the Rust and LLVM coverage stack for this use case. Narrowing the initial behavior keeps the tool shippable while still forcing a metric model that will not collapse when additional metric types are introduced later.
  Date/Author: 2026-03-07 / Codex

- Decision: No core implementation surface in v1 may hard-code region-specific assumptions into names, data models, renderer contracts, or threshold evaluation flow.
  Rationale: The next two planned follow-up milestones are expected to add Coverlet and Istanbul inputs, which will shift attention toward line and branch coverage. If core types, interfaces, or renderers are written as though "coverage" always means "region coverage," those follow-ups will turn into rewrites instead of additive work.
  Date/Author: 2026-03-11 / Codex

- Decision: The first implementation will parse native LLVM JSON coverage export rather than Cobertura XML or LCOV.
  Rationale: LLVM JSON preserves the coverage model needed for region-aware gating. Cobertura and LCOV flatten coverage down to line and optional branch records, which would discard the main signal this tool is meant to enforce.
  Date/Author: 2026-03-07 / Codex

- Decision: `covgate` will support both console output and Markdown summary output in v1.
  Rationale: Console output is needed for local use and CI logs, while Markdown output is the lowest-friction way to produce readable pull-request-adjacent reporting through `$GITHUB_STEP_SUMMARY` without committing to a broader GitHub API integration surface.
  Date/Author: 2026-03-07 / Codex

- Decision: A single `covgate` invocation should be able to write Markdown output and plain-text console output at the same time while still using its process exit code as the gate result.
  Rationale: GitHub Actions steps should not need wrapper shell logic just to preserve exit status after copying a generated report into `$GITHUB_STEP_SUMMARY`. Letting one process handle stdout, optional Markdown emission, and exit status keeps CI configuration simpler and makes local behavior match CI behavior.
  Date/Author: 2026-03-10 / Codex

- Decision: The GitHub Markdown summary in v1 should show both the diff-coverage result and an informational overall coverage summary when the parsed LLVM JSON already provides the needed totals, but only diff coverage may influence the exit code or threshold evaluation.
  Rationale: Reviewers benefit from seeing whether a pull request passed the changed-code gate and whether the repository-wide coverage trend is broadly healthy. Keeping the overall metric informational preserves the product’s narrow scope and avoids reintroducing a repository-wide gate through the back door.
  Date/Author: 2026-03-10 / Codex

- Decision: The Markdown report should present both diff coverage and overall coverage in GitHub-flavored Markdown tables rather than bullet lists.
  Rationale: Tables make it easier to scan multiple files and compare covered versus missed counts in `$GITHUB_STEP_SUMMARY`. They also scale better once the report includes both diff-focused and repository-wide context.
  Date/Author: 2026-03-11 / Codex

- Decision: The architecture will separate coverage parsing, diff parsing, metric computation, gating, and output rendering into distinct modules.
  Rationale: Future support for other metrics and report formats should be additive. A monolithic implementation would make later support for line, branch, combined, Cobertura, or other native JSON formats much harder to add safely.
  Date/Author: 2026-03-07 / Codex

- Decision: The testing strategy will combine focused unit tests with copied-fixture CLI integration tests.
  Rationale: The most important failure modes are end-to-end: coverage file parsing, diff selection, threshold evaluation, and output rendering. Copied fixtures in temporary working directories keep those tests realistic without mutating checked-in source fixtures.
  Date/Author: 2026-03-07 / Codex

- Decision: Fixture repositories should use a split architecture with checked-in baseline files plus per-test injected change files copied into a temporary working tree before coverage collection and removed with the temporary directory afterward.
  Rationale: This gives the tests two things at once: stable committed files that stay out of the diff and deterministic changed files that always appear in the diff. It also keeps fixture repositories reviewable in Git while avoiding repeated manual mutation of checked-in fixture trees.
  Date/Author: 2026-03-11 / Codex

- Decision: The fixture layout should reserve parallel language slots for Rust, Dotnet, and Vitest even though v1 will only execute the Rust fixtures.
  Rationale: The next expected follow-up plans target Coverlet and Istanbul. Establishing one shared fixture architecture now avoids inventing a second testing convention as soon as those parsers land.
  Date/Author: 2026-03-11 / Codex

- Decision: V1 should keep diff collection behind a subprocess call to the installed `git` CLI rather than adopting `git2-rs` or `gitoxide` (`gix`) immediately.
  Rationale: The first implementation needs one narrow Git capability: stable zero-context diff text whose semantics match ordinary `git diff` in developer machines and CI. Shelling out keeps the implementation small and aligned with user expectations. `git2-rs` would add a `libgit2` dependency and can differ from the Git CLI behavior people compare against. `gix` is the more attractive Rust-native fallback because it is pure Rust, but it would still expand the API surface and implementation scope before `covgate` has validated its core coverage model.
  Date/Author: 2026-03-10 / Codex

- Decision: The primary CI use case in v1 is comparing the current pull-request branch against the repository’s mainline branch, usually exposed as a base such as `origin/main`.
  Rationale: That is the dominant way teams will consume `covgate` in GitHub Actions and similar CI systems. The implementation and tests therefore need to treat "PR branch versus main" as the default Git-base scenario, even while still allowing explicit diff-file input for controlled tests and edge cases.
  Date/Author: 2026-03-11 / Codex

- Decision: V1 should support a repository-local TOML configuration file whose values provide defaults for selected CLI options, including `--base` and `--fail-under-<METRIC>`, with explicit command-line flags taking precedence.
  Rationale: Teams using `covgate` in CI and local development will usually want one checked-in default base reference and one or more default gate thresholds. Keeping those defaults in TOML avoids repeating them in every invocation while preserving the normal CLI expectation that explicit flags override configuration-file values.
  Date/Author: 2026-03-11 / Codex

- Decision: V1 will discover repository-local defaults from `./covgate.toml`, interpreted relative to the current working directory from which `covgate` runs.
  Rationale: The current CLI already expects to run at the repository root so Git diff collection and repository-relative paths behave naturally. Using one conventional filename in that same directory keeps discovery simple for CI, local development, and copied-fixture tests without adding another required flag before the config surface is proven out.
  Date/Author: 2026-03-11 / Codex

- Decision: The fail-under CLI should use pluralized metric-specific flags such as `--fail-under-regions`, `--fail-under-lines`, and `--fail-under-branches` instead of a generic `--fail-under METRIC=PERCENT` form.
  Rationale: The expected metric set is small and mostly stable, and most non-LLVM ecosystems will primarily care about line and branch thresholds. Metric-specific flags are easier to read in `--help`, CI configuration, and examples, and they line up cleanly with TOML keys like `[gates].regions`.
  Date/Author: 2026-03-11 / Codex

- Decision: `--fail-uncovered-*` thresholds are deferred until a later milestone.
  Rationale: The current gate model only represents percentage-based thresholds. Adding uncovered-count gates cleanly requires a second threshold type, updated result reporting, and decisions about how percent and count gates compose in one run. That deserves a dedicated follow-up instead of being folded into the flag-shape refactor.
  Date/Author: 2026-03-11 / Codex

## Outcomes & Retrospective

This plan exists before implementation, so the current outcome is a scoped specification rather than working behavior. The main design result so far is clarity about what belongs in v1 and what does not. V1 must do one thing well: fail a diff-based LLVM coverage gate and explain the result clearly in CI and local output. In practice that first gate is changed-region coverage, but the architecture must stay ready for imminent follow-up work around Coverlet and Istanbul, where line and branch metrics matter more. Future-proofing matters, but only at the architecture layer, not as extra parser or metric work in the first delivery.

The first implementation slice now exists and works. The crate has a library-first structure, a functioning CLI/config path, repository-local TOML default loading from `covgate.toml`, a basic LLVM JSON parser, unified diff parsing, changed-region metric computation, threshold evaluation, console and Markdown renderers, focused unit tests, and copied-fixture Rust CLI integration tests for pass and fail diff-file scenarios, Markdown summary emission, and branch-versus-main Git-base scenarios. The largest remaining gaps are breadth and robustness rather than total absence: more parser cases and broader future-language fixture coverage.

The main risk to watch during implementation is over-generalizing too early. If the code tries to fully solve cross-language coverage normalization in the first pass, the tool will likely become slow to build and hard to validate. The intended balance is a narrow first parser and metric with a clean internal model that makes later formats and metrics additions straightforward.

The same constraint applies to Git integration. Removing the subprocess boundary may become worth doing later, but it is not where the first user-visible value comes from. The plan should therefore optimize for matching familiar `git diff` behavior while keeping the diff reader isolated enough that a later migration to `gix` remains additive.

## Context and Orientation

`covgate` is a small standalone Rust CLI in its own repository, with its own `Cargo.toml`, `src/`, `tests/`, fixture directories, and workflow files. The problem it solves is a narrow but common one: many teams want a pull-request coverage gate that only judges the code changed in the diff, not the entire repository, and they want that gate to use a stronger metric than line coverage where the language and tooling support it.

The test-fixture strategy matters because this tool depends on the interaction between three separate systems: source code, Git diff state, and external coverage generators. A useful fixture is therefore not just a JSON file. It is a miniature repository whose baseline files are committed, whose changed files can be injected deterministically during a test run, and whose coverage report can be generated by the language-native toolchain. In v1 that means a miniature Rust project whose coverage comes from `cargo llvm-cov`. The same fixture architecture should leave room for a miniature Dotnet project using Coverlet and a miniature Vite or Vitest project using Istanbul-style coverage in the next follow-up plans.

In this plan, a "coverage opportunity" means a measurable unit of executable code that may be covered or uncovered. In v1, the relevant opportunity type is a coverage region exported by LLVM, but the term is intentionally broader because upcoming follow-up work is expected to add line-oriented and branch-oriented opportunities from Coverlet and Istanbul. A "changed opportunity" means a coverage opportunity whose source span overlaps the lines changed in the pull request diff. A "metric" means a specific coverage ratio computed over a set of opportunities, such as line coverage, branch coverage, region coverage, or a combined metric. A "format parser" means code that reads one coverage report format and converts it into the internal coverage model used by the rest of the tool. A "gate" means the pass or fail decision produced by comparing a computed metric against a configured threshold.

The expected external inputs are:

- a coverage report generated from the repository under test, initially LLVM JSON from `cargo llvm-cov --json --output-path coverage.json`
- a Git diff range, usually expressed as a base reference such as `origin/main` for the current pull-request branch, a base and head pair, or a precomputed unified diff file; in v1, the default base reference may come from a repository-local TOML configuration file and still be overridden by `--base`
- one or more threshold settings, initially only a changed-region threshold at the user-facing layer even though the internal threshold model must remain metric-agnostic; in v1, default threshold values may come from a repository-local TOML configuration file and still be overridden by metric-specific flags such as `--fail-under-regions`
- optional output configuration controlling Markdown summary emission

The user-facing configuration surface in v1 should have two layers. The first layer is explicit CLI arguments passed to `covgate` for one-off runs and CI overrides. The second layer is a repository-local TOML configuration file checked into the repository under test. That file should be able to declare default values for the same concepts the CLI already exposes, including the default Git base reference and the default fail-under threshold for each supported metric family. The CLI and TOML naming should stay aligned, so a flag such as `--fail-under-regions` corresponds naturally to `[gates].regions`. The precedence rule must be simple and documented in help text, tests, and examples: explicit CLI arguments win, TOML configuration supplies defaults when the CLI omits a value, and missing values that are required after that merge should still produce actionable configuration errors.

The core architectural challenge is not parsing one format. It is preserving enough semantic structure that the code can later support other formats and metrics without breaking the v1 gate or forcing a redesign when Coverlet and Istanbul support arrive. That means the internal data model must distinguish between:

1. raw parser-specific coverage records
2. normalized source spans and coverage opportunities
3. changed-span selection from Git diff
4. metric computation
5. threshold evaluation
6. output rendering

The plan assumes Git is available in the execution environment for local runs and CI, and that repositories using the tool already know how to generate LLVM JSON coverage before invoking `covgate`. That assumption is deliberate for v1: the diff reader will rely on the installed Git CLI instead of embedding a Git implementation immediately.

The test harness should mirror that assumption. Integration tests should create a temporary working directory, copy a checked-in fixture repository into it, initialize a nested Git repository there, perform the Git operations needed for the scenario, copy in one or more preauthored changed files from a separate overlay area when the scenario requires working-tree changes, generate coverage using the fixture language toolchain, run `covgate`, assert on output and exit code, and then rely on temporary-directory cleanup to remove the nested repository and injected files. Checked-in baseline fixture files should remain committed and unchanged so they do not appear in the test diff unless a specific scenario requires it.

## Plan of Work

Start by creating a new binary crate for `covgate` with a library-first layout. Keep the main binary entrypoint thin so it only parses arguments, loads repository-local TOML configuration when present, resolves CLI-over-config precedence, calls a library function, prints the selected output, and exits with a status code derived from the gate result. Put the real implementation behind the library crate and split the code into a small set of modules whose responsibilities stay stable even as new metrics and formats are added later. At minimum, the codebase needs clear boundaries for CLI parsing, configuration loading and merge rules, diff loading, coverage parsing, normalized internal data, metric and gate evaluation, and output rendering. Exact filenames do not matter yet as long as those responsibilities remain separated.

Define the fixture-repository layout early because it affects parser tests, diff tests, and CLI integration tests. A good starting shape is a `tests/fixtures/` tree with one directory per language family and one directory per scenario underneath it. Each scenario should contain at least a committed repository skeleton, an overlay directory containing files that should be copied into the working tree to create the diff for that scenario when needed, and expected-output snapshots or assertions. For example, the Rust v1 scenarios might live under paths such as `tests/fixtures/rust/basic-pass/repo/` and `tests/fixtures/rust/basic-pass/overlay/`. Reserve sibling top-level directories such as `tests/fixtures/dotnet/` and `tests/fixtures/vitest/` now even if they only contain README-style placeholders in v1.

The scenario runner should be explicit about its Git setup steps. For each copied fixture worktree, the test harness should:

1. initialize a nested Git repository inside the copied temporary directory
2. commit the checked-in baseline fixture state
3. create whatever branch or base-reference arrangement the scenario requires
4. apply overlay files or additional commits as needed to model the desired diff-base case
5. run either `covgate --base <mainline-ref>` for branch-versus-main scenarios or `covgate --diff-file <path>` for direct patch scenarios

That structure makes it possible to cover both the main CI case, "current PR branch versus main," and lower-level parser scenarios without inventing separate fixture conventions.

Define the internal model first. The model must not hard-code region coverage as the only possible metric even though v1 only computes changed-region coverage. Avoid names, enums, structs, helper functions, and renderer contracts that imply the active metric is always region-based. It should have a way to distinguish metric families such as region, line, branch, and combined coverage, and it should represent thresholds independently from CLI parsing so later expansion remains additive. It should also define a parser-neutral source span representation that can at least hold a repository-relative file path plus start and end line information, because diff intersection and report rendering both need that information even when future formats provide richer column-level data.

For coverage opportunities, define an internal shape that can support multiple opportunity kinds later. In v1, it may only materialize region opportunities, but the model should still preserve the distinction between the kind of thing being measured, its source span, and whether it was covered. Do not bake region-only field names into shared types if those types will also need to represent line or branch opportunities in the next follow-up plans.

The LLVM JSON parser module should do two things only: parse the JSON safely into Rust types and convert the relevant file-level region records into the normalized model used by the metrics layer. Do not let parser-specific JSON details leak into the metric, gate, or rendering modules. If the JSON contains more detail than v1 needs, preserve only what is necessary for changed-region computation and useful failure reporting. The parser should normalize file paths consistently and reject malformed or incomplete reports with actionable error messages. Parser-specific region details must stay inside the LLVM adapter rather than shaping the shared model around LLVM terminology.

For fixture generation in v1, prefer producing real LLVM JSON from the miniature Rust repositories rather than hand-authoring large synthetic reports when a scenario can be expressed naturally with code. Small malformed JSON inputs may still be checked in directly for parser-failure tests, but behavioral integration tests should lean on real coverage tool output so the path from source change to coverage report stays realistic.

Implement Git diff handling separately from coverage parsing. In v1, the diff module should shell out to the installed Git CLI with a fixed command equivalent to `git diff --unified=0 --no-ext-diff <base>...HEAD` when the user supplies a base reference, or read a unified diff file directly when the user supplies a diff path. The base-reference path must be treated as the primary CI path, especially for branch-versus-main comparisons such as `origin/main...HEAD`. Normalize either source into repository-relative changed-file entries with changed line ranges. Keep the subprocess wrapper narrow and deterministic so optional user customizations such as external diff tools do not affect parsing. The intersection algorithm should then compare changed line ranges against normalized region spans and decide which coverage regions count toward the changed-region metric. Make the intersection rule explicit in code and tests: a region counts as changed when its source span overlaps at least one added or modified line in the diff. Deleted-only lines should not create coverage obligations because there is no remaining executable code to measure.

With parsing and diff selection in place, implement the metric layer. The first required metric is changed-region coverage:

    changed_region_coverage = covered_changed_regions / total_changed_regions

The metric layer should return both the aggregate ratio and the underlying uncovered changed opportunities so the renderers can explain failures. Even though combined, line, and branch gates are out of scope for implementation, the metric layer must already be structured so future metric calculators can be added without changing the diff or parser modules.

Implement threshold evaluation in a dedicated gate module as a separate step after metric computation. The gate should accept one or more threshold definitions even though v1 only supports a region threshold in the first metric implementation. That keeps the code ready for later additions without making the current user-facing interface confusing. The gate result should capture pass or fail, the active metric family, the computed percentage, the configured threshold, and the uncovered changed opportunities that caused failure. Configuration resolution should happen before this step so the gate module receives fully merged effective thresholds regardless of whether they came from TOML defaults or explicit CLI flags.

Renderers should consume the gate result, not raw parser output. The console renderer should produce a concise but informative summary suitable for local development and CI logs. It should follow a compact report shape: short header, diff description, per-file lines, then final totals. The pass or fail state, computed percentage, and threshold must be obvious without reading the whole output. Per-file lines should stay compact and should list uncovered changed spans only when coverage is incomplete. The Markdown renderer should produce a GitHub-friendly summary with headings and compact tables that remain readable in GitHub’s summary UI. Keep Markdown output deterministic and plain; this is a machine-written CI artifact, not a rich report site. When Markdown output is requested, `covgate` should still print its normal console summary to standard output and then exit according to the gate result; Markdown emission must be an additional side effect, not a mode switch that suppresses the text summary or changes exit-code behavior.

The Markdown summary should have a small "Diff Coverage" section that drives the gate and, when available from the parsed coverage report without extra tooling, a separate "Overall Coverage" section that reports repository-wide totals for the same metric family. Label the overall section as informational so readers do not confuse it with the gating decision. The plain-text console output may stay focused on the diff result in v1 to keep local logs concise.

For v1, both sections should be table-based. The diff table should be grouped by changed file and should include enough columns to explain the gate result at a glance, such as file path, covered changed opportunities, total changed opportunities, coverage percent, and a compact missed-span summary. The overall table should summarize repository-wide coverage for the active metric family using file-level rows plus a repository total row. Because v1 only computes a region gate, the first implementation will fill these tables with region counts, but the shared renderer inputs and column labels should stay generic enough that line- or branch-based rows can replace them later without a redesign.

The CLI contract should be explicit and stable in v1. A novice should be able to run `covgate --help` and understand:

- where to point the tool at LLVM JSON coverage data
- how to specify the Git base reference or diff file
- how repository-local TOML configuration supplies defaults and how CLI flags override those defaults
- how to set a changed-region threshold from the CLI with `--fail-under-regions` and how a default threshold may come from `[gates].regions` in TOML configuration
- how to write Markdown output to a file or directly to `$GITHUB_STEP_SUMMARY`
- that Markdown output does not replace the standard-output summary
- that overall coverage shown in Markdown is informational and does not affect the exit code in v1
- what exit code behavior to expect on pass, fail, and configuration or parse errors

Do not overreach into repository-specific coverage generation in v1. `covgate` should assume the coverage JSON already exists and should not try to invoke `cargo llvm-cov` internally. That keeps the tool focused on gating rather than test orchestration.

## Concrete Steps

Run the following commands from the `covgate` repository root.

1. Define the internal types and module boundaries before implementing parsers.

    Working directory: the `covgate` crate root

    Example commands:

        cargo test model
        cargo test metrics

    Expected outcome: Unit tests prove that source spans, metric families, thresholds, and gate-facing result data can represent future line, branch, region, and combined metrics even though only region metrics are actually computed in v1. Tests should explicitly guard against region-specific assumptions leaking into shared types.

2. Implement LLVM JSON parsing and normalized coverage opportunities.

    Working directory: the `covgate` crate root

    Example commands:

        cargo test llvm_json
        cargo test llvm_json -- malformed_fixture

    Expected outcome: Checked-in parser fixtures parse successfully into normalized opportunities, malformed fixtures fail clearly, and parser details do not leak outside the coverage module. Integration-oriented fixture scenarios should prefer LLVM JSON produced from miniature Rust repositories rather than large hand-maintained synthetic reports.

3. Implement Git diff parsing and changed-line normalization.

    Working directory: the `covgate` crate root

    Example commands:

        cargo test diff
        cargo test intersection
        cargo test cli -- pr_branch_against_main_fixture

    Expected outcome: Tests prove that changed lines are selected correctly from fixture diffs or temporary Git repositories, deleted-only hunks are ignored for gating, branch-versus-main Git-base scenarios work as expected, and changed lines intersect correctly with region spans.

4. Implement changed-region metric computation, threshold evaluation, and rendering.

    Working directory: the `covgate` crate root

    Example commands:

        cargo test gate
        cargo test render
        cargo test cli -- basic_pass_rust_fixture
        cargo test cli -- markdown_summary_rust_fixture

    Expected outcome: The CLI prints a readable console report to standard output on every run, writes Markdown output when requested, and exits with success or failure according to the effective changed-region threshold after merging TOML defaults with CLI overrides. When the parsed coverage report includes repository-wide totals, the Markdown output should include both the diff result and an informational overall summary, each rendered as a table. Writing Markdown must not require a separate shell step to preserve the exit code.

5. Add end-to-end CLI integration tests using copied fixtures.

    Working directory: the `covgate` crate root

    Example commands:

        cargo test cli
        cargo test --quiet

    Expected outcome: Integration tests copy fixture repositories into temporary working directories, inject scenario-specific overlay files to create deterministic diffs, run the compiled CLI against fixture coverage JSON and Git histories, assert exit codes and outputs, confirm that the checked-in baseline fixtures remain unchanged, and verify that diff collection is not affected by external diff customization.

6. Add CI workflow coverage for the tool itself.

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

Running `covgate` in a repository that contains the supported TOML configuration file should load default values for the Git base reference and fail-under thresholds when the CLI omits them. At minimum, tests must prove that `covgate` uses the configured default base reference when `--base` is absent, uses the configured default threshold when `--fail-under-regions` is absent, and prefers the explicit CLI flag value when both sources specify the same setting.

The LLVM JSON parser must be validated with checked-in fixtures representing at least:

- a clean small report with covered and uncovered regions
- a report containing multiple files
- malformed or incomplete JSON that should fail clearly
- file paths that need normalization relative to the repository root

The fixture architecture itself must be validated. At minimum, tests must prove that:

- committed baseline files remain outside the Git diff when a scenario only injects overlay files
- copied overlay files reliably appear in the Git diff for the scenario under test
- deleting the temporary working directory removes the injected files and leaves checked-in fixtures untouched
- the same fixture scenario can be rerun without manual cleanup
- initializing nested Git repositories inside copied fixture directories is sufficient to model the intended base-reference scenarios

The diff parser must be validated with checked-in fixtures or temporary Git repositories representing at least:

- a file with added lines that overlap uncovered regions
- a file with changed lines that overlap covered regions
- deleted-only hunks that must not count toward changed coverage
- multiple files in one diff
- a file present in the coverage report but absent from the diff, which must not count toward changed coverage
- a pull-request-like branch-versus-main scenario where `--base origin/main` or an equivalent local mainline ref is the diff source

CLI integration tests must use copied fixtures in temporary working directories. The checked-in fixtures should include miniature repository skeletons plus reproducible fixture data that the tests copy before invoking the compiled `covgate` binary. For v1, those skeletons should be real Rust projects capable of producing LLVM JSON through `cargo llvm-cov`. The fixture tree should also reserve directories for future Dotnet and Vitest scenarios so the next follow-up plans can add Coverlet and Istanbul-based coverage generation without changing the test harness layout. Acceptance is not complete until those integration tests assert:

- success exit code when changed-region coverage meets the threshold
- failure exit code when changed-region coverage falls below the threshold
- success exit code when the effective threshold comes only from TOML configuration and the diff coverage meets that default
- failure exit code when a CLI `--fail-under-regions` override is stricter than the TOML default and the computed coverage no longer passes
- console output includes the metric name, computed percentage, threshold, and uncovered changed regions
- console output or error output clearly identifies effective configuration problems after merging TOML values with CLI inputs
- Markdown output can be written to a file that matches expected content closely enough to detect regressions
- a single invocation can both write Markdown output and emit the standard-output summary without changing exit-code behavior
- Markdown output includes an informational overall coverage summary when the input report already provides repository-wide totals
- overall coverage shown in Markdown is clearly labeled informational and does not change the gate result
- Markdown uses GitHub-flavored tables for both diff coverage and overall coverage summaries
- overlay-based changed files appear in the diff while committed baseline files do not
- copied fixture directories can be turned into nested Git repositories and manipulated to model different diff-base cases
- rerunning the CLI on the same fixture is idempotent
- subprocess-based diff collection remains deterministic even when Git configuration would normally enable an external diff tool

The architecture must be demonstrably future-proof in code shape even though future metrics and formats are out of scope for implementation. Acceptance is not complete until the code has a stable place for:

- additional metric kinds such as line, branch, and combined
- additional opportunity kinds beyond regions
- additional coverage parsers such as Cobertura XML or other native JSON formats

This does not require implementing those formats or metrics, but it does require proving via tests or type-level structure that adding them later will be additive rather than requiring a redesign of the core model.

Acceptance is not complete if shared types, renderer inputs, or gate logic are named or shaped so specifically around regions that the expected Coverlet and Istanbul follow-up plans would need to replace them rather than extend them.

The console report should be concise enough for local use. It should resemble a `diff-cover`-style summary, but with wording derived from the active metric instead of hard-coded line terminology. For the v1 LLVM path, that means region-oriented wording. A passing run should show a short header, the diff description, compact per-file rows, and final totals with threshold. A failing run should additionally show uncovered changed spans inline on the affected file rows. The Markdown report should be suitable for direct use in `$GITHUB_STEP_SUMMARY` without additional processing. In Markdown, the diff result should appear first as a table, and any overall coverage numbers should appear afterward in a clearly informational table.

A GitHub Actions step should be able to look like the example below, with no extra shell logic to preserve status after writing the summary file:

    covgate --coverage-json coverage.json --base origin/main --fail-under-regions 90 --markdown-output "$GITHUB_STEP_SUMMARY"

A repository that prefers checked-in defaults should also be able to use a workflow step like the example below, where the TOML configuration file supplies the default base reference and diff gate threshold:

    covgate --coverage-json coverage.json --markdown-output "$GITHUB_STEP_SUMMARY"

## Idempotence and Recovery

The implementation steps in this plan are additive and should be safe to repeat. The CLI itself is read-only with respect to source code under test. Re-running `covgate` on the same coverage JSON and the same diff should produce the same result and should not modify repository files unless the user explicitly chooses a Markdown output path that overwrites an existing file.

The fixture harness should also be repeatable. A safe rerun means recreating the temporary working directory from the checked-in repository skeleton, reapplying overlay files, regenerating coverage, and rerunning assertions. Do not mutate the checked-in fixture skeleton in place as part of a test.

If a parser or diff-handling step is only partially implemented and tests fail midway through development, the safe recovery path is to keep the incomplete logic behind non-exported functions or feature-complete module boundaries until the associated tests pass. Avoid partially wiring unfinished parser details into the main CLI path, because that will make end-to-end failures harder to diagnose.

If repository paths or build commands change during implementation, update this plan immediately so a novice can still execute it from top to bottom using only the current repository state.

## Artifacts and Notes

Record concise evidence here as implementation proceeds. Replace the placeholders below with real transcripts and examples.

Expected passing console excerpt:

    -------------
    Diff Coverage: PASS
    Diff: origin/main...HEAD
    Metric: region
    -------------
    src/gate.rs (100.00%)
    src/metrics.rs (90.91%)
    -------------
    Changed regions: 13
    Covered regions: 12
    Coverage: 92.31%
    Threshold: 90.00%
    -------------

    This example demonstrates the v1 active metric. Shared renderer inputs should not require these exact region-specific labels.

Expected failing console excerpt:

    -------------
    Diff Coverage: FAIL
    Diff: origin/main...HEAD
    Metric: region
    -------------
    src/metrics.rs (66.67%): uncovered changed spans 41-47, 73-79
    src/gate.rs (100.00%)
    -------------
    Changed regions: 6
    Covered regions: 4
    Coverage: 66.67%
    Threshold: 90.00%
    -------------

    This example demonstrates the v1 active metric. Shared renderer inputs should not require these exact region-specific labels.

Expected Markdown summary excerpt:

    ## Covgate

    ### Diff Coverage

    | Result | Metric | Changed Coverage | Threshold |
    | --- | --- | ---: | ---: |
    | FAIL | region | 66.67% | 90.00% |

    | File | Covered Changed Regions | Total Changed Regions | Cover | Missed Changed Spans |
    | --- | ---: | ---: | ---: | --- |
    | `src/metrics.rs` | 4 | 6 | 66.67% | `41-47`, `73-79` |

    ### Overall Coverage

    Informational only. Does not affect the gate result in v1.

    | File | Covered Regions | Total Regions | Cover |
    | --- | ---: | ---: | ---: |
    | `src/metrics.rs` | 84 | 100 | 84.00% |
    | `TOTAL` | 421 | 500 | 84.20% |

    These tables demonstrate v1 region output. Shared renderer inputs and internal result types should stay generic enough that future line- or branch-based tables can be rendered without redesign.

## Interfaces and Dependencies

Use Rust stable unless implementation evidence proves a nightly-only feature is required. Prefer small, well-scoped dependencies.

The CLI should use `clap` for argument parsing. Error handling may use `anyhow` at the top-level command boundary, but internal modules should prefer typed structures where that improves clarity. Git interaction in v1 should use a narrow subprocess wrapper around the installed `git` CLI rather than `git2-rs` or `gix`. The diff module must hide that choice behind a stable internal interface so a later migration to `gix` stays isolated to one part of the codebase. Do not add `git2-rs` in v1 because it introduces a native `libgit2` dependency and can diverge from the exact CLI behavior users compare against. Do not add `gix` in v1 either; if a later milestone removes the subprocess dependency, prefer evaluating `gix` first because it is pure Rust.

For integration fixtures, rely on the real language toolchains rather than mock coverage generators whenever practical. In v1 that means `cargo llvm-cov` for Rust fixture projects. Plan for follow-up environment setup that makes `dotnet` plus Coverlet-compatible coverage collection and `node` plus Vite or Vitest Istanbul coverage collection available to the test harness, but do not block the first milestone on those ecosystems being wired up.

In the coverage layer, define one parser entrypoint or trait boundary that can support multiple formats later without leaking LLVM JSON details into metrics, gating, or rendering. In the model layer, define stable concepts for source spans, coverage opportunities, metric families, thresholds, and gate results, but do not treat any exact Rust type layout in this plan as fixed unless implementation pressure proves it necessary.

The renderer layer should consume normalized gate-oriented data plus lightweight metadata rather than parser-specific coverage records. That boundary is important because future report outputs, such as SARIF, GitHub Checks API payloads, or richer Markdown summaries, should not require any changes to the LLVM parser or diff logic.

At the bottom of this plan, append a revision note every time the plan changes materially, describing what changed and why.

Revision note: Initial standalone plan created for `covgate`, scoped to LLVM JSON plus changed-region gating in v1 while requiring a code architecture that can later absorb additional metrics, thresholds, and report formats.

Revision note: Evaluated replacing the `git` subprocess with `git2-rs` or `gitoxide` (`gix`) and decided to keep shell-based diff collection for v1. The plan now records the rationale, names `gix` as the preferred future Rust-native option if the subprocess boundary is removed later, and adds acceptance criteria for deterministic subprocess behavior.

Revision note: Clarified that one `covgate` invocation must support all three CI behaviors at once: writing Markdown to `$GITHUB_STEP_SUMMARY`, printing a plain-text summary to standard output, and returning the gate exit code directly. This avoids requiring wrapper shell scripts to capture status after report generation.

Revision note: Expanded the Markdown reporting scope so GitHub summaries can show both diff coverage and overall coverage context. The plan now requires the overall summary to remain informational-only in v1 so the tool stays a diff gate rather than becoming a repository-wide gate.

Revision note: Trimmed early implementation detail from the plan so it prescribes behavior, boundaries, and invariants without locking in exact file layouts, trait signatures, or struct definitions before implementation begins.

Revision note: Switched the planned GitHub summary format from bullets to GitHub-flavored tables for both diff coverage and overall coverage. The plan now requires table columns that match the actual v1 metric scope instead of implying unsupported line, branch, or function summaries.

Revision note: Refined the planned console output to follow a `diff-cover`-like structure with a header, diff description, per-file rows, and a totals block. The wording now stays aligned with changed-region coverage instead of line coverage.

Revision note: Strengthened the anti-hardcoding guidance around region coverage. The plan now explicitly says shared models, gate flow, and renderer contracts must remain metric-agnostic because the next expected follow-up plans target Coverlet and Istanbul inputs with line and branch coverage.

Revision note: Added a concrete fixture-repository architecture. The plan now calls for checked-in baseline repository skeletons plus copied overlay files in temporary working directories, starts with real Rust coverage generation, and reserves matching fixture slots for Dotnet and Vitest follow-up work.

Revision note: Removed stale crate-bootstrap steps, aligned concrete command examples with the copied-fixture test architecture, and clarified that the region-shaped console and Markdown excerpts are examples of the active v1 metric rather than required shared-interface labels.

Revision note: Clarified that the main CI diff path is the current PR branch against main and made the fixture harness spell out nested Git initialization plus scenario-specific Git operations in copied worktrees.

Revision note: Clarified that v1 includes repository-local TOML configuration and that TOML values may supply defaults for `--base` and `--fail-under-<METRIC>`. The plan now requires explicit CLI flags to override config-file defaults and adds acceptance coverage for that precedence rule.

Revision note: Implemented the first TOML configuration slice in the codebase. `covgate` now loads `./covgate.toml`, merges config defaults with CLI values, supports config-provided `base` and gate defaults, and tests that explicit CLI threshold values override the checked-in config.

Revision note: Switched the fail-under CLI from a generic `METRIC=PERCENT` value to pluralized metric-specific flags such as `--fail-under-regions`. The plan now aligns TOML gate keys with those flag names and explicitly defers `--fail-uncovered-*` gates to a later milestone.

Revision note: Renamed the repository-local TOML section from `[thresholds]` to `[gates]` so the configuration vocabulary matches the product more closely and leaves room for non-percent gate rules.

Revision note: Expanded copied-fixture Rust CLI coverage with explicit pass, Markdown-output, and branch-versus-main scenarios. The plan now records that the same temporary-worktree fixture harness covers both diff-file and Git-base execution paths.

Revision note: Added repository-relative normalization for absolute LLVM JSON file paths after the first CI dogfooding attempt exposed false zero-changed-region results. The plan now records that parser fidelity work includes real-path handling, not just synthetic fixture parsing.

Revision note: Added an end-to-end CLI regression test for absolute LLVM JSON paths so the CI dogfooding failure mode is covered above the parser layer as well as inside unit tests.

Revision note: Fixed deleted-file unified diff handling via TDD after review identified that `+++ /dev/null` could leave the parser without a valid current file. The plan now records that deleted-file hunks are ignored safely and no longer break the next file in the diff.
