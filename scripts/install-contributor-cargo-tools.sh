#!/usr/bin/env bash
# Install the pinned contributor cargo QA tool inventory, or one selected subset of it.

set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/common.sh
. "${script_dir}/common.sh"
script_dir="$(htmlcut_resolve_script_dir "${BASH_SOURCE[0]}")"
readonly script_dir
# shellcheck source=/dev/null
source "${script_dir}/contributor-rust-tools.sh"

export HOME="${HOME:-/home/$(id -un)}"
export CARGO_HOME="${CARGO_HOME:-${HOME}/.cargo}"
export PATH="${CARGO_HOME}/bin:${PATH}"

tool_current_version() {
    local crate_name="$1"
    local binary_name="$2"
    local version_output
    local parsed_version

    if [[ -f "${CARGO_HOME}/.crates.toml" ]]; then
        parsed_version="$(
            awk -v crate_name="${crate_name}" -v binary_name="${binary_name}" '
                index($0, "\"" crate_name " ") == 1 && index($0, "[\"" binary_name "\"]") > 0 {
                    if (match($0, /"[^ ]+ ([0-9]+\.[0-9]+\.[0-9]+) \(/)) {
                        line = substr($0, RSTART, RLENGTH)
                        sub(/^"[^ ]+ /, "", line)
                        sub(/ \($/, "", line)
                        print line
                        exit
                    }
                }
            ' "${CARGO_HOME}/.crates.toml"
        )"
        if [[ -n "${parsed_version}" ]]; then
            printf '%s\n' "${parsed_version}"
            return 0
        fi
    fi

    if [[ "${binary_name}" == cargo-* ]] && command -v cargo >/dev/null 2>&1; then
        # Probe cargo plugins through `cargo <subcommand>` first so persisted plugin installs
        # behave the same way contributors actually invoke them from the shell.
        version_output="$(cargo "${binary_name#cargo-}" --version 2>/dev/null | head -n1 || true)"
        parsed_version="$(grep -Eo '[0-9]+\.[0-9]+\.[0-9]+' <<<"${version_output}" | head -n1 || true)"
        if [[ -n "${parsed_version}" ]]; then
            printf '%s\n' "${parsed_version}"
            return 0
        fi
    fi

    if ! command -v "${binary_name}" >/dev/null 2>&1; then
        return 1
    fi

    version_output="$("${binary_name}" --version 2>/dev/null | head -n1 || true)"
    parsed_version="$(grep -Eo '[0-9]+\.[0-9]+\.[0-9]+' <<<"${version_output}" | head -n1 || true)"
    if [[ -n "${parsed_version}" ]]; then
        printf '%s\n' "${parsed_version}"
        return 0
    fi

    return 1
}

install_tool_if_needed() {
    local crate_name="$1"
    local version="$2"
    local binary_name="$3"
    local current_version

    current_version="$(tool_current_version "${crate_name}" "${binary_name}" || true)"
    if [[ "${current_version}" == "${version}" ]]; then
        return 0
    fi

    cargo install "${crate_name}" --locked --version "${version}" --force
}

while read -r crate_name version binary_name; do
    install_tool_if_needed "${crate_name}" "${version}" "${binary_name}"
done < <(htmlcut_selected_contributor_cargo_tools "$@")
