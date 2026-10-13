#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0
set -euo pipefail
cd "$(dirname "$0")"
if ! python3 -c 'import sys; sys.exit(sys.version_info < (3, 11))' >/dev/null 2>&1; then
    printf '%s\n' 'check.sh requires Python 3.11 or newer on PATH.' >&2
    exit 2
fi
if ! command -v cargo-deny >/dev/null 2>&1; then
    printf '%s\n' 'check.sh requires cargo-deny on PATH.' >&2
    exit 2
fi
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked --workspace --all-features
RUSTDOCFLAGS=-Dwarnings cargo doc --locked --workspace --all-features --no-deps
python3 -m unittest discover -s tests -v
cargo deny --locked check
./scripts/check-conformance-frozen.sh
