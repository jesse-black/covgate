#!/usr/bin/env bash
set -euo pipefail

SETUP_LABEL="${1:-setup-agent-env}"

if [[ "${CODEX_DEBUG:-}" == "1" || "${CODEX_SETUP_DEBUG:-}" == "1" || "${JULES_DEBUG:-}" == "1" || "${JULES_SETUP_DEBUG:-}" == "1" ]]; then
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
		echo "${SETUP_LABEL}: linked fd -> ${fdfind_path}"
	fi
}

ensure_cargo_tool() {
	local binary_name="$1"
	local package_name="$2"

	if ! cargo "$binary_name" --version >/dev/null 2>&1; then
		echo "${SETUP_LABEL}: installing ${package_name}"
		cargo install "${package_name}" --locked
	else
		echo "${SETUP_LABEL}: ${package_name} already installed"
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
	echo "${SETUP_LABEL}: installed apt packages: ${APT_PACKAGES[*]}"
else
	echo "${SETUP_LABEL}: required apt-managed tools already present; nothing to install."
fi

ensure_fd_command

# Rust workflow tools used by cargo xtask validate.
if need_cmd rustup; then
	echo "${SETUP_LABEL}: rustup not found, skipping rust toolchain setup"
else
	rustup component add llvm-tools-preview || true
fi

ensure_cargo_tool "llvm-cov" "cargo-llvm-cov"
ensure_cargo_tool "machete" "cargo-machete"
ensure_cargo_tool "deny" "cargo-deny"

echo "${SETUP_LABEL}: Complete!"
