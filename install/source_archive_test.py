# SPDX-License-Identifier: AGPL-3.0-only
"""Package license copies and tracked-source publication boundaries."""
import hashlib
import importlib.util
import io
import os
from pathlib import Path, PurePosixPath
import shutil
import subprocess
import sys
import tarfile
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
            with self.assertRaisesRegex(ValueError, "Untracked source") as failure:
                source_archive.create_archive(root, output, "a" * 40)
            self.assertIn("unreviewed.rs", str(failure.exception))
            self.assertFalse(output.exists())

    @unittest.skipUnless(os.name == "posix", "production source packaging runs on a POSIX global runner")
    def test_release_entrypoints_do_not_create_untracked_bytecode(self):
        with tempfile.TemporaryDirectory() as tmp:
            temporary = Path(tmp)
            root = temporary / "repo"
            source = root / "install"
            source.mkdir(parents=True)
            originals = Path(__file__).resolve().parent
            for name in ["prepare-release.py", "prepare-installers.py", "prepare-test-release.py", "qualify-release.py", "source-archive.py"]:
                shutil.copyfile(originals / name, source / name)
            (source / "install.sh").write_text("docs_revision='main'\n")
            (source / "install.ps1").write_bytes(b"$documentationRevision = 'main'\r\n")
            (root / ".gitignore").write_text("/target/\n")
            subprocess.run(["git", "init", "--quiet", str(root)], check=True)
            subprocess.run(["git", "add", "."], cwd=root, check=True)
            subprocess.run(["git", "-c", "user.name=Source Fixture", "-c", "user.email=source@example.invalid", "-c", "commit.gpgsign=false", "-c", "core.hooksPath=" + str(temporary / "no-hooks"), "commit", "--quiet", "-m", "Source fixture"], cwd=root, check=True)
            tools = temporary / "tools"
            tools.mkdir()
            cargo = tools / "cargo"
            cargo.write_text("#!/usr/bin/env python3\nfrom pathlib import Path\nimport sys\nif sys.argv[1:] != ['vendor', '--locked', '--versioned-dirs', 'vendor']: raise SystemExit('unexpected Cargo invocation')\nPath('vendor/fixture').mkdir(parents=True)\nPath('vendor/fixture/source.rs').write_text('fixture dependency source')\nprint('[source.crates-io]\\nreplace-with = \"vendored-sources\"\\n[source.vendored-sources]\\ndirectory = \"vendor\"')\n")
            cargo.chmod(0o755)
            environment = dict(os.environ, PATH=str(tools) + os.pathsep + os.environ["PATH"])
            environment.pop("PYTHONDONTWRITEBYTECODE", None)
            environment.pop("PYTHONPYCACHEPREFIX", None)
            produced = subprocess.run([sys.executable, str(source / "prepare-release.py")], cwd=root, env=environment, capture_output=True, text=True)
            self.assertEqual(produced.returncode, 0, produced.stdout + produced.stderr)
            self.assertFalse(list(root.rglob("__pycache__")))
            delivered = temporary / "release"
            shutil.copytree(root / "target/installer-release", delivered)
            archive = delivered / "SOURCE.tar.gz"
            with tarfile.open(archive, "r:gz") as original:
                members = [(member, original.extractfile(member).read()) for member in original]
            # A precise source-boundary failure stops before Cargo metadata/build;
            # it still reaches tracked_source after all qualifier local imports.
            with tarfile.open(archive, "w:gz") as changed:
                extra = tarfile.TarInfo("htmlcut-source/untracked.txt")
                extra.size = 1
                changed.addfile(extra, io.BytesIO(b"x"))
                for member, data in members:
                    changed.addfile(member, io.BytesIO(data))
            Path(str(archive) + ".sha256").write_text(hashlib.sha256(archive.read_bytes()).hexdigest() + "  SOURCE.tar.gz\n")
            native = delivered / "htmlcut-x86_64-unknown-linux-musl.tar.gz"
            native.write_bytes(b"fixture native archive")
            (delivered / "sha256.sum").write_text(hashlib.sha256(native.read_bytes()).hexdigest() + "  " + native.name + "\n")
            qualified = subprocess.run([sys.executable, str(source / "qualify-release.py"), str(delivered)], cwd=root, env=environment, capture_output=True, text=True)
            self.assertNotEqual(qualified.returncode, 0)
            self.assertIn("unexpected non-vendor member", qualified.stdout + qualified.stderr)
            self.assertFalse(list(root.rglob("__pycache__")))


if __name__ == "__main__":
    unittest.main()
