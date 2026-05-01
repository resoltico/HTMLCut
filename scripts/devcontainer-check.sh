#!/usr/bin/env bash
# Run the full maintainer gate through the committed contributor container from the host.

set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/common.sh
. "${script_dir}/common.sh"
script_dir="$(htmlcut_resolve_script_dir "${BASH_SOURCE[0]}")"
readonly script_dir
repo_root="$(htmlcut_repo_root_from_script_dir "${script_dir}")"
readonly repo_root
readonly dockerfile_path="${repo_root}/.devcontainer/Dockerfile"
readonly image_tag="htmlcut-devcontainer-check:local"
readonly cargo_volume="htmlcut-cargo-home"
readonly rustup_volume="htmlcut-rustup-home"
readonly cache_volume="htmlcut-general-cache"

command -v docker >/dev/null 2>&1 || htmlcut_die \
    "docker is required to run the contributor devcontainer maintainer gate"
[[ -f "${dockerfile_path}" ]] || htmlcut_die "missing ${dockerfile_path}"

printf 'devcontainer gate: building contributor image\n'
docker build \
    --file "${dockerfile_path}" \
    --tag "${image_tag}" \
    "${repo_root}/.devcontainer" >/dev/null

printf 'devcontainer gate: running maintainer gate inside contributor container\n'
docker run --rm \
    --volume "${repo_root}:/workspaces/htmlcut" \
    --volume "${cargo_volume}:/home/vscode/.cargo" \
    --volume "${rustup_volume}:/home/vscode/.rustup" \
    --volume "${cache_volume}:/home/vscode/.cache" \
    --workdir /workspaces/htmlcut \
    --env HTMLCUT_DEVCONTAINER=1 \
    --env CARGO_HOME=/home/vscode/.cargo \
    --env RUSTUP_HOME=/home/vscode/.rustup \
    --env PATH=/home/vscode/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin \
    "${image_tag}" bash -lc '
        set -euo pipefail
        ./scripts/devcontainer-prepare-user-home.sh
        ./scripts/devcontainer-bootstrap.sh
        ./check.sh
    '
