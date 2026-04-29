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

print_usage() {
    local command_name="$1"

    cat <<EOF
Usage: ${command_name} [tag-name]

Verify the published GitHub release object for one maintained HTMLCut tag.

Inputs:
  tag-name             Optional release tag such as v${version}. Defaults to
                       RELEASE_TAG, then GITHUB_REF_NAME.

Required environment:
  GH_TOKEN             GitHub token accepted by the gh CLI.
EOF
}

main() {
    local command_name="${BASH_SOURCE[0]}"
    script_dir="$(htmlcut_resolve_script_dir "${BASH_SOURCE[0]}")"
    readonly script_dir
    repo_root="$(htmlcut_repo_root_from_script_dir "${script_dir}")"
    readonly repo_root
    # shellcheck disable=SC1091
    . "${script_dir}/release-targets.sh"
    version="$(htmlcut_workspace_version "${script_dir}" "${repo_root}")"
    readonly version

    if htmlcut_is_help_flag "${1:-}"; then
        print_usage "${command_name}"
        return 0
    fi

    tag_name="$(htmlcut_resolve_release_tag "${1:-${RELEASE_TAG:-${GITHUB_REF_NAME:-}}}")"
    readonly tag_name

    [[ -n "${GH_TOKEN:-}" ]] || htmlcut_die "GH_TOKEN is required"
    htmlcut_assert_release_tag_matches_workspace_version "${tag_name}" "${version}"

    release_tag="$(gh release view "${tag_name}" --json tagName --jq '.tagName')"
    [[ "${release_tag}" == "${tag_name}" ]] || htmlcut_die \
        "expected release tag ${tag_name}, got ${release_tag}"

    is_draft="$(gh release view "${tag_name}" --json isDraft --jq '.isDraft')"
    [[ "${is_draft}" == "false" ]] || htmlcut_die "release ${tag_name} is still a draft"

    is_prerelease="$(gh release view "${tag_name}" --json isPrerelease --jq '.isPrerelease')"
    [[ "${is_prerelease}" == "false" ]] || htmlcut_die "release ${tag_name} is marked prerelease"

    mapfile -t expected_assets < <(release_asset_names_for_version "${version}")
    (( ${#expected_assets[@]} > 0 )) || htmlcut_die "release asset inventory is empty"

    for asset_name in "${expected_assets[@]}"; do
        has_asset="$(gh release view "${tag_name}" --json assets --jq \
            ".assets | map(.name) | index(\"${asset_name}\") != null")"
        [[ "${has_asset}" == "true" ]] || htmlcut_die \
            "release ${tag_name} is missing required asset ${asset_name}"
    done

    release_url="$(gh release view "${tag_name}" --json url --jq '.url')"
    printf 'Verified GitHub release handoff: %s\n' "${release_url}"
}

if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
    main "$@"
fi
