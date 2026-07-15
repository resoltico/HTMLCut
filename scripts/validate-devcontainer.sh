#!/usr/bin/env bash
# Build-time and contract-level validation for the committed HTMLCut contributor devcontainer.

set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/common.sh
. "${script_dir}/common.sh"
script_dir="$(htmlcut_resolve_script_dir "${BASH_SOURCE[0]}")"
readonly script_dir
repo_root="$(htmlcut_repo_root_from_script_dir "${script_dir}")"
readonly repo_root
readonly dockerfile_path="${repo_root}/.devcontainer/Dockerfile"
readonly config_path="${repo_root}/.devcontainer/devcontainer.json"
readonly stable_toolchain_manifest="${repo_root}/rust-toolchain.toml"
readonly helper_dockerfile_path="${repo_root}/scripts/devcontainer-cli-helper.Dockerfile"
readonly bootstrap_script="${repo_root}/scripts/devcontainer-bootstrap.sh"
readonly prepare_script="${repo_root}/scripts/devcontainer-prepare-user-home.sh"
readonly helper_image_tag="htmlcut-devcontainer-cli-helper:local"
readonly volume_mode="${HTMLCUT_DEVCONTAINER_VOLUME_MODE:-isolated}"
readonly repo_command_probe_mode="${HTMLCUT_DEVCONTAINER_REPO_COMMAND_PROBES:-full}"

stable_toolchain_channel="$(sed -nE 's/^[[:space:]]*channel[[:space:]]*=[[:space:]]*"([^"]+)".*/\1/p' "${stable_toolchain_manifest}")"
[[ -n "${stable_toolchain_channel}" ]] || htmlcut_die "missing pinned Rust channel in ${stable_toolchain_manifest}"
readonly stable_toolchain_channel

assert_repo_command_probe_mode() {
    case "${repo_command_probe_mode}" in
        full|skip) ;;
        *)
            htmlcut_die \
                "unsupported HTMLCUT_DEVCONTAINER_REPO_COMMAND_PROBES=${repo_command_probe_mode}; expected full or skip"
            ;;
    esac
}

select_volume_name() {
    local shared_name="$1"
    local isolated_name="$2"

    case "${volume_mode}" in
        isolated)
            printf '%s\n' "${isolated_name}"
            ;;
        shared-ci)
            printf '%s\n' "${shared_name}"
            ;;
        *)
            htmlcut_die \
                "unsupported HTMLCUT_DEVCONTAINER_VOLUME_MODE=${volume_mode}; expected isolated or shared-ci"
            ;;
    esac
}

validate_inner_runtime() {
    [[ "${HTMLCUT_DEVCONTAINER:-}" == "1" ]] || htmlcut_die "inner runtime mode requires HTMLCUT_DEVCONTAINER=1"

    cd "${repo_root}"
    [[ -f Cargo.toml ]] || htmlcut_die "inner runtime probe requires the HTMLCut workspace checkout"
    "${prepare_script}"
    rustc --version | grep -F "rustc ${stable_toolchain_channel} " >/dev/null
    cargo +nightly llvm-cov --version >/dev/null
    cargo +nightly miri --version >/dev/null
    cargo nextest --version >/dev/null
    cargo audit --version >/dev/null
    cargo deny --version >/dev/null
    cargo semver-checks --version >/dev/null
    cargo outdated --version >/dev/null
    cargo fuzz --version >/dev/null
}

if [[ "${HTMLCUT_DEVCONTAINER:-}" == "1" ]]; then
    validate_inner_runtime
    exit 0
fi

command -v docker >/dev/null 2>&1 || htmlcut_die \
    "docker is required to validate the contributor devcontainer"
command -v python3 >/dev/null 2>&1 || htmlcut_die "python3 is required to validate devcontainer.json"
[[ -f "${dockerfile_path}" ]] || htmlcut_die "missing ${dockerfile_path}"
[[ -f "${config_path}" ]] || htmlcut_die "missing ${config_path}"
[[ -f "${helper_dockerfile_path}" ]] || htmlcut_die "missing ${helper_dockerfile_path}"
[[ -f "${bootstrap_script}" ]] || htmlcut_die "missing ${bootstrap_script}"
[[ -f "${prepare_script}" ]] || htmlcut_die "missing ${prepare_script}"
assert_repo_command_probe_mode

python3 - <<'PY' "${config_path}"
import json
import sys
from pathlib import Path

config = json.loads(Path(sys.argv[1]).read_text())

if config.get("remoteUser") != "vscode":
    raise SystemExit("remoteUser must stay 'vscode'")

if config.get("workspaceFolder") != "/workspaces/htmlcut":
    raise SystemExit("workspaceFolder must stay /workspaces/htmlcut")

workspace_mount = config.get("workspaceMount", "")
if "target=/workspaces/htmlcut" not in workspace_mount:
    raise SystemExit("workspaceMount must target /workspaces/htmlcut")

mounts = config.get("mounts", [])
if not any("target=/home/vscode/.cargo" in mount for mount in mounts):
    raise SystemExit("devcontainer must keep a named cargo-home volume")
if not any("target=/home/vscode/.rustup" in mount for mount in mounts):
    raise SystemExit("devcontainer must keep a named rustup-home volume")
if not any("target=/home/vscode/.cache" in mount for mount in mounts):
    raise SystemExit("devcontainer must keep a named general-cache volume")

env = config.get("containerEnv", {})
expected_env = {
    "HTMLCUT_DEVCONTAINER": "1",
    "CARGO_HOME": "/home/vscode/.cargo",
    "RUSTUP_HOME": "/home/vscode/.rustup",
}
for key, value in expected_env.items():
    if env.get(key) != value:
        raise SystemExit(f"containerEnv {key} must stay {value!r}")
if "/home/vscode/.cargo/bin" not in env.get("PATH", ""):
    raise SystemExit("containerEnv PATH must include /home/vscode/.cargo/bin")

if config.get("postCreateCommand") != "bash -lc './scripts/devcontainer-prepare-user-home.sh && ./scripts/devcontainer-bootstrap.sh'":
    raise SystemExit("postCreateCommand must repair user-home mounts and run the bootstrap script")

if config.get("postStartCommand") != "./scripts/devcontainer-prepare-user-home.sh":
    raise SystemExit("postStartCommand must keep repairing user-home mounts on every start")

settings = config.get("customizations", {}).get("vscode", {}).get("settings", {})
extension_kind = settings.get("remote.extensionKind", {})
for extension_id in ("rust-lang.rust-analyzer", "tamasfe.even-better-toml"):
    if extension_kind.get(extension_id) != ["workspace"]:
        raise SystemExit(f"{extension_id} must stay forced into the workspace/container extension host")

extensions = config.get("customizations", {}).get("vscode", {}).get("extensions", [])
for extension_id in ("EditorConfig.EditorConfig", "rust-lang.rust-analyzer", "tamasfe.even-better-toml"):
    if extension_id not in extensions:
        raise SystemExit(f"{extension_id} must remain installed in the devcontainer")
PY

readonly image_tag="htmlcut-devcontainer-validate:local"
cargo_volume="$(
    select_volume_name "htmlcut-cargo-home" "htmlcut-devcontainer-validate-cargo-$$"
)"
readonly cargo_volume
rustup_volume="$(
    select_volume_name "htmlcut-rustup-home" "htmlcut-devcontainer-validate-rustup-$$"
)"
readonly rustup_volume
cache_volume="$(
    select_volume_name "htmlcut-general-cache" "htmlcut-devcontainer-validate-cache-$$"
)"
readonly cache_volume

cleanup() {
    if [[ "${volume_mode}" == "isolated" ]]; then
        docker volume rm -f "${cargo_volume}" "${rustup_volume}" "${cache_volume}" >/dev/null 2>&1 || true
    fi
    while IFS= read -r container_id; do
        docker rm -f "${container_id}" >/dev/null 2>&1 || true
    done < <(docker ps -aq --filter "label=devcontainer.local_folder=${repo_root}")
}
trap cleanup EXIT

printf 'devcontainer validation: build raw contributor image\n'
docker build \
    --file "${dockerfile_path}" \
    --tag "${image_tag}" \
    "${repo_root}/.devcontainer" >/dev/null

printf 'devcontainer validation: probe raw image package contract\n'
docker run --rm "${image_tag}" bash -lc '
    set -euo pipefail
    . /etc/os-release
    [[ "${ID}" == "ubuntu" ]]
    [[ "${VERSION_ID}" == "24.04" ]]
    clang --version >/dev/null
    curl --version >/dev/null
    gh --version >/dev/null
    git --version >/dev/null
    jq --version >/dev/null
    musl-gcc --version >/dev/null
    python3 --version >/dev/null
    rg --version >/dev/null
    shellcheck --version >/dev/null
    sudo --version >/dev/null
'

if [[ "${volume_mode}" == "shared-ci" ]]; then
    printf 'devcontainer validation: reset shared contributor volumes for CI reuse\n'
    docker volume rm -f "${cargo_volume}" "${rustup_volume}" "${cache_volume}" >/dev/null 2>&1 || true
fi

docker volume create "${cargo_volume}" >/dev/null
docker volume create "${rustup_volume}" >/dev/null
docker volume create "${cache_volume}" >/dev/null

printf 'devcontainer validation: seed root-owned cache, cargo, and rustup mounts\n'
docker run --rm --user root \
    --volume "${cargo_volume}:/home/vscode/.cargo" \
    --volume "${rustup_volume}:/home/vscode/.rustup" \
    --volume "${cache_volume}:/home/vscode/.cache" \
    "${image_tag}" bash -lc '
        set -euo pipefail
        install -d -o root -g root /home/vscode/.cargo/registry /home/vscode/.rustup/toolchains /home/vscode/.cache/probe
        touch /home/vscode/.cargo/registry/root-owned-marker
        touch /home/vscode/.rustup/toolchains/root-owned-marker
        touch /home/vscode/.cache/probe/root-owned-marker
    '

printf 'devcontainer validation: repair mounts and bootstrap the raw contributor image\n'
docker run --rm \
    --volume "${repo_root}:/workspaces/htmlcut:ro" \
    --volume "${cargo_volume}:/home/vscode/.cargo" \
    --volume "${rustup_volume}:/home/vscode/.rustup" \
    --volume "${cache_volume}:/home/vscode/.cache" \
    --env HTMLCUT_STABLE_TOOLCHAIN="${stable_toolchain_channel}" \
    --env HTMLCUT_DEVCONTAINER_REPO_COMMAND_PROBES="${repo_command_probe_mode}" \
    "${image_tag}" bash -lc '
        set -euo pipefail
        /workspaces/htmlcut/scripts/devcontainer-prepare-user-home.sh
        /workspaces/htmlcut/scripts/devcontainer-bootstrap.sh
        touch /home/vscode/.cargo/user-writable-marker
        touch /home/vscode/.rustup/user-writable-marker
        touch /home/vscode/.cache/user-writable-marker
        rustc --version | grep -F "rustc ${HTMLCUT_STABLE_TOOLCHAIN} " >/dev/null
        cargo +nightly llvm-cov --version >/dev/null
        cargo +nightly miri --version >/dev/null
        cargo nextest --version >/dev/null
        cargo audit --version >/dev/null
        cargo deny --version >/dev/null
        cargo semver-checks --version >/dev/null
        cargo outdated --version >/dev/null
        cargo fuzz --version >/dev/null
        cd /workspaces/htmlcut
        case "${HTMLCUT_DEVCONTAINER_REPO_COMMAND_PROBES}" in
            full)
                export CARGO_TARGET_DIR=/tmp/htmlcut-artifacts/target
                export CARGO_BUILD_BUILD_DIR=/tmp/htmlcut-artifacts/build
                ./scripts/xtask.sh --help >/dev/null
                cargo run --quiet -- --help >/dev/null
                ;;
            skip) ;;
            *)
                echo "unsupported HTMLCUT_DEVCONTAINER_REPO_COMMAND_PROBES=${HTMLCUT_DEVCONTAINER_REPO_COMMAND_PROBES}" >&2
                exit 1
                ;;
        esac
    '

printf 'devcontainer validation: build helper image for devcontainer CLI coverage\n'
docker build \
    --file "${helper_dockerfile_path}" \
    --tag "${helper_image_tag}" \
    "${repo_root}/scripts" >/dev/null

printf 'devcontainer validation: bring up the committed devcontainer through the client path\n'
docker run --rm \
    --volume /var/run/docker.sock:/var/run/docker.sock \
    --volume "${repo_root}:${repo_root}" \
    --workdir "${repo_root}" \
    --env HOME=/tmp/devcontainer-home \
    "${helper_image_tag}" bash -lc '
        set -euo pipefail
        trap '\''docker rm -f $(docker ps -aq --filter "label=devcontainer.local_folder='"${repo_root}"'") >/dev/null 2>&1 || true'\'' EXIT
        devcontainer up --remove-existing-container --workspace-folder '"${repo_root}"' >/dev/null
        printf '\''devcontainer validation: run inner runtime probe through devcontainer exec\n'\''
        devcontainer exec --workspace-folder '"${repo_root}"' ./scripts/validate-devcontainer.sh
    '

printf 'devcontainer validation: success\n'
