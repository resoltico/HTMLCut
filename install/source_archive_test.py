# SPDX-License-Identifier: AGPL-3.0-only
"""Package license copies and tracked-source publication boundaries."""
import importlib.util
from pathlib import Path, PurePosixPath
import subprocess
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("source_archive", Path(__file__).with_name("source-archive.py"))
source_archive = importlib.util.module_from_spec(spec)
spec.loader.exec_module(source_archive)


class SourceArchiveTests(unittest.TestCase):
    def test_package_license_copies_match_authoritative_license(self):
        root = Path(__file__).resolve().parent.parent
        authoritative = (root / "LICENSE").read_bytes()
        for name in ["crates/htmlcut/LICENSE", "crates/htmlcut-core/LICENSE"]:
            self.assertEqual((root / name).read_bytes(), authoritative)

    def test_source_members_exclude_unknown_first_party_files(self):
        tracked = {"Cargo.toml", "src/main.rs"}
        for name in ["Cargo.toml", "src/main.rs", ".cargo/config.toml", "SOURCE.txt", "vendor/dependency/src/lib.rs"]:
            self.assertTrue(source_archive.source_member_allowed(PurePosixPath(name), tracked), name)
        for name in ["private-notes.txt", "target/build-cache", "src/untracked.rs", "vendor"]:
            self.assertFalse(source_archive.source_member_allowed(PurePosixPath(name), tracked), name)

    def test_untracked_source_is_refused_before_vendoring(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            subprocess.run(["git", "init", "--quiet", str(root)], check=True)
            (root / "unreviewed.rs").write_text("untracked source")
            output = root / "SOURCE.tar.gz"
            with self.assertRaisesRegex(ValueError, "Untracked source"):
                source_archive.create_archive(root, output, "a" * 40)
            self.assertFalse(output.exists())


if __name__ == "__main__":
    unittest.main()
