# Environment Execution Contexts: Devcontainer, Codex Cloud, and Jules

This document records the detailed execution model for agent environments in this repository.

The quick reference lives in `docs/TOOLS.md`.

## References

- Codex Cloud universal image Dockerfile: <https://raw.githubusercontent.com/openai/codex-universal/refs/heads/main/Dockerfile>
- Codex Cloud default domain allowlist presets: <https://developers.openai.com/codex/cloud/internet-access#preset-domain-lists>
- Jules Environment Setup: <https://jules.google/docs/environment/>

## Intent

Environment setup follows the repository knowledge philosophy of progressive disclosure: keep a short operational guide (`docs/TOOLS.md`) and maintain deeper rationale here.

## Principles

1. **Versioned setup over UI-only setup**: setup logic belongs in the repo so it can be branch-tested, code-reviewed, and updated with traceable history.
2. **Idempotent bootstrap**: setup should be safe to rerun and should only install what is missing.
3. **Repo-specific parity target**: parity is defined by tools needed for covgate workflows, not by mirroring every language/runtime in universal images.
4. **Avoid redundant tooling in cloud**: do not bootstrap tools that duplicate native platform integrations (for example, GitHub CLI in Codex Cloud).

## Decision inputs for shared agent setup and maintenance scripts

Tooling included in the shared setup implementation in `scripts/agent-env-setup.sh` is determined by cross-referencing these sources:

- Devcontainer toolchain baseline: `.devcontainer/Dockerfile`
- Codex Cloud universal image baseline: <https://raw.githubusercontent.com/openai/codex-universal/refs/heads/main/Dockerfile>
- Jules VM baseline (Ubuntu Linux preinstalled with Node.js, Python, Go, Java, Rust): <https://jules.google/docs/environment/>
- Repository workflow requirements and quality gates in code/docs/CI

The setup scripts should only include tools that are needed for covgate workflows and are not already reliably provided by Codex Cloud or Jules integrations, or their respective base images.
They should also prefer distribution-derived values (for example, codename detection from `/etc/os-release`) over hardcoded repository codenames so the bootstrap remains portable as base images evolve.
The repository now keeps a single setup implementation so all agent environments stay in sync.
That shared setup now also bootstraps the Microsoft apt feed when needed so `.NET` SDK installation remains portable across supported Debian/Ubuntu images.
It also explicitly installs the Clang and LLVM command-line tools needed to generate LLVM coverage JSON for future C/C++ fixtures instead of assuming those binaries are present in the base image.
Swift is bootstrapped through Swift.org's supported `swiftly` installer so Linux agent environments can build Swift fixtures and emit LLVM-based coverage data without depending on distro package availability.
A lightweight maintenance entrypoint (`scripts/agent-env-maintenance.sh`) now uses raw Git plumbing to keep a stable per-task base ref: repeated runs on the same branch are idempotent, and branch changes refresh `refs/worktree/covgate/base`, all without requiring a `covgate` build.

## Tool-selection rationale by environment

The devcontainer Dockerfile is intended to support both human developers and CLI/VS Code extension agents running inside the container. That dual audience guides a broader tool selection there.

Codex Cloud and Jules setup is narrower and should include only tooling required for repository workflows that is not already provided by the platform or base image.

## Operational workflow

1. Update setup script and docs in a branch.
2. Configure environment setup command to call `scripts/agent-env-setup.sh`.
3. Configure environment maintenance command to call `scripts/agent-env-maintenance.sh`.
4. Validate required tool availability and task execution from that branch.
5. Merge after validation.

## Network allowlist considerations

Current setup scripts do not require additional repository-specific allowlist entries beyond Codex Cloud defaults. If that changes in the future, document the required domains in this file.
