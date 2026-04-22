#!/usr/bin/env bash

set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/common.sh
. "${script_dir}/common.sh"

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
