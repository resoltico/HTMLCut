# SPDX-License-Identifier: MPL-2.0
"""Build exact-source native archives with a single native archiver."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tarfile
import tempfile
import zipfile
from .. import config, process
from . import signature
from .source import git, resolve


def guide(version, target, source):
    binary_name = "htmlcut.exe" if target["archive"] == "zip" else "htmlcut"
    binary_command = (
        ".\\" + binary_name if target["archive"] == "zip" else "./" + binary_name
    )
    fixture = '<article><a class="more" href="../guide.html">Read more</a></article>'
    example_shell = "powershell" if target["archive"] == "zip" else "sh"
    fixture_command = (
        f"'{fixture}' | Set-Content -Encoding UTF8 ./page.html"
        if example_shell == "powershell"
        else f"printf '%s\\n' '{fixture}' > ./page.html"
    )
    text = (
        Path(__file__)
        .with_name("package-readme.md")
        .read_text(encoding="utf-8")
        .format(
            version=version,
            target_triple=target["triple"],
            binary_name=binary_name,
            binary_command=binary_command,
            example_shell=example_shell,
            fixture_command=fixture_command,
            source_commit=source,
        )
    )
    if target.get("apple_floor"):
        text += """\n## macOS signature\n\nThe executable is signed ad hoc for sealed-code integrity, without Developer ID\nidentity or notarization. Verify extracted bytes with:\n\n```sh\ncodesign --verify --strict --verbose=2 ./htmlcut\n```\n\nAn ad-hoc signature does not guarantee Gatekeeper acceptance of quarantined\ndownloads. For a trusted download follow [Apple’s per-application approval guidance](https://support.apple.com/en-us/102445).\nChecksums and GitHub provenance are separate reference checks.\n"""
    return text


def pack(directory, archive, form):
    directory = Path(directory)
    archive = Path(archive)
    if archive.exists():
        raise ValueError("archive output must be new")
    if form == "zip":
        with zipfile.ZipFile(archive, "x", compression=zipfile.ZIP_DEFLATED) as bundle:
            for path in sorted(directory.iterdir()):
                bundle.write(path, directory.name + "/" + path.name)
    elif form == "tar.gz":
        with tarfile.open(archive, "x:gz") as bundle:
            bundle.add(directory, arcname=directory.name)
    else:
        raise ValueError("unsupported archive form")


def build(triple, output, root=config.ROOT):
    config.prerequisites()
    root = Path(root)
    output = Path(output).resolve()
    target = next((r for r in config.targets() if r["triple"] == triple), None)
    if target is None:
        raise ValueError("unsupported native target")
    if target.get("apple_floor") and not Path("/usr/bin/codesign").is_file():
        raise ValueError("Apple packaging requires native codesign")
    sha, version = resolve(root, "HEAD")
    if git(root, "status", "--porcelain"):
        raise ValueError("package source must be clean")
    output.mkdir(parents=True, exist_ok=True)
    work = Path(
        tempfile.mkdtemp(prefix="htmlcut-package-", dir=os.environ.get("RUNNER_TEMP"))
    )
    try:
        env = {
            **os.environ,
            "CARGO_TARGET_DIR": str(work / "target"),
            "CARGO_BUILD_BUILD_DIR": str(work / "build"),
        }
        if target.get("apple_floor"):
            env["MACOSX_DEPLOYMENT_TARGET"] = target["apple_floor"]
        command = [
            "cargo",
            "+" + config.toolchain(),
            "build",
            "--locked",
            "--profile",
            "dist",
            "-p",
            "htmlcut-cli",
            "--bin",
            "htmlcut",
            "--target",
            triple,
            "--message-format=json",
        ]
        result = process.run(
            command,
            work,
            disk_limit_mib=4096,
            timeout=1200,
            label="build",
            log_limit_mib=16,
            cwd=root,
            env=env,
        )
        if process.status(result):
            raise ValueError("native build failed; evidence " + str(work))
        binaries = []
        for line in (work / "build.log").read_text(encoding="utf-8").splitlines():
            try:
                message = json.loads(line)
            except ValueError:
                continue
            if (
                message.get("reason") == "compiler-artifact"
                and message.get("executable")
                and message["target"]["name"] == "htmlcut"
            ):
                binaries.append(Path(message["executable"]))
        if len(binaries) != 1:
            raise ValueError("Cargo did not identify exactly one executable artifact")
        directory = work / f"htmlcut-{version}-{triple}"
        directory.mkdir()
        binary = directory / (
            "htmlcut.exe" if target["archive"] == "zip" else "htmlcut"
        )
        shutil.copy2(binaries[0], binary)
        if os.name != "nt":
            binary.chmod(0o755)
        shutil.copy2(root / "LICENSE", directory / "LICENSE")
        (directory / "PATENTS.md").write_text(
            re.sub(
                r"^<!--\n.*?^-->\n",
                "",
                (root / "PATENTS.md").read_text(encoding="utf-8"),
                flags=re.M | re.S,
            ),
            encoding="utf-8",
        )
        (directory / "README.md").write_text(guide(version, target, sha), encoding="utf-8")
        subprocess.run(
            [
                sys.executable,
                "-m",
                "tools.htmlcut_maintenance",
                "release",
                "notice",
                "--target",
                triple,
                "--source",
                sha,
                "--output",
                str(directory / "NOTICE"),
            ],
            cwd=root,
            check=True,
            timeout=600,
        )
        if git(root, "rev-parse", "HEAD") != sha or git(root, "status", "--porcelain"):
            raise ValueError("package source changed during build")
        if target.get("apple_floor"):
            subprocess.run(
                [
                    "/usr/bin/codesign",
                    "--force",
                    "--sign",
                    "-",
                    "--timestamp=none",
                    "--identifier",
                    "htmlcut",
                    str(binary),
                ],
                check=True,
                timeout=30,
            )
            signature.verify(binary)
        original = hashlib.sha256(binary.read_bytes()).hexdigest()
        archive = output / (directory.name + "." + target["archive"])
        pack(directory, archive, target["archive"])
        from .accept import unpack

        with tempfile.TemporaryDirectory(prefix="htmlcut-roundtrip-") as temporary:
            delivered = unpack(archive, target, version, Path(temporary))
            unpacked = delivered / binary.name
            if hashlib.sha256(unpacked.read_bytes()).hexdigest() != original:
                raise ValueError("archiving changed executable bytes")
            if target.get("apple_floor"):
                signature.verify(unpacked)
        shutil.rmtree(work)
        return archive
    except BaseException:
        print("Package evidence retained at " + str(work), file=sys.stderr)
        raise


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("target")
    p.add_argument("--output", type=Path, required=True)
    a = p.parse_args()
    print(build(a.target, a.output))
    return 0
