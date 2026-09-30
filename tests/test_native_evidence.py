"""Package evidence must bind bounded bytes and an actual successful smoke process."""
import hashlib
import importlib.util
from pathlib import Path
import subprocess
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / "scripts/native-package-evidence.py"
spec = importlib.util.spec_from_file_location("native_evidence", SCRIPT)
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class NativeEvidenceTest(unittest.TestCase):
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
            (root / ".gitignore").write_text("dist/\n")
            git("add", ".")
            git("commit", "-m", "fixture")
            (root / "dist").mkdir()
            package = root / "dist/package.tar.gz"
            package.write_bytes(b"fixture bytes")
            evidence = root / "dist/evidence.json"
            result = subprocess.run(["python3", str(SCRIPT), "--package", str(package),
                                     "--version", "15.0.0", "--target", "fixture-target",
                                     "--output", str(evidence), "--smoke-log", str(root / "dist/smoke.log")],
                                    cwd=root, capture_output=True, timeout=10)
            self.assertNotEqual(result.returncode, 0)
            self.assertFalse(evidence.exists())
