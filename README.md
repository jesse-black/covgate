# covgate

[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](./LICENSE)

A blazing-fast, zero-dependency CI/CD quality gate that enforces code coverage strictly on your pull request diffs. 

Built in Rust, `covgate` evaluates **line, branch, and region coverage** by parsing lossless, compiler-native JSON reports. It operates entirely locally in your CI runner, preventing untested code from merging without the false positives or SaaS costs associated with other coverage tools.

## The "Lossless-First" Philosophy

**Why doesn't `covgate` support Cobertura or LCOV?**

Legacy coverage formats like Cobertura and LCOV are inherently lossy. They force complex Abstract Syntax Trees (ASTs) and compiler Intermediate Language (IL) into flat, line-based XML or text files. When modern compilers export to these formats, branch and region data is frequently shifted, squashed, or lost entirely. 

If you use Cobertura or LCOV for PR gating, you will inevitably block developers for compiler-generated "ghost branches" they cannot test (such as hidden `async/await` state machines in C#), or accidentally pass PRs where multi-line conditional logic was left uncovered.

Furthermore, native formats allow `covgate` to support advanced metrics like **Region Coverage**. A code region is a continuous span of execution that can start and stop mid-line to represent logical forks. Because it precisely tracks exactly what parts of a complex statement executed, region coverage is a strictly superior metric to line and branch coverage combined, but it is entirely stripped out when exporting to legacy formats.

`covgate` takes a strict, opinionated stance: **We only parse the ground-truth JSON directly from the compiler.** This guarantees mathematically accurate intersections between your Git diff and your coverage data.

## Supported Ecosystems

`covgate` currently supports the native JSON formats for several popular language ecosystems:

* **Rust / C / C++ / Swift (LLVM JSON):** Uses `llvm-cov export -format=json` to gate on highly accurate LLVM Region coverage.
* **JavaScript / TypeScript (Istanbul JSON):** Uses `coverage-final.json` to map exact AST nodes for accurate line and branch gating.
* **C# / .NET (Coverlet JSON):** Uses `dotnet test --collect:"XPlat Code Coverage;Format=json"` to extract raw IL sequence and branch points.

## Installation

Because `covgate` is a statically linked Rust binary, it requires no language runtimes (like Python, Node, or .NET) to execute in your CI pipeline.

**Via Cargo (Local Development):**

```bash
cargo install covgate
```

**Via GitHub Actions:**
*(Note: A dedicated setup action is planned, but you can currently pull the binary directly)*

```yaml
- name: Install covgate
  run: |
    curl -L [https://github.com/yourusername/covgate/releases/latest/download/covgate-linux-amd64](https://github.com/yourusername/covgate/releases/latest/download/covgate-linux-amd64) -o /usr/local/bin/covgate
    chmod +x /usr/local/bin/covgate
```

## Usage

Run `covgate` in your CI pipeline after your tests generate their coverage artifacts. You must provide the target JSON format and your desired thresholds. 

### CLI Arguments

* `--istanbul <FILE>`: Path to Istanbul `coverage-final.json`
* `--coverlet <FILE>`: Path to Coverlet `coverage.json`
* `--llvm-json <FILE>`: Path to `llvm-cov` JSON export
* `--fail-under-line <PERCENT>`: Fails if diff line coverage is below this threshold (0-100)
* `--fail-under-branch <PERCENT>`: Fails if diff branch coverage is below this threshold (0-100)
* `--fail-under-region <PERCENT>`: Fails if diff region coverage is below this threshold (0-100)

### Examples

**Gating a Node.js Pull Request (Line & Branch):**

```bash
covgate --istanbul coverage/coverage-final.json \
        --fail-under-line 85 \
        --fail-under-branch 90
```

**Gating a Rust or C++ Pull Request (Region):**

```bash
llvm-cov export --format=json > coverage.json
covgate --llvm coverage.json \
        --fail-under-region 100
```

**Gating a .NET Pull Request (Line & Branch):**

```bash
covgate --coverlet TestResults/**/coverage.json \
        --fail-under-line 80 \
        --fail-under-branch 80
```

### How does `covgate` compare to existing tools?

There are several tools in the code quality space, but `covgate` was built specifically to eliminate the CI friction, mathematical inaccuracies, and SaaS costs associated with legacy coverage formats.

**SonarQube & Codecov**
These are the enterprise heavyweights of the code quality space, providing organization-wide analytics and historical tracking.
* **The Trade-off:** They are proprietary, paid SaaS products. To use them on private repositories, you must pay expensive per-user licensing fees or pay based on your repository's Lines of Code (LoC). Furthermore, they require you to upload your proprietary source code and coverage artifacts to their closed-source cloud servers.
* **The `covgate` Advantage:** `covgate` is 100% free and open-source. It executes entirely locally within your existing GitHub Actions or GitLab CI runners. Your code and coverage data never leave your infrastructure, and there is zero additional cost to integrate it, no matter how many developers join your team.

**`diff_cover` (Python)**
`diff_cover` is the gold standard for diff-based coverage gating in the Python ecosystem.
* **The Trade-off:** It requires a Python runtime in your CI pipeline, adding pipeline bloat and installation time for environments that don't natively use Python. It exclusively reports and gates on line coverage, and it relies on lossy legacy formats like Cobertura and LCOV, which can lead to inaccurate calculations when evaluating complex branch logic.
* **The `covgate` Advantage:** Distributed as a standalone, statically linked binary, `covgate` requires zero runtime dependencies. By parsing lossless, compiler-native JSON (Istanbul, Coverlet, LLVM), it guarantees mathematically accurate PR gating. Crucially, it goes beyond basic line coverage by reporting and enforcing strict quality gates on branch, region, and combined coverage metrics.

**`covrs` (Rust)**
`covrs` is a fantastic Rust tool for aggregating coverage data and posting informational PR comments.
* **The Trade-off:** `covrs` is primarily observational; it does not have a strict `--fail-under` mechanism to block PRs, and it targets legacy LCOV formats, missing out on the superior LLVM region coverage entirely.
* **The `covgate` Advantage:** `covgate` is designed from the ground up to be a strict, threshold-based CI/CD gate. Instead of just leaving an informational comment, it explicitly fails the pipeline if your configured thresholds aren't met, giving you automated, granular control over line, branch, and region enforcement to block untested code from merging.

## Contributing

Contributions are welcome! If you want to add support for a new language ecosystem, please note our "Lossless-First" philosophy. PRs adding support for legacy translation formats (Cobertura/LCOV) will not be merged. If your ecosystem has a native JSON or binary coverage format that maps accurately to AST/IL data, we would love to collaborate on a parser.

## License

Apache 2.0. See [LICENSE](./LICENSE) for details.