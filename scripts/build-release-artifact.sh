#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0

set -euo pipefail

script_source="$(printf '%s\n' "${BASH_SOURCE[0]}" | sed 's#\\#/#g')"
if [[ "${script_source}" =~ ^([A-Za-z]):/(.*)$ ]]; then
    script_source="/${BASH_REMATCH[1],,}/${BASH_REMATCH[2]}"
fi
script_dir="$(cd -- "$(dirname -- "${script_source}")" && pwd)"
# shellcheck source=scripts/common.sh
. "${script_dir}/common.sh"

is_windows_environment() {
    [[ "${OS:-}" == "Windows_NT" ]] || command -v cygpath >/dev/null 2>&1
}

native_path() {
    local path="$1"

    if command -v cygpath >/dev/null 2>&1; then
        cygpath -w "${path}"
        return
    fi

    printf '%s\n' "${path}"
}

create_zip_with_dotnet() {
    local source_parent_path="$1"
    local package_root_name="$2"
    local archive_output_path="$3"

    # shellcheck disable=SC2016
    env \
        SOURCE_PARENT_PATH="$(native_path "${source_parent_path}")" \
        PACKAGE_ROOT_NAME="${package_root_name}" \
        ARCHIVE_OUTPUT_PATH="$(native_path "${archive_output_path}")" \
        powershell.exe -NoLogo -NoProfile -Command \
        '
        Add-Type -AssemblyName System.IO.Compression
        Add-Type -AssemblyName System.IO.Compression.FileSystem
        $ErrorActionPreference = "Stop"
        $sourceParent = $env:SOURCE_PARENT_PATH
        $packageRoot = $env:PACKAGE_ROOT_NAME
        $archivePath = $env:ARCHIVE_OUTPUT_PATH
        $rootPath = Join-Path $sourceParent $packageRoot

        $archive = [System.IO.Compression.ZipFile]::Open(
            $archivePath,
            [System.IO.Compression.ZipArchiveMode]::Create
        )
        try {
            Get-ChildItem -LiteralPath $rootPath -File -Recurse | ForEach-Object {
                $entryName = $_.FullName.Substring($sourceParent.Length + 1) -replace "\\", "/"
                [System.IO.Compression.ZipFileExtensions]::CreateEntryFromFile(
                    $archive,
                    $_.FullName,
                    $entryName,
                    [System.IO.Compression.CompressionLevel]::Optimal
                ) | Out-Null
            }
        } finally {
            $archive.Dispose()
        }'
}

create_zip_with_7zip() {
    local archiver="$1"
    local source_parent_path="$2"
    local package_root_name="$3"
    local archive_output_path="$4"

    (
        cd "${source_parent_path}"
        "${archiver}" a -tzip -bd -mx=9 "$(native_path "${archive_output_path}")" "${package_root_name}" >/dev/null
    )
}

create_release_archive() {
    local source_parent_path="$1"
    local package_root_name="$2"
    local archive_output_path="$3"
    local archive_output_extension="$4"

    rm -f "${archive_output_path}"

    case "${archive_output_extension}" in
        tar.gz)
            COPYFILE_DISABLE=1 tar -C "${source_parent_path}" -czf "${archive_output_path}" "${package_root_name}"
            ;;
        zip)
            if command -v zip >/dev/null 2>&1; then
                (
                    cd "${source_parent_path}"
                    zip -qr "${archive_output_path}" "${package_root_name}"
                )
                return
            fi

            if is_windows_environment && command -v powershell.exe >/dev/null 2>&1; then
                create_zip_with_dotnet "${source_parent_path}" "${package_root_name}" "${archive_output_path}"
                return
            fi

            local seven_zip_archiver
            for seven_zip_archiver in 7z 7zz 7za; do
                if command -v "${seven_zip_archiver}" >/dev/null 2>&1; then
                    create_zip_with_7zip "${seven_zip_archiver}" "${source_parent_path}" "${package_root_name}" "${archive_output_path}"
                    return
                fi
            done

            if command -v powershell.exe >/dev/null 2>&1; then
                create_zip_with_dotnet "${source_parent_path}" "${package_root_name}" "${archive_output_path}"
                return
            fi

            htmlcut_die "no ZIP archiver found (expected zip, powershell.exe, or 7z)"
            ;;
        *)
            htmlcut_die "unsupported release archive extension: ${archive_output_extension}"
            ;;
    esac
}

packaged_binary_invocation() {
    local binary_name="$1"

    if [[ "${binary_name}" == *.exe ]]; then
        printf '.\\%s\n' "${binary_name}"
    else
        printf './%s\n' "${binary_name}"
    fi
}

write_packaged_readme() {
    local package_dir="$1"
    local version="$2"
    local target_triple="$3"
    local binary_name="$4"
    local binary_command="$5"
    local source_commit="$6"

    cat > "${package_dir}/README.md" <<EOF
# HTMLCut ${version}

This package contains the standalone HTMLCut CLI for \`${target_triple}\`.

## Package Contents

- \`${binary_name}\` — the packaged HTMLCut executable
- \`README.md\` — package-specific install and verification guide
- \`LICENSE\`, \`NOTICE\`, and \`PATENTS.md\` — legal notices for this package

## Run It

Run the binary from this directory or move it onto your PATH.

\`\`\`bash
${binary_command} --version
\`\`\`

## Quick Smoke

Create a tiny fixture and extract one link:

\`\`\`bash
printf '%s\n' '<article><a class="more" href="../guide.html">Read more</a></article>' > ./page.html
${binary_command} extract --file ./page.html --select 'article a.more' --read attr:href --raw
\`\`\`

## Source availability and licensing

Original HTMLCut source is MPL-2.0. Upstream dependencies retain their licenses;
NOTICE contains the complete locked native dependency attribution.

Matching source archives:
- https://github.com/resoltico/HTMLCut/releases/download/v${version}/htmlcut-source-${version}.tar.gz
- https://github.com/resoltico/HTMLCut/releases/download/v${version}/htmlcut-source-${version}.zip

Exact source commit: https://github.com/resoltico/HTMLCut/tree/${source_commit}
Registry dependency source is available at the exact name/version links in NOTICE.

## More

- CLI guide: https://github.com/resoltico/HTMLCut/blob/${source_commit}/docs/cli.md
- Getting started: https://github.com/resoltico/HTMLCut/blob/${source_commit}/README.md
- Core embedding guide: https://github.com/resoltico/HTMLCut/blob/${source_commit}/docs/core.md
EOF
    case "${target_triple}" in
        aarch64-apple-darwin|x86_64-apple-darwin)
            cat >> "${package_dir}/README.md" <<'EOF'

## macOS signature

The executable is signed ad hoc: its signature checks sealed-code integrity, but
provides no Developer ID identity or Apple notarization. Verify the extracted bytes:

```sh
codesign --verify --strict --verbose=2 ./htmlcut
```

A valid ad-hoc signature does not guarantee Gatekeeper acceptance of a quarantined
download. If macOS blocks a download you trust, follow
[Apple's per-application approval guidance](https://support.apple.com/en-us/102445).
Package checksums and GitHub provenance are separate reference checks.
EOF
            ;;
    esac
}

print_usage() {
    local command_name="$1"

    cat <<EOF
Usage: ${command_name} <target-triple>

Build one maintained standalone HTMLCut release artifact under ./dist.

Supported target triples:
$(release_target_triples | sed 's/^/  /')

Use ./scripts/release-targets.sh matrix-json to inspect the canonical release matrix.
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

    local target_triple="${1:-}"
    [[ -n "${target_triple}" ]] || htmlcut_usage_error \
        "${command_name}" \
        "target triple is required"
    is_supported_release_target "${target_triple}" || htmlcut_usage_error \
        "${command_name}" \
        "unsupported release target triple: ${target_triple}"
    case "${target_triple}" in
        aarch64-apple-darwin|x86_64-apple-darwin)
            [[ -x /usr/bin/codesign ]] || htmlcut_die "Apple packages require /usr/bin/codesign; build on macOS with its command-line tools"
            ;;
    esac

    local version
    version="$(htmlcut_workspace_version "${script_dir}" "${repo_root}")"
    readonly version
    local source_commit
    source_commit="$(git -C "${repo_root}" rev-parse --verify HEAD)"
    readonly source_commit
    [[ "${source_commit}" =~ ^[0-9a-f]{40}$ ]] || htmlcut_die "package source must be an exact commit"
    [[ -z "$(git -C "${repo_root}" status --porcelain)" ]] || htmlcut_die "package source must be clean"
    local artifact_name
    artifact_name="$(release_package_name_for_target "${version}" "${target_triple}")"
    readonly artifact_name
    local output_dir="${repo_root}/dist"
    readonly output_dir
    local artifact_path="${output_dir}/${artifact_name}"
    readonly artifact_path
    local cargo_profile="dist"
    readonly cargo_profile
    local deployment_target
    deployment_target="$(macos_deployment_target_for_target "${target_triple}")"
    readonly deployment_target
    local package_dir_name
    package_dir_name="$(release_package_basename_for_target "${version}" "${target_triple}")"
    readonly package_dir_name
    local archive_extension
    archive_extension="$(release_archive_extension_for_target "${target_triple}")"
    readonly archive_extension
    local compiled_binary_name
    compiled_binary_name="$(binary_name_for_target "${target_triple}")"
    readonly compiled_binary_name
    local packaged_binary_command
    packaged_binary_command="$(packaged_binary_invocation "${compiled_binary_name}")"
    readonly packaged_binary_command
    local compiled_binary_path
    compiled_binary_path="$(
        htmlcut_cargo_compiled_binary_path \
            "${repo_root}" \
            "${target_triple}" \
            "${cargo_profile}" \
            "${compiled_binary_name}"
    )"
    readonly compiled_binary_path
    local temp_root
    temp_root="$(htmlcut_temp_root)"
    readonly temp_root
    local staging_root
    staging_root="$(mktemp -d "${temp_root}/htmlcut-release-${target_triple}.XXXXXX")"
    readonly staging_root
    local package_dir="${staging_root}/${package_dir_name}"
    readonly package_dir

    HTMLCUT_RELEASE_ARTIFACT_STAGING_ROOT="${staging_root}"
    readonly HTMLCUT_RELEASE_ARTIFACT_STAGING_ROOT
    trap 'rm -rf -- "${HTMLCUT_RELEASE_ARTIFACT_STAGING_ROOT}"' EXIT

    mkdir -p "${output_dir}"
    rm -f "${artifact_path}"

    (
        cd "${repo_root}"
        if [[ -n "${deployment_target}" ]]; then
            export MACOSX_DEPLOYMENT_TARGET="${deployment_target}"
        fi
        cargo build --profile "${cargo_profile}" --locked -p htmlcut-cli --bin htmlcut --target "${target_triple}"
    )

    mkdir -p "${package_dir}"
    cp "${compiled_binary_path}" "${package_dir}/${compiled_binary_name}"
    if ! is_windows_environment; then
        chmod +x "${package_dir}/${compiled_binary_name}"
    fi
    cp "${repo_root}/LICENSE" "${package_dir}/LICENSE"
    python3 "${repo_root}/scripts/generate-package-notice.py" --target "${target_triple}" \
        --source "${source_commit}" --output "${package_dir}/NOTICE"
    sed '/^<!--$/,/^-->$/d' "${repo_root}/PATENTS.md" > "${package_dir}/PATENTS.md"
    write_packaged_readme \
        "${package_dir}" \
        "${version}" \
        "${target_triple}" \
        "${compiled_binary_name}" \
        "${packaged_binary_command}" \
        "${source_commit}"
    [[ "$(git -C "${repo_root}" rev-parse --verify HEAD)" == "${source_commit}" ]] || htmlcut_die "package source changed during build"
    [[ -z "$(git -C "${repo_root}" status --porcelain)" ]] || htmlcut_die "package source became dirty during build"
    case "${target_triple}" in
        aarch64-apple-darwin|x86_64-apple-darwin)
            /usr/bin/codesign --force --sign - --timestamp=none --identifier htmlcut "${package_dir}/${compiled_binary_name}" \
                || htmlcut_die "ad-hoc signing of staged macOS executable failed"
            python3 "${script_dir}/verify-macos-signature.py" "${package_dir}/${compiled_binary_name}" \
                || htmlcut_die "staged macOS signature failed strict verification or ad-hoc metadata checks"
            ;;
    esac
    create_release_archive "${staging_root}" "${package_dir_name}" "${artifact_path}" "${archive_extension}"

    printf 'Built %s for HTMLCut %s with Cargo profile %s\n' "${artifact_name}" "${version}" "${cargo_profile}"
    if [[ -n "${deployment_target}" ]]; then
        printf 'Pinned macOS deployment target to %s\n' "${deployment_target}"
    fi
    printf 'Wrote %s\n' "${artifact_path}"
}

if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
    main "$@"
fi
