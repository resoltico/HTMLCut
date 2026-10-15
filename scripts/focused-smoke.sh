#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0
# Exec the single-owner controller; Bash must never retain child PIDs to signal.
set -euo pipefail
cd "$(dirname "$0")/.."
if ! python3 -c 'import sys; sys.exit(sys.version_info < (3, 11))' >/dev/null 2>&1; then
    echo 'focused-smoke.sh requires Python 3.11+ on PATH (3.13+ on macOS for bounded QA).' >&2; exit 2
fi
source ./scripts/contributor-rust-tools.sh
export HTMLCUT_CONTRIBUTOR_RUST_NIGHTLY_TOOLCHAIN
exec python3 scripts/focused-smoke.py "$@"
