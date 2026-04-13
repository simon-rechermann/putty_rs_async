#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 6 ]]; then
  echo "usage: $0 <src-binary> <binary-name> <archive-base> <version> <output-dir> <readme-path>" >&2
  exit 1
fi

src_binary="$1"
binary_name="$2"
archive_base="$3"
version="$4"
output_dir="$5"
readme_path="$6"

mkdir -p "$output_dir"

stage_dir="$(mktemp -d)"
trap 'rm -rf "$stage_dir"' EXIT

pkg_dir="${stage_dir}/${archive_base}"
mkdir -p "$pkg_dir"

cp "$src_binary" "${pkg_dir}/${binary_name}"
cp LICENSE "${pkg_dir}/LICENSE"
cp "$readme_path" "${pkg_dir}/README.md"

archive_path="${output_dir}/${archive_base}-${version}.tar.gz"
tar -C "$stage_dir" -czf "$archive_path" "$archive_base"

echo "$archive_path"
