#!/usr/bin/env bash

set -euo pipefail

script_source="$(printf '%s\n' "${BASH_SOURCE[0]}" | sed 's#\\#/#g')"
if [[ "${script_source}" =~ ^([A-Za-z]):/(.*)$ ]]; then
    script_source="/${BASH_REMATCH[1],,}/${BASH_REMATCH[2]}"
fi
script_dir="$(cd -- "$(dirname -- "${script_source}")" && pwd)"
# shellcheck source=scripts/common.sh
. "${script_dir}/common.sh"

print_usage() {
    local command_name="$1"

    cat <<EOF
Usage: ${command_name} [manifest-path]

Print the [workspace.package] version from one Cargo manifest.

Inputs:
  manifest-path        Optional path to a Cargo.toml file. Defaults to ./Cargo.toml at the
                       repository root. Use - to read the manifest from standard input.
EOF
}

main() {
    local command_name="${BASH_SOURCE[0]}"

    if htmlcut_is_help_flag "${1:-}"; then
        print_usage "${command_name}"
        return 0
    fi

    script_dir="$(htmlcut_resolve_script_dir "${BASH_SOURCE[0]}")"
    repo_root="$(htmlcut_repo_root_from_script_dir "${script_dir}")"
    manifest_path="${1:-${repo_root}/Cargo.toml}"

    python3 - "${manifest_path}" 3<&0 <<'PY'
import io
import os
import pathlib
import re
import sys

manifest_input = sys.argv[1]
if manifest_input == "-":
    manifest_bytes = os.fdopen(3, "rb").read()
    manifest_display_path = "standard input"
else:
    manifest_path = pathlib.Path(manifest_input)
    manifest_bytes = manifest_path.read_bytes()
    manifest_display_path = str(manifest_path)

try:
    import tomllib
except ModuleNotFoundError:
    tomllib = None

if tomllib is not None:
    manifest = tomllib.load(io.BytesIO(manifest_bytes))
    workspace = manifest.get("workspace", {})
    workspace_package = workspace.get("package", {})
    version = workspace_package.get("version")
    if not version:
        print(
            f"error: [workspace.package] version not found in {manifest_display_path}",
            file=sys.stderr,
        )
        raise SystemExit(1)
    print(version)
    raise SystemExit(0)

workspace_package_section = False
version_pattern = re.compile(r'^\s*version\s*=\s*"([^"]+)"\s*$')
for raw_line in manifest_bytes.decode("utf-8").splitlines():
    stripped = raw_line.strip()
    if stripped.startswith("[") and stripped.endswith("]"):
        workspace_package_section = stripped == "[workspace.package]"
        continue
    if not workspace_package_section or not stripped or stripped.startswith("#"):
        continue
    match = version_pattern.match(raw_line)
    if match:
        print(match.group(1))
        raise SystemExit(0)

print(
    f"error: [workspace.package] version not found in {manifest_display_path}",
    file=sys.stderr,
)
raise SystemExit(1)
PY
}

if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
    main "$@"
fi
