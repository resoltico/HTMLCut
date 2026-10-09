#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0
set -euo pipefail
cd "$(dirname "$0")"
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked --workspace --all-features
cargo test --locked -p htmlcut-core --lib million_element_selector_inspection_fails_closed -- --ignored
RUSTDOCFLAGS=-Dwarnings cargo doc --locked --workspace --all-features --no-deps
python3 -m unittest discover -s tests -v
cargo deny --locked check
