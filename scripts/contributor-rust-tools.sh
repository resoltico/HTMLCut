#!/usr/bin/env bash
# Canonical contributor Rust toolchain and cargo-tool inventory shared by bootstrap scripts, docs,
# and CI.

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/common.sh
. "${script_dir}/common.sh"
repo_root="$(htmlcut_repo_root_from_script_dir "${script_dir}")"

HTMLCUT_CONTRIBUTOR_RUST_STABLE_TOOLCHAIN="$(
    python3 - <<'PY' "${repo_root}/rust-toolchain.toml"
import pathlib
import re
import sys

manifest_path = pathlib.Path(sys.argv[1])
try:
    import tomllib
except ModuleNotFoundError:
    tomllib = None

if tomllib is not None:
    with manifest_path.open("rb") as handle:
        manifest = tomllib.load(handle)
    print(manifest["toolchain"]["channel"])
    raise SystemExit(0)

toolchain_section = False
channel_pattern = re.compile(r'^\s*channel\s*=\s*"([^"]+)"\s*$')
for raw_line in manifest_path.read_text(encoding="utf-8").splitlines():
    stripped = raw_line.strip()
    if stripped.startswith("[") and stripped.endswith("]"):
        toolchain_section = stripped == "[toolchain]"
        continue
    if not toolchain_section or not stripped or stripped.startswith("#"):
        continue
    match = channel_pattern.match(raw_line)
    if match:
        print(match.group(1))
        raise SystemExit(0)

print(f"error: toolchain channel not found in {manifest_path}", file=sys.stderr)
raise SystemExit(1)
PY
)"
export HTMLCUT_CONTRIBUTOR_RUST_STABLE_TOOLCHAIN
HTMLCUT_CONTRIBUTOR_RUST_NIGHTLY_TOOLCHAIN="nightly"
export HTMLCUT_CONTRIBUTOR_RUST_NIGHTLY_TOOLCHAIN
readonly HTMLCUT_CONTRIBUTOR_RUST_STABLE_TOOLCHAIN
readonly HTMLCUT_CONTRIBUTOR_RUST_NIGHTLY_TOOLCHAIN
readonly -a HTMLCUT_CONTRIBUTOR_RUST_STABLE_COMPONENTS=("clippy" "rustfmt")
readonly -a HTMLCUT_CONTRIBUTOR_RUST_NIGHTLY_COMPONENTS=(
    "llvm-tools-preview"
    "miri"
    "rust-src"
)

htmlcut_contributor_rustup_toolchain_install() {
    local toolchain="$1"
    shift

    local rustup_args=("${toolchain}" "--profile" "minimal")
    local component
    for component in "$@"; do
        rustup_args+=("--component" "${component}")
    done

    rustup toolchain install "${rustup_args[@]}"
}

htmlcut_contributor_install_nightly_toolchain() {
    htmlcut_contributor_rustup_toolchain_install \
        "${HTMLCUT_CONTRIBUTOR_RUST_NIGHTLY_TOOLCHAIN}" \
        "${HTMLCUT_CONTRIBUTOR_RUST_NIGHTLY_COMPONENTS[@]}"
}

htmlcut_contributor_install_stable_toolchain_components() {
    rustup component add \
        "${HTMLCUT_CONTRIBUTOR_RUST_STABLE_COMPONENTS[@]}" \
        --toolchain "${HTMLCUT_CONTRIBUTOR_RUST_STABLE_TOOLCHAIN}"
}

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
        local matched
        matched=0
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
