#!/usr/bin/env bash
# Bootstrap the pinned Rust contributor tooling inside the committed devcontainer.

set -euo pipefail

htmlcut_devcontainer_bootstrap_script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/common.sh
. "${htmlcut_devcontainer_bootstrap_script_dir}/common.sh"
htmlcut_devcontainer_bootstrap_script_dir="$(htmlcut_resolve_script_dir "${BASH_SOURCE[0]}")"
readonly htmlcut_devcontainer_bootstrap_script_dir
# shellcheck source=/dev/null
source "${htmlcut_devcontainer_bootstrap_script_dir}/contributor-rust-tools.sh"

export HOME="${HOME:-/home/$(id -un)}"
export CARGO_HOME="${CARGO_HOME:-${HOME}/.cargo}"
export RUSTUP_HOME="${RUSTUP_HOME:-${HOME}/.rustup}"
export PATH="${CARGO_HOME}/bin:${PATH}"

retry_command() {
    local attempts="$1"
    local delay_seconds="$2"
    shift 2

    local attempt=1
    until "$@"; do
        if (( attempt >= attempts )); then
            return 1
        fi

        printf 'devcontainer bootstrap: retrying failed command (%s/%s) in %ss: %s\n' \
            "${attempt}" \
            "${attempts}" \
            "${delay_seconds}" \
            "$*" >&2
        sleep "${delay_seconds}"
        attempt=$((attempt + 1))
    done
}

install_rustup_once() {
    export RUSTUP_INIT_SKIP_PATH_CHECK=yes
    curl --proto '=https' --tlsv1.2 https://sh.rustup.rs -sSf \
        | sh -s -- -y --profile minimal --default-toolchain none
}

install_rustup_if_missing() {
    if command -v rustup >/dev/null 2>&1; then
        printf 'devcontainer bootstrap: rustup already available\n'
        return
    fi

    printf 'devcontainer bootstrap: installing rustup\n'
    retry_command 3 5 install_rustup_once
}

ensure_toolchains() {
    printf 'devcontainer bootstrap: installing stable toolchain %s\n' "${HTMLCUT_CONTRIBUTOR_RUST_STABLE_TOOLCHAIN}"
    retry_command 3 5 rustup toolchain install "${HTMLCUT_CONTRIBUTOR_RUST_STABLE_TOOLCHAIN}" --profile minimal
    printf 'devcontainer bootstrap: installing nightly toolchain %s with %s\n' \
        "${HTMLCUT_CONTRIBUTOR_RUST_NIGHTLY_TOOLCHAIN}" \
        "${HTMLCUT_CONTRIBUTOR_RUST_NIGHTLY_COMPONENTS[*]}"
    retry_command 3 5 htmlcut_contributor_install_nightly_toolchain
    printf 'devcontainer bootstrap: adding %s to %s\n' \
        "${HTMLCUT_CONTRIBUTOR_RUST_STABLE_COMPONENTS[*]}" \
        "${HTMLCUT_CONTRIBUTOR_RUST_STABLE_TOOLCHAIN}"
    retry_command 3 5 htmlcut_contributor_install_stable_toolchain_components
    printf 'devcontainer bootstrap: setting default toolchain to %s\n' "${HTMLCUT_CONTRIBUTOR_RUST_STABLE_TOOLCHAIN}"
    rustup default "${HTMLCUT_CONTRIBUTOR_RUST_STABLE_TOOLCHAIN}"
}

install_rustup_if_missing
ensure_toolchains
printf 'devcontainer bootstrap: installing contributor cargo QA tools\n'
"${htmlcut_devcontainer_bootstrap_script_dir}/install-contributor-cargo-tools.sh"

printf 'devcontainer bootstrap: validating installed toolchain surfaces\n'
rustc --version >/dev/null
cargo --version >/dev/null
cargo nextest --version >/dev/null
cargo audit --version >/dev/null
cargo deny --version >/dev/null
cargo semver-checks --version >/dev/null
cargo outdated --version >/dev/null
cargo llvm-cov --version >/dev/null
cargo +nightly miri --version >/dev/null
cargo fuzz --version >/dev/null
printf 'devcontainer bootstrap: ready\n'
