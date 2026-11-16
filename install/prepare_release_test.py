# SPDX-License-Identifier: AGPL-3.0-only
"""Standalone installer checksum artifacts identify the generated script bytes."""
import hashlib
import importlib.util
from pathlib import Path
import shutil
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("release_generator", Path(__file__).with_name("prepare-release.py"))
generator = importlib.util.module_from_spec(spec)
spec.loader.exec_module(generator)


class ReleaseGeneratorTests(unittest.TestCase):
    def test_checksum_manifest_identifies_both_stamped_scripts(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source = root / "install"
            source.mkdir()
            shutil.copyfile(Path(__file__).with_name("prepare-installers.py"), source / "prepare-installers.py")
            (source / "install.sh").write_text("docs_revision='main'\n")
            (source / "install.ps1").write_bytes(b"$documentationRevision = 'main'\r\n")
            generator.prepare_installers(root, "a" * 40)
            output = root / "target/installer-release"
            self.assertEqual(set(path.name for path in output.iterdir()), {"install.sh", "install.ps1", "installers.sha256"})
            entries = (output / "installers.sha256").read_text().splitlines()
            self.assertEqual(len(entries), 2)
            for line in entries:
                digest, name = line.split("  ")
                data = (output / name).read_bytes()
                self.assertEqual(digest, hashlib.sha256(data).hexdigest())
                self.assertIn(b"a" * 40, data)


if __name__ == "__main__":
    unittest.main()
