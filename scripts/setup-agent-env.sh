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

linux_id_version() {
	if [[ -r /etc/os-release ]]; then
		# shellcheck disable=SC1091
		. /etc/os-release
		if [[ -n "${ID:-}" && -n "${VERSION_ID:-}" ]]; then
			echo "$ID:$VERSION_ID"
			return 0
		fi
	fi

	return 1
}

microsoft_prod_deb_url() {
	local id version
	local id_version
	id_version="$(linux_id_version || true)"

	if [[ -z "$id_version" ]]; then
		return 1
	fi

	id="${id_version%%:*}"
	version="${id_version##*:}"

	case "$id" in
	ubuntu | debian)
		echo "https://packages.microsoft.com/config/${id}/${version}/packages-microsoft-prod.deb"
		return 0
		;;
	esac

	return 1
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

has_pkg() {
	local pkg="$1"
	printf '%s\n' "${APT_PACKAGES[@]}" | grep -qx "$pkg"
}

APT_PACKAGES=()

# Build and coverage tooling for fixture projects.
need_cmd cc && APT_PACKAGES+=(build-essential)
need_cmd c++ && ! has_pkg build-essential && APT_PACKAGES+=(build-essential)
need_cmd cmake && APT_PACKAGES+=(cmake)
need_cmd ninja && APT_PACKAGES+=(ninja-build)
need_cmd clang && APT_PACKAGES+=(clang)
need_cmd llvm-cov && APT_PACKAGES+=(llvm)
need_cmd llvm-profdata && ! has_pkg llvm && APT_PACKAGES+=(llvm)

# Useful agentic tooling
need_cmd dotnet && APT_PACKAGES+=(dotnet-sdk-10.0)
need_cmd jq && APT_PACKAGES+=(jq)
need_cmd rg && APT_PACKAGES+=(ripgrep)
need_cmd yq && APT_PACKAGES+=(yq)
need_cmd fdfind && APT_PACKAGES+=(fd-find)
need_cmd eza && APT_PACKAGES+=(eza)
need_cmd shellcheck && APT_PACKAGES+=(shellcheck)
need_cmd shfmt && APT_PACKAGES+=(shfmt)

if ((${#APT_PACKAGES[@]} > 0)); then
	$SUDO mkdir -p /etc/apt/keyrings

	if has_pkg dotnet-sdk-10.0; then
		if [[ ! -f /etc/apt/sources.list.d/microsoft-prod.list ]]; then
			local_ms_prod_url="$(microsoft_prod_deb_url || true)"
			if [[ -z "${local_ms_prod_url}" ]]; then
				echo "Unable to derive Microsoft package bootstrap URL for this distro." >&2
				exit 1
			fi

			tmp_deb="$(mktemp)"
			curl -fsSL "${local_ms_prod_url}" -o "${tmp_deb}"
			$SUDO dpkg -i "${tmp_deb}"
			rm -f "${tmp_deb}"
		fi
	fi

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
