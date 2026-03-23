# covgate

[![CI](https://github.com/jesse-black/covgate/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/jesse-black/covgate/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/covgate.svg)](https://crates.io/crates/covgate)
[![docs.rs](https://img.shields.io/docsrs/covgate)](https://docs.rs/covgate)
[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](./LICENSE)

A zero-dependency CI/CD quality gate that enforces code coverage strictly on pull request diffs—built for human developers and autonomous AI agents.

Built in Rust, `covgate` evaluates **line, branch, region, and function coverage** by parsing lossless compiler coverage reports. It operates entirely locally in your CI runner or agent workspace, preventing untested code from merging without the false positives or SaaS costs associated with other coverage tools.

Teams do not merge a repository; they merge a diff. `covgate` makes that diff the strict unit of enforcement.

## The Global Gate Flaw

Standard tools like `cargo llvm-cov`, `dotnet test`, and `vitest` establish global coverage baselines. However, relying solely on global gates in a continuous integration pipeline creates a blind spot: **The "Watering Down" Effect.**

If a 10,000-line codebase has 85% global coverage, an 80% CI gate easily passes. A developer can add 200 lines of complex, untested logic, which might only drop global coverage to 83.3%. 

The gate passes, the untested code merges, and overall coverage slowly bleeds down to the global threshold. High coverage on legacy code subsidizes untested new code.

## The Value of Diff Coverage Gates

A diff coverage tool isolates the exact lines modified or added in a Git branch and calculates coverage exclusively against that changeset. 

* **The Ratchet Effect:** An 80% diff coverage requirement enforces that *every* new PR meets the standard, guaranteeing your global coverage stays flat or increases over time.
* **Refactoring Safety:** Global gates penalize refactoring; deleting highly-tested obsolete code drops the global percentage. Diff coverage ignores the global denominator, allowing developers to clean up technical debt safely.
* **Actionable Feedback:** Failing a build over a 0.3% global coverage drop is abstract. Failing because "the 15 lines added in `auth.rs` lack test coverage" is immediate and localized.

## The "Lossless-First" Philosophy

Diff gating requires trustworthy coverage data.

**Why doesn’t `covgate` support Cobertura or LCOV?**

Legacy coverage formats like Cobertura and LCOV are inherently lossy. They flatten compiler models into line-oriented XML or text formats, discarding the exact information needed for precise PR gating. Gating on lossy formats blocks developers on untestable "ghost branches" or accidentally passes diffs with untouched logical paths. 

Native formats preserve structural signals. In LLVM’s case, this includes **region coverage**: continuous spans of executable logic that reflect actual compiler forks.

`covgate` takes a strict stance: **Parse ground-truth coverage data directly from the producer, then apply it strictly to the diff.**

## The Agent Feedback Loop

AI coding agents like Codex Cloud and Google Jules expose a flaw in existing coverage workflows: reliance on continuous integration pipelines for feedback.

Agents pushing changes and waiting for CI creates inefficiencies. Some platforms cannot automatically ingest PR results, requiring a human to manually copy CI failures back into the chat. Others can ingest the results but still waste compute minutes idling for the external pipeline to run. 

Furthermore, cloud agent sandboxes deliberately remove base branches like `main` from the checkout to ensure isolation, breaking standard local diff-coverage tools.

`covgate` solves this. By supporting a stable, per-worktree base commit, it empowers AI agents to execute a local gate check *inside* the task sandbox. This creates a tight feedback loop without ever pushing a commit or waiting on CI.

## Supported Ecosystems

`covgate` supports native coverage formats across several language ecosystems:

* **Rust (LLVM JSON):** Region-aware gating from `llvm-cov` / `cargo llvm-cov`.
* **JavaScript / TypeScript (Istanbul JSON):** Accurate line and branch gating from direct JSON output.
* **C# / .NET (Coverlet JSON):** Line and branch gating from Coverlet’s JSON.

## Installation

`covgate` is distributed as a standalone, statically linked binary. It runs instantly in your CI pipeline with zero runtime dependencies.

**Via Cargo:**

```bash
# Install covgate globally via cargo
cargo install covgate
```

## Usage

Run `covgate` in your CI pipeline after your tests generate coverage artifacts. Invoke it with either a Git base reference or a diff file.

### CLI Surface

`check <coverage-report>` runs coverage checks for the provided report.
Options:
- `--base <REF>` selects the Git base reference to diff against.
- `--diff-file <FILE>` uses a precomputed unified diff instead of Git base discovery.
- `--fail-under-regions <PERCENT>` fails if changed-region coverage is below this threshold.
- `--fail-under-lines <PERCENT>` fails if changed-line coverage is below this threshold.
- `--fail-under-branches <PERCENT>` fails if changed-branch coverage is below this threshold.
- `--fail-under-functions <PERCENT>` fails if changed-function coverage is below this threshold.
- `--fail-uncovered-regions <MAX>` fails if the raw count of uncovered regions exceeds this limit.
- `--fail-uncovered-lines <MAX>` fails if the raw count of uncovered lines exceeds this limit.
- `--fail-uncovered-branches <MAX>` fails if the raw count of uncovered branches exceeds this limit.
- `--fail-uncovered-functions <MAX>` fails if the raw count of uncovered functions exceeds this limit.
- `--markdown-output <FILE>` writes a Markdown summary for CI interfaces like GitHub Actions.

`record-base` captures a stable task-start base when normal branch refs are unavailable.

### Gating a Pull Request Locally

```bash
# Generate JSON coverage report
cargo llvm-cov --json --output-path coverage.json

# Run covgate against the origin/main branch, failing if region coverage is below 80%
covgate check coverage.json --base origin/main --fail-under-regions 80
```

### Standard Checkout Workflow

In a standard checkout, the normal workflow is simply `covgate check <coverage-report>`. If `--base` is omitted, `covgate` automatically checks `origin/HEAD`, `origin/main`, `origin/master`, `main`, and `master`. No `record-base` step is needed in that case.

When diffing against a Git base, `covgate` compares the merge-base snapshot to your current worktree. This includes committed changes plus staged/unstaged tracked edits, so local diagnosis reflects in-progress work.

```bash
# Generate JSON coverage report
cargo llvm-cov --json --output-path coverage.json

# Let covgate auto-discover the base ref in a standard checkout
covgate check coverage.json --fail-under-lines 90 --fail-under-regions 85
```

If your team wants a non-default base, pass it explicitly:

```bash
covgate check coverage.json --base origin/main --fail-under-regions 80
```

### Cloud-Agent Workflow

Use `covgate record-base` only in constrained cloud-agent or sandboxed worktree environments where normal base branches such as `main` or `origin/main` are unavailable.

Run `covgate record-base` at the beginning of a task before the agent makes Git changes. Running it immediately before `covgate check` is too late because that would capture the post-change `HEAD` instead of the task-start base.

When `--base` is omitted, `covgate` first tries the standard branch refs listed above and only falls back to `refs/worktree/covgate/base` when those refs are unavailable. Explicit `--base` still takes precedence. If a default base branch ref is already available, `covgate record-base` will do nothing.

The recorded base is kept per branch so separate agent task branches keep separate stable diff anchors.

```bash
# Capture a stable base commit at task start
covgate record-base

# ...agent performs the task work...

# Generate coverage and gate locally against the recorded base
cargo llvm-cov --json --output-path coverage.json
covgate check coverage.json --fail-under-lines 90 --fail-under-regions 85
```

The Codex Cloud environment settings maintenance script should include `covgate record-base` so coverage checks can validate the task reliably. Jules does not have a maintenance-script setting, so `AGENTS` instructions should require running `covgate record-base` before every task.

### Configuration (`covgate.toml`)

`covgate` reads repository-local defaults from `covgate.toml` so teams can keep their configuration checked in with the code. CLI flags always override config values.

You can specify a default `base` and `markdown_output` at the top level, along with minimum percentage (`fail_under_*`) and maximum uncovered count (`fail_uncovered_*`) rules under `[gates]`.

```toml
# Set a default comparison base and output file
base = "origin/main"
markdown_output = "summary.md"

[gates]
# Percentage-based gates (fail if coverage percentage is less than this value)
fail_under_functions = 100
fail_under_lines = 90
fail_under_regions = 85
fail_under_branches = 80

# Raw count gates (fail if the count is greater than this value)
fail_uncovered_functions = 0
```

With `covgate.toml` checked in, local invocations become frictionless:

```bash
# Run covgate using the thresholds and base defined in covgate.toml
covgate check coverage.json
```

## GitHub Actions

Generate JSON coverage, run `covgate`, and seamlessly write the results to your PR summary. 

When running `covgate` against the default branch in GitHub Actions, set `fetch-depth: 0` on the checkout action so it includes the default branch as the base to diff against. This is not required when using `--diff-file`.

```yaml
- uses: actions/checkout@v6
  with:
    fetch-depth: 0

- name: Generate Coverage
  run: cargo llvm-cov --json --output-path coverage.json

- name: Gate Pull Request
  run: covgate check coverage.json --markdown-output "$GITHUB_STEP_SUMMARY"
```

Because `covgate` supports repository-local defaults, a checked-in `covgate.toml` guarantees local hooks and CI pipelines enforce the exact same thresholds.

## How does `covgate` compare to existing tools?

While other coverage tools exist, `covgate` focuses entirely on local diff enforcement. It eliminates SaaS overhead, lossy-data compromises, and the CI-wait friction that slows developers and autonomous agents.

**Hosted CI Platforms (SonarQube & Codecov)**
* **The Trade-off:** CI-bound platforms offer analytics but require sending proprietary data to a hosted system and paying SaaS fees. They break the autonomous feedback loop by forcing developers and agents into a slow wait for external pipelines to finish.
* **The `covgate` Advantage:** Stays entirely inside your trusted runner or agent container, providing an instantaneous, local feedback loop.

**Local Diff Tools (e.g., `diff_cover`)**
* **The Trade-off:** Python's `diff_cover` avoids the CI-wait by running locally, but assumes a traditional Git checkout. Lacking a `record-base` equivalent, it breaks down in agent environments where `main` is stripped out. It also relies on lossy legacy formats and requires a Python runtime.
* **The `covgate` Advantage:** First-class AI agent support via `record-base`. Ships as a single compiled binary, reads compiler-native JSON directly, and enforces precise metrics without losing fidelity.

**`covrs` (Rust)**
* **The Trade-off:** Its center of gravity is reporting and SQLite aggregation rather than acting as a hard diff gate.
* **The `covgate` Advantage:** Built specifically to produce an explicit pass/fail on changed code, blocking untested code from merging.

## Contributing

Contributions are welcome. If your ecosystem has a native JSON or similarly lossless coverage format that maps accurately to executable structure, that is the exact kind of integration `covgate` is meant to support.

## License

Apache 2.0. See [LICENSE](./LICENSE) for details.
