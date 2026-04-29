#!/usr/bin/env bash

htmlcut_die() {
    printf 'error: %s\n' "$1" >&2
    exit 1
}

htmlcut_is_help_flag() {
    local candidate="${1:-}"

    [[ "${candidate}" == "-h" || "${candidate}" == "--help" ]]
}

htmlcut_usage_error() {
    local command_name="$1"
    local message="$2"

    printf 'error: %s\n' "${message}" >&2
    printf 'Run %s --help for usage.\n' "${command_name}" >&2
    exit 1
}

htmlcut_normalize_bash_path() {
    local candidate="$1"

    candidate="${candidate//\\//}"
    if [[ "${candidate}" =~ ^([A-Za-z]):/(.*)$ ]]; then
        local drive_letter="${BASH_REMATCH[1],,}"
        local remainder="${BASH_REMATCH[2]}"
        if [[ -n "${remainder}" ]]; then
            candidate="/${drive_letter}/${remainder}"
        else
            candidate="/${drive_letter}"
        fi
    fi

    printf '%s\n' "${candidate}"
}

htmlcut_resolve_script_dir() {
    local source_path="$1"

    source_path="$(htmlcut_normalize_bash_path "${source_path}")"
    while [[ -h "${source_path}" ]]; do
        local source_dir
        source_dir="$(cd -P -- "$(dirname -- "${source_path}")" && pwd)"
        source_path="$(readlink "${source_path}")"
        if [[ "${source_path}" != /* ]]; then
            source_path="${source_dir}/${source_path}"
        fi
        source_path="$(htmlcut_normalize_bash_path "${source_path}")"
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

htmlcut_temp_root() {
    local candidate="${RUNNER_TEMP:-${TMPDIR:-${TEMP:-${TMP:-/tmp}}}}"

    if command -v cygpath >/dev/null 2>&1; then
        case "${candidate}" in
            [A-Za-z]:\\*|[A-Za-z]:/*)
                candidate="$(cygpath -u "${candidate}")"
                ;;
        esac
    fi

    printf '%s\n' "${candidate}"
}
