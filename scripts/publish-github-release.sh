#!/usr/bin/env bash

set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/common.sh
. "${script_dir}/common.sh"
# shellcheck source=scripts/release-tag.sh
. "${script_dir}/release-tag.sh"

release_exists() {
    gh release view "${tag_name}" >/dev/null 2>&1
}

release_has_asset() {
    local asset_name="$1"
    gh release view "${tag_name}" --json assets --jq \
        ".assets | map(.name) | index(\"${asset_name}\") != null"
}

release_is_draft() {
    gh release view "${tag_name}" --json isDraft --jq '.isDraft'
}

publish_release() {
    gh release edit "${tag_name}" \
        --title "${tag_name}" \
        --draft=false \
        --prerelease=false \
        --latest \
        --verify-tag >/dev/null
}

ensure_release_draft_exists() {
    if release_exists; then
        return
    fi

    if gh release create "${tag_name}" \
        --title "${tag_name}" \
        --generate-notes \
        --draft \
        --verify-tag >/dev/null 2>&1; then
        return
    fi

    release_exists || htmlcut_die "failed to create draft release ${tag_name}"
}

ensure_release_is_uploadable() {
    local asset_name="$1"

    if [[ "$(release_is_draft)" == "true" ]]; then
        return
    fi

    if [[ "$(release_has_asset "${asset_name}")" == "true" ]]; then
        return
    fi

    htmlcut_die \
        "release ${tag_name} is already published and missing ${asset_name}; refusing to mutate a published release"
}

upload_if_missing() {
    local asset_path="$1"
    local asset_name
    asset_name="$(basename -- "${asset_path}")"

    [[ -f "${asset_path}" ]] || htmlcut_die "missing asset ${asset_path}"

    if [[ "$(release_has_asset "${asset_name}")" == "true" ]]; then
        return
    fi

    ensure_release_is_uploadable "${asset_name}"

    if gh release upload "${tag_name}" "${asset_path}" >/dev/null 2>&1; then
        return
    fi

    [[ "$(release_has_asset "${asset_name}")" == "true" ]] || htmlcut_die \
        "failed to upload ${asset_name} to release ${tag_name}"
}

print_usage() {
    local command_name="$1"

    cat <<EOF
Usage: ${command_name} [tag-name]

Publish or converge the GitHub release object for one maintained HTMLCut tag.

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

    ensure_release_draft_exists

    mapfile -t expected_assets < <(release_asset_names_for_version "${version}")
    (( ${#expected_assets[@]} > 0 )) || htmlcut_die "release asset inventory is empty"

    for asset_name in "${expected_assets[@]}"; do
        upload_if_missing "${repo_root}/dist/${asset_name}"
    done

    publish_release

    printf 'GitHub release publication converged for %s\n' "${tag_name}"
}

if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
    main "$@"
fi
