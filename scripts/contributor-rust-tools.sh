#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0
# Nightly is used only for libFuzzer instrumentation, not ordinary test coverage.
# Pinned QA tools; install with cargo install NAME --version VERSION --locked.
HTMLCUT_CONTRIBUTOR_RUST_STABLE_TOOLCHAIN="$(python3 -c 'import tomllib; print(tomllib.load(open("rust-toolchain.toml", "rb"))["toolchain"]["channel"])')"
HTMLCUT_CONTRIBUTOR_RUST_NIGHTLY_TOOLCHAIN="nightly-2026-09-30"
htmlcut_contributor_cargo_tool_inventory() {
    cat <<'TOOLS'
cargo-deny 0.20.2 cargo-deny
cargo-about 0.9.2 cargo-about
cargo-fuzz 0.13.2 cargo-fuzz
cargo-mutants 27.1.0 cargo-mutants
TOOLS
}

# Cache only the selected build inputs: unrelated inventory changes do not
# invalidate other tools. Runner OS/architecture/image remain part of the key.
htmlcut_contributor_cargo_tool_fingerprint() {
    local tool="$1" row
    row="$(htmlcut_contributor_cargo_tool_inventory | awk -v tool="$tool" '$1 == tool')"
    [[ -n "$row" ]] || { printf 'Unknown contributor Cargo tool: %s\n' "$tool" >&2; return 1; }
    {
        printf '%s\n' "$row"
        declare -f htmlcut_install_contributor_cargo_tool
    } | python3 -c 'import hashlib, sys; from pathlib import Path; h=hashlib.sha256(sys.stdin.buffer.read()); h.update(Path("rust-toolchain.toml").read_bytes()); print(h.hexdigest())'
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
        (
            local temporary="${RUNNER_TEMP:-${TMPDIR:-/tmp}}" build_root target_directory build_directory result succeeded=0
            if command -v cygpath >/dev/null 2>&1; then temporary="$(cygpath -u "$temporary")"; fi
            build_root=$(mktemp -d "$temporary/htmlcut-cargo-install.XXXXXX") || exit
            target_directory="$build_root/target"
            build_directory="$build_root/build"
            if command -v cygpath >/dev/null 2>&1; then
                target_directory="$(cygpath -w "$target_directory")"
                build_directory="$(cygpath -w "$build_directory")"
            fi
            # Only these freshly owned compiler products are removed, including
            # failed installations. Never inherit an owner's build-directory override.
            cleanup_installation() {
                if python3 -c 'import json, sys; from pathlib import Path; p=Path(sys.argv[1]); sys.exit(0 if p.is_file() and not json.loads(p.read_text()).get("cleanup_errors", ["unconfirmed"]) else 1)' "$build_root/$name-install.resources.json"; then
                    if [[ "$succeeded" == 1 ]]; then rm -rf -- "$build_root";
                    else rm -rf -- "$build_root/target" "$build_root/build"; fi
                else
                    printf 'Compiler products retained because child cleanup was not confirmed: %s\n' "$build_root" >&2
                    exit 125
                fi
            }
            trap cleanup_installation EXIT
            if CARGO_TARGET_DIR="$target_directory" CARGO_BUILD_BUILD_DIR="$build_directory" \
                python3 scripts/run-resource-bounded.py --root "$build_root" \
                --disk-limit-mib 4096 --timeout 900 --log-limit-mib 4 \
                --label "$name-install" -- \
                cargo +"$HTMLCUT_CONTRIBUTOR_RUST_STABLE_TOOLCHAIN" install "$name" --version "$version" --locked --root "$install_root" "${features[@]}"; then
                succeeded=1
            else
                result=$?
                printf 'Contributor tool installation failed; bounded log and resource evidence: %s\n' "$build_root" >&2
                tail -n 80 "$build_root/$name-install.log" >&2 || true
                exit "$result"
            fi
        ) || return
    fi
    actual="$("$executable" --version)" || return
    [[ "$actual" == "$binary $version" ]] || {
        printf 'Contributor tool version mismatch: expected %s %s; got %s\n' "$binary" "$version" "$actual" >&2
        return 1
    }
}
