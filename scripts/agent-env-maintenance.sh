#!/usr/bin/env bash
set -euo pipefail

SETUP_LABEL="${1:-agent-env-maintenance}"

if [[ "${DEBUG:-}" == "1" ]]; then
	set -x
fi

record_base_ref() {
	if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
		echo "${SETUP_LABEL}: not a git worktree; skipping covgate record-base"
		return 0
	fi

	if ! command -v covgate >/dev/null 2>&1; then
		if ! command -v cargo >/dev/null 2>&1; then
			echo "${SETUP_LABEL}: covgate and cargo not found; skipping covgate record-base" >&2
			return 0
		fi

		if cargo run --quiet -- record-base; then
			echo "${SETUP_LABEL}: recorded stable base ref via cargo run"
		else
			echo "${SETUP_LABEL}: cargo run -- record-base failed; continuing" >&2
		fi
		return 0
	fi

	if covgate record-base; then
		echo "${SETUP_LABEL}: recorded stable base ref"
	else
		echo "${SETUP_LABEL}: covgate record-base failed; continuing" >&2
	fi
}


record_base_ref

echo "${SETUP_LABEL}: Complete!"
