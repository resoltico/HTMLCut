#!/usr/bin/env bash
# Bootstrap the pinned Rust contributor tooling inside the committed devcontainer.

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
export RUSTUP_HOME="${RUSTUP_HOME:-${HOME}/.rustup}"
export PATH="${CARGO_HOME}/bin:${PATH}"

install_rustup_if_missing() {
    if command -v rustup >/dev/null 2>&1; then
        printf 'devcontainer bootstrap: rustup already available\n'
        return
    fi

    printf 'devcontainer bootstrap: installing rustup\n'
    export RUSTUP_INIT_SKIP_PATH_CHECK=yes
    curl --proto '=https' --tlsv1.2 https://sh.rustup.rs -sSf \
        | sh -s -- -y --profile minimal --default-toolchain none
}

ensure_toolchains() {
    printf 'devcontainer bootstrap: installing stable toolchain %s\n' "${HTMLCUT_CONTRIBUTOR_RUST_STABLE_TOOLCHAIN}"
    rustup toolchain install "${HTMLCUT_CONTRIBUTOR_RUST_STABLE_TOOLCHAIN}" --profile minimal
    printf 'devcontainer bootstrap: installing nightly toolchain %s with %s\n' \
        "${HTMLCUT_CONTRIBUTOR_RUST_NIGHTLY_TOOLCHAIN}" \
        "${HTMLCUT_CONTRIBUTOR_RUST_NIGHTLY_COMPONENTS[*]}"
    htmlcut_contributor_install_nightly_toolchain
    printf 'devcontainer bootstrap: adding %s to %s\n' \
        "${HTMLCUT_CONTRIBUTOR_RUST_STABLE_COMPONENTS[*]}" \
        "${HTMLCUT_CONTRIBUTOR_RUST_STABLE_TOOLCHAIN}"
    htmlcut_contributor_install_stable_toolchain_components
    printf 'devcontainer bootstrap: setting default toolchain to %s\n' "${HTMLCUT_CONTRIBUTOR_RUST_STABLE_TOOLCHAIN}"
    rustup default "${HTMLCUT_CONTRIBUTOR_RUST_STABLE_TOOLCHAIN}"
}

install_rustup_if_missing
ensure_toolchains
printf 'devcontainer bootstrap: installing contributor cargo QA tools\n'
"${script_dir}/install-contributor-cargo-tools.sh"

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
