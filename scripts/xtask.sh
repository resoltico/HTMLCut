#!/usr/bin/env bash
# Stable repo-owned launcher for maintained xtask workflows.

set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/common.sh
. "${script_dir}/common.sh"
script_dir="$(htmlcut_resolve_script_dir "${BASH_SOURCE[0]}")"
readonly script_dir
repo_root="$(htmlcut_repo_root_from_script_dir "${script_dir}")"
readonly repo_root

cd "${repo_root}"

tmp_root="$(htmlcut_temp_root)"
detached_root="$(TMPDIR="${tmp_root}" mktemp -d -t htmlcut-xtask-XXXXXX)"
readonly detached_root
trap 'rm -rf "${detached_root}"' EXIT

launcher_target_dir="${detached_root}/target"
launcher_build_dir="${detached_root}/build"
readonly launcher_target_dir
readonly launcher_build_dir

# Keep the gate driver outside HTMLCut's managed Cargo roots so a clean rebuild cannot create
# unmarked artifacts before xtask's hygiene policy has prepared them.
CARGO_TARGET_DIR="${launcher_target_dir}" \
    CARGO_BUILD_BUILD_DIR="${launcher_build_dir}" \
    cargo build --quiet -p xtask --locked

compiled_xtask="${launcher_target_dir}/debug/xtask$(htmlcut_host_executable_suffix)"
[[ -f "${compiled_xtask}" ]] || htmlcut_die "missing compiled xtask binary ${compiled_xtask}"

"${compiled_xtask}" "$@"
