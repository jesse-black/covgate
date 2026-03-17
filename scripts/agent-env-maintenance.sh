#!/usr/bin/env bash
set -euo pipefail

SETUP_LABEL="${1:-agent-env-maintenance}"

if [[ "${DEBUG:-}" == "1" ]]; then
	set -x
fi

record_base_ref() {
	local recorded_sha current_branch marker_path recorded_branch

	if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
		echo "${SETUP_LABEL}: not a git worktree; skipping base ref maintenance"
		return 0
	fi

	current_branch="$(git symbolic-ref --quiet --short HEAD 2>/dev/null || true)"
	marker_path="$(git rev-parse --git-path refs/worktree/covgate/base.branch 2>/dev/null || true)"
	recorded_branch=""
	if [[ -n "${marker_path}" && -r "${marker_path}" ]]; then
		recorded_branch="$(tr -d "\n" < "${marker_path}")"
	fi

	if recorded_sha="$(git rev-parse -q --verify refs/worktree/covgate/base 2>/dev/null)"; then
		if [[ -n "${current_branch}" && -n "${recorded_branch}" && "${current_branch}" != "${recorded_branch}" ]]; then
			if git update-ref refs/worktree/covgate/base HEAD; then
				recorded_sha="$(git rev-parse -q --verify refs/worktree/covgate/base 2>/dev/null || true)"
				if [[ -n "${marker_path}" ]]; then
					mkdir -p "$(dirname "${marker_path}")"
					printf "%s\n" "${current_branch}" > "${marker_path}"
				fi
				echo "${SETUP_LABEL}: refreshed stable base ref at ${recorded_sha} for branch ${current_branch}"
			else
				echo "${SETUP_LABEL}: failed to refresh stable base ref; continuing" >&2
			fi
			return 0
		fi

		echo "${SETUP_LABEL}: stable base ref already existed at ${recorded_sha}"
		if [[ -n "${current_branch}" && -z "${recorded_branch}" && -n "${marker_path}" ]]; then
			mkdir -p "$(dirname "${marker_path}")"
			printf "%s\n" "${current_branch}" > "${marker_path}"
		fi
		return 0
	fi

	if git update-ref refs/worktree/covgate/base HEAD; then
		recorded_sha="$(git rev-parse -q --verify refs/worktree/covgate/base 2>/dev/null || true)"
		if [[ -n "${marker_path}" && -n "${current_branch}" ]]; then
			mkdir -p "$(dirname "${marker_path}")"
			printf "%s\n" "${current_branch}" > "${marker_path}"
		fi
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
