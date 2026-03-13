# Release Binary Trust and CI Checks

This document records the recommended trust model for `covgate` as an open source project that distributes release binaries and is primarily consumed through a GitHub Action that downloads and runs those binaries inside other repositories' CI pipelines.

The short version is:

- CI should prove the source tree and dependency graph are healthy before a release is cut.
- Release automation should produce binaries that are easy to verify and trace back to the exact source and workflow that built them.
- The consuming GitHub Action should verify the downloaded artifact before executing it.

## Why This Matters

`covgate` is not only a Rust crate. In practice, it is a downloaded executable that other teams will run in their CI pipelines. That makes the release binary part of the product's trust boundary.

Users who adopt `covgate` through the GitHub Action are trusting three things at once:

1. The source repository and dependency graph were checked before release.
2. The published binary actually came from this repository's release workflow.
3. The binary downloaded by the action is the exact artifact the maintainers intended to publish.

This document separates those concerns so we can decide which controls belong in repository CI, which belong in release automation, and which belong in the consuming GitHub Action.

## Recommended CI Checks

The existing repository CI already covers formatting, compilation, linting, tests, diff-gate dogfooding, and dependency hygiene via `cargo machete`.

The next useful additions are:

### `cargo deny`

`cargo deny` is the best fit for the repository's dependency-hygiene job because it checks the policy questions maintainers care about before shipping a release:

- RustSec advisories
- yanked crates
- unmaintained crates
- license policy
- banned crates
- source policy such as unexpected git dependencies or registries

For this repository, `cargo deny` should be treated as the primary dependency policy check. It is a better fit than `cargo audit` alone because it covers advisories and also lets the project define license and source policy in one place.

### Overall coverage enforcement

`covgate.toml` currently uses regions as the stricter diff gate, which is a sensible project policy because LLVM region coverage is more precise than line coverage. That should remain the primary dogfood gate.

Separately, CI should still enforce the repository's stated minimum overall coverage floor with `cargo llvm-cov`. This is a different question from diff gating:

- Diff gating asks whether the changed code in this branch is acceptable.
- Overall coverage asks whether the project-wide test baseline has regressed too far.

### Keep line-metric paths exercised

Even though region coverage is the stricter default policy, line coverage is part of the supported product surface. CI should keep at least one explicit line-metric path alive so that line-specific code cannot quietly regress while the region path still passes.

The current line metric unit and integration tests are a good start.

## `cargo deny` Configuration Guidance

For this repository, the most useful `cargo deny` sections are:

### Advisories

Configure `cargo deny` to fail on known vulnerable dependencies. Denying yanked crates is usually also appropriate in CI. Unmaintained crates can start as warnings if that produces too much noise, then be tightened later if the dependency tree stays small.

### Licenses

Define an explicit allowlist of acceptable licenses instead of relying on defaults. For a small permissive Rust CLI, a practical starting set is:

- `MIT`
- `Apache-2.0`
- `BSD-2-Clause`
- `BSD-3-Clause`
- `ISC`
- `Unicode-3.0`

Set `default = "deny"` and `copyleft = "deny"` unless the project intentionally chooses to allow specific copyleft licenses. Add exceptions only when a real transitive dependency requires one.

The repository should also declare its own license clearly in `Cargo.toml`, not only in the top-level `LICENSE` file, so that downstream tooling sees an explicit SPDX expression.

### Bans

Use bans to detect dependency drift, especially duplicate versions of the same crate. For a young project, warnings are usually the right starting point because they surface graph churn without making CI brittle.

### Sources

Restrict dependencies to expected sources. For this project, allowing `crates.io` and denying unknown registries or ad hoc git sources is a reasonable default.

## `cargo audit` Compared to `cargo deny`

`cargo audit` is still a respectable tool, but if this repository already uses `cargo deny` for advisory checks then `cargo audit` adds more familiarity than unique assurance.

That means:

- `cargo deny` alone is enough to show that CI checks RustSec advisories, if it is configured clearly.
- `cargo audit` can still be added later for extra optics or a second advisory signal.
- `cargo audit` should not be prioritized ahead of getting `cargo deny` configured well.

For most potential users, seeing a well-configured `cargo deny` check in CI will be an adequate sign that dependency security is taken seriously.

## Release Binary Validation

Because `covgate` is mainly intended to be consumed as a downloaded release binary inside a GitHub Action, release-time and consume-time validation matter more than they would for a crate that users mostly install from source.

The release pipeline should publish enough material for the consuming action to validate what it downloads.

Recommended release artifacts and metadata:

- The binary for each supported platform and architecture
- A checksum file, such as SHA-256 checksums for every asset
- Provenance or attestation, such as GitHub artifact attestation or Sigstore/Cosign-based verification

These checks matter more than `cargo-auditable` if the consuming action is not extracting or inspecting auditable metadata.

## Validation in the Consuming GitHub Action

The consuming GitHub Action should verify the release binary before executing it. The highest-value checks are:

### Pin the requested version

Do not default to "latest" when a caller can instead provide or inherit an explicit tag. Pinning a version is the easiest way to make downstream runs reproducible.

### Verify checksums

The action should download both the binary and a published checksum file, then fail if the checksum does not match. This defends against accidental corruption and some classes of tampering.

### Verify provenance or attestation

If releases publish artifact attestations or signatures, the action should verify them before executing the binary. This is the strongest way to prove the binary came from the expected repository workflow rather than only matching a checksum from an untrusted location.

### Verify the binary's self-reported version

After download, run `covgate --version` and compare it to the pinned release tag. This catches wrong-asset selection and some packaging mistakes.

### Validate platform and asset selection

The action should fail closed if the expected asset for the runner's operating system and CPU architecture is missing. It should not silently fall back to another platform's artifact.

## Where `cargo-auditable` Fits

`cargo-auditable` embeds dependency metadata into the produced binary. That can be valuable for post-release inspection, incident response, or future supply-chain tooling.

For this repository, its value depends on whether someone actually uses that metadata.

### When it is valuable

`cargo-auditable` is worthwhile if the project intends to:

- let users inspect the contents of release binaries after download
- generate or publish software bill of materials style metadata from released artifacts
- support incident response by making dependency contents easy to recover from shipped binaries

### When it is not enough on its own

`cargo-auditable` does not replace:

- checksum verification
- provenance or attestation verification
- dependency checks in CI such as `cargo deny`

If the consuming GitHub Action only downloads the binary, verifies a checksum, and runs it, then `cargo-auditable` is not actively validated in that flow. In that case it is best understood as an optional transparency feature, not a primary security control.

### Recommendation for `covgate`

Given the current distribution model, `cargo-auditable` is a reasonable future enhancement for release builds, but it should not be prioritized ahead of:

1. `cargo deny` in repository CI
2. published checksums
3. provenance or attestation for release assets
4. validation of those release artifacts inside the consuming GitHub Action

If it is added, it should be attached to release automation rather than treated as a replacement for CI or action-level verification.

## Practical Priority Order

If the project wants to improve trust incrementally, the highest-signal order is:

1. Add `cargo deny` to repository CI with advisories, licenses, bans, and sources configured.
2. Publish checksums for release assets.
3. Make the consuming GitHub Action verify checksums and the requested version before execution.
4. Add provenance or attestation to the release workflow and verify it in the consuming action.
5. Consider `cargo-auditable` for release builds if the project wants stronger post-release transparency.

This order reflects the actual risk profile of the project: users will primarily trust `covgate` as a downloaded binary running in CI, so controls that validate the published artifact are more important than controls that only describe it.
