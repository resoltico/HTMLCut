#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Verify packaged macOS code integrity, optionally exercising real rejection controls."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile


def run(*args):
    result = subprocess.run(args, capture_output=True, text=True, timeout=30)
    if result.returncode:
        raise RuntimeError(f"{args[0]} failed ({result.returncode}): {result.stderr.strip()}")
    return result.stdout + result.stderr


def verify(binary):
    if not Path('/usr/bin/codesign').is_file():
        raise RuntimeError('macOS package verification requires /usr/bin/codesign on macOS')
    strict = run('/usr/bin/codesign', '--verify', '--strict', '--verbose=2', str(binary))
    display = run('/usr/bin/codesign', '--display', '--verbose=4', str(binary))
    lines = display.splitlines()
    if 'Signature=adhoc' not in lines or 'Identifier=htmlcut' not in lines:
        raise RuntimeError('macOS package requires an ad-hoc signature with Identifier=htmlcut')
    if any(line.startswith(('Authority=', 'Timestamp=')) for line in lines):
        raise RuntimeError('macOS ad-hoc package must have no certificate authority or timestamp service')
    return dict(binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),
                identifier='htmlcut', signature='adhoc', strict_exit_status=0,
                strict_output=strict.strip(), display_output=display.strip())


def rejection_controls(binary):
    # otool supplies the sealed section's file offset; mutate code, never a Mach-O header.
    layout = run('/usr/bin/otool', '-l', str(binary))
    section = re.search(r'sectname __text\s+segname __TEXT\s+addr 0x[0-9a-fA-F]+\s+'
                        r'size (0x[0-9a-fA-F]+)\s+offset ([0-9]+)', layout)
    if section is None or int(section[1], 16) == 0:
        raise RuntimeError('signature control needs a nonempty Mach-O __TEXT,__text section')
    offset = int(section[2])
    original = binary.read_bytes()
    if not 0 < offset < len(original):
        raise RuntimeError('Mach-O code section offset is outside the executable')
    rows = []
    with tempfile.TemporaryDirectory(prefix='htmlcut-signature-controls-') as directory:
        for name in ('unsigned', 'tampered'):
            copy = Path(directory) / name
            shutil.copy2(binary, copy)
            if name == 'unsigned':
                run('/usr/bin/codesign', '--remove-signature', str(copy))
            else:
                with copy.open('r+b') as stream:
                    stream.seek(offset)
                    stream.write(bytes([original[offset] ^ 1]))
                changed = copy.read_bytes()
                if len(changed) != len(original) or sum(a != b for a, b in zip(original, changed)) != 1:
                    raise RuntimeError('tamper control must alter exactly one sealed code byte')
                if run('/usr/bin/otool', '-l', str(copy)).splitlines()[1:] != layout.splitlines()[1:]:
                    raise RuntimeError('tamper control changed the Mach-O load commands')
            # An unsigned control must still be a readable Mach-O, not a corrupt fixture.
            run('/usr/bin/otool', '-h', str(copy))
            result = subprocess.run(['/usr/bin/codesign', '--verify', '--strict', '--verbose=2', str(copy)],
                                    capture_output=True, text=True, timeout=30)
            reason = 'not signed at all' if name == 'unsigned' else 'invalid signature'
            if result.returncode == 0 or reason not in result.stderr.lower():
                raise RuntimeError(f'{name} control did not demonstrate signature rejection: {result.stderr}')
            # Exercise the same public verifier used by package smoke and require nonzero exit.
            admission = subprocess.run([sys.executable, str(Path(__file__).resolve()), str(copy)],
                                       capture_output=True, text=True, timeout=30)
            if admission.returncode == 0:
                raise RuntimeError(f'{name} control passed the package signature boundary')
            row = dict(control=name, strict_exit_status=result.returncode,
                       verifier_exit_status=admission.returncode, diagnostic=result.stderr.strip(),
                       binary_sha256=hashlib.sha256(copy.read_bytes()).hexdigest(), mach_o_readable=True)
            if name == 'tampered':
                row.update(section='__TEXT,__text', changed_byte_offset=offset, changed_bytes=1,
                           load_commands_unchanged=True)
            rows.append(row)
    return rows


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('binary', type=Path)
    parser.add_argument('--rejection-controls', action='store_true')
    args = parser.parse_args()
    if args.binary.is_symlink() or not args.binary.is_file():
        raise RuntimeError('signature verification requires a regular packaged executable')
    evidence = verify(args.binary)
    if args.rejection_controls:
        evidence['rejection_controls'] = rejection_controls(args.binary)
        if verify(args.binary)['binary_sha256'] != evidence['binary_sha256']:
            raise RuntimeError('original signed executable changed during rejection controls')
    print(json.dumps(evidence, indent=2))


if __name__ == '__main__':
    main()
