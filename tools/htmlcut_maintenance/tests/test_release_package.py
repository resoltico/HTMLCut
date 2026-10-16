# SPDX-License-Identifier: MPL-2.0
"""Exact package inventory, type/mode and byte-preserving archive boundaries."""

import io
from pathlib import Path
import tarfile
import tempfile
import unittest
import zipfile
from tools.htmlcut_maintenance.release.accept import unpack
from tools.htmlcut_maintenance.release.package import pack
from tools.htmlcut_maintenance.config import targets


class PackageTests(unittest.TestCase):
    def fixture(self, root, target):
        package = root / f"htmlcut-23.0.0-{target['triple']}"
        package.mkdir()
        binary = "htmlcut.exe" if target["archive"] == "zip" else "htmlcut"
        for name in [binary, "LICENSE", "NOTICE", "PATENTS.md", "README.md"]:
            (package / name).write_bytes(b"original\x00\xff\r\n" + name.encode())
        (package / binary).chmod(0o755)
        return package, binary

    def test_tar_and_zip_roundtrip_preserve_exact_members_and_executable_mode(self):
        for row in targets():
            with (
                self.subTest(target=row["triple"]),
                tempfile.TemporaryDirectory() as temporary,
            ):
                root = Path(temporary)
                package, binary = self.fixture(root, row)
                archive = root / (package.name + "." + row["archive"])
                pack(package, archive, row["archive"])
                destination = root / "unpacked"
                destination.mkdir()
                result = unpack(archive, row, "23.0.0", destination)
                self.assertEqual(
                    {p.name for p in result.iterdir()},
                    {p.name for p in package.iterdir()},
                )
                for source in package.iterdir():
                    self.assertEqual(
                        (result / source.name).read_bytes(), source.read_bytes()
                    )
                if row["archive"] != "zip":
                    self.assertEqual((result / binary).stat().st_mode & 0o777, 0o755)

    def test_missing_extra_duplicate_linked_empty_and_nonexecutable_packages_refuse(
        self,
    ):
        row = next(row for row in targets() if row["triple"] == "aarch64-apple-darwin")
        for fault in [
            "missing",
            "extra",
            "empty",
            "nonexecutable",
            "linked",
            "duplicate",
            "directory-duplicate",
        ]:
            with self.subTest(fault=fault), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                package, binary = self.fixture(root, row)
                if fault == "missing":
                    (package / "NOTICE").unlink()
                if fault == "extra":
                    (package / "unreviewed").write_text("extra")
                if fault == "empty":
                    (package / "LICENSE").write_bytes(b"")
                if fault == "nonexecutable":
                    (package / binary).chmod(0o644)
                if fault == "linked":
                    (package / "NOTICE").unlink()
                    (package / "NOTICE").symlink_to("LICENSE")
                archive = root / "package.tar.gz"
                pack(package, archive, "tar.gz")
                if fault in ("duplicate", "directory-duplicate"):
                    with tarfile.open(archive, "w:gz") as bundle:
                        bundle.add(package, arcname=package.name)
                        if fault == "duplicate":
                            info = tarfile.TarInfo(package.name + "/NOTICE")
                            info.size = 1
                            bundle.addfile(info, io.BytesIO(b"x"))
                        else:
                            info = tarfile.TarInfo(package.name)
                            info.type = tarfile.DIRTYPE
                            bundle.addfile(info)
                destination = root / "unpacked"
                destination.mkdir()
                with self.assertRaises(ValueError):
                    unpack(archive, row, "23.0.0", destination)

    def test_oversized_directory_header_refuses_before_decompressing_its_claimed_payload(
        self,
    ):
        row = next(row for row in targets() if row["archive"] == "tar.gz")
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            archive = root / "package.tar.gz"
            with tarfile.open(archive, "w:gz") as bundle:
                directory = tarfile.TarInfo("htmlcut-23.0.0-" + row["triple"])
                directory.type = tarfile.DIRTYPE
                directory.size = 2**40
                bundle.addfile(directory)
            with self.assertRaises(ValueError):
                unpack(archive, row, "23.0.0", root / "out")

    def test_version_path_components_refuse_before_opening_the_archive(self):
        row = targets()[0]
        for version in (
            "../../outside",
            "1.2.3/../../outside",
            "v23.0.0",
            "23.0.0-rc1",
        ):
            with (
                self.subTest(version=version),
                self.assertRaisesRegex(ValueError, "canonical stable"),
            ):
                unpack(
                    Path("nonexistent-archive"), row, version, Path("must-not-create")
                )
