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
                       repository root.
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

    awk '
BEGIN {
    in_workspace_package = 0
    found = 0
}
/^[[:space:]]*\[workspace\.package\][[:space:]]*$/ {
    in_workspace_package = 1
    next
}
/^[[:space:]]*\[/ {
    if (in_workspace_package) {
        exit
    }
    next
}
/^[[:space:]]*#/ {
    next
}
{
    if (!in_workspace_package) {
        next
    }

    line = $0
    if (line ~ /^[[:space:]]*version[[:space:]]*=[[:space:]]*"/) {
        sub(/^[[:space:]]*version[[:space:]]*=[[:space:]]*"/, "", line)
        sub(/".*$/, "", line)
        print line
        found = 1
        exit
    }
}
END {
    if (!found) {
        print "error: [workspace.package] version not found in " FILENAME > "/dev/stderr"
        exit 1
    }
}
' "${manifest_path}"
}

if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
    main "$@"
fi
