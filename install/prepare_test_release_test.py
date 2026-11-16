# SPDX-License-Identifier: AGPL-3.0-only
"""Keep local qualification from laundering changed release archives."""
import hashlib
import importlib.util
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("release_staging", Path(__file__).with_name("prepare-test-release.py"))
staging = importlib.util.module_from_spec(spec)
spec.loader.exec_module(staging)


class ReleaseStagingTests(unittest.TestCase):
    def test_archive_hash_is_verified_before_staging(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            archive = root / "htmlcut-aarch64-apple-darwin.tar.gz"
            archive.write_bytes(b"reviewed archive")
            line = hashlib.sha256(archive.read_bytes()).hexdigest() + " *" + archive.name + "\n"
            manifest = root / "sha256.sum"
            manifest.write_text(line)
            archive.write_bytes(b"changed archive")
            with self.assertRaisesRegex(ValueError, "original release manifest"):
                staging.prepare(root, "main")
            self.assertEqual(manifest.read_text(), line)
            self.assertFalse((root / "install.sh").exists())
            archive.write_bytes(b"reviewed archive")
            staging.prepare(root, "main")
            self.assertTrue(manifest.read_text().startswith(line))
            self.assertIn("  install.sh\n", manifest.read_text())
            self.assertIn("  install.ps1\n", manifest.read_text())
            with self.assertRaisesRegex(ValueError, "existing installer"):
                staging.prepare(root, "main")

    def test_native_dist_sidecar_checksum_is_verified(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            archive = root / "htmlcut-aarch64-apple-darwin.tar.gz"
            archive.write_bytes(b"native archive")
            sidecar = Path(str(archive) + ".sha256")
            with self.assertRaisesRegex(ValueError, "original per-archive"):
                staging.prepare(root, "main")
            sidecar.write_text("0" * 64 + " *" + archive.name + "\n")
            with self.assertRaisesRegex(ValueError, "original release manifest"):
                staging.prepare(root, "main")
            sidecar.write_text(hashlib.sha256(archive.read_bytes()).hexdigest() + " *" + archive.name + "\n")
            staging.prepare(root, "main")
            self.assertIn("  install.ps1\n", (root / "sha256.sum").read_text())

    def test_missing_or_duplicate_archive_hash_is_refused(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            archive = root / "htmlcut-aarch64-apple-darwin.tar.gz"
            archive.write_bytes(b"archive")
            manifest = root / "sha256.sum"
            manifest.write_text("0" * 64 + "  other.zip\n")
            with self.assertRaisesRegex(ValueError, "original release manifest"):
                staging.prepare(root, "main")
            line = hashlib.sha256(archive.read_bytes()).hexdigest() + "  " + archive.name + "\n"
            manifest.write_text(line + line)
            with self.assertRaisesRegex(ValueError, "duplicate"):
                staging.prepare(root, "main")
            self.assertFalse((root / "install.sh").exists())


if __name__ == "__main__":
    unittest.main()
