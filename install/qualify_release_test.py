# SPDX-License-Identifier: MPL-2.0
"""Release consumption rejects stale scripts and invalid installer manifests."""
import hashlib
import importlib.util
from pathlib import Path
import shutil
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("release_qualification", Path(__file__).with_name("qualify-release.py"))
qualification = importlib.util.module_from_spec(spec)
spec.loader.exec_module(qualification)


class ReleaseQualificationTests(unittest.TestCase):
    def test_stale_script_or_manifest_fails_before_execution(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source = root / "install"
            source.mkdir()
            for name in ["prepare-installers.py", "prepare-test-release.py"]:
                shutil.copyfile(Path(__file__).with_name(name), source / name)
            (source / "install.sh").write_text("docs_revision='main'\n")
            (source / "install.ps1").write_bytes(b"$documentationRevision = 'main'\r\n")
            delivered = root / "release"
            delivered.mkdir()
            archive = delivered / "htmlcut-x86_64-unknown-linux-musl.tar.gz"
            archive.write_bytes(b"verified payload bytes")
            archive_manifest = delivered / "sha256.sum"
            archive_manifest.write_text(hashlib.sha256(archive.read_bytes()).hexdigest() + "  " + archive.name + "\n")
            (delivered / "install.sh").write_text("stale installer")
            with self.assertRaisesRegex(ValueError, "differs from source revision: install.sh"):
                qualification.qualify(root, delivered, "a" * 40)
            (delivered / "install.sh").write_text("docs_revision='" + "a" * 40 + "'\n")
            (delivered / "install.ps1").write_bytes(b"$documentationRevision = '" + b"a" * 40 + b"'\r\n")
            (delivered / "installers.sha256").write_text("invalid manifest")
            with self.assertRaisesRegex(ValueError, "checksum manifest"):
                qualification.qualify(root, delivered, "a" * 40)
            (delivered / "installers.sha256").unlink()
            with self.assertRaises(FileNotFoundError):
                qualification.qualify(root, delivered, "a" * 40)
            archive_manifest.write_text("0" * 64 + "  " + archive.name + "\n")
            with self.assertRaisesRegex(ValueError, "original release manifest"):
                qualification.qualify(root, delivered, "a" * 40)
            archive_manifest.unlink()
            with self.assertRaises(FileNotFoundError) as failure:
                qualification.qualify(root, delivered, "a" * 40)
            self.assertEqual(Path(failure.exception.filename).name, "sha256.sum")


if __name__ == "__main__":
    unittest.main()
