#!/usr/bin/env bash
set -euo pipefail

if [[ "${CODEX_DEBUG:-}" == "1" || "${CODEX_SETUP_DEBUG:-}" == "1" ]]; then
	set -x
fi

if [[ "$(id -u)" -eq 0 ]]; then
	SUDO=""
else
	SUDO="sudo"
fi

if ! command -v apt-get >/dev/null 2>&1; then
	echo "This setup script currently supports Debian/Ubuntu environments with apt-get." >&2
	exit 1
fi

export DEBIAN_FRONTEND=noninteractive

need_cmd() {
	local cmd="$1"
	! command -v "$cmd" >/dev/null 2>&1
}

ensure_fd_command() {
	if need_cmd fd && ! need_cmd fdfind; then
		local fdfind_path
		fdfind_path="$(command -v fdfind)"
		$SUDO ln -sf "$fdfind_path" /usr/local/bin/fd
		echo "setup-codex-cloud: linked fd -> ${fdfind_path}"
	fi
}

APT_PACKAGES=()

# Useful agentic tooling
need_cmd jq && APT_PACKAGES+=(jq)
need_cmd rg && APT_PACKAGES+=(ripgrep)
need_cmd yq && APT_PACKAGES+=(yq)
need_cmd fdfind && APT_PACKAGES+=(fd-find)
need_cmd eza && APT_PACKAGES+=(eza)
need_cmd shellcheck && APT_PACKAGES+=(shellcheck)
need_cmd shfmt && APT_PACKAGES+=(shfmt)

if ((${#APT_PACKAGES[@]} > 0)); then
	$SUDO apt-get update
	$SUDO apt-get install -y --no-install-recommends "${APT_PACKAGES[@]}"
	echo "setup-codex-cloud: installed apt packages: ${APT_PACKAGES[*]}"
else
	echo "setup-codex-cloud: required apt-managed tools already present; nothing to install."
fi

ensure_fd_command

# Rust workflow tools
if need_cmd rustup; then
	echo "setup-codex-cloud: rustup not found, skipping rust toolchain setup"
else
	rustup component add llvm-tools-preview || true
fi

if ! cargo llvm-cov --version >/dev/null 2>&1; then
	echo "setup-codex-cloud: installing cargo-llvm-cov"
	cargo install cargo-llvm-cov --locked
else
	echo "setup-codex-cloud: cargo-llvm-cov already installed"
fi

echo "setup-codex-cloud: Complete!"
