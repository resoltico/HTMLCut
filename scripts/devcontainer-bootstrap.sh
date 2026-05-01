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
        return
    fi

    export RUSTUP_INIT_SKIP_PATH_CHECK=yes
    curl --proto '=https' --tlsv1.2 https://sh.rustup.rs -sSf \
        | sh -s -- -y --profile minimal --default-toolchain none
}

ensure_toolchains() {
    rustup toolchain install "${HTMLCUT_CONTRIBUTOR_RUST_STABLE_TOOLCHAIN}" --profile minimal
    rustup toolchain install "${HTMLCUT_CONTRIBUTOR_RUST_NIGHTLY_TOOLCHAIN}" --profile minimal --component llvm-tools-preview
    rustup component add clippy rustfmt --toolchain "${HTMLCUT_CONTRIBUTOR_RUST_STABLE_TOOLCHAIN}"
    rustup default "${HTMLCUT_CONTRIBUTOR_RUST_STABLE_TOOLCHAIN}"
}

install_rustup_if_missing
ensure_toolchains
"${script_dir}/install-contributor-cargo-tools.sh"

rustc --version >/dev/null
cargo --version >/dev/null
cargo nextest --version >/dev/null
cargo audit --version >/dev/null
cargo deny --version >/dev/null
cargo semver-checks --version >/dev/null
cargo outdated --version >/dev/null
cargo llvm-cov --version >/dev/null
cargo fuzz --version >/dev/null
