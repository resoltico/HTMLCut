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

cargo build -p xtask --locked

compiled_xtask="$(htmlcut_cargo_host_binary_path "${repo_root}" "debug" "xtask")"
[[ -f "${compiled_xtask}" ]] || htmlcut_die "missing compiled xtask binary ${compiled_xtask}"

tmp_root="$(htmlcut_temp_root)"
detached_root="$(TMPDIR="${tmp_root}" mktemp -d -t htmlcut-xtask-XXXXXX)"
readonly detached_root
trap 'rm -rf "${detached_root}"' EXIT

detached_xtask="${detached_root}/$(basename "${compiled_xtask}")"
cp "${compiled_xtask}" "${detached_xtask}"
chmod +x "${detached_xtask}" 2>/dev/null || true

"${detached_xtask}" "$@"
