#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0
# Seeds and instrumented builds belong to this run, never the owner's build cache.
set -euo pipefail
cd "$(dirname "$0")/.."
if ! python3 -c 'import sys; sys.exit(sys.version_info < (3, 11))' >/dev/null 2>&1; then
    echo 'focused-smoke.sh requires Python 3.11 or newer on PATH.' >&2; exit 2
fi
source ./scripts/contributor-rust-tools.sh
runs=10000
if [[ ${1:-} == --runs ]]; then
    [[ ${2:-} =~ ^[1-9][0-9]*$ ]] && [[ ${#2} -le 5 ]] && (( $2 <= 10000 )) || {
        echo '--runs must be an integer from 1 to 10000' >&2; exit 2;
    }
    runs=$2
    shift 2
fi
require_mutations=0
if [[ ${1:-} == --require-mutations ]]; then
    require_mutations=1
    shift
fi
[[ $# -le 1 ]] || { echo 'Usage: focused-smoke.sh [--runs 1..10000] [--require-mutations] [OUTPUT_DIRECTORY]' >&2; exit 2; }
temporary_output=0
scratch_parent=$(python3 -c 'from pathlib import Path; import sys; print(Path(sys.argv[1]).resolve())' "${TMPDIR:-/tmp}")
case "$scratch_parent/" in "$(pwd -P)/"*) echo 'Fuzz scratch must be outside the checkout' >&2; exit 2 ;; esac
if [[ $# -eq 0 ]]; then
    output=$(mktemp -d "$scratch_parent/htmlcut-fuzz.XXXXXX")
    temporary_output=1
else
    parent=$(python3 -c 'from pathlib import Path; import sys; print(Path(sys.argv[1]).resolve())' "$1")
    case "$parent/" in "$(pwd -P)/"*) echo 'Fuzz output must be outside the checkout' >&2; exit 2 ;; esac
    mkdir -p "$parent"
    output=$(mktemp -d "$parent/run.XXXXXX")
fi
output=$(cd "$output" && pwd -P)
build=$(mktemp -d "$scratch_parent/htmlcut-fuzz-build.XXXXXX")
pids=()
build_started=0
workers_started=()
cleanup_confirmed() {
    python3 - "$1" <<'PY'
from pathlib import Path
import json
import sys
path=Path(sys.argv[1])
try:
    report=json.loads(path.read_text())
    errors=report['cleanup_errors']
    if not isinstance(errors, list) or errors:
        raise ValueError(f'cleanup_errors={errors!r}')
except (OSError, ValueError, KeyError, TypeError) as error:
    print(f'Unconfirmed process cleanup in {path}: {error}', file=sys.stderr)
    raise SystemExit(1)
PY
}
cleanup() {
    local status=$?
    for pid in "${pids[@]}"; do kill -TERM "$pid" 2>/dev/null || true; done
    for pid in "${pids[@]}"; do wait "$pid" 2>/dev/null || true; done
    if [[ ! -f "$output/build.resources.json" && -f "$build/build.log" ]]; then
        cp "$build"/build.* "$output/"
    fi
    if [[ $build_started -eq 1 ]] && ! cleanup_confirmed "$build/build.resources.json"; then
        printf 'Fuzz build retained because process cleanup is unconfirmed: %s\n' "$build" >&2
        if [[ $status -eq 0 ]]; then status=125; fi
    else
        rm -rf -- "$build"
    fi
    local target
    for target in "${workers_started[@]}"; do
        if ! cleanup_confirmed "$output/$target/run.resources.json"; then
            printf 'Fuzz target retained because process cleanup is unconfirmed: %s\n' "$output/$target" >&2
            if [[ $status -eq 0 ]]; then status=125; fi
        fi
    done
    if [[ $temporary_output -eq 1 && $status -eq 0 ]]; then
        rm -rf -- "$output"
    else
        printf 'Fuzz evidence retained at %s\n' "$output" >&2
    fi
    exit "$status"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
python3 - "$output" <<'PY'
from pathlib import Path
import sys
import json
output=Path(sys.argv[1])
maximum_seeds=10000
inventory={target: [] for target in ['contract_bytes','extract']}
def seed_paths(directory, files_only=False):
    paths=[]
    for path in directory.iterdir():
        if files_only and not path.is_file():
            continue
        if len(paths) >= maximum_seeds:
            raise SystemExit(f'{directory} exceeds the bounded initial corpus of {maximum_seeds} seeds')
        paths.append(path)
    return sorted(paths)
cases=seed_paths(Path('conformance/contract'))
documents=seed_paths(Path('fuzz/seeds/documents'), files_only=True)
if len(cases)+len(documents) > maximum_seeds:
    raise SystemExit(f'extract exceeds the bounded initial corpus of {maximum_seeds} seeds')
def read_seed(path, remaining=4096):
    if path.stat().st_size > remaining:
        raise SystemExit(f'Fuzz seed {path} exceeds the smoke input bound of 4096 bytes')
    # Bound the read as well as checking metadata, including a file that grows.
    with path.open('rb') as source:
        value=source.read(remaining+1)
    if len(value) > remaining:
        raise SystemExit(f'Fuzz seed {path} exceeds the smoke input bound of 4096 bytes')
    return value
def write_seed(target, name, value):
    with (output/target/'corpus'/name).open('xb') as destination:
        destination.write(value)
    inventory[target].append({'name': name, 'bytes': len(value)})
for target in ['contract_bytes','extract']:
    (output/target/'corpus').mkdir(parents=True)
    (output/target/'crashes').mkdir()
for case in cases:
    contract=read_seed(case/'contract.json', 4094)
    page=read_seed(case/'page.html', 4094-len(contract))
    write_seed('contract_bytes', case.name, contract)
    write_seed('extract', case.name, len(contract).to_bytes(2,'little')+contract+page)
contract=b'{"select":":scope"}'
for seed in documents:
    write_seed('extract', 'document-'+seed.name,
               len(contract).to_bytes(2,'little')+contract+read_seed(seed, 4094-len(contract)))
(output/'seeds.json').write_text(json.dumps(inventory, indent=2)+'\n')
PY
# One build shares dependencies across both harnesses. Parallel execution below avoids
# Cargo's build lock and retains the same per-target seed and execution count.
export CARGO_INCREMENTAL=0
# Explicit routing also covers Cargo's separate intermediate build directory.
export CARGO_TARGET_DIR="$build/target"
export CARGO_BUILD_TARGET_DIR="$build/target"
export CARGO_BUILD_BUILD_DIR="$build/target"
build_started=1
python3 scripts/run-resource-bounded.py --root "$build" --disk-limit-mib 1024 \
    --timeout 600 --log-limit-mib 4 --label build -- \
    cargo +"$HTMLCUT_CONTRIBUTOR_RUST_NIGHTLY_TOOLCHAIN" fuzz build \
    --target-dir "$build/target" --codegen-units 16 &
pids+=("$!")
if wait "${pids[0]}"; then
    pids=()
else
    status=$?
    pids=()
    cp "$build"/build.* "$output/"
    exit "$status"
fi
cp "$build"/build.* "$output/"
triple=$(rustc +"$HTMLCUT_CONTRIBUTOR_RUST_NIGHTLY_TOOLCHAIN" -vV | sed -n 's/^host: //p')
for target in contract_bytes extract; do
    workers_started+=("$target")
    python3 scripts/run-resource-bounded.py --root "$output/$target" --disk-limit-mib 64 \
        --timeout 180 --log-limit-mib 4 --label run -- \
        "$build/target/$triple/release/$target" "$output/$target/corpus" \
        "-runs=$runs" -reload=0 -max_len=4096 -timeout=10 -rss_limit_mb=2048 -seed=1008 \
        "-artifact_prefix=$output/$target/crashes/" &
    pids+=("$!")
done
# Bash 3.2 on macOS has no wait -n. Reap completed children in either order;
# the EXIT trap cancels the sibling immediately when a target fails.
remaining=("${pids[@]}")
while [[ ${#remaining[@]} -gt 0 ]]; do
    running=()
    for pid in "${remaining[@]}"; do
        if kill -0 "$pid" 2>/dev/null; then
            running+=("$pid")
        else
            # Stop cancellation ownership of the completed child before
            # collecting its saved status; never retain reaped worker PIDs.
            live=()
            for owned in "${pids[@]}"; do
                if [[ $owned != "$pid" ]]; then live+=("$owned"); fi
            done
            pids=("${live[@]}")
            if wait "$pid"; then status=0; else status=$?; fi
            if [[ $status -ne 0 ]]; then exit "$status"; fi
        fi
    done
    remaining=("${running[@]}")
    if [[ ${#remaining[@]} -gt 0 ]]; then sleep 0.1; fi
done
pids=()
python3 - "$output" "$runs" "$require_mutations" <<'PY'
from pathlib import Path
import sys
import re
root=Path(sys.argv[1]); runs=int(sys.argv[2])
for target in ['contract_bytes','extract']:
    text=(root/target/'run.log').read_text(encoding='utf-8')
    completed=re.search(r'Done (\d+) runs', text)
    if completed is None or int(completed[1]) < runs:
        raise SystemExit(f'{target} did not complete all {runs} iterations')
    if sys.argv[3] == '1':
        initialized=re.search(r'^#(\d+)\s+INITED\b', text, re.MULTILINE)
        # Reload is disabled: after INITED, only mutation execution advances the
        # callback count. Seed replay alone must not satisfy the PR preflight.
        if initialized is None or int(completed[1]) <= int(initialized[1]):
            raise SystemExit(f'{target} completed no mutation after seed initialization; increase --runs explicitly')
if runs != 10000:
    print(f'Short test only: {runs} runs per target; required smoke is 10000.')
PY
