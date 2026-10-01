#!/usr/bin/env bash
# Source archives are always created from one resolved immutable commit, never an advancing HEAD.
set -euo pipefail
script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/common.sh
. "${script_dir}/common.sh"
# shellcheck source=scripts/release-targets.sh
. "${script_dir}/release-targets.sh"
if htmlcut_is_help_flag "${1:-}"; then
    printf 'Usage: %s source-ref [output-directory]\n' "$0"
    exit 0
fi
[[ -n "${1:-}" ]] || htmlcut_die "source ref is required"
source_commit="$(git rev-parse --verify "${1}^{commit}")"
version="$(git show "${source_commit}:Cargo.toml" | "${script_dir}/workspace-version.sh" -)"
[[ "${version}" =~ ^[1-9][0-9]*\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$ ]] || htmlcut_die "stable source version is required"
output_directory="${2:-dist}"
mkdir -p -- "${output_directory}"
basename="$(release_source_archive_basename_for_version "${version}")"
git archive --format=zip --prefix="${basename}/" -o "${output_directory}/${basename}.zip" "${source_commit}"
git archive --format=tar.gz --prefix="${basename}/" -o "${output_directory}/${basename}.tar.gz" "${source_commit}"
printf '%s\n' "${source_commit}" >"${output_directory}/${basename}.source-commit"
