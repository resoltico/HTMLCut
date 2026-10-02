#!/usr/bin/env python3
"""Record bounded native package verification evidence without publishing a release."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import subprocess
import sys
import tarfile
import tempfile
import zipfile


def command(*args):
    return subprocess.run(args, check=True, capture_output=True, text=True, timeout=10).stdout.strip()


def package_digest(path):
    if path.is_symlink() or not path.is_file():
        raise ValueError("Native package must be a regular file")
    size = path.stat().st_size
    if size == 0 or size > 128 * 1024 * 1024:
        raise ValueError("Native package is empty or exceeds the artifact budget")
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(64 * 1024), b""):
            digest.update(chunk)
    return size, digest.hexdigest()


def validate_binding(package, target, version, source, shell):
    if not re.fullmatch(r"[0-9a-f]{40}", source):
        raise ValueError("Expected exact source commit")
    if command("git", "status", "--porcelain") or command("git", "rev-parse", "HEAD") != source:
        raise ValueError("Native source differs or is dirty")
    if command(shell, "./scripts/workspace-version.sh") != version:
        raise ValueError("Native version differs from source workspace")
    targets = command(shell, "./scripts/release-targets.sh", "triples").splitlines()
    if target not in targets:
        raise ValueError("Unsupported native target")
    names = command(shell, "./scripts/release-targets.sh", "assets", "--version", version).splitlines()
    expected = [name for name in names if f"-{target}." in name]
    if len(expected) != 1 or package.resolve() != (Path.cwd() / "dist" / expected[0]).resolve():
        raise ValueError("Evidence package differs from canonical smoke package")
    system = platform.system().lower()
    machine = platform.machine().lower()
    if ("apple" in target and system != "darwin") or ("linux" in target and system != "linux") or ("windows" in target and system != "windows"):
        raise ValueError("Native target differs from runner operating system")
    if (target.startswith("aarch64") and machine not in ("arm64", "aarch64")) or (target.startswith("x86_64") and machine not in ("amd64", "x86_64")):
        raise ValueError("Native target differs from runner architecture")



def unpack_binary(package, target, destination):
    name = "htmlcut.exe" if "windows" in target else "htmlcut"
    extension = ".zip" if "windows" in target else ".tar.gz"
    member_name = package.name[:-len(extension)] + "/" + name
    if extension == ".zip":
        with zipfile.ZipFile(package) as archive:
            member = archive.getinfo(member_name)
            if member.is_dir() or not 0 < member.file_size <= 128 * 1024 * 1024:
                raise ValueError("Packaged executable has an invalid size or kind")
            with archive.open(member) as stream:
                data = stream.read(member.file_size + 1)
            if len(data) != member.file_size:
                raise ValueError("Packaged executable size differs from archive metadata")
    else:
        with tarfile.open(package, "r:gz") as archive:
            member = archive.getmember(member_name)
            if not member.isfile() or not 0 < member.size <= 128 * 1024 * 1024:
                raise ValueError("Packaged executable must be a bounded regular member")
            with archive.extractfile(member) as stream:
                data = stream.read(member.size + 1)
            if len(data) != member.size:
                raise ValueError("Packaged executable size differs from archive metadata")
    binary = destination / name
    binary.write_bytes(data)
    binary.chmod(0o755)
    return binary


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--package", type=Path, required=True)
    parser.add_argument("--target", required=True)
    parser.add_argument("--version", required=True)
    parser.add_argument("--source-sha", required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--smoke-log", type=Path, required=True)
    parser.add_argument("--shell", default="bash", help="Exact native Bash executable (Git Bash on Windows)")
    args = parser.parse_args()
    paths = [args.package, args.output, args.smoke_log]
    if len({path.resolve() for path in paths}) != len(paths) or any(path.is_symlink() for path in paths):
        raise ValueError("Package, evidence and smoke log must be distinct regular destinations")
    dist = (Path.cwd() / "dist").resolve()
    if any(not path.resolve().is_relative_to(dist) for path in (args.output, args.smoke_log)):
        raise ValueError("Native evidence destinations must remain in dist")
    validate_binding(args.package, args.target, args.version, args.source_sha, args.shell)
    before = package_digest(args.package)
    io_command = ["cargo", "test", "-p", "htmlcut-cli", "--test", "io_boundary", "--target", args.target, "--locked"]
    binary_sha256 = None
    try:
        with args.smoke_log.open("wb") as log:
            subprocess.run([args.shell, "./scripts/smoke-release-artifact.sh", args.target],
                           check=True, stdout=log, stderr=subprocess.STDOUT, timeout=120)
            with tempfile.TemporaryDirectory(prefix="native-package-", dir=dist) as directory:
                binary = unpack_binary(args.package, args.target, Path(directory)).resolve()
                binary_sha256 = hashlib.sha256(binary.read_bytes()).hexdigest()
                matrix_path = args.output.with_name(args.output.stem + "-matrix.json")
                subprocess.run([sys.executable, "scripts/native-reliability-matrix.py", "--binary", str(binary),
                                "--output", str(matrix_path)], check=True, stdout=log, stderr=subprocess.STDOUT, timeout=600)
                environment = os.environ.copy()
                environment["HTMLCUT_TEST_BINARY"] = str(binary)
                if "windows" in args.target:
                    windows_path = args.output.with_name(args.output.stem + "-windows-stdio.json")
                    environment["HTMLCUT_WINDOWS_STDIO_EVIDENCE"] = str(windows_path.resolve())
                    windows_pipe_path = args.output.with_name(args.output.stem + "-windows-pipe-input.json")
                    environment["HTMLCUT_WINDOWS_PIPE_EVIDENCE"] = str(windows_pipe_path.resolve())
                subprocess.run(io_command, check=True, stdout=log, stderr=subprocess.STDOUT,
                               env=environment, timeout=600)
                if "linux" in args.target:
                    tls_path = args.output.with_name(args.output.stem + "-tls.json")
                    subprocess.run([sys.executable, "scripts/native-trusted-tls.py", "--binary", str(binary),
                                    "--source-sha", args.source_sha, "--output", str(tls_path),
                                    "--disposable-hosted-runner"], check=True, stdout=log,
                                   stderr=subprocess.STDOUT, timeout=300)
                if hashlib.sha256(binary.read_bytes()).hexdigest() != binary_sha256:
                    raise ValueError("Packaged executable changed during native I/O tests")
    except (subprocess.CalledProcessError, subprocess.TimeoutExpired):
        with args.smoke_log.open("rb") as log:
            log.seek(max(0, args.smoke_log.stat().st_size - 8192))
            sys.stderr.write(log.read(8192).decode("utf-8", errors="replace"))
        raise
    if args.smoke_log.stat().st_size > 2 * 1024 * 1024:
        raise ValueError("Native smoke log exceeds its evidence budget")
    validate_binding(args.package, args.target, args.version, args.source_sha, args.shell)
    size, digest = package_digest(args.package)
    if (size, digest) != before:
        raise ValueError("Native package changed during verification")
    evidence = {
        "schema": "htmlcut.native-package-evidence", "version": 2,
        "source_commit": command("git", "rev-parse", "HEAD"),
        "source_tree": command("git", "rev-parse", "HEAD^{tree}"),
        "product_version": args.version, "target": args.target,
        "runner_system": platform.system(), "runner_machine": platform.machine(),
        "rustc": command("rustc", "-Vv"),
        "package": args.package.name, "package_bytes": size, "package_sha256": digest,
        "smoke_command": ["./scripts/smoke-release-artifact.sh", args.target],
        "smoke_exit_status": 0,
        "binary_sha256": binary_sha256,
        "native_io_command": io_command,
        "native_io_exit_status": 0,
        "matrix": matrix_path.name,
        "matrix_sha256": hashlib.sha256(matrix_path.read_bytes()).hexdigest(),
        "smoke_log_sha256": hashlib.sha256(args.smoke_log.read_bytes()).hexdigest(),
        "github_run_id": os.environ.get("GITHUB_RUN_ID"),
        "github_run_attempt": os.environ.get("GITHUB_RUN_ATTEMPT"),
    }
    if "linux" in args.target:
        evidence["trusted_tls"] = tls_path.name
        evidence["trusted_tls_sha256"] = hashlib.sha256(tls_path.read_bytes()).hexdigest()
    if "windows" in args.target:
        windows_proof = json.loads(windows_path.read_text(encoding="utf-8"))
        if not windows_proof["passed"] or windows_proof["binary_sha256"] != binary_sha256:
            raise ValueError("Native Windows stdio proof differs from the packaged executable")
        evidence["windows_stdio"] = windows_path.name
        evidence["windows_stdio_sha256"] = hashlib.sha256(windows_path.read_bytes()).hexdigest()
        windows_pipe_proof = json.loads(windows_pipe_path.read_text(encoding="utf-8"))
        if not windows_pipe_proof["passed"] or windows_pipe_proof["binary_sha256"] != binary_sha256:
            raise ValueError("Native Windows pipe proof differs from the packaged executable")
        evidence["windows_pipe_input"] = windows_pipe_path.name
        evidence["windows_pipe_input_sha256"] = hashlib.sha256(windows_pipe_path.read_bytes()).hexdigest()
    args.output.write_text(json.dumps(evidence, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
