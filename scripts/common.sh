#!/usr/bin/env bash

htmlcut_die() {
    printf 'error: %s\n' "$1" >&2
    exit 1
}

htmlcut_resolve_script_dir() {
    local source_path="$1"

    while [[ -h "${source_path}" ]]; do
        local source_dir
        source_dir="$(cd -P -- "$(dirname -- "${source_path}")" && pwd)"
        source_path="$(readlink "${source_path}")"
        if [[ "${source_path}" != /* ]]; then
            source_path="${source_dir}/${source_path}"
        fi
    done

    cd -P -- "$(dirname -- "${source_path}")" && pwd
}

htmlcut_repo_root_from_script_dir() {
    local helper_script_dir="$1"

    cd -P -- "${helper_script_dir}/.." && pwd
}

htmlcut_workspace_version() {
    local helper_script_dir="$1"
    local helper_repo_root="$2"

    "${helper_script_dir}/workspace-version.sh" "${helper_repo_root}/Cargo.toml"
}
