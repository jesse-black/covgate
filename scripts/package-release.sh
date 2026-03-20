#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 4 ]]; then
  echo "usage: $0 <binary-path> <version> <target> <archive-format>" >&2
  exit 1
fi

binary_path=$1
version=$2
target=$3
archive_format=$4

if [[ ! -f "$binary_path" ]]; then
  echo "binary not found: $binary_path" >&2
  exit 1
fi

binary_name=$(basename "$binary_path")
stage_dir=${STAGE_DIR:-dist/stage}
out_dir=${OUT_DIR:-dist}
archive_base="covgate-${version}-${target}"
archive_dir="$stage_dir/$archive_base"
archive_dir_abs="$(pwd)/$archive_dir"
out_dir_abs="$(pwd)/$out_dir"

rm -rf "$archive_dir"
mkdir -p "$archive_dir" "$out_dir"
cp "$binary_path" "$archive_dir/$binary_name"
cp README.md LICENSE "$archive_dir/"

case "$archive_format" in
  tar.gz)
    tar -C "$stage_dir" -czf "$out_dir/$archive_base.tar.gz" "$archive_base"
    ;;
  zip)
    if command -v zip >/dev/null 2>&1; then
      (
        cd "$stage_dir"
        rm -f "$OLDPWD/$out_dir/$archive_base.zip"
        zip -qr "$OLDPWD/$out_dir/$archive_base.zip" "$archive_base"
      )
    else
      pwsh -NoLogo -NoProfile -Command \
        "Compress-Archive -Path '$archive_dir_abs' -DestinationPath '$out_dir_abs/$archive_base.zip' -Force"
    fi
    ;;
  *)
    echo "unsupported archive format: $archive_format" >&2
    exit 1
    ;;
esac
