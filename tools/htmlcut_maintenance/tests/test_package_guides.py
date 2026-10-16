# SPDX-License-Identifier: MPL-2.0
"""Package README rendering and native-shell example checks."""

from pathlib import Path
import re
import shlex
import shutil
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[3]


class PackageGuideTests(unittest.TestCase):
    version = "7.8.9"
    source = "a" * 40

    def render(self, root, target):
        package = root / target
        package.mkdir()
        binary = "htmlcut.exe" if target == "x86_64-pc-windows-msvc" else "htmlcut"
        from tools.htmlcut_maintenance.release.package import guide
        from tools.htmlcut_maintenance.config import targets

        row = next(row for row in targets() if row["triple"] == target)
        (package / "README.md").write_text(guide(self.version, row, self.source))
        return package, (package / "README.md").read_text()

    def test_guides_keep_native_shells_source_and_legal_instructions(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for target in (
                "aarch64-apple-darwin",
                "x86_64-apple-darwin",
                "x86_64-unknown-linux-musl",
                "x86_64-pc-windows-msvc",
            ):
                with self.subTest(target=target):
                    _, guide = self.render(root, target)
                    blocks = re.findall(r"^```(\w+)\n(.*?)^```", guide, re.M | re.S)
                    windows = target == "x86_64-pc-windows-msvc"
                    self.assertEqual(
                        [shell for shell, _ in blocks[:2]],
                        ["powershell" if windows else "sh"] * 2,
                    )
                    command = r".\htmlcut.exe" if windows else "./htmlcut"
                    self.assertEqual(blocks[0][1], command + " --version\n")
                    self.assertIn(
                        command
                        + " get 'article a.more' ./page.html --read attr:href --raw\n",
                        blocks[1][1],
                    )
                    if windows:
                        self.assertIn(
                            "Set-Content -Encoding UTF8 ./page.html", blocks[1][1]
                        )
                        self.assertNotIn("printf", blocks[1][1])
                    self.assertIn("`LICENSE`, `NOTICE`, and `PATENTS.md`", guide)
                    self.assertIn(
                        f"/download/v{self.version}/htmlcut-source-{self.version}.tar.gz",
                        guide,
                    )
                    self.assertIn(
                        f"/download/v{self.version}/htmlcut-source-{self.version}.zip",
                        guide,
                    )
                    self.assertIn("/tree/" + self.source, guide)
                    self.assertIn(
                        "complete locked native dependency attribution", guide
                    )
                    if "apple" in target:
                        self.assertIn(
                            "codesign --verify --strict --verbose=2 ./htmlcut", guide
                        )
                        self.assertIn("without Developer ID", guide)
                        self.assertIn("https://support.apple.com/en-us/102445", guide)
                    else:
                        self.assertNotIn("codesign", guide)
                        self.assertNotIn("Gatekeeper", guide)

    def exercise_examples(self, target, shell, powershell=False):
        with tempfile.TemporaryDirectory() as directory:
            package, guide = self.render(Path(directory), target)
            # Replace only the executable endpoint, so native shells still parse
            # and execute the documented fixture creation and CLI arguments.
            consumer = package / "guide-consumer.py"
            consumer.write_text("""import pathlib, sys
if sys.argv[1:] == ['--version']:
    print('htmlcut 7.8.9')
else:
    assert sys.argv[1:] == ['get', 'article a.more', './page.html', '--read', 'attr:href', '--raw'], sys.argv
    page = pathlib.Path('page.html').read_text(encoding='utf-8-sig').rstrip('\\r\\n')
    assert page == '<article><a class="more" href="../guide.html">Read more</a></article>', repr(page)
    print('../guide.html')
""")
            language = "powershell" if powershell else "sh"
            blocks = re.findall(r"^```" + language + r"\n(.*?)^```", guide, re.M | re.S)
            self.assertEqual(len(blocks), 2)
            if powershell:
                executable = sys.executable.replace("'", "''")
                endpoint = "& '" + executable + "' './guide-consumer.py'"
                marker = r".\htmlcut.exe"
            else:
                endpoint = shlex.quote(sys.executable) + " ./guide-consumer.py"
                marker = "./htmlcut"
            for block, expected in zip(
                blocks, ["htmlcut " + self.version, "../guide.html"]
            ):
                self.assertEqual(block.count(marker), 1)
                command = block.replace(marker, endpoint)
                invocation = (
                    [
                        shell,
                        "-NoLogo",
                        "-NoProfile",
                        "-NonInteractive",
                        "-Command",
                        "$ErrorActionPreference='Stop'; " + command,
                    ]
                    if powershell
                    else [shell, "-c", command]
                )
                result = subprocess.run(
                    invocation, cwd=package, capture_output=True, text=True, timeout=20
                )
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(result.stdout.splitlines(), [expected])
                self.assertEqual(result.stderr, "")

    @unittest.skipUnless(shutil.which("sh"), "requires a POSIX shell")
    def test_unix_guide_examples_execute_in_sh(self):
        self.exercise_examples("x86_64-unknown-linux-musl", shutil.which("sh"))

    @unittest.skipUnless(
        shutil.which("powershell.exe") or shutil.which("pwsh"), "requires PowerShell"
    )
    def test_windows_guide_examples_execute_in_powershell(self):
        self.exercise_examples(
            "x86_64-pc-windows-msvc",
            shutil.which("powershell.exe") or shutil.which("pwsh"),
            powershell=True,
        )
