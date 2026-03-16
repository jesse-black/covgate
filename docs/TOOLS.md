# Tools

Brief guide to tooling available to an agent running in this repository's devcontainer or after running the shared setup script `scripts/agent-env-setup.sh`.

See `docs/reference/environment-execution-contexts.md` for deeper rationale, source references, and setup-decision process details.

## Repo-relevant tooling summary

### Installed by `scripts/agent-env-setup.sh` when missing

- Core CLI/build tools: `jq`, `ripgrep`, `yq`, `fd` (via `fd-find` + symlink), `eza`
- Shell tooling: `shellcheck`, `shfmt`
- C/C++ workflows: `build-essential`, `cmake`, `ninja`, `clang`, `llvm-cov`, `llvm-profdata`
- Swift workflows: Swift via `swiftly`
- .NET workflows: `dotnet` SDK
- Rust workflows: `cargo llvm-cov`, `cargo-machete`, `cargo-deny` (plus `llvm-tools-preview` via `rustup component add` when `rustup` is present)

### Base-ref maintenance for cloud agents

- `scripts/agent-env-setup.sh` installs and updates tools only; it no longer tries to fetch `origin/main`.
- `scripts/agent-env-maintenance.sh` runs `covgate record-base` when available so agent worktrees can record a stable base ref without relying on remote-tracking branches.

### Available in devcontainer

- Core CLI/build tools: `git`, `curl`, `jq`, `ripgrep`, `fd`, `zip/unzip`, `build-essential`
- Rust workflows: `rustup`, `cargo fmt`, `cargo check`, `cargo clippy`, `cargo test`, `cargo llvm-cov`
- Additional quality-of-life tooling: `gh` (GitHub CLI)
