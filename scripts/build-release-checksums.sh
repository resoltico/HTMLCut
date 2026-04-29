#!/usr/bin/env bash

set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/common.sh
. "${script_dir}/common.sh"

checksum_line() {
    local file_path="$1"
    local asset_basename="$2"

    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "${file_path}" | awk -v name="${asset_basename}" '{print $1 "  " name}'
        return
    fi

    if command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "${file_path}" | awk -v name="${asset_basename}" '{print $1 "  " name}'
        return
    fi

    htmlcut_die "no SHA-256 checksum tool found (expected sha256sum or shasum)"
}

print_usage() {
    local command_name="$1"

    cat <<EOF
Usage: ${command_name}

Write the canonical SHA-256 checksum manifest for the maintained ./dist release assets.
EOF
}

main() {
    local command_name="${BASH_SOURCE[0]}"
    local script_dir
    script_dir="$(htmlcut_resolve_script_dir "${BASH_SOURCE[0]}")"
    readonly script_dir
    local repo_root
    repo_root="$(htmlcut_repo_root_from_script_dir "${script_dir}")"
    readonly repo_root
    # shellcheck disable=SC1091
    . "${script_dir}/release-targets.sh"

    if htmlcut_is_help_flag "${1:-}"; then
        print_usage "${command_name}"
        return 0
    fi
    [[ $# -eq 0 ]] || htmlcut_usage_error "${command_name}" "this script does not accept arguments"

    local version
    version="$(htmlcut_workspace_version "${script_dir}" "${repo_root}")"
    readonly version
    local output_dir="${repo_root}/dist"
    readonly output_dir
    local manifest_name
    manifest_name="$(release_checksum_manifest_name_for_version "${version}")"
    readonly manifest_name
    local manifest_path="${output_dir}/${manifest_name}"
    readonly manifest_path

    mkdir -p "${output_dir}"
    : > "${manifest_path}"

    mapfile -t expected_assets < <(release_asset_names_for_version "${version}")
    (( ${#expected_assets[@]} > 0 )) || htmlcut_die "release asset inventory is empty"

    for asset_name in "${expected_assets[@]}"; do
        local local_path="${output_dir}/${asset_name}"

        if [[ "${asset_name}" == "${manifest_name}" ]]; then
            continue
        fi

        [[ -f "${local_path}" ]] || htmlcut_die "missing asset ${local_path}"
        checksum_line "${local_path}" "${asset_name}" >> "${manifest_path}"
    done

    printf 'Wrote %s\n' "${manifest_path}"
}

if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
    main "$@"
fi
