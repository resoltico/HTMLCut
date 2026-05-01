#!/usr/bin/env bash
# Canonical contributor Rust tool inventory shared by bootstrap scripts, docs, and CI.

export HTMLCUT_CONTRIBUTOR_RUST_STABLE_TOOLCHAIN="1.95.0"
export HTMLCUT_CONTRIBUTOR_RUST_NIGHTLY_TOOLCHAIN="nightly"
readonly HTMLCUT_CONTRIBUTOR_RUST_STABLE_TOOLCHAIN
readonly HTMLCUT_CONTRIBUTOR_RUST_NIGHTLY_TOOLCHAIN

htmlcut_contributor_cargo_tool_inventory() {
    cat <<'EOF'
cargo-nextest 0.9.133 cargo-nextest
cargo-audit 0.22.1 cargo-audit
cargo-deny 0.19.4 cargo-deny
cargo-semver-checks 0.47.0 cargo-semver-checks
cargo-outdated 0.19.0 cargo-outdated
cargo-llvm-cov 0.8.5 cargo-llvm-cov
cargo-fuzz 0.13.1 cargo-fuzz
EOF
}

htmlcut_selected_contributor_cargo_tools() {
    if (($# == 0)); then
        htmlcut_contributor_cargo_tool_inventory
        return 0
    fi

    local requested_tool
    local crate_name
    local version
    local binary_name

    for requested_tool in "$@"; do
        local matched=0
        while read -r crate_name version binary_name; do
            if [[ "${requested_tool}" == "${crate_name}" || "${requested_tool}" == "${binary_name}" ]]; then
                printf '%s %s %s\n' "${crate_name}" "${version}" "${binary_name}"
                matched=1
                break
            fi
        done < <(htmlcut_contributor_cargo_tool_inventory)

        if (( matched == 0 )); then
            printf "error: unknown contributor cargo tool '%s'\n" "${requested_tool}" >&2
            return 1
        fi
    done
}

htmlcut_contributor_install_action_csv() {
    local first=1
    local crate_name
    local version
    local binary_name

    while read -r crate_name version binary_name; do
        if (( first )); then
            first=0
        else
            printf ','
        fi
        printf '%s@%s' "${crate_name}" "${version}"
    done < <(htmlcut_selected_contributor_cargo_tools "$@")

    printf '\n'
}
