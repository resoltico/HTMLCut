# Contributor devcontainer for HTMLCut.
#
# Why the Rust toolchains and cargo QA commands are not baked into this image:
# - the committed devcontainer keeps named volumes mounted at /home/vscode/.cargo and
#   /home/vscode/.rustup so toolchains and registries persist across container rebuilds;
# - baking rustup into those same paths would be hidden the moment the named volumes attach;
# - the committed bootstrap script installs the pinned Rust surfaces into those mounted paths on
#   first create instead of leaving the image and the runtime contract at odds with each other.
#
# Why Docker stays on the host instead of inside the contributor shell:
# - HTMLCut uses Docker to materialize the contributor environment and to validate the committed
#   devcontainer client path from the host side;
# - day-to-day repository work inside the contributor shell is Rust-, Cargo-, and shell-tooling
#   focused rather than Docker-driven;
# - keeping Docker out of the contributor shell avoids mounted-socket permission drift and keeps
#   this Ubuntu 26.04 contract close to the simpler GridGrind contributor-container shape.
FROM ubuntu:26.04@sha256:5e275723f82c67e387ba9e3c24baa0abdcb268917f276a0561c97bef9450d0b4

SHELL ["/bin/bash", "-o", "pipefail", "-c"]

ARG USERNAME=vscode
USER root

RUN apt-get update \
    && export DEBIAN_FRONTEND=noninteractive \
    && apt-get install -y --no-install-recommends \
        build-essential \
        ca-certificates \
        clang \
        curl \
        git \
        gh \
        jq \
        less \
        libssl-dev \
        musl-tools \
        pkg-config \
        procps \
        python3 \
        ripgrep \
        shellcheck \
        sudo \
        unzip \
        xz-utils \
        zip \
    && groupadd --force "${USERNAME}" \
    && if ! id -u "${USERNAME}" >/dev/null 2>&1; then useradd -m -s /bin/bash --gid "${USERNAME}" "${USERNAME}"; fi \
    && install -d -m 0755 /etc/sudoers.d \
    && echo "${USERNAME} ALL=(root) NOPASSWD:ALL" > "/etc/sudoers.d/${USERNAME}" \
    && chmod 0440 "/etc/sudoers.d/${USERNAME}" \
    && install -d -o "${USERNAME}" -g "${USERNAME}" \
        /workspaces/htmlcut \
        "/home/${USERNAME}/.cargo" \
        "/home/${USERNAME}/.rustup" \
        "/home/${USERNAME}/.cache" \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

ENV CARGO_HOME=/home/vscode/.cargo
ENV RUSTUP_HOME=/home/vscode/.rustup
ENV PATH=/home/vscode/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin

WORKDIR /workspaces/htmlcut

USER vscode
