#!/usr/bin/env python3
"""Verify release bytes and metadata before publication or an idempotent retry."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import threading
import time

MAX_ASSET = 128 * 1024 * 1024
MAX_TOTAL = 512 * 1024 * 1024
MAX_METADATA = 1024 * 1024


def reject_downgrade(version, releases):
    current = tuple(map(int, version.split(".")))
    if len(releases) >= 1000:
        raise ValueError("Stable release history exceeds verification bound")
    for release in releases:
        match = re.fullmatch(r"v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)", release["tagName"])
        if match and tuple(map(int, match.groups())) > current:
            raise ValueError("Refusing initial publication below an existing stable release")


def digest(path):
    total = 0
    result = hashlib.sha256()
    with Path(path).open("rb") as stream:
        while chunk := stream.read(64 * 1024):
            total += len(chunk)
            if total > MAX_ASSET:
                raise ValueError("Release asset exceeds its byte bound")
            result.update(chunk)
    if not total:
        raise ValueError("Release asset is empty")
    return total, result.hexdigest()


def parse_checksums(raw):
    rows = {}
    for line in raw.decode("ascii").splitlines():
        match = re.fullmatch(r"([0-9a-f]{64})  ([A-Za-z0-9_.-]+)", line)
        if not match or match[2] in rows:
            raise ValueError("Malformed or duplicate checksum entry")
        rows[match[2]] = match[1]
    return rows


def checksum_inventory(raw, expected):
    rows = parse_checksums(raw)
    if rows != expected:
        raise ValueError("Checksum manifest does not identify the exact prepared assets")


def provider_inventory(release, tag, notes, wanted, allow_missing):
    if type(release["isDraft"]) is not bool or type(release["isPrerelease"]) is not bool:
        raise ValueError("Release status fields must be boolean")
    if release["tagName"] != tag or release["name"] != tag or release["isPrerelease"]:
        raise ValueError("Release identity/title/status differs")
    if release["body"].rstrip("\n") != notes.rstrip("\n"):
        raise ValueError("Release notes differ from immutable source notes")
    names = [asset["name"] for asset in release["assets"]]
    if len(names) != len(set(names)) or set(names) - set(wanted):
        raise ValueError("Duplicate or unexpected release assets")
    if not allow_missing or not release["isDraft"]:
        if set(names) != set(wanted):
            raise ValueError("Release asset inventory is incomplete")
    total = 0
    for asset in release["assets"]:
        size = asset["size"]
        if type(size) is not int or not 0 < size <= MAX_ASSET:
            raise ValueError("Release provider asset size is invalid")
        total += size
    if total > MAX_TOTAL:
        raise ValueError("Release provider aggregate exceeds its byte bound")


def gh_stream(arguments, maximum, deadline, collect=False):
    remaining = min(60, deadline - time.monotonic())
    if remaining <= 0:
        raise ValueError("Release verification deadline exceeded")
    process = subprocess.Popen(["gh", *arguments], stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
    timer = threading.Timer(remaining, process.kill)
    timer.daemon = True
    timer.start()
    size = 0
    result = hashlib.sha256()
    data = bytearray()
    try:
        while chunk := process.stdout.read(64 * 1024):
            size += len(chunk)
            if size > maximum:
                raise ValueError("GitHub response exceeds its byte bound")
            result.update(chunk)
            if collect:
                data.extend(chunk)
        if process.wait(timeout=10) != 0:
            raise ValueError("GitHub release operation failed or timed out")
    finally:
        timer.cancel()
        if process.poll() is None:
            process.kill()
            process.wait(timeout=10)
        process.stdout.close()
    return size, result.hexdigest(), bytes(data)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--tag", required=True)
    parser.add_argument("--version", required=True)
    parser.add_argument("--dist", type=Path, help="Prepared local assets; omit for published consumer verification")
    parser.add_argument("--notes", type=Path, required=True)
    parser.add_argument("--allow-missing", action="store_true")
    args = parser.parse_args()
    if args.tag != "v" + args.version or not re.fullmatch(r"v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)", args.tag):
        parser.error("Expected canonical stable release identity")
    root = Path(__file__).resolve().parents[1]
    names = subprocess.check_output(["bash", str(root / "scripts/release-targets.sh"),
        "assets", "--version", args.version], text=True, timeout=10).splitlines()
    manifest = f"htmlcut-{args.version}-checksums.txt"
    if manifest not in names or len(names) != len(set(names)):
        raise ValueError("Invalid canonical release registry")
    deadline = time.monotonic() + 300
    _, _, raw = gh_stream(["release", "view", args.tag, "--json",
        "tagName,name,isDraft,isPrerelease,body,assets"], MAX_METADATA, deadline, collect=True)
    release = json.loads(raw)
    provider_inventory(release, args.tag, args.notes.read_text(encoding="utf-8"), names, args.allow_missing)
    if release["isDraft"]:
        _, _, history = gh_stream(["release", "list", "--exclude-drafts", "--exclude-pre-releases",
            "--limit", "1000", "--json", "tagName"], MAX_METADATA, deadline, collect=True)
        reject_downgrade(args.version, json.loads(history))
    if args.dist:
        wanted = {name: digest(args.dist / name) for name in names}
        if sum(size for size, _ in wanted.values()) > MAX_TOTAL:
            raise ValueError("Prepared release aggregate exceeds its byte bound")
        with (args.dist / manifest).open("rb") as stream:
            raw_manifest = stream.read(64 * 1024 + 1)
        if len(raw_manifest) > 64 * 1024:
            raise ValueError("Checksum manifest exceeds its byte bound")
        checksum_inventory(raw_manifest, {name: value for name, (_, value) in wanted.items() if name != manifest})
    else:
        if args.allow_missing or release["isDraft"]:
            raise ValueError("Consumer verification requires a complete published release")
        size, value, raw_manifest = gh_stream(["release", "download", args.tag,
            "--pattern", manifest, "--output", "-", "--allow-escape-sequences"], 64 * 1024, deadline, collect=True)
        checksums = parse_checksums(raw_manifest)
        if set(checksums) != set(names) - {manifest}:
            raise ValueError("Published checksum inventory differs from registry")
        wanted = {asset["name"]: (asset["size"], checksums.get(asset["name"], value)) for asset in release["assets"]}
        if wanted[manifest] != (size, value):
            raise ValueError("Checksum provider size differs")
    for asset in release["assets"]:
        name = asset["name"]
        if asset["size"] != wanted[name][0]:
            raise ValueError(f"Existing release asset size differs: {name}")
        size, value, _ = gh_stream(["release", "download", args.tag,
            "--pattern", name, "--output", "-", "--allow-escape-sequences"], MAX_ASSET, deadline)
        if (size, value) != wanted[name]:
            raise ValueError(f"Existing release asset bytes differ: {name}")
    print(f"Verified immutable metadata and {len(release['assets'])} matching asset byte streams.")


if __name__ == "__main__":
    main()
