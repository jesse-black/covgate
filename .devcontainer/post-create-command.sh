#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "${SCRIPT_DIR}"

nix run "path:${SCRIPT_DIR}#homeConfigurations.vscode.activationPackage"

export PATH="${HOME}/.nix-profile/bin:${HOME}/.cargo/bin:${PATH}"

cd "${WORKSPACE_FOLDER:-${SCRIPT_DIR}/..}"

if ! rustup show active-toolchain >/dev/null 2>&1; then
  rustup toolchain install
fi

if ! command -v covgate >/dev/null 2>&1; then
  cargo install covgate --locked
fi
