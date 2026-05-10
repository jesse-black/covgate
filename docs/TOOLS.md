---
description: "Agent tooling reference for the covgate devcontainer and agent-env-setup.sh; read when checking which CLI tools are available, how to install missing tools, or setting up a new execution environment."
---

# Tools

Brief guide to tooling available to an agent running in this repository's devcontainer or after running the shared setup script `scripts/agent-env-setup.sh`.

## Repo-relevant tooling summary

### Installed by `scripts/agent-env-setup.sh` when missing

- Core CLI/build tools: `jq`, `ripgrep`, Mike Farah `yq`, `fd` (via `fd-find` + symlink), `eza`
- Shell tooling: `shellcheck`, `shfmt`
- C/C++ workflows: `build-essential`, `cmake`, `ninja`, `clang`, `llvm-cov`, `llvm-profdata`
- Swift workflows: Swift via `swiftly`
- .NET workflows: `dotnet` SDK
- Rust workflows: `covgate`, `cargo llvm-cov`, `cargo-machete`, `cargo-deny` (plus `llvm-tools-preview` via `rustup component add` when `rustup` is present)

### Available in devcontainer

- Core CLI/build tools: `git`, `curl`, `jq`, `ripgrep`, `fd`, `zip/unzip`, `build-essential`
- Rust workflows: `rustup`, `cargo fmt`, `cargo check`, `cargo clippy`, `cargo test`, `cargo llvm-cov`
- Additional quality-of-life tooling: `gh` (GitHub CLI)
