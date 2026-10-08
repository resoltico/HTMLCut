#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0
# Pinned QA tools; install with cargo install NAME --version VERSION --locked.
HTMLCUT_CONTRIBUTOR_RUST_STABLE_TOOLCHAIN="$(python3 -c 'import tomllib; print(tomllib.load(open("rust-toolchain.toml", "rb"))["toolchain"]["channel"])')"
HTMLCUT_CONTRIBUTOR_RUST_NIGHTLY_TOOLCHAIN="nightly-2026-09-30"
# Immutable source of the published 20.0.0 core and its native install instructions.
HTMLCUT_PUBLISHED_BASELINE_SOURCE="61b4404d83616022fa8d3395ece7e173ecb4f8c7"
htmlcut_contributor_cargo_tool_inventory() {
    cat <<'TOOLS'
cargo-deny 0.20.2 cargo-deny
cargo-about 0.9.2 cargo-about
cargo-semver-checks 0.50.0 cargo-semver-checks
TOOLS
}
