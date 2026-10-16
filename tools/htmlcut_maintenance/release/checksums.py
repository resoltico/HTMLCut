# SPDX-License-Identifier: MPL-2.0
"""Complete derived asset inventory and byte digests."""

import argparse
from pathlib import Path
from .. import config
from .assets import digest, checksum_inventory
from .source import STABLE


def write(version, directory):
    if not STABLE.fullmatch(version):
        raise ValueError("canonical stable version required")
    directory = Path(directory)
    names = config.asset_names(version)
    manifest = names[-1]
    rows = []
    for name in names[:-1]:
        size, value = digest(directory / name)
        if size == 0:
            raise ValueError("release assets cannot be empty")
        rows.append(value + "  " + name + "\n")
    raw = "".join(rows).encode()
    checksum_inventory(raw, {name: digest(directory / name)[1] for name in names[:-1]})
    (directory / manifest).write_bytes(raw)
    return directory / manifest


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--version", required=True)
    p.add_argument("--dist", type=Path, required=True)
    a = p.parse_args()
    print(write(a.version, a.dist))
    return 0
