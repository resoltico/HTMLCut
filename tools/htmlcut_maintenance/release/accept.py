# SPDX-License-Identifier: MPL-2.0
"""Inspect then execute the unpacked native production binary."""

import argparse
import hashlib
import json
import re
import sys
import platform
import os
from pathlib import Path, PurePosixPath
import shutil
import stat
import subprocess
import tarfile
import tempfile
import zipfile
from .. import config, process
from . import signature
from .source import STABLE


def unpack(archive, target, version, destination):
    if not STABLE.fullmatch(version):
        raise ValueError("package version must be canonical stable MAJOR.MINOR.PATCH")
    archive = Path(archive)
    destination = Path(destination)
    base = f"htmlcut-{version}-{target['triple']}"
    binary = "htmlcut.exe" if target["archive"] == "zip" else "htmlcut"
    expected = {
        base + "/" + name
        for name in (binary, "LICENSE", "NOTICE", "PATENTS.md", "README.md")
    }
    seen = set()
    directory_seen = False
    total = 0
    if target["archive"] == "zip":
        bundle = zipfile.ZipFile(archive)
        rows = (
            (i.filename, i.file_size, i.external_attr >> 16, i.is_dir(), i)
            for i in bundle.infolist()
        )
        reader = lambda entry: bundle.open(entry)
    else:
        bundle = tarfile.open(archive, "r:gz")

        def members():
            for entry in bundle:
                if not (entry.isfile() or entry.isdir()):
                    raise ValueError(
                        "package must contain only regular files and its root directory"
                    )
                yield entry.name, entry.size, entry.mode, entry.isdir(), entry

        rows = members()
        reader = lambda entry: bundle.extractfile(entry)
    try:
        for name, size, mode, isdir, entry in rows:
            if isdir:
                if name.rstrip("/") != base or size != 0 or directory_seen:
                    raise ValueError("unexpected package directory")
                directory_seen = True
                continue
            if (
                name not in expected
                or name in seen
                or stat.S_IFMT(mode) not in (0, stat.S_IFREG)
            ):
                raise ValueError("unexpected, duplicate or linked package entry")
            if size <= 0 or size > 128 * 1024 * 1024:
                raise ValueError("invalid package member size")
            total += size
            if total > 256 * 1024 * 1024:
                raise ValueError("package exceeds expanded byte bound")
            seen.add(name)
            path = destination.joinpath(*PurePosixPath(name).parts)
            path.parent.mkdir(parents=True, exist_ok=True)
            with reader(entry) as source, path.open("xb") as output:
                shutil.copyfileobj(source, output)
            if path.stat().st_size != size:
                raise ValueError("package member size mismatch")
            if target["archive"] != "zip":
                path.chmod(mode & 0o777)
        if seen != expected:
            raise ValueError("package inventory is incomplete")
        path = destination / base / binary
        if target["archive"] != "zip" and not os.access(path, os.X_OK):
            raise ValueError("packaged binary is not executable")
    finally:
        bundle.close()
    return destination / base


def accept(archive, target, version, root=config.ROOT):
    root = Path(root).resolve()
    archive = Path(archive).resolve()
    target = next((row for row in config.targets() if row["triple"] == target), None)
    if target is None:
        raise ValueError("unsupported native target")
    if not STABLE.fullmatch(version):
        raise ValueError("package version must be canonical stable MAJOR.MINOR.PATCH")
    from .assets import digest as asset_digest

    archive_size, archive_sha = asset_digest(archive)
    output = root / "dist"
    output.mkdir(exist_ok=True)
    evidence = Path(
        tempfile.mkdtemp(prefix="native-acceptance-" + target["id"] + ".", dir=output)
    )
    work = Path(
        tempfile.mkdtemp(
            prefix="htmlcut-native-acceptance-", dir=os.environ.get("RUNNER_TEMP")
        )
    )
    report = dict(
        target=target["triple"],
        host=platform.platform(),
        archive=str(archive),
        archive_sha256=archive_sha,
        archive_bytes=archive_size,
        status="incomplete",
        work=str(work),
        phases=[],
    )

    def phase(label, command, *, cwd, env=None, timeout=1200):
        record = process.run(
            command,
            work,
            disk_limit_mib=4096,
            timeout=timeout,
            log_limit_mib=16,
            label=label,
            cwd=cwd,
            env=env,
        )
        report["phases"].append(dict(label=label, **record))
        for suffix in (".log", ".resources.json"):
            shutil.copy2(work / (label + suffix), evidence / (label + suffix))
        if process.status(record):
            raise ValueError(
                f"Native acceptance phase {label} failed; evidence {evidence}"
            )
        return (work / (label + ".log")).read_bytes()

    try:
        package = unpack(archive, target, version, work)
        binary = package / ("htmlcut.exe" if target["archive"] == "zip" else "htmlcut")
        if target.get("apple_floor"):
            report["signature"] = signature.verify(binary)
            report["signature_rejections"] = signature.rejection_controls(binary)
        from .source import resolve
        from .package import guide

        sha, source_version = resolve(root, "HEAD")
        if source_version != version:
            raise ValueError("native package version differs from acceptance source")
        report["source_sha"] = sha
        if (package / "README.md").read_text() != guide(version, target, sha):
            raise ValueError("package guide differs from immutable source/target")
        if (package / "LICENSE").read_bytes() != (root / "LICENSE").read_bytes():
            raise ValueError("package original license differs")
        patents = re.sub(
            r"^<!--\n.*?^-->\n",
            "",
            (root / "PATENTS.md").read_text(),
            flags=re.M | re.S,
        )
        if (package / "PATENTS.md").read_text() != patents:
            raise ValueError("package patent notice differs")
        notice = (package / "NOTICE").read_text()
        for text in (
            f"Native dependency attribution: HTMLCut {version}, {target['triple']}",
            "/tree/" + sha,
            f"htmlcut-runtime {version}",
            f"htmlcut-core {version}",
        ):
            if text not in notice:
                raise ValueError(
                    "package dependency attribution/source binding is incomplete"
                )
        digest = hashlib.sha256(binary.read_bytes()).hexdigest()
        report["binary_sha256"] = digest
        # Execute the authored target guide, using the unpacked production binary
        # outside the checkout and without a maintenance runtime dependency.
        language = "powershell" if target["archive"] == "zip" else "sh"
        blocks = re.findall(
            r"^```" + language + r"\n(.*?)^```",
            (package / "README.md").read_text(),
            re.M | re.S,
        )
        if len(blocks) < 2:
            raise ValueError("package guide examples are incomplete")
        for index, (block, expected) in enumerate(
            zip(blocks[:2], [f"htmlcut {version}".encode(), b"../guide.html"])
        ):
            command = (
                [
                    "powershell.exe",
                    "-NoLogo",
                    "-NoProfile",
                    "-NonInteractive",
                    "-Command",
                    "$ErrorActionPreference='Stop'; "
                    + block
                    + "\nif($LASTEXITCODE -ne 0){exit $LASTEXITCODE}",
                ]
                if language == "powershell"
                else ["sh", "-eu", "-c", block]
            )
            if (
                phase("guide-" + str(index), command, cwd=package, timeout=20).strip()
                != expected
            ):
                raise ValueError("authored package guide output differs")
        (work / "tests").mkdir()
        env = {
            **os.environ,
            "HTMLCUT_TEST_BINARY": str(binary),
            "CARGO_TARGET_DIR": str(work / "target"),
            "CARGO_BUILD_BUILD_DIR": str(work / "build"),
            "TMPDIR": str(work / "tests"),
            "TMP": str(work / "tests"),
            "TEMP": str(work / "tests"),
        }
        log = phase(
            "native-tests",
            [
                "cargo",
                "test",
                "--locked",
                "-p",
                "htmlcut-cli",
                "--tests",
                "--",
                "--include-ignored",
                "--nocapture",
            ],
            cwd=root,
            env=env,
        )
        counts = [
            int(count) for count in re.findall(rb"HTMLCUT_CONFORMANCE_CASES=(\d+)", log)
        ]
        expected = sum(
            sum(path.is_dir() for path in (root / "conformance" / kind).iterdir())
            for kind in ("contract", "outline")
        )
        if expected == 0 or counts != [expected]:
            raise ValueError(
                "delivered-binary corpus evidence is missing or incomplete"
            )
        report["conformance_cases"] = expected
        phase(
            "acquisition",
            [
                sys.executable,
                "-m",
                "unittest",
                "tools.htmlcut_maintenance.tests.test_acquisition",
                "-v",
            ],
            cwd=root,
            env=env,
            timeout=120,
        )
        if hashlib.sha256(binary.read_bytes()).hexdigest() != digest:
            raise ValueError("acceptance changed production executable bytes")
        report["status"] = "pass"
    except BaseException as error:
        report["status"] = "fail"
        report["error"] = str(error)
        print(
            "Native acceptance evidence retained at "
            + str(evidence)
            + "; work "
            + str(work),
            file=sys.stderr,
        )
        raise
    else:
        shutil.rmtree(work)
        print("Native acceptance evidence: " + str(evidence), file=sys.stderr)
        return digest
    finally:
        (evidence / "acceptance.json").write_text(json.dumps(report, indent=2) + "\n")


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--archive", type=Path, required=True)
    p.add_argument("--target", required=True)
    p.add_argument("--version", required=True)
    a = p.parse_args()
    print(accept(a.archive, a.target, a.version))
    return 0
