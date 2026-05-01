---
afad: "4.0"
version: "7.0.0"
domain: SETUP
updated: "2026-05-01"
route:
  keywords: [devcontainer, contributor container, ubuntu 26.04, dev container cli, vscode, cargo xtask, rustup bootstrap, devcontainer check]
  questions: ["what is the preferred contributor container workflow for HTMLCut?", "how do I use the HTMLCut devcontainer?", "do I need Rust installed on the host if I use the HTMLCut container?", "why does the HTMLCut devcontainer bootstrap Rust on first create?", "how do I validate the HTMLCut devcontainer?", "how do I run the full maintainer gate through the HTMLCut devcontainer from the host?"]
---

# Contributor Devcontainer Workflow

**Purpose**: document HTMLCut's preferred contributor workflow from first open through full local
verification.
**Prerequisites**: a running Docker engine, a local checkout on the host filesystem, and either
Visual Studio Code with Dev Containers support or another devcontainer-spec-aware client such as
the Dev Container CLI.
**Companion references**: [developer-setup.md](developer-setup.md),
[quality-gates.md](quality-gates.md), [workspace-layout.md](workspace-layout.md)

## Canonical Stance

HTMLCut's preferred contributor path is the committed devcontainer:

- keep the Git checkout on the host filesystem
- bind-mount that checkout into the container
- run Rust, Cargo, release helpers, and repository verification from the container terminal
- keep editor language tooling inside the container workspace host instead of the host process tree

This contributor container is not a published runtime artifact. It exists to give maintainers one
repeatable Ubuntu `26.04` development surface that can run the repo's pinned Rust toolchains,
Cargo QA tools, and maintainer gates without requiring a host-native Rust install.

The committed owner files are:

- [../.devcontainer/devcontainer.json](../.devcontainer/devcontainer.json)
- [../.devcontainer/Dockerfile](../.devcontainer/Dockerfile)
- [../scripts/devcontainer-prepare-user-home.sh](../scripts/devcontainer-prepare-user-home.sh)
- [../scripts/devcontainer-bootstrap.sh](../scripts/devcontainer-bootstrap.sh)
- [../scripts/devcontainer-cli-helper.Dockerfile](../scripts/devcontainer-cli-helper.Dockerfile)
- [../scripts/devcontainer-check.sh](../scripts/devcontainer-check.sh)
- [../scripts/validate-devcontainer.sh](../scripts/validate-devcontainer.sh)

## Why Rust Bootstraps On First Create

Unlike GridGrind's Java devcontainer, HTMLCut cannot bake the Rust toolchains into
`/home/vscode/.cargo` and `/home/vscode/.rustup` while also mounting those paths as named Docker
volumes.

Those volumes are the right place to persist Cargo registries, installed subcommands, and Rustup
toolchains across container rebuilds. If the image preinstalled them into the same paths, the
mounted volumes would hide them immediately.

So the committed contract is:

- the image bakes only the system prerequisites such as `clang`, `shellcheck`, `musl-gcc`, `git`,
  `jq`, `python3`, and `gh`
- the devcontainer mounts named volumes at `/home/vscode/.cargo`, `/home/vscode/.rustup`, and
  `/home/vscode/.cache`
- the first create runs `./scripts/devcontainer-bootstrap.sh` after repairing volume ownership
- later starts keep only the user-home repair step

That makes first create slower than later starts, but it keeps the mounted-cache design and the
installed-tool design aligned instead of fighting each other.

## Why Docker Stays On The Host

HTMLCut needs Docker to materialize the contributor environment, not as a day-to-day dependency of
the contributor shell.

That boundary is intentional:

- the contributor shell owns Rust, Cargo, shell tooling, and repository verification
- the host Docker engine owns image builds plus the real devcontainer-client materialization path
- the validator proves both surfaces without requiring the contributor shell to inherit the host
  Docker socket

This keeps the Ubuntu `26.04` contributor contract close to GridGrind's simpler shape and avoids
mounted-socket permission drift inside already-running editor sessions.

## First Open In VS Code

1. Start Docker on the host.
2. Open the repository in VS Code.
3. Reopen the folder in the container.
4. Wait for the image build plus the first-create Rust bootstrap to finish.
5. Open a container terminal and verify:

```bash
rustc --version
cargo nextest --version
./scripts/validate-devcontainer.sh
./check.sh
```

Expected contributor shape:

- `rustc --version` reports Rust `1.95.0`
- `cargo nextest --version` succeeds because the QA tool bootstrap completed
- `./scripts/validate-devcontainer.sh` succeeds
- `./check.sh` succeeds from the container shell without requiring host-native Rust

## Tooling-Agnostic Devcontainer CLI Workflow

If you do not want VS Code, use a devcontainer-spec-aware client against the committed
`.devcontainer/` contract.

One truthful workflow is:

1. Change into the repository:

   ```bash
   cd /absolute/path/to/HTMLCut
   ```

2. Confirm Docker is reachable from the host shell:

   ```bash
   docker info >/dev/null && echo "Docker is running"
   ```

3. Confirm the devcontainer client is available:

   ```bash
   devcontainer --version
   ```

4. Materialize the committed contributor container:

   ```bash
   devcontainer up --workspace-folder .
   ```

5. Verify the contributor shell:

   ```bash
   devcontainer exec --workspace-folder . bash -lc 'rustc --version && cargo nextest --version && ./scripts/validate-devcontainer.sh'
   ```

6. Run the full maintainer gate from the host through the committed contributor container:

   ```bash
   ./scripts/devcontainer-check.sh
   ```

That host-side wrapper reuses the committed contributor image, named volume contract, lifecycle
scripts, and `./check.sh` entrypoint without requiring a host-side Rust toolchain. If you want the
devcontainer CLI primitive directly, the equivalent command is `devcontainer exec --workspace-folder
. ./check.sh` after `devcontainer up`.

## Host-Native Alternative

If you do not want the contributor container, use [developer-setup.md](developer-setup.md).
That guide owns the host-native Rust bootstrap.

The container workflow is preferred because it keeps the verified Rust and Cargo QA toolchain
inside one Ubuntu `26.04` surface. The host-native path remains available when you explicitly want
local-shell Rust instead.

## Validation Boundary

Run this validator whenever you change `.devcontainer/`, the devcontainer lifecycle scripts, or
the contributor-container documentation:

```bash
./scripts/validate-devcontainer.sh
```

Run the full maintainer gate through the committed contributor container from the host with:

```bash
./scripts/devcontainer-check.sh
```

That validator:

- checks the committed devcontainer JSON contract
- builds the Ubuntu `26.04` contributor image
- verifies the required system tools are present
- proves the user-home repair script can recover root-owned cache and toolchain volumes
- runs the Rust bootstrap against fresh named volumes
- proves `cargo xtask --help` and repo-root `cargo run -- --help` start from inside the raw contributor image
- proves the real devcontainer-client path can materialize the committed spec and run `devcontainer exec`

The host-side `./scripts/devcontainer-check.sh` wrapper then replays the same contributor image,
named volume contract, lifecycle scripts, and `./check.sh` entrypoint to run the full maintainer
gate without requiring devcontainer-client orchestration for the long-running step. Use both host
commands when you are changing the contributor-container surface itself.

## Troubleshooting

If the container starts but Rust commands are missing:

- rerun `./scripts/devcontainer-prepare-user-home.sh` inside the container
- rerun `./scripts/devcontainer-bootstrap.sh` inside the container
- if that fixes the problem, rebuild the devcontainer so the committed lifecycle hooks run again

If the first create fails during the Rust bootstrap:

- confirm the container can reach the network
- confirm `clang`, `curl`, and `git` are present by running `./scripts/validate-devcontainer.sh`
- treat the first failing Cargo or Rustup step as the next toolchain issue to repair instead of
  weakening the bootstrap contract

If `devcontainer up` fails from the host:

- confirm the host Docker engine is running
- confirm the devcontainer client is installed and reachable from the host shell
- rerun `./scripts/validate-devcontainer.sh` from the host shell after any `.devcontainer/` changes
- rerun `./scripts/devcontainer-check.sh` after the validator passes so the full maintainer gate
  proves the repaired contributor path
