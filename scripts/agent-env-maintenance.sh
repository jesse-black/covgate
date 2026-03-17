#!/usr/bin/env bash
set -euo pipefail

SETUP_LABEL="${1:-agent-env-maintenance}"

if [[ "${DEBUG:-}" == "1" ]]; then
	set -x
fi

record_base_ref() {
	local recorded_sha

	if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
		echo "${SETUP_LABEL}: not a git worktree; skipping base ref maintenance"
		return 0
	fi

	if recorded_sha="$(git rev-parse -q --verify refs/worktree/covgate/base 2>/dev/null)"; then
		echo "${SETUP_LABEL}: stable base ref already existed at ${recorded_sha}"
		return 0
	fi

	if git update-ref refs/worktree/covgate/base HEAD; then
		recorded_sha="$(git rev-parse -q --verify refs/worktree/covgate/base 2>/dev/null || true)"
		if [[ -n "${recorded_sha}" ]]; then
			echo "${SETUP_LABEL}: created stable base ref at ${recorded_sha}"
		else
			echo "${SETUP_LABEL}: created stable base ref"
		fi
	else
		echo "${SETUP_LABEL}: failed to record stable base ref; continuing" >&2
	fi
}


record_base_ref

echo "${SETUP_LABEL}: Complete!"
