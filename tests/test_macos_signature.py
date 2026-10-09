# SPDX-License-Identifier: MPL-2.0
"""Real macOS code-signature controls at the extracted package boundary."""
import hashlib
import importlib.util
from pathlib import Path
import platform
import shutil
import subprocess
import tarfile
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / 'scripts/verify-macos-signature.py'
spec = importlib.util.spec_from_file_location('macos_signature', SCRIPT)
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


@unittest.skipUnless(platform.system() == 'Darwin', 'requires real macOS codesign')
class MacosSignatureTests(unittest.TestCase):
    def test_package_smoke_refuses_unsigned_and_tampered_before_execution(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'dist').mkdir()
            shutil.copy2(ROOT / 'Cargo.toml', root / 'Cargo.toml')
            version = subprocess.check_output(['bash', str(ROOT / 'scripts/workspace-version.sh')], text=True).strip()
            target = 'aarch64-apple-darwin' if platform.machine() == 'arm64' else 'x86_64-apple-darwin'
            name = f'htmlcut-{version}-{target}'
            package = root / name; package.mkdir()
            source = root / 'fixture.c'
            source.write_text('#include <stdio.h>\nint main(void) { puts("EXECUTED"); return 0; }\n')
            binary = package / 'htmlcut'
            subprocess.run(['/usr/bin/clang', str(source), '-o', str(binary)], check=True, capture_output=True)
            for filename in ('LICENSE', 'PATENTS.md'):
                shutil.copy2(ROOT / filename, package / filename)
            (package / 'NOTICE').write_text('Native dependency attribution:\nPermission\n')
            subprocess.run(['bash', '-c', 'source "$1"; write_packaged_readme "$2" "$3" "$4" htmlcut ./htmlcut "$5"',
                            'guide', str(ROOT / 'scripts/build-release-artifact.sh'), str(package), version, target, 'a' * 40],
                           check=True, capture_output=True)
            original = binary.read_bytes()
            for control in ('unsigned', 'tampered'):
                binary.write_bytes(original)
                subprocess.run(['/usr/bin/codesign', '--force', '--sign', '-', '--timestamp=none',
                                '--identifier', 'htmlcut', str(binary)], check=True, capture_output=True)
                if control == 'unsigned':
                    subprocess.run(['/usr/bin/codesign', '--remove-signature', str(binary)], check=True, capture_output=True)
                else:
                    layout = module.run('/usr/bin/otool', '-l', str(binary))
                    section = module.re.search(r'sectname __text\s+segname __TEXT\s+addr 0x[0-9a-fA-F]+\s+'
                                               r'size 0x[0-9a-fA-F]+\s+offset ([0-9]+)', layout)
                    offset = int(section[1])
                    with binary.open('r+b') as stream:
                        stream.seek(offset); byte = stream.read(1)
                        stream.seek(offset); stream.write(bytes([byte[0] ^ 1]))
                with tarfile.open(root / 'dist' / f'{name}.tar.gz', 'w:gz') as stream:
                    stream.add(package, arcname=name)
                result = subprocess.run(['bash', '-c',
                    'source "$1"; fixture_root="$2"; '
                    'htmlcut_repo_root_from_script_dir() { printf "%s\\n" "$fixture_root"; }; main "$3"',
                    'smoke', str(ROOT / 'scripts/smoke-release-artifact.sh'), str(root), target],
                    capture_output=True, text=True, timeout=30)
                self.assertNotEqual(result.returncode, 0, control)
                self.assertIn('codesign failed', result.stderr, control)
                self.assertNotIn('EXECUTED', result.stdout, control)

    def test_signed_archive_roundtrip_and_real_rejections(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / 'fixture.c'
            source.write_text('int main(void) { return 0; }\n')
            binary = root / 'htmlcut'
            subprocess.run(['/usr/bin/clang', str(source), '-o', str(binary)], check=True, capture_output=True)
            subprocess.run(['/usr/bin/codesign', '--force', '--sign', '-', '--timestamp=none',
                            '--identifier', 'htmlcut', str(binary)], check=True, capture_output=True)
            expected = hashlib.sha256(binary.read_bytes()).hexdigest()
            archive = root / 'package.tar.gz'
            with tarfile.open(archive, 'w:gz') as stream:
                stream.add(binary, arcname='htmlcut')
            extracted = root / 'extracted'; extracted.mkdir()
            with tarfile.open(archive) as stream:
                (extracted / 'htmlcut').write_bytes(stream.extractfile('htmlcut').read())
            proof = module.verify(extracted / 'htmlcut')
            self.assertEqual(proof['binary_sha256'], expected)
            self.assertEqual(proof['identifier'], 'htmlcut')
            controls = module.rejection_controls(extracted / 'htmlcut')
            self.assertEqual([row['control'] for row in controls], ['unsigned', 'tampered'])
            for row in controls:
                self.assertNotEqual(row['strict_exit_status'], 0)
                self.assertNotEqual(row['verifier_exit_status'], 0)
                self.assertTrue(row['mach_o_readable'])
            self.assertTrue(controls[1]['load_commands_unchanged'])
            self.assertEqual(module.verify(extracted / 'htmlcut')['binary_sha256'], expected)
            # A valid signature with the wrong product identifier must also be refused.
            subprocess.run(['/usr/bin/codesign', '--force', '--sign', '-', '--timestamp=none',
                            '--identifier', 'other-product', str(binary)], check=True, capture_output=True)
            with self.assertRaisesRegex(RuntimeError, 'Identifier=htmlcut'):
                module.verify(binary)


class PackageGuideTests(unittest.TestCase):
    def test_signature_instructions_only_in_apple_packages(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for target in ('aarch64-apple-darwin', 'x86_64-apple-darwin',
                           'x86_64-unknown-linux-musl', 'x86_64-pc-windows-msvc'):
                package = root / target; package.mkdir()
                subprocess.run(['bash', '-c', 'source "$1"; write_packaged_readme "$2" 21.0.0 "$3" htmlcut ./htmlcut "$4"',
                                'guide', str(ROOT / 'scripts/build-release-artifact.sh'), str(package), target, 'a' * 40],
                               check=True, capture_output=True)
                guide = (package / 'README.md').read_text()
                if 'apple' in target:
                    self.assertIn('codesign --verify --strict --verbose=2 ./htmlcut', guide)
                    self.assertIn('no Developer ID identity or Apple notarization', guide)
                    self.assertIn('https://support.apple.com/en-us/102445', guide)
                else:
                    self.assertNotIn('codesign', guide)
                    self.assertNotIn('Gatekeeper', guide)
