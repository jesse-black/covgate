# Tools

Brief guide to tooling available to an agent running in this repository's devcontainer, in Codex Cloud after running `scripts/setup-codex-cloud.sh`, or in Jules after running `scripts/setup-jules.sh`.

See `docs/reference/environment-execution-contexts.md` for deeper rationale, source references, and setup-decision process details.

## Repo-relevant tooling summary

### Installed by `scripts/setup-codex-cloud.sh` and `scripts/setup-jules.sh` when missing

- Core CLI/build tools: `jq`, `ripgrep`, `yq`, `fd` (via `fd-find` + symlink), `eza`
- Shell tooling: `shellcheck`, `shfmt`
- Rust workflows: `cargo llvm-cov` (plus `llvm-tools-preview` via `rustup component add` when `rustup` is present)

### Available in devcontainer

- Core CLI/build tools: `git`, `curl`, `jq`, `ripgrep`, `fd`, `zip/unzip`, `build-essential`
- Rust workflows: `rustup`, `cargo fmt`, `cargo check`, `cargo clippy`, `cargo test`, `cargo llvm-cov`
- Additional quality-of-life tooling: `gh` (GitHub CLI)
