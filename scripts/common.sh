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

htmlcut_resolve_repo_path() {
    local helper_repo_root="$1"
    local candidate="$2"

    candidate="$(htmlcut_normalize_bash_path "${candidate}")"
    if [[ "${candidate}" == /* ]]; then
        printf '%s\n' "${candidate}"
    else
        printf '%s\n' "${helper_repo_root}/${candidate}"
    fi
}

htmlcut_cargo_target_dir() {
    local helper_repo_root="$1"

    if [[ -n "${CARGO_TARGET_DIR:-}" ]]; then
        htmlcut_resolve_repo_path "${helper_repo_root}" "${CARGO_TARGET_DIR}"
        return
    fi

    if [[ -f "${helper_repo_root}/Cargo.toml" ]] && command -v cargo >/dev/null 2>&1 && command -v python3 >/dev/null 2>&1; then
        local cargo_metadata_json
        cargo_metadata_json="$(
            cd "${helper_repo_root}" &&
                cargo metadata --format-version 1 --no-deps 2>/dev/null || true
        )"
        local cargo_metadata_target_dir
        cargo_metadata_target_dir="$(
            printf '%s' "${cargo_metadata_json}" |
                python3 -c 'import json, sys; print(json.load(sys.stdin)["target_directory"])' 2>/dev/null || true
        )"
        if [[ -n "${cargo_metadata_target_dir}" ]]; then
            htmlcut_normalize_bash_path "${cargo_metadata_target_dir}"
            return
        fi
    fi

    printf '%s\n' "${helper_repo_root}/target"
}

htmlcut_cargo_compiled_binary_path() {
    local helper_repo_root="$1"
    local target_triple="$2"
    local cargo_profile="$3"
    local binary_name="$4"
    local cargo_target_dir

    cargo_target_dir="$(htmlcut_cargo_target_dir "${helper_repo_root}")"
    printf '%s/%s/%s/%s\n' "${cargo_target_dir%/}" "${target_triple}" "${cargo_profile}" "${binary_name}"
}

htmlcut_host_executable_suffix() {
    case "${OS:-$(uname -s)}" in
        Windows_NT|CYGWIN*|MSYS*|MINGW*) printf '.exe\n' ;;
        *) printf '\n' ;;
    esac
}

htmlcut_cargo_host_binary_path() {
    local helper_repo_root="$1"
    local cargo_profile="$2"
    local binary_name="$3"
    local cargo_target_dir
    local executable_suffix

    cargo_target_dir="$(htmlcut_cargo_target_dir "${helper_repo_root}")"
    executable_suffix="$(htmlcut_host_executable_suffix)"
    printf '%s/%s/%s%s\n' "${cargo_target_dir%/}" "${cargo_profile}" "${binary_name}" "${executable_suffix}"
}
