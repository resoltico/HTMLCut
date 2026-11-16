#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Ervins Strauhmanis
"""Capture matching tracked source and Cargo's locked vendored dependencies."""
import gzip
import hashlib
import io
import json
import os
from pathlib import Path, PurePosixPath
import shutil
import subprocess
import tarfile
import tempfile


def git(root, *args):
    return subprocess.check_output(["git", *args], cwd=root)


def tracked_source(root):
    if git(root, "ls-files", "--others", "--exclude-standard", "-z"):
        raise ValueError("Untracked source files exist; add intended files to Git's index before packaging")
    index = git(root, "ls-files", "--stage", "-z")
    entries = []
    for entry in index.split(b"\0"):
        if not entry:
            continue
        meta, name = entry.split(b"\t", 1)
        mode, _, stage = meta.split()
        if stage != b"0" or mode not in (b"100644", b"100755"):
            raise ValueError("Source packaging requires resolved, regular tracked files")
        path = root / os.fsdecode(name)
        if path.is_symlink() or not path.is_file():
            raise ValueError(f"Tracked source is missing or not regular: {path}")
        entries.append((os.fsdecode(name), int(mode[-3:], 8)))
    return index, entries


def source_notice(revision, timestamp):
    return (
        "HTMLCut Corresponding Source\n"
        f"Source revision: {revision}\n"
        f"SOURCE_DATE_EPOCH: {timestamp}\n"
        "Build: cargo build --offline --locked --release -p htmlcut\n"
        "The archive includes tracked project/build/install inputs and locked vendored dependencies.\n"
        "Install the Rust toolchain and native linker described in CONTRIBUTING.md separately.\n"
    ).encode("utf-8")


def create_archive(root, output, revision):
    index, entries = tracked_source(root)
    if git(root, "rev-parse", "HEAD").decode().strip() != revision:
        raise ValueError("Source revision differs from the release checkout")
    timestamp = int(git(root, "show", "-s", "--format=%ct", "HEAD"))
    output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="htmlcut-source-") as tmp:
        source = Path(tmp) / "htmlcut-source"
        source.mkdir()
        originals = {}
        for name, mode in entries:
            data = (root / name).read_bytes()
            originals[name] = hashlib.sha256(data).digest()
            target = source / name
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(data)
            target.chmod(mode)
        vendor_environment = dict(os.environ, CARGO_TARGET_DIR=str(Path(tmp) / "cargo-target"))
        configuration = subprocess.check_output(
            ["cargo", "vendor", "--locked", "--versioned-dirs", "vendor"], cwd=source, env=vendor_environment, text=True
        )
        config = source / ".cargo/config.toml"
        previous = config.read_text(encoding="utf-8") if config.exists() else ""
        config.parent.mkdir(parents=True, exist_ok=True)
        with config.open("w", encoding="utf-8", newline="\n") as stream:
            stream.write(previous.rstrip() + "\n\n" + configuration)
        (source / "SOURCE.txt").write_bytes(source_notice(revision, timestamp))
        for name, _ in entries:
            path = root / name
            if path.is_symlink() or not path.is_file() or hashlib.sha256(path.read_bytes()).digest() != originals[name]:
                raise ValueError(f"Source changed during capture: {name}")
        if git(root, "ls-files", "--stage", "-z") != index or git(root, "ls-files", "--others", "--exclude-standard", "-z"):
            raise ValueError("Source index or untracked inputs changed during capture")
        staged = Path(tmp) / "SOURCE.tar.gz"
        with staged.open("wb") as raw:
            with gzip.GzipFile(filename="", fileobj=raw, mode="wb", mtime=timestamp) as compressed:
                with tarfile.open(fileobj=compressed, mode="w", format=tarfile.PAX_FORMAT) as archive:
                    for path in sorted(source.rglob("*")):
                        if path.is_symlink():
                            raise ValueError(f"Source archive contains a symbolic link: {path}")
                        if not path.is_file():
                            continue
                        data = path.read_bytes()
                        info = tarfile.TarInfo(path.relative_to(source.parent).as_posix())
                        info.size, info.mtime = len(data), timestamp
                        info.mode = 0o755 if path.stat().st_mode & 0o111 else 0o644
                        archive.addfile(info, io.BytesIO(data))
        with tempfile.NamedTemporaryFile(dir=output.parent, delete=False) as pending:
            pending_path = Path(pending.name)
        try:
            shutil.copyfile(staged, pending_path)
            os.replace(pending_path, output)
        finally:
            pending_path.unlink(missing_ok=True)


def source_member_allowed(name, tracked):
    path = name.as_posix()
    return (
        path in tracked
        or path in {".cargo/config.toml", "SOURCE.txt"}
        or len(name.parts) > 1 and name.parts[0] == "vendor"
    )


def verify_archive(root, archive_path, revision):
    expected_hash = hashlib.sha256(archive_path.read_bytes()).hexdigest()
    expected_line = f"{expected_hash}  SOURCE.tar.gz\n".encode("ascii")
    if Path(str(archive_path) + ".sha256").read_bytes() != expected_line:
        raise ValueError("Source archive checksum does not match SOURCE.tar.gz")
    _, entries = tracked_source(root)
    tracked = {name for name, _ in entries}
    timestamp = int(git(root, "show", "-s", "--format=%ct", "HEAD"))
    with tempfile.TemporaryDirectory(prefix="htmlcut-source-verify-") as tmp:
        temporary = Path(tmp)
        seen = set()
        with tarfile.open(archive_path, "r:gz") as archive:
            for member in archive:
                name = PurePosixPath(member.name)
                if name.is_absolute() or ".." in name.parts or not name.parts or name.parts[0] != "htmlcut-source" or not member.isfile() or name in seen:
                    raise ValueError("Source archive has an unsafe, duplicate or nonregular member")
                if not source_member_allowed(PurePosixPath(*name.parts[1:]), tracked):
                    raise ValueError(f"Source archive has an unexpected non-vendor member: {name}")
                seen.add(name)
                target = temporary.joinpath(*name.parts)
                target.parent.mkdir(parents=True, exist_ok=True)
                with archive.extractfile(member) as reader:
                    target.write_bytes(reader.read())
                target.chmod(member.mode & 0o777)
        source = temporary / "htmlcut-source"
        if (source / "SOURCE.txt").read_bytes() != source_notice(revision, timestamp):
            raise ValueError("Source archive identifies a different release checkout")
        for name, mode in entries:
            if (source / name).stat().st_mode & 0o777 != mode:
                raise ValueError(f"Source archive changed tracked file permissions: {name}")
            if name == ".cargo/config.toml":
                original = (root / name).read_text(encoding="utf-8").rstrip()
                if not (source / name).read_text(encoding="utf-8").startswith(original + "\n"):
                    raise ValueError("Source archive lost project Cargo configuration")
            elif (source / name).read_bytes() != (root / name).read_bytes():
                raise ValueError(f"Source archive differs from release source: {name}")
        environment = dict(os.environ, CARGO_HOME=str(temporary / "cargo-home"), CARGO_TARGET_DIR=str(temporary / "target"))
        metadata = subprocess.check_output(["cargo", "metadata", "--offline", "--locked", "--format-version", "1"], cwd=source, env=environment, text=True)
        packages = json.loads(metadata)["packages"]
        version = next(package["version"] for package in packages if package["name"] == "htmlcut")
        built = subprocess.check_output(["cargo", "build", "--offline", "--locked", "--release", "--message-format=json", "-p", "htmlcut"], cwd=source, env=environment, text=True)
        artifacts = [json.loads(line) for line in built.splitlines()]
        binaries = [artifact["executable"] for artifact in artifacts if artifact.get("reason") == "compiler-artifact" and artifact.get("target", {}).get("name") == "htmlcut" and artifact.get("executable")]
        if len(binaries) != 1:
            raise ValueError("Offline source build did not identify one HTMLCut executable")
        actual = subprocess.check_output([binaries[0], "--version"], text=True).strip()
        if actual != "htmlcut " + version:
            raise ValueError("Offline source build reports a different release version")
        (source / "vendor").rename(source / "vendor-held")
        rejected = subprocess.run(["cargo", "metadata", "--offline", "--locked", "--format-version", "1"], cwd=source, env=environment, capture_output=True)
        if rejected.returncode == 0:
            raise ValueError("Offline source verification accepted missing vendored dependencies")
