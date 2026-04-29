#!/usr/bin/env bash

set -euo pipefail

htmlcut_resolve_release_tag() {
    local candidate="$1"

    [[ -n "${candidate}" ]] || htmlcut_die "tag name is required"
    printf '%s\n' "${candidate}"
}

htmlcut_assert_release_tag_matches_workspace_version() {
    local helper_tag_name="$1"
    local helper_version="$2"
    local expected_tag="v${helper_version}"

    [[ "${helper_tag_name}" == "${expected_tag}" ]] || htmlcut_die \
        "expected tag ${expected_tag}, got ${helper_tag_name}"
}

print_usage() {
    local command_name="$1"

    cat <<EOF
Usage: ${command_name} [tag-name]

Validate one release tag against the current workspace version and print the resolved tag.

Inputs:
  tag-name             Optional release tag such as vX.Y.Z. Defaults to RELEASE_TAG,
                       then GITHUB_REF_NAME.
EOF
}

if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
    script_source="$(printf '%s\n' "${BASH_SOURCE[0]}" | sed 's#\\#/#g')"
    if [[ "${script_source}" =~ ^([A-Za-z]):/(.*)$ ]]; then
        script_source="/${BASH_REMATCH[1],,}/${BASH_REMATCH[2]}"
    fi
    script_dir="$(cd -- "$(dirname -- "${script_source}")" && pwd)"
    # shellcheck source=scripts/common.sh
    . "${script_dir}/common.sh"

    if htmlcut_is_help_flag "${1:-}"; then
        print_usage "${BASH_SOURCE[0]}"
        exit 0
    fi

    script_dir="$(htmlcut_resolve_script_dir "${BASH_SOURCE[0]}")"
    repo_root="$(htmlcut_repo_root_from_script_dir "${script_dir}")"
    candidate="${1:-${RELEASE_TAG:-${GITHUB_REF_NAME:-}}}"
    version="$(htmlcut_workspace_version "${script_dir}" "${repo_root}")"
    tag_name="$(htmlcut_resolve_release_tag "${candidate}")"

    htmlcut_assert_release_tag_matches_workspace_version "${tag_name}" "${version}"
    printf '%s\n' "${tag_name}"
fi
