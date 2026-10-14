#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0
# Seed both fuzz targets from authoritative inputs; mutations stay outside the checkout.
set -euo pipefail
cd "$(dirname "$0")/.."
source ./scripts/contributor-rust-tools.sh
temporary_output=0
if [[ $# -eq 0 ]]; then
    output=$(mktemp -d "${TMPDIR:-/tmp}/htmlcut-fuzz.XXXXXX")
    temporary_output=1
else
    output=$1
fi
cleanup() {
    local status=$?
    if [[ $temporary_output -eq 1 ]]; then
        if [[ $status -eq 0 ]]; then
            rm -rf -- "$output"
        else
            printf 'Fuzz evidence retained at %s\n' "$output" >&2
        fi
    fi
}
trap cleanup EXIT
mkdir -p "$output"
output=$(cd "$output" && pwd -P)
case "$output/" in "$(pwd -P)/"*) echo "Fuzz output must be outside the checkout" >&2; exit 2 ;; esac
python3 - "$output" <<'PY'
from pathlib import Path
import sys
output=Path(sys.argv[1])
for target in ['contract_bytes','extract']:
    (output/target/'corpus').mkdir(parents=True,exist_ok=True)
    (output/target/'crashes').mkdir(exist_ok=True)
for case in sorted(Path('conformance/contract').iterdir()):
    contract=(case/'contract.json').read_bytes()
    page=(case/'page.html').read_bytes()
    (output/'contract_bytes'/'corpus'/case.name).write_bytes(contract)
    (output/'extract'/'corpus'/case.name).write_bytes(len(contract).to_bytes(2,'little')+contract+page)
contract=b'{"select":":scope"}'
for seed in Path('fuzz/seeds/documents').iterdir():
    if seed.is_file():
        (output/'extract'/'corpus'/('document-'+seed.name)).write_bytes(len(contract).to_bytes(2,'little')+contract+seed.read_bytes())
PY
for target in contract_bytes extract; do
    cargo +"$HTMLCUT_CONTRIBUTOR_RUST_NIGHTLY_TOOLCHAIN" fuzz run \
        "$target" "$output/$target/corpus" -- \
        -runs=10000 -max_len=4096 -timeout=10 -rss_limit_mb=2048 -seed=1008 \
        "-artifact_prefix=$output/$target/crashes/" 2>&1 | tee "$output/$target/run.log"
    python3 - "$output/$target/run.log" <<'PY'
from pathlib import Path
import sys
text=Path(sys.argv[1]).read_text()
if '#10000' not in text or 'Done 10000 runs' not in text:
    raise SystemExit('Fuzz target did not complete all 10000 iterations')
PY
done
