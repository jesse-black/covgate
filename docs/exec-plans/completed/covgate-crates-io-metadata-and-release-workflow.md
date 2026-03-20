# Add crates.io package metadata and a tagged release workflow with checksums and provenance

The canonical completed copy of this ExecPlan lives at `docs/exec-plans/completed/covgate-crates-io-metadata-and-release-workflow.md`. Keep any follow-up release-channel work in a new active ExecPlan rather than moving this file back.

This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds.

Maintain this document in accordance with `docs/PLANS.md`. Re-read that file before revising this plan and keep this plan aligned with its rules.

## Purpose / Big Picture

`covgate` already builds and tests like a serious Rust CLI, but it is not yet prepared for a polished public release flow. The crate manifest is missing the descriptive metadata that makes a crates.io page credible and discoverable, and the repository does not yet contain a dedicated release workflow that can build tagged binaries, publish checksums, and attach provenance for downstream verification.

After this work, a novice maintainer should be able to inspect `Cargo.toml`, see a complete crates.io-ready package definition, run a package dry run locally, create a release tag, and watch GitHub Actions produce release artifacts that are traceable and verifiable. The user-visible outcome is twofold: `cargo install covgate` is backed by a well-described crate page, and GitHub releases ship binaries plus verification material suitable for the trust model already described in this repository.

## Progress

- [x] (2026-03-19 18:20Z) Re-read `docs/PLANS.md`, inspected `Cargo.toml`, `README.md`, `.github/workflows/ci.yml`, `deny.toml`, and `docs/reference/release-binary-trust-and-ci.md`, and confirmed the current release gap.
- [x] (2026-03-19 18:30Z) Created this active ExecPlan with repository-specific file targets, validation commands, and a first-pass scope for crates.io metadata plus release automation.
- [x] (2026-03-19 18:45Z) Narrowed the plan so crates.io publication is an explicit manual maintainer step using `cargo publish`, while GitHub Actions handles binary release artifacts only.
- [x] (2026-03-20 00:40Z) Added crates.io package metadata to `Cargo.toml`, set `rust-version = "1.85"` to match the Rust 2024 edition floor, and trimmed the published crate to a reviewed include allowlist that keeps contributor-only docs, scripts, and fixtures out of the tarball while retaining `.cargo/config.toml`.
- [x] (2026-03-20 00:45Z) Initially updated `README.md` with the public release story, then reverted those README additions during review so release-process details remain in maintainer-facing planning artifacts instead of the user-facing README.
- [x] (2026-03-20 01:00Z) Added `.github/workflows/release.yml` plus a deterministic `scripts/package-release.sh` helper to build, archive, checksum, and publish tagged release artifacts for Linux, Windows, and Apple Silicon macOS targets.
- [x] (2026-03-20 01:05Z) Added GitHub artifact attestation generation in each release build job so every published archive gets provenance from the repository workflow.
- [x] (2026-03-20 00:45Z) Recorded the maintainer release sequence in repository planning notes rather than the user-facing README after review feedback requested reverting README release notes.
- [x] (2026-03-20 01:20Z) Validated package contents with `cargo package --allow-dirty --list`, exercised `cargo publish --dry-run`, syntax-checked `scripts/package-release.sh`, and ran repository validation with `cargo xtask quick` plus `cargo xtask validate`.
- [x] (2026-03-20 01:25Z) Updated the retrospective with the shipped release process, recorded packaging and target-platform decisions, and moved this ExecPlan to `docs/exec-plans/completed/`.
- [x] (2026-03-20 02:25Z) Applied post-review follow-up: reverted the README release notes, removed the retiring `x86_64-apple-darwin` target from the release matrix, and kept the maintainer-facing release rationale in this completed ExecPlan.

## Surprises & Discoveries

- Observation: `cargo package` already reports the most immediate crates.io gap without requiring a real publish attempt.
  Evidence: local packaging warned that the manifest has no `description`, `documentation`, `homepage`, or `repository`.

- Observation: The repository already has strong source-tree quality checks for a release candidate, but they currently live only in the general CI workflow, not in a dedicated release pipeline.
  Evidence: `.github/workflows/ci.yml` runs formatting, clippy, tests, `cargo llvm-cov`, `cargo machete`, and `cargo deny`, while `.github/workflows/` contains no release workflow file.

- Observation: The repository already contains a release trust design note, so the missing work is implementation and operationalization rather than first-principles design.
  Evidence: `docs/reference/release-binary-trust-and-ci.md` explicitly recommends binaries, checksums, and provenance or attestation for releases.

- Observation: The current package tarball includes a very large amount of documentation and fixture material by default.
  Evidence: `cargo package --offline --allow-dirty --list` includes `docs/`, `scripts/`, and `tests/fixtures/...` in the packaged crate.

- Observation: A minimal explicit `include` list is a good first candidate for this repository because the likely unwanted package contents are whole top-level areas such as `docs/`, `scripts/`, `tests/`, `ARCHITECTURE.md`, and `AGENTS.md`, not a few scattered files.
  Evidence: the current package dry-run list shows source and metadata files mixed with contributor-only docs, fixtures, and helper scripts that are not obviously needed by downstream crate consumers.

- Observation: the explicit package allowlist needed `.cargo/config.toml` to preserve the repository's linker and target defaults during `cargo package` verification, but it did not need `tests/`, `docs/`, or fixture trees.
  Evidence: `cargo package --allow-dirty --list` succeeded with a package surface limited to `src/**`, `.cargo/config.toml`, `Cargo.toml`, `Cargo.lock`, `README.md`, and `LICENSE*`, and `cargo publish --dry-run` completed successfully from that same manifest.

- Observation: Cargo still auto-includes a few README artifacts and warns that integration tests are omitted when the crate uses a narrow `include` allowlist.
  Evidence: `cargo package --allow-dirty --list` retained `tests/fixtures/dotnet/README.md` and `tests/fixtures/vitest/README.md`, and `cargo publish --dry-run --allow-dirty` warned that integration tests such as `tests/cli_interface.rs` are ignored because they are not shipped in the published package.

- Observation: release-time artifact attestations fit cleanly into the build matrix as long as each job attests its packaged archive before the publish job aggregates assets into a GitHub release.
  Evidence: `.github/workflows/release.yml` can generate `dist/covgate-<tag>-<target>.<ext>` and call `actions/attest-build-provenance` against `dist/*` without needing extra packaging state in the publish job.

## Decision Log

- Decision: Treat crates.io metadata and GitHub release automation as one coherent release-readiness milestone rather than two unrelated cleanups.
  Rationale: The product is distributed both as a Rust crate and as downloadable binaries. The repository’s own trust model says those channels should be designed together so published assets and manifest metadata tell a consistent story.
  Date/Author: 2026-03-19 / Codex

- Decision: Use the existing `docs/reference/release-binary-trust-and-ci.md` as the source of truth for release workflow expectations.
  Rationale: That document already defines the highest-value controls for this project, including checksums and provenance, so the implementation plan should follow it instead of inventing a second release policy.
  Date/Author: 2026-03-19 / Codex

- Decision: Keep `xtask` unpublished and scope this plan to the public `covgate` crate plus repository-level release automation.
  Rationale: `xtask/Cargo.toml` already declares `publish = false`, and nothing in the current release goal requires changing that internal helper crate’s distribution model.
  Date/Author: 2026-03-19 / Codex

- Decision: Prefer an explicit tagged release workflow file under `.github/workflows/` rather than overloading the existing CI workflow with release responsibilities.
  Rationale: CI and release have different triggers, secrets, permissions, and outputs. A separate workflow is easier for a novice maintainer to reason about and safer to audit.
  Date/Author: 2026-03-19 / Codex

- Decision: Keep crates.io publishing manual for the first release workflow and require maintainers to run `cargo publish` themselves.
  Rationale: This avoids storing a crates.io API token in GitHub Actions, keeps the first release workflow simpler, and separates irreversible crate publication from repeatable binary packaging.
  Date/Author: 2026-03-19 / Codex

- Decision: Start package trimming with an explicit `include` allowlist rather than an `exclude` denylist.
  Rationale: The repository has several large top-level directories and contributor-focused files that should stay out of the published crate by default. An allowlist is easier to audit and naturally keeps `tests/`, `docs/`, `scripts/`, `ARCHITECTURE.md`, and `AGENTS.md` out unless the package verification steps prove that one of them must be added back.
  Date/Author: 2026-03-19 / Codex

- Decision: Set `rust-version = "1.85"` in `Cargo.toml`.
  Rationale: `covgate` already uses the Rust 2024 edition, whose minimum stable compiler is Rust 1.85, so making that floor explicit helps crates.io consumers and release tooling fail fast with a clear message.
  Date/Author: 2026-03-20 / Codex

- Decision: Keep `rust-version = "1.85"` even though the field syntax looks like a single version.
  Rationale: In Cargo manifests, `rust-version` declares a minimum supported compiler version rather than an exact pin, so `1.85` accurately states the edition-imposed floor without preventing newer compilers.
  Date/Author: 2026-03-20 / Codex

- Decision: Publish four first-release binary targets: `x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl`, `x86_64-pc-windows-msvc`, and `aarch64-apple-darwin`.
  Rationale: Before the first public release, it is cheaper to set the Linux portability story up front than to migrate it later. Shipping musl-linked Linux binaries avoids glibc-version coupling for downloaded CLI users, and adding Linux arm64 keeps the release set aligned with the two common Linux CPU architectures. Zig-backed musl cross-compilation keeps that extra toolchain complexity isolated to the release workflow instead of changing the repository's normal development or CI paths.
  Date/Author: 2026-03-20 / Codex

- Decision: Use a checked-in `scripts/package-release.sh` helper for archive assembly.
  Rationale: Packaging logic is easier to validate locally with `bash -n` and easier for a novice maintainer to audit when the archive naming and included files live in a single deterministic script instead of duplicated workflow shell fragments.
  Date/Author: 2026-03-20 / Codex

## Outcomes & Retrospective

This ExecPlan is complete. `covgate` now has crates.io-ready manifest metadata, a narrowed published crate surface, and a dedicated GitHub Actions release workflow that builds tagged archives, publishes `checksums.txt`, and generates GitHub artifact attestations for each packaged binary. Review feedback ultimately kept maintainer release instructions out of the user-facing README.

The most important implementation lesson was that release readiness depended on small operational details as much as on the high-level policy: the Rust 2024 edition forced an explicit `rust-version = "1.85"`, Cargo packaging only needed `.cargo/config.toml` in addition to source and top-level metadata, and the release workflow stayed easier to audit once Linux musl cross-compilation was isolated to explicit workflow steps while crate publishing remained a manual `cargo publish` step outside GitHub Actions. A future follow-up can add more targets or consumer-side attestation verification without reopening this completed milestone.

## Context and Orientation

`covgate` is a Rust workspace rooted at `Cargo.toml`. The public crate is the root package `covgate`; the helper crate in `xtask/` is intentionally unpublished. The current root manifest includes only the minimal package fields `name`, `version`, `edition`, and `license`. It does not yet declare the descriptive package metadata that crates.io and downstream tooling use for discovery and attribution.

The user-facing installation and usage guidance lives in `README.md`. This file already explains the product’s purpose, supported ecosystems, and installation via `cargo install covgate`, so release-process details should only be added there if they clearly help end users rather than maintainers.

Repository automation currently lives in `.github/workflows/ci.yml`. That workflow validates source quality but does not create GitHub releases, upload binaries, publish checksums, or attest released artifacts. The repository does contain release guidance in `docs/reference/release-binary-trust-and-ci.md`. In plain language, a “checksum” is a small text file that records a hash, such as SHA-256, for each release artifact so consumers can verify downloads were not changed. “Provenance” or “attestation” means signed machine-readable evidence that a release artifact was produced by a specific repository workflow run.

The main files expected to change are:

- `Cargo.toml`, where crates.io-facing package metadata will be added and package include or exclude rules may be adjusted.
- `README.md`, if installation or release verification instructions need a concise update after the metadata and workflow changes land.
- `.github/workflows/release.yml` or a similarly named new workflow file, which will own tagged build, packaging, checksum generation, and release publication.
- Potentially `.github/workflows/ci.yml`, but only if a small integration point is needed to keep CI and release expectations aligned.
- Potentially a new helper script under `scripts/` if checksum generation or asset packaging would otherwise be duplicated in workflow shell steps.
- Potentially a release-oriented documentation file or a short section in `README.md` if maintainers need a precise release procedure checked into the repository.

This plan uses “crate metadata” to mean descriptive fields in `Cargo.toml` such as `description`, `repository`, `homepage`, `documentation`, `keywords`, `categories`, and `rust-version`. It uses “release workflow” to mean a GitHub Actions workflow triggered by tags or manual dispatch that builds artifacts, publishes them to a GitHub release, and emits verification material.

## Plan of Work

Start with the crate manifest because that work is the smallest, most direct path to a crates.io-ready package. Update `Cargo.toml` so the root `covgate` package has a clear one-sentence description consistent with `README.md`, an explicit repository URL, a homepage and documentation URL if they are distinct and stable, a minimum supported Rust version if the project has one, and keywords and categories that accurately describe a Rust command-line quality gate.

For package contents, begin with this explicit allowlist in the root package manifest:

    [package]
    include = [
      "src/**",
      "Cargo.toml",
      "Cargo.lock",
      "README.md",
      "LICENSE*",
    ]

Treat that list as the starting point, not an untouchable final answer. Its intent is to exclude contributor-only material by default, especially `tests/`, `docs/`, `scripts/`, `ARCHITECTURE.md`, and `AGENTS.md`. After adding it, immediately sanity-check whether normal Cargo packaging commands still function correctly. In particular, verify that `cargo package` and, if network permits, `cargo publish --dry-run` still succeed when `tests/` is absent from the packaged crate. If packaging verification fails because Cargo expects a missing file, add back only the smallest required path and record that change in the `Decision Log`.

Next, keep crates.io publication manual and explicit. The maintainer flow for this plan is: update crate metadata and version fields, run local validation, run `cargo publish` from the repository root, and only then create the version tag that drives the GitHub binary release workflow. This order makes the irreversible crates.io step an intentional human action and keeps the workflow free of registry secrets.

Then add a dedicated release workflow under `.github/workflows/`. It should trigger on version tags in a clearly named pattern, and it may also support `workflow_dispatch` so maintainers can rehearse or retry binary release creation intentionally. The workflow should check out the tagged source, install the Rust toolchain, and build `covgate` for each supported operating system and architecture that the project wants to advertise. The deliverable for each target is a compressed archive containing the `covgate` binary and a small amount of release metadata if needed. After all target builds finish, a publish job should assemble SHA-256 checksums for every release asset, create or update the GitHub release for the tag, upload the archives and checksum file, and attach provenance or artifact attestation so downstream users and the future GitHub Action can verify them.

Keep the release workflow aligned with the existing trust note. That means the release assets should be version-pinned by tag, checksums should be published as first-class release artifacts, and provenance or attestation should be generated by the workflow rather than left as a future comment. If the repository later adds a consumer GitHub Action, that action should be able to point directly at this release output without inventing a second packaging format.

Finally, document the maintainer procedure in repository prose. A novice maintainer should be able to answer these questions from checked-in files: which manifest fields must be updated for a new release, which command publishes the crate to crates.io, when to create the release tag, what assets to expect on the GitHub release page, and what verification material is published. Keep the documentation short and operational. The goal is not a release handbook; it is a reliable, repeatable checklist embedded in repository context.

## Concrete Steps

Run all commands from the repository root, the directory containing `Cargo.toml`.

1. Inspect the current package metadata and packaged file set before editing.

    sed -n '1,80p' Cargo.toml
    cargo metadata --no-deps --format-version 1
    cargo package --offline --allow-dirty --list

   Expected result: the metadata fields that are currently missing are visible, and the packaged file list provides a baseline for any later `include` or `exclude` changes.

2. Add the crates.io-facing manifest metadata, add the proposed `include` allowlist, and verify packaging again.

    cargo package --offline --allow-dirty --list

   Expected result: the package dry run no longer warns about missing `description`, `documentation`, `homepage`, or `repository`, and the packaged file set is intentionally narrow, with `tests/`, `docs/`, `scripts/`, `ARCHITECTURE.md`, and `AGENTS.md` excluded unless one of them has already been proven necessary.

3. Sanity-check that Cargo packaging and publish-oriented commands still work with `tests/` excluded from the package.

    cargo package --allow-dirty
    cargo publish --dry-run

   Expected result: package verification succeeds even though `tests/` is not shipped in the crate tarball. If either command fails because a file under `tests/` or another excluded path is unexpectedly required, add back only the minimum required path in `include`, rerun the same commands, and record the adjustment here and in the `Decision Log`.

4. Add the tagged binary release workflow and validate its YAML shape locally as far as repository tooling allows.

    sed -n '1,260p' .github/workflows/release.yml
    git diff -- .github/workflows/release.yml Cargo.toml README.md

   Expected result: the workflow file clearly shows tag-triggered release behavior, build jobs, checksum generation, and provenance or attestation configuration, and the diff is reviewable without hidden generated state.

5. Run the repository’s normal development validation loop after the metadata and workflow edits.

    cargo xtask quick
    cargo xtask validate

   Expected result: the repository still passes formatting, linting, tests, coverage validation, and dependency checks after the release-facing edits.

6. If the workflow uses shell helpers or archive naming logic, exercise those helpers locally with a representative target name.

    bash -n scripts/<release-helper>.sh

   Expected result: any checked-in helper script is syntactically valid and deterministic. If no helper script is added, omit this step and update the plan accordingly.

7. If networked publish testing was not possible in step 3, record that limitation explicitly and capture the compensating validation used instead.

    cargo package --offline --allow-dirty --list

   Expected result: the final plan states clearly whether `cargo publish --dry-run` was exercised. If it was not, the fallback evidence is the successful package verification plus the reviewed package file list and the documented reason the networked dry run could not be performed.

## Validation and Acceptance

This plan is complete only when all of the following are true and observable by a novice maintainer:

`Cargo.toml` contains a complete, intentional set of crates.io-facing package metadata for the public `covgate` crate. At minimum, the manifest must stop emitting missing-metadata warnings during package validation.

The packaged crate contents are intentional. If the repository chooses to keep shipping large docs or fixture trees, that decision must be explicit in the `Decision Log`. If the repository chooses to trim package contents, the final include or exclude rules must still allow package verification and normal source builds to succeed.

The initial package-trimming attempt must start from the explicit `include` allowlist recorded in this plan unless implementation evidence proves it insufficient. If that starting point changes, the final manifest and `Decision Log` must explain exactly which additional paths were needed and why.

A dedicated GitHub release workflow exists under `.github/workflows/` and is clearly separated from ordinary CI. It must build binary release artifacts from tags, publish a checksum file, and emit provenance or artifact attestation aligned with `docs/reference/release-binary-trust-and-ci.md`.

The repository contains a checked-in explanation of the maintainer release procedure, including that crates.io publication is manual via `cargo publish`, how version tags drive binary releases, and what verification artifacts should appear on the GitHub release.

`cargo xtask quick` passes during development and `cargo xtask validate` passes before the plan is closed. If `cargo publish --dry-run` cannot be run in the implementation environment, that limitation and the exact fallback validation used must be documented in `Outcomes & Retrospective`.

## Idempotence and Recovery

Manifest metadata edits are naturally idempotent: re-running the validation commands should produce the same warnings or lack of warnings without mutating repository state beyond expected package scratch files in `target/`.

The release workflow must be safe to inspect and rerun. Prefer tag-based triggers for real releases and `workflow_dispatch` for rehearsals or retries so maintainers do not need to create throwaway tags just to test workflow behavior. If a release job fails after assets are partially uploaded, recovery should mean rerunning the workflow or deleting and recreating the release in GitHub, not editing checked-in files to match transient workflow state.

If package include or exclude rules accidentally omit required files, recover by restoring the missing paths and rerunning `cargo package --offline --allow-dirty --list` before proceeding. Do not guess. Use the file list to prove the package contents match intent.

If provenance or attestation cannot be implemented cleanly in the first pass, do not silently drop it. Record the blocker in `Surprises & Discoveries`, keep checksum publication in place, and either leave the plan active or explicitly document the narrower first release scope as an intentional, reviewed decision.

## Artifacts and Notes

Representative local metadata validation before the manifest edit:

    $ cargo package --offline --allow-dirty --list
    warning: manifest has no description, documentation, homepage or repository

Representative manifest shape after the metadata edit:

    [package]
    name = "covgate"
    version = "0.1.0"
    edition = "2024"
    license = "Apache-2.0"
    description = "..."
    repository = "https://github.com/jesse-black/covgate"
    documentation = "https://docs.rs/covgate"
    include = [
      "src/**",
      "Cargo.toml",
      "Cargo.lock",
      "README.md",
      "LICENSE*",
    ]

Representative package-surface expectation after the initial include filter:

    included:
    - src/**
    - Cargo.toml
    - Cargo.lock
    - README.md
    - LICENSE*

    excluded unless proven necessary by package verification:
    - tests/**
    - docs/**
    - scripts/**
    - ARCHITECTURE.md
    - AGENTS.md

Representative release assets after a tagged workflow run:

    covgate-x86_64-unknown-linux-musl.tar.gz
    covgate-aarch64-unknown-linux-musl.tar.gz
    covgate-aarch64-apple-darwin.tar.gz
    covgate-x86_64-pc-windows-msvc.zip
    checksums.txt

Representative verification expectations for the final GitHub release:

    - each asset name includes the target platform
    - a checksum file lists SHA-256 hashes for every uploaded asset
    - provenance or attestation is attached or published by the workflow

If the implementation chooses a different archive format or target set, update this section and the `Decision Log` so the final repository record stays self-consistent.

## Interfaces and Dependencies

Use the existing Rust workspace and GitHub Actions infrastructure. Do not introduce an alternative release service or a second package manager path.

The final implementation should leave these interfaces and responsibilities clear:

- `Cargo.toml` defines the public crates.io metadata for `covgate` and starts package trimming from the explicit `include` allowlist recorded in this plan, expanding it only if package verification proves an omitted path is required.
- `xtask/Cargo.toml` remains `publish = false` and is not part of crates.io distribution.
- `.github/workflows/ci.yml` continues to own ordinary source validation.
- `.github/workflows/release.yml` or the chosen new workflow file owns tagged binary release creation, checksum publication, and provenance or attestation emission. It does not publish the crate to crates.io.
- `README.md` and any added release note documentation explain the supported installation and release-verification story without duplicating the entire workflow implementation.
- If a helper script is added for packaging or checksum generation, it must live under `scripts/`, be deterministic, and be callable from GitHub Actions without hidden environment assumptions beyond those stated in the workflow.

At the bottom of this plan, append a revision note every time the plan changes materially, describing what changed and why.

Revision note: Initial plan created to add missing crates.io package metadata and implement a dedicated tagged release workflow with checksums and provenance, based on the repository’s existing release trust guidance.

Revision note: Simplified the plan by deciding that crates.io publication remains a manual maintainer step via `cargo publish`; the GitHub workflow now covers binary releases only.

Revision note: Added a concrete starting `include` allowlist for the crate package and explicit sanity-check steps to verify that `cargo package` and `cargo publish --dry-run` still work when `tests/` is not included in the published crate.

Revision note: Implemented the plan by shipping crates.io metadata, a trimmed package include list, release documentation, a checked-in packaging helper, and a dedicated tagged release workflow with checksums plus GitHub artifact attestations; then moved the plan to `docs/exec-plans/completed/`.

Revision note: After review, reverted the README release-flow additions, narrowed the release matrix by removing `x86_64-apple-darwin`, and kept the maintainer-process detail in this completed ExecPlan instead of the user-facing README.
