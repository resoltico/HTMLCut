#!/usr/bin/env bash
# Install the pinned contributor cargo QA tool inventory, or one selected subset of it.

set -euo pipefail

htmlcut_install_contributor_cargo_tools_script_dir="$(
    cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd
)"
# shellcheck source=scripts/common.sh
. "${htmlcut_install_contributor_cargo_tools_script_dir}/common.sh"
htmlcut_install_contributor_cargo_tools_script_dir="$(
    htmlcut_resolve_script_dir "${BASH_SOURCE[0]}"
)"
readonly htmlcut_install_contributor_cargo_tools_script_dir
# shellcheck source=/dev/null
source "${htmlcut_install_contributor_cargo_tools_script_dir}/contributor-rust-tools.sh"

export HOME="${HOME:-/home/$(id -un)}"
export CARGO_HOME="${CARGO_HOME:-${HOME}/.cargo}"
export PATH="${CARGO_HOME}/bin:${PATH}"

ensure_native_prerequisites() {
    if [[ "$(uname -s)" != "Darwin" ]]; then
        return 0
    fi

    if ! command -v pkg-config >/dev/null 2>&1; then
        cat >&2 <<'EOF'
error: missing pkg-config on macOS
install Homebrew pkgconf and openssl@3 first:
  brew install pkgconf openssl@3
EOF
        return 1
    fi

    if pkg-config --exists openssl; then
        return 0
    fi

    if command -v brew >/dev/null 2>&1; then
        local openssl_prefix
        openssl_prefix="$(brew --prefix openssl@3 2>/dev/null || true)"
        if [[ -n "${openssl_prefix}" && -d "${openssl_prefix}/lib/pkgconfig" ]]; then
            export PKG_CONFIG_PATH="${openssl_prefix}/lib/pkgconfig${PKG_CONFIG_PATH:+:${PKG_CONFIG_PATH}}"
        fi
    fi

    if pkg-config --exists openssl; then
        return 0
    fi

    cat >&2 <<'EOF'
error: OpenSSL development metadata is unavailable for cargo tool builds
install Homebrew pkgconf and openssl@3 first:
  brew install pkgconf openssl@3
If they are already installed, export PKG_CONFIG_PATH to the openssl@3 pkgconfig directory and rerun.
EOF
    return 1
}

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
        printf 'contributor cargo tool: %s %s already installed as %s\n' "${binary_name}" "${version}" "${current_version}"
        return 0
    fi

    if [[ -n "${current_version}" ]]; then
        printf 'contributor cargo tool: updating %s from %s to %s\n' "${binary_name}" "${current_version}" "${version}"
    else
        printf 'contributor cargo tool: installing %s %s\n' "${binary_name}" "${version}"
    fi
    cargo install "${crate_name}" --locked --version "${version}" --force
}

ensure_native_prerequisites

while read -r crate_name version binary_name; do
    install_tool_if_needed "${crate_name}" "${version}" "${binary_name}"
done < <(htmlcut_selected_contributor_cargo_tools "$@")
