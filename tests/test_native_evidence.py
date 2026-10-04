# SPDX-License-Identifier: MPL-2.0
"""Package evidence must bind bounded bytes and an actual successful smoke process."""
import hashlib
import importlib.util
from pathlib import Path
import subprocess
import platform
import sys
import shutil
import tempfile
import unittest
from unittest.mock import patch

SCRIPT = Path(__file__).resolve().parents[1] / "scripts/native-package-evidence.py"
spec = importlib.util.spec_from_file_location("native_evidence", SCRIPT)
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class NativeEvidenceTest(unittest.TestCase):
    def test_binding_rejects_wrong_package_version_source_or_host(self):
        source = "a" * 40
        package = Path.cwd() / "dist/htmlcut-15.0.0-aarch64-apple-darwin.tar.gz"
        def reply(*args):
            if args[:2] == ("git", "status"): return ""
            if args[:2] == ("git", "rev-parse"): return source
            if args[1] == "./scripts/workspace-version.sh": return "15.0.0"
            if args[-1] == "triples": return "aarch64-apple-darwin"
            return package.name
        with patch.object(module, "command", side_effect=reply), patch.object(module.platform, "system", return_value="Darwin"), patch.object(module.platform, "machine", return_value="arm64"):
            module.validate_binding(package, "aarch64-apple-darwin", "15.0.0", source, "bash")
            for bad_package, bad_version, bad_source in [(package.with_name("other.tar.gz"), "15.0.0", source), (package, "14.0.0", source), (package, "15.0.0", "b" * 40)]:
                with self.assertRaises(ValueError): module.validate_binding(bad_package, "aarch64-apple-darwin", bad_version, bad_source, "bash")
            with patch.object(module.platform, "machine", return_value="x86_64"):
                with self.assertRaises(ValueError): module.validate_binding(package, "aarch64-apple-darwin", "15.0.0", source, "bash")
            with patch.object(module.platform, "system", return_value="Linux"):
                with self.assertRaises(ValueError): module.validate_binding(package, "aarch64-apple-darwin", "15.0.0", source, "bash")

    def test_digest_rejects_empty_and_oversized_packages_before_reading(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "package"
            path.write_bytes(b"abc")
            self.assertEqual(module.package_digest(path), (3, hashlib.sha256(b"abc").hexdigest()))
            path.write_bytes(b"")
            with self.assertRaises(ValueError):
                module.package_digest(path)
            with path.open("wb") as stream:
                stream.truncate(128 * 1024 * 1024 + 1)
            with self.assertRaises(ValueError):
                module.package_digest(path)

    def test_failed_smoke_cannot_produce_a_success_manifest(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            def git(*args):
                subprocess.run(["git", *args], cwd=root, check=True, capture_output=True)
            git("init")
            git("config", "user.name", "Evidence fixture")
            git("config", "user.email", "fixture@example.invalid")
            (root / "scripts").mkdir()
            (root / "scripts/smoke-release-artifact.sh").write_text("#!/bin/sh\nexit 17\n")
            for script in ["release-targets.sh", "workspace-version.sh", "common.sh"]:
                shutil.copy2(SCRIPT.parent / script, root / "scripts" / script)
            (root / "Cargo.toml").write_text('[workspace.package]\nversion = "15.0.0"\n')
            (root / ".gitignore").write_text("dist/\n")
            git("add", ".")
            git("commit", "-m", "fixture")
            (root / "dist").mkdir()
            target = "aarch64-apple-darwin" if platform.system() == "Darwin" and platform.machine().lower() in ("arm64", "aarch64") else "x86_64-apple-darwin" if platform.system() == "Darwin" else "x86_64-pc-windows-msvc" if platform.system() == "Windows" else "x86_64-unknown-linux-musl"
            package = root / f"dist/htmlcut-15.0.0-{target}.{'zip' if platform.system() == 'Windows' else 'tar.gz'}"
            package.write_bytes(b"fixture bytes")
            evidence = root / "dist/evidence.json"
            # This test owns orchestration after binding, not host admission. Keep the real
            # failing smoke subprocess and filesystem; binding rejection has separate controls.
            harness = "import importlib.util,sys; spec=importlib.util.spec_from_file_location('evidence',sys.argv[1]); m=importlib.util.module_from_spec(spec); spec.loader.exec_module(m); m.validate_binding=lambda *args:None; sys.argv=sys.argv[1:]; m.main()"
            result = subprocess.run([sys.executable, "-c", harness, str(SCRIPT), "--package", str(package),
                                     "--version", "15.0.0", "--target", target,
                                     "--source-sha", subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip(),
                                     "--output", str(evidence), "--smoke-log", str(root / "dist/smoke.log")],
                                    cwd=root, capture_output=True, timeout=10)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn(b"returned non-zero exit status 17", result.stderr)
            self.assertFalse(evidence.exists())
