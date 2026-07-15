#!/usr/bin/env bash

set -euo pipefail

script_source="$(printf '%s\n' "${BASH_SOURCE[0]}" | sed 's#\\#/#g')"
if [[ "${script_source}" =~ ^([A-Za-z]):/(.*)$ ]]; then
    script_source="/${BASH_REMATCH[1],,}/${BASH_REMATCH[2]}"
fi
script_dir="$(cd -- "$(dirname -- "${script_source}")" && pwd)"
# shellcheck source=scripts/common.sh
. "${script_dir}/common.sh"
# shellcheck source=scripts/release-tag.sh
. "${script_dir}/release-tag.sh"

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
Usage: ${command_name} [tag-name]

Write the canonical SHA-256 checksum manifest for the maintained ./dist release assets.

Inputs:
  tag-name             Optional release tag such as vX.Y.Z. Defaults to RELEASE_TAG,
                       then GITHUB_REF_NAME.
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
    [[ $# -le 1 ]] || htmlcut_usage_error "${command_name}" "expected at most one tag name"

    local tag_name
    tag_name="$(htmlcut_resolve_release_tag "${1:-${RELEASE_TAG:-${GITHUB_REF_NAME:-}}}")"
    readonly tag_name
    local version
    version="$(htmlcut_release_version_for_tag "${script_dir}" "${repo_root}" "${tag_name}")"
    readonly version
    htmlcut_assert_release_tag_matches_workspace_version "${tag_name}" "${version}"
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
