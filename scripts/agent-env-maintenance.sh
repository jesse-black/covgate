#!/usr/bin/env bash
set -euo pipefail

SETUP_LABEL="${1:-agent-env-maintenance}"

if [[ "$(id -u)" -eq 0 ]]; then
	SUDO=""
else
	SUDO="sudo"
fi

if [[ "${CODEX_DEBUG:-}" == "1" || "${CODEX_SETUP_DEBUG:-}" == "1" || "${JULES_DEBUG:-}" == "1" || "${JULES_SETUP_DEBUG:-}" == "1" ]]; then
	set -x
fi

record_base_ref() {
	if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
		echo "${SETUP_LABEL}: not a git worktree; skipping covgate record-base"
		return 0
	fi

	if ! command -v covgate >/dev/null 2>&1; then
		echo "${SETUP_LABEL}: covgate not found; skipping covgate record-base" >&2
		return 0
	fi

	if covgate record-base; then
		echo "${SETUP_LABEL}: recorded stable base ref"
	else
		echo "${SETUP_LABEL}: covgate record-base failed; continuing" >&2
	fi
}

# no-op placeholder to keep compatibility if future maintenance tasks require elevated ops.
: "$SUDO"

record_base_ref

echo "${SETUP_LABEL}: Complete!"
