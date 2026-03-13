# Environment Execution Contexts: Devcontainer, Codex Cloud, and Jules

This document records the detailed execution model for agent environments in this repository.

The quick reference lives in `docs/TOOLS.md`.

## References

- Codex Cloud universal image Dockerfile: <https://raw.githubusercontent.com/openai/codex-universal/refs/heads/main/Dockerfile>
- Codex Cloud default domain allowlist presets: <https://developers.openai.com/codex/cloud/internet-access#preset-domain-lists>
- Repo-specific domain additions: `` `docs/references/codex-cloud-setup-domain-allowlist.md` ``
- Jules Environment Setup: <https://jules.google/docs/environment/>

## Intent

Environment setup follows the repository knowledge philosophy of progressive disclosure: keep a short operational guide (`docs/TOOLS.md`) and maintain deeper rationale here.

## Principles

1. **Versioned setup over UI-only setup**: setup logic belongs in the repo so it can be branch-tested, code-reviewed, and updated with traceable history.
2. **Idempotent bootstrap**: setup should be safe to rerun and should only install what is missing.
3. **Repo-specific parity target**: parity is defined by tools needed for Voicer workflows, not by mirroring every language/runtime in universal images.
4. **Avoid redundant tooling in cloud**: do not bootstrap tools that duplicate native platform integrations (for example, GitHub CLI in Codex Cloud).


## Decision inputs for `` `scripts/setup-codex-cloud.sh` `` and `scripts/setup-jules.sh`

Tooling included in `` `scripts/setup-codex-cloud.sh` `` and `scripts/setup-jules.sh` is determined by cross-referencing these sources:

- Devcontainer toolchain baseline: `.devcontainer/Dockerfile`
- Codex Cloud universal image baseline: <https://raw.githubusercontent.com/openai/codex-universal/refs/heads/main/Dockerfile>
- Jules VM baseline (Ubuntu Linux preinstalled with Node.js, Python, Go, Java, Rust): <https://jules.google/docs/environment/>
- Repository workflow requirements and quality gates in code/docs/CI

The setup scripts should only include tools that are needed for Voicer workflows and are not already reliably provided by Codex Cloud or Jules integrations, or their respective base images.
It should also prefer distribution-derived values (for example, codename detection from `` `/etc/os-release` ``) over hardcoded repository codenames so the bootstrap remains portable as base images evolve.
This includes deriving OS family and version from `` `/etc/os-release` `` when selecting distribution-specific bootstrap URLs (such as the Microsoft package feed bootstrap `.deb`).

## Tool-selection rationale by environment

The devcontainer Dockerfile is intended to support both human developers and CLI/VS Code extension agents running inside the container. That dual audience guides a broader tool selection there.

Codex Cloud and Jules setup is narrower and should include only tooling required for repository workflows that is not already provided by the respective cloud platform or base image.

For some tools (for example Task and TFLint), the bootstrap may prefer GitHub release binaries over third-party apt feeds to reduce external repository dependencies and avoid extra allowlist requirements.

## Operational workflow

1. Update setup script and docs in a branch.
2. Configure Codex Cloud setup command to call `` `scripts/setup-codex-cloud.sh` `` or Jules to call `scripts/setup-jules.sh`.
3. Validate required tool availability and task execution from that branch.
4. Merge after validation.

## Network allowlist considerations

When setup scripts add external package sources, record required additional domains in `` `docs/references/codex-cloud-setup-domain-allowlist.md` `` so administrators can update Codex Cloud network policies intentionally.