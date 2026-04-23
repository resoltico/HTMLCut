#!/usr/bin/env bash

set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/common.sh
. "${script_dir}/common.sh"

is_windows_environment() {
    [[ "${OS:-}" == "Windows_NT" ]] || command -v cygpath >/dev/null 2>&1
}

extract_zip_with_powershell() {
    local release_archive_path="$1"
    local release_extract_dir="$2"

    powershell.exe -NoLogo -NoProfile -Command \
        "Expand-Archive -Path '$(cygpath -w "${release_archive_path}")' -DestinationPath '$(cygpath -w "${release_extract_dir}")' -Force"
}

extract_release_archive() {
    local release_archive_path="$1"
    local release_archive_extension="$2"
    local release_extract_dir="$3"

    case "${release_archive_extension}" in
        tar.gz)
            tar -xzf "${release_archive_path}" -C "${release_extract_dir}"
            ;;
        zip)
            if command -v unzip >/dev/null 2>&1; then
                unzip -q "${release_archive_path}" -d "${release_extract_dir}"
                return
            fi

            if is_windows_environment && command -v tar >/dev/null 2>&1; then
                tar -xf "${release_archive_path}" -C "${release_extract_dir}"
                return
            fi

            if command -v powershell.exe >/dev/null 2>&1; then
                extract_zip_with_powershell "${release_archive_path}" "${release_extract_dir}"
                return
            fi

            htmlcut_die "no ZIP extractor found (expected unzip, tar, or powershell.exe)"
            ;;
        *)
            htmlcut_die "unsupported release archive extension: ${release_archive_extension}"
            ;;
    esac
}

main() {
    script_dir="$(htmlcut_resolve_script_dir "${BASH_SOURCE[0]}")"
    readonly script_dir
    repo_root="$(htmlcut_repo_root_from_script_dir "${script_dir}")"
    readonly repo_root
    # shellcheck disable=SC1091
    . "${script_dir}/release-targets.sh"

    target_triple="${1:-}"
    readonly target_triple
    [[ -n "${target_triple}" ]] || htmlcut_die "target triple is required"
    is_supported_release_target "${target_triple}" || htmlcut_die "unsupported release target triple: ${target_triple}"

    version="$(htmlcut_workspace_version "${script_dir}" "${repo_root}")"
    readonly version
    package_name="$(release_package_name_for_target "${version}" "${target_triple}")"
    readonly package_name
    package_dir_name="$(release_package_basename_for_target "${version}" "${target_triple}")"
    readonly package_dir_name
    archive_extension="$(release_archive_extension_for_target "${target_triple}")"
    readonly archive_extension
    binary_name="$(binary_name_for_target "${target_triple}")"
    readonly binary_name
    package_path="${repo_root}/dist/${package_name}"
    readonly package_path
    extract_root="$(mktemp -d "${TMPDIR:-/tmp}/htmlcut-smoke-${target_triple}.XXXXXX")"
    readonly extract_root
    extracted_package_dir="${extract_root}/${package_dir_name}"
    readonly extracted_package_dir
    binary_path="${extracted_package_dir}/${binary_name}"
    readonly binary_path

    cleanup() {
        rm -rf "${extract_root}"
    }

    trap cleanup EXIT

    [[ -f "${package_path}" ]] || htmlcut_die "missing package ${package_path}"

    extract_release_archive "${package_path}" "${archive_extension}" "${extract_root}"

    [[ -f "${binary_path}" ]] || htmlcut_die "missing packaged binary ${binary_path}"
    [[ -f "${extracted_package_dir}/README.md" ]] || htmlcut_die "missing packaged README.md"
    [[ -f "${extracted_package_dir}/LICENSE" ]] || htmlcut_die "missing packaged LICENSE"
    [[ -f "${extracted_package_dir}/NOTICE" ]] || htmlcut_die "missing packaged NOTICE"
    [[ -f "${extracted_package_dir}/PATENTS.md" ]] || htmlcut_die "missing packaged PATENTS.md"

    if [[ "${target_triple}" != x86_64-pc-windows-msvc ]]; then
        [[ -x "${binary_path}" ]] || htmlcut_die "packaged binary is not executable: ${binary_path}"
    fi

    "${binary_path}" --version | tr -d '\r' | grep "^htmlcut ${version}$"
    "${binary_path}" --help | tr -d '\r' | grep "inspect"

    printf 'Smoke-tested %s\n' "${package_name}"
}

if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
    main "$@"
fi
