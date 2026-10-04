#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0
set -euo pipefail

# Stable maintainer entrypoint: keep local docs, CI muscle memory, and the Rust gate on one path.
repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
cd "$repo_root"

./scripts/xtask.sh check
