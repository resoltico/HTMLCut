#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Extract exactly one changelog section from an immutable Git source revision."""
import argparse
import re
import subprocess
import sys

MAX_BYTES = 1024 * 1024

def section(text, name):
    headings = list(re.finditer(r"^## \[([^\]]+)\].*$", text, re.MULTILINE))
    matches = [i for i, match in enumerate(headings) if match.group(1) == name]
    if len(matches) != 1:
        raise ValueError('release section must occur exactly once')
    index = matches[0]
    end = headings[index + 1].start() if index + 1 < len(headings) else len(text)
    value = text[headings[index].end():end].strip()
    if not value:
        raise ValueError('release section must not be empty')
    return value + '\n'

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--ref', required=True)
    parser.add_argument('--version', required=True)
    parser.add_argument('--preview-unreleased', action='store_true')
    args = parser.parse_args()
    if not re.fullmatch(r'[1-9][0-9]*\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)', args.version):
        raise ValueError('stable release version required')
    commit = subprocess.check_output(['git', 'rev-parse', '--verify', args.ref + '^{commit}'], text=True).strip()
    child = subprocess.Popen(['git', 'show', commit + ':changelog.md'], stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
    data = child.stdout.read(MAX_BYTES + 1)
    if len(data) > MAX_BYTES:
        child.kill()
        child.wait()
        raise ValueError('changelog exceeds byte limit')
    if child.wait() != 0:
        raise ValueError('source changelog unavailable')
    notes = section(data.decode('utf-8'), 'Unreleased' if args.preview_unreleased else args.version)
    sys.stdout.write(notes)

if __name__ == '__main__':
    try:
        main()
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        print('release notes: ' + str(error), file=sys.stderr)
        sys.exit(1)
