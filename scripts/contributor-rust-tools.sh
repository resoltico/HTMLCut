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
cargo-fuzz 0.13.2 cargo-fuzz
cargo-semver-checks 0.50.0 cargo-semver-checks
TOOLS
}

# Install roots contain only one pinned tool, keeping caches away from rustup and products.
htmlcut_install_contributor_cargo_tool() {
    local tool="$1" install_root="$2" row name version binary actual
    row="$(htmlcut_contributor_cargo_tool_inventory | awk -v tool="$tool" '$1 == tool')"
    [[ -n "$row" ]] || { printf 'Unknown contributor Cargo tool: %s\n' "$tool" >&2; return 1; }
    read -r name version binary <<< "$row"
    local executable="$install_root/bin/$binary"
    if [[ "${OS:-}" == Windows_NT ]]; then executable+='.exe'; fi
    if [[ ! -x "$executable" ]]; then
        local features=()
        if [[ "$name" == cargo-about ]]; then features=(--features cli); fi
        cargo +"$HTMLCUT_CONTRIBUTOR_RUST_STABLE_TOOLCHAIN" install "$name" --version "$version" --locked --root "$install_root" "${features[@]}" || return
    fi
    actual="$("$executable" --version)" || return
    [[ "$actual" == "$binary $version" ]] || {
        printf 'Contributor tool version mismatch: expected %s %s; got %s\n' "$binary" "$version" "$actual" >&2
        return 1
    }
}
