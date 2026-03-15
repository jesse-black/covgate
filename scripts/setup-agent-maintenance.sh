#!/usr/bin/env bash
set -euo pipefail

SETUP_LABEL="${1:-setup-agent-maintenance}"

if [[ "$(id -u)" -eq 0 ]]; then
	SUDO=""
else
	SUDO="sudo"
fi

if [[ "${CODEX_DEBUG:-}" == "1" || "${CODEX_SETUP_DEBUG:-}" == "1" || "${JULES_DEBUG:-}" == "1" || "${JULES_SETUP_DEBUG:-}" == "1" ]]; then
	set -x
fi

ensure_origin_main_ref() {
	if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
		echo "${SETUP_LABEL}: not a git worktree; skipping origin/main bootstrap"
		return 0
	fi

	if ! git remote get-url origin >/dev/null 2>&1; then
		echo "${SETUP_LABEL}: origin remote missing; skipping origin/main bootstrap"
		return 0
	fi

	if git fetch --no-tags --depth=1 origin +refs/heads/main:refs/remotes/origin/main >/dev/null 2>&1; then
		echo "${SETUP_LABEL}: fetched origin/main (depth=1)"
	else
		echo "${SETUP_LABEL}: failed to fetch origin/main (depth=1); continuing" >&2
	fi
}

# no-op placeholder to keep compatibility if future maintenance tasks require elevated ops.
: "$SUDO"

ensure_origin_main_ref

echo "${SETUP_LABEL}: Complete!"
