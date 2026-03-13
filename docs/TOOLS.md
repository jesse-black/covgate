# Tools

Brief guide to tooling available to an agent running either in a Codex Cloud environment, a Jules environment, or in this repository's devcontainer.

See `docs/references/environment-execution-contexts.md` for deeper rationale, source references, and setup-decision process details.

## Repo-relevant tooling summary

### Available in both environments

- Core CLI/build tools: `git`, `curl`, `jq`, `ripgrep`, `fd`, `zip/unzip`, `build-essential`
- Rust workflows: `rustup`, `cargo fmt`, `cargo check`, `cargo clippy`, `cargo test`, `cargo llvm-cov`

### Installed in devcontainer and bootstrapped for Codex Cloud/Jules

- `cargo-llvm-cov` for coverage checks
- `yq` for structured edits/queries in GitHub Actions workflow YAML files and Markdown frontmatter
- `eza` for filesystem inspection
- `shellcheck` and `shfmt` for shell script quality and formatting in `scripts/` and related automation

### Devcontainer-only by default

- `gh` (GitHub CLI) is useful in local/devcontainer workflows, but Codex Cloud/Jules agents rely on native GitHub integration and do not require `gh`.