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

docker_run_args=(
    --rm
    --volume "${repo_root}:/workspaces/htmlcut"
    --volume "${cargo_volume}:/home/vscode/.cargo"
    --volume "${rustup_volume}:/home/vscode/.rustup"
    --volume "${cache_volume}:/home/vscode/.cache"
    --workdir /workspaces/htmlcut
    --env HTMLCUT_DEVCONTAINER=1
    --env CARGO_HOME=/home/vscode/.cargo
    --env RUSTUP_HOME=/home/vscode/.rustup
    --env PATH=/home/vscode/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
)

append_worktree_git_metadata_mounts() {
    local repo_git_path="${repo_root}/.git"
    [[ -f "${repo_git_path}" ]] || return 0

    local gitdir_line
    gitdir_line="$(<"${repo_git_path}")"
    [[ "${gitdir_line}" == gitdir:\ * ]] || return 0

    local worktree_git_dir="${gitdir_line#gitdir: }"
    if [[ "${worktree_git_dir}" != /* ]]; then
        worktree_git_dir="${repo_root}/${worktree_git_dir}"
    fi
    [[ -d "${worktree_git_dir}" ]] || htmlcut_die "missing worktree gitdir ${worktree_git_dir}"
    [[ -f "${worktree_git_dir}/commondir" ]] || htmlcut_die \
        "worktree gitdir ${worktree_git_dir} is missing commondir"

    local common_git_dir
    common_git_dir="$(
        cd -P -- "${worktree_git_dir}" \
            && cd -P -- "$(cat commondir)" \
            && pwd
    )"

    docker_run_args+=(
        --volume "${repo_root}:${repo_root}:ro"
        --volume "${common_git_dir}:${common_git_dir}:ro"
    )
}

append_worktree_git_metadata_mounts

printf 'devcontainer gate: building contributor image\n'
docker build \
    --file "${dockerfile_path}" \
    --tag "${image_tag}" \
    "${repo_root}/.devcontainer" >/dev/null

printf 'devcontainer gate: running maintainer gate inside contributor container\n'
docker run "${docker_run_args[@]}" \
    "${image_tag}" bash -lc '
        set -euo pipefail
        ./scripts/devcontainer-prepare-user-home.sh
        export CARGO_TARGET_DIR=/home/vscode/.cache/htmlcut-target
        git config --global --add safe.directory /workspaces/htmlcut
        ./scripts/devcontainer-bootstrap.sh
        ./check.sh
    '
