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
htmlcut_install_contributor_cargo_tool() (
    # The subshell owns these variables through EXIT, including Bash 3.2,
    # which discards function-local bindings before running its EXIT trap.
    tool="$1" install_root="$2" succeeded=0
    row="$(htmlcut_contributor_cargo_tool_inventory | awk -v tool="$tool" '$1 == tool')"
    [[ -n "$row" ]] || { printf 'Unknown contributor Cargo tool: %s\n' "$tool" >&2; return 1; }
    read -r name version binary <<< "$row"
    executable="$install_root/bin/$binary"
    if [[ "${OS:-}" == Windows_NT ]]; then executable+='.exe'; fi
    python3 - <<'PY' || return
import runpy, sys
try:
    runpy.run_path('scripts/run-resource-bounded.py')['check_process_ownership']()
except ValueError as error:
    print(str(error), file=sys.stderr)
    sys.exit(2)
PY
    temporary="${RUNNER_TEMP:-${TMPDIR:-/tmp}}"
    if command -v cygpath >/dev/null 2>&1; then temporary="$(cygpath -u "$temporary")"; fi
    build_root=$(mktemp -d "$temporary/htmlcut-cargo-install.XXXXXX") || return
    phase_reports=()
    confirmed_tool_processes() {
        python3 - ${phase_reports[@]+"${phase_reports[@]}"} <<'PY'
import json, sys
try:
    for path in sys.argv[1:]:
        with open(path) as source:
            report = json.load(source)
        if report.get('cleanup_errors', ['unconfirmed']) != [] or type(report.get('returncode')) is not int:
            raise ValueError('process cleanup or exit was not confirmed')
except (OSError, ValueError, TypeError) as error:
    print(str(error), file=sys.stderr)
    sys.exit(1)
PY
    }
    cleanup_installation() {
        cleanup_status=$?
        if confirmed_tool_processes; then
            if [[ "$succeeded" == 1 ]]; then cleanup_paths=("$build_root");
            else cleanup_paths=("$build_root/target" "$build_root/build"); fi
            if ! rm -rf -- "${cleanup_paths[@]}"; then
                printf 'Confirmed tool processes stopped, but temporary cleanup failed; retained at %s\n' "$build_root" >&2
                if [[ "$cleanup_status" -ne 0 ]]; then exit "$cleanup_status"; fi
                exit 125
            fi
        else
            printf 'Tool products retained because child cleanup was not confirmed: %s\n' "$build_root" >&2
            exit 125
        fi
    }
    trap cleanup_installation EXIT
    if [[ ! -x "$executable" ]]; then
        features=()
        if [[ "$name" == cargo-about ]]; then features=(--features cli); fi
        target_directory="$build_root/target"
        build_directory="$build_root/build"
        if command -v cygpath >/dev/null 2>&1; then
            target_directory="$(cygpath -w "$target_directory")"
            build_directory="$(cygpath -w "$build_directory")"
        fi
        phase_reports+=("$build_root/$name-install.resources.json")
        if CARGO_TARGET_DIR="$target_directory" CARGO_BUILD_BUILD_DIR="$build_directory" \
            python3 scripts/run-resource-bounded.py --root "$build_root" \
            --disk-limit-mib 4096 --timeout 900 --log-limit-mib 4 \
            --label "$name-install" -- \
            cargo +"$HTMLCUT_CONTRIBUTOR_RUST_STABLE_TOOLCHAIN" install "$name" --version "$version" --locked --root "$install_root" ${features[@]+"${features[@]}"}; then
            # Do not hold compiler products during the separate version probe.
            confirmed_tool_processes || return 125
            rm -rf -- "$build_root/target" "$build_root/build" || {
                printf 'Confirmed compiler processes stopped, but temporary cleanup failed; retained at %s\n' "$build_root" >&2
                return 125
            }
        else
            result=$?
            printf 'Contributor tool installation failed; bounded log and resource evidence: %s\n' "$build_root" >&2
            tail -n 80 "$build_root/$name-install.log" >&2 || true
            exit "$result"
        fi
    fi
    version_arguments=(--version)
    if [[ "$name" == cargo-mutants ]]; then version_arguments=(mutants --version); fi
    mkdir "$build_root/version" || return
    phase_reports+=("$build_root/version/$name-version.resources.json")
    if python3 scripts/run-resource-bounded.py --root "$build_root/version" \
        --disk-limit-mib 4 --timeout 10 --log-limit-mib 1 --poll-seconds 0.05 \
        --label "$name-version" -- "$executable" "${version_arguments[@]}"; then result=0; else result=$?; fi
    actual="$(cat "$build_root/version/$name-version.log")" || return
    if [[ $result -ne 0 ]]; then
        printf 'Contributor tool version query failed (exit %s): %.512s\n' "$result" "$actual" >&2
        printf 'Installation evidence retained at %s\n' "$build_root" >&2
        return "$result"
    fi
    [[ "$actual" == "$binary $version" ]] || {
        printf 'Contributor tool version mismatch: expected %s %s; got %.512s\n' "$binary" "$version" "$actual" >&2
        printf 'Installation evidence retained at %s\n' "$build_root" >&2
        return 1
    }
    succeeded=1
)
