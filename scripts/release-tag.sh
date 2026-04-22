#!/usr/bin/env bash

set -euo pipefail

htmlcut_resolve_release_tag() {
    local candidate="$1"

    [[ -n "${candidate}" ]] || htmlcut_die "tag name is required"
    printf '%s\n' "${candidate}"
}

htmlcut_assert_release_tag_matches_workspace_version() {
    local tag_name="$1"
    local version="$2"
    local expected_tag="v${version}"

    [[ "${tag_name}" == "${expected_tag}" ]] || htmlcut_die \
        "expected tag ${expected_tag}, got ${tag_name}"
}

if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
    script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
    # shellcheck source=scripts/common.sh
    . "${script_dir}/common.sh"

    script_dir="$(htmlcut_resolve_script_dir "${BASH_SOURCE[0]}")"
    repo_root="$(htmlcut_repo_root_from_script_dir "${script_dir}")"
    candidate="${1:-${RELEASE_TAG:-${GITHUB_REF_NAME:-}}}"
    version="$(htmlcut_workspace_version "${script_dir}" "${repo_root}")"
    tag_name="$(htmlcut_resolve_release_tag "${candidate}")"

    htmlcut_assert_release_tag_matches_workspace_version "${tag_name}" "${version}"
    printf '%s\n' "${tag_name}"
fi
