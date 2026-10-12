#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0
# Existing contract cases are byte-frozen against the latest stable tag of this major.
set -euo pipefail
cd "$(dirname "$0")/.."
python3 - <<'PY'
from pathlib import Path
import re,subprocess,tomllib
major=tomllib.loads(Path('Cargo.toml').read_text())['workspace']['package']['version'].split('.')[0]
tags=subprocess.check_output(['git','tag','--list',f'v{major}.*'],text=True).splitlines()
stable=[tag for tag in tags if re.fullmatch(r'v[1-9][0-9]*\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)',tag)]
if not stable:
    print(f'No released major-{major} conformance baseline exists.')
    raise SystemExit(0)
tag=max(stable,key=lambda t:tuple(map(int,t[1:].split('.'))))
paths=subprocess.check_output(['git','ls-tree','-r','--name-only','-z',tag,'--','conformance/contract']).split(b'\0')
failures=[]
for raw in paths:
    if not raw:continue
    path=raw.decode()
    expected=subprocess.check_output(['git','show',f'{tag}:{path}'])
    try:actual=Path(path).read_bytes()
    except OSError:actual=None
    if actual!=expected:failures.append(path)
if failures:
    raise SystemExit('Frozen contract cases changed or disappeared against '+tag+':\n'+'\n'.join(failures))
print('Existing contract cases match '+tag+' byte for byte; additions are allowed.')
PY
