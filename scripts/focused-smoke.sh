#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0
# Required bounded parser/selector smoke; writable inputs and evidence stay external.
set -euo pipefail
cd "$(dirname "$0")/.."
source ./scripts/contributor-rust-tools.sh
output=${1:-$(mktemp -d "${TMPDIR:-/tmp}/htmlcut-focused-smoke.XXXXXX")}
mkdir -p "$output"
output=$(cd "$output" && pwd -P)
case "$output/" in
    "$(pwd -P)/"*) echo "Smoke output must be outside the checkout" >&2; exit 2 ;;
esac
printf 'Smoke evidence: %s\n' "$output"
for target in parse_document_bytes relational_selector_budget; do
    mkdir -p "$output/$target/corpus" "$output/$target/crashes"
    cp -R "fuzz/corpus/$target/." "$output/$target/corpus/"
    cargo +"$HTMLCUT_CONTRIBUTOR_RUST_NIGHTLY_TOOLCHAIN" fuzz run --features fuzzing \
        "$target" "$output/$target/corpus" -- \
        -runs=1000 -max_len=4096 -max_total_time=60 -timeout=10 -rss_limit_mb=2048 -seed=1008 \
        "-artifact_prefix=$output/$target/crashes/" 2>&1 | tee "$output/$target/run.log"
done
