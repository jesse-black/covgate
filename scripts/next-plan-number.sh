#!/usr/bin/env bash
set -euo pipefail

if [[ "${DEBUG:-}" == "1" ]]; then
	set -x
fi

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd -- "${script_dir}/.." && pwd)"

plan_dirs=(
	"${repo_root}/docs/exec-plans/active"
	"${repo_root}/docs/exec-plans/completed"
)

max=0
shopt -s nullglob

for plan_dir in "${plan_dirs[@]}"; do
	for plan_path in "${plan_dir}"/[0-9][0-9][0-9][0-9]-*.md; do
		plan_file="${plan_path##*/}"
		sequence="${plan_file%%-*}"

		if ((10#${sequence} > max)); then
			max=$((10#${sequence}))
		fi
	done
done

printf "%04d\n" "$((max + 1))"
