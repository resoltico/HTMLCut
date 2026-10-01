"""Existing names, forged checksums and old-tag reruns must not create false release success."""
import copy
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("release_assets", Path(__file__).resolve().parents[1] / "scripts/release-assets.py")
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class ReleaseAssetsTest(unittest.TestCase):
    def test_initial_publication_cannot_downgrade_latest(self):
        module.reject_downgrade("15.0.0", [dict(tagName="v14.0.0")])
        module.reject_downgrade("15.0.0", [])
        with self.assertRaises(ValueError): module.reject_downgrade("15.0.0", [dict(tagName="v16.0.0")])
        with self.assertRaises(ValueError): module.reject_downgrade("15.0.0", [dict(tagName="v14.0.0")] * 1000)
    def test_checksum_inventory_requires_exact_names_and_bytes(self):
        wanted = {"a.tar.gz": hashlib.sha256(b"a").hexdigest(), "b.zip": hashlib.sha256(b"b").hexdigest()}
        raw = "".join(f"{value}  {name}\n" for name, value in wanted.items()).encode()
        module.checksum_inventory(raw, wanted)
        for bad in [raw + raw, raw.splitlines(keepends=True)[0], raw.replace(b"a.tar.gz", b"../a.tar.gz"), raw.replace(b"b.zip", b"c.zip"), raw.replace(wanted["a.tar.gz"].encode(), b"0" * 64)]:
            with self.assertRaises(ValueError): module.checksum_inventory(bad, wanted)

    def test_provider_retry_only_allows_missing_draft_assets(self):
        release = dict(tagName="v15.0.0", name="v15.0.0", body="source notes\n", isDraft=True, isPrerelease=False,
            assets=[dict(name="a", size=1)])
        module.provider_inventory(release, "v15.0.0", "source notes", ["a", "b"], True)
        with self.assertRaises(ValueError): module.provider_inventory(release, "v15.0.0", "source notes", ["a", "b"], False)
        public = copy.deepcopy(release); public["isDraft"] = False
        with self.assertRaises(ValueError): module.provider_inventory(public, "v15.0.0", "source notes", ["a", "b"], True)
        for field, value in [("tagName", "v14.0.0"), ("name", "other"), ("body", "other notes"), ("isPrerelease", True)]:
            bad = copy.deepcopy(release); bad[field] = value
            with self.assertRaises(ValueError): module.provider_inventory(bad, "v15.0.0", "source notes", ["a"], True)
        for assets in [[dict(name="a", size=1)] * 2, [dict(name="foreign", size=1)], [dict(name="a", size=module.MAX_ASSET + 1)], [dict(name="a", size=True)]]:
            bad = copy.deepcopy(release); bad["assets"] = assets
            with self.assertRaises(ValueError): module.provider_inventory(bad, "v15.0.0", "source notes", ["a"], True)

    def test_published_retry_returns_before_any_edit(self):
        project = Path(__file__).resolve().parents[1]
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            def git(*args): subprocess.run(["git", *args], cwd=root, check=True, capture_output=True)
            git("init"); git("config", "user.name", "Release fixture"); git("config", "user.email", "fixture@example.invalid")
            (root / "Cargo.toml").write_text('[workspace.package]\nversion = "15.0.0"\n')
            (root / "changelog.md").write_text('## [15.0.0]\nsource notes\n')
            git("add", "."); git("commit", "-m", "fixture"); git("tag", "-a", "v15.0.0", "-m", "fixture")
            names = subprocess.check_output(["bash", str(project / "scripts/release-targets.sh"), "assets", "--version", "15.0.0"], text=True).splitlines()
            local = root / "dist"; local.mkdir()
            manifest = "htmlcut-15.0.0-checksums.txt"
            for name in names:
                if name != manifest: (local / name).write_bytes((name + "\x1b[31m\r\n").encode())
            (local / manifest).write_text(''.join(f'{hashlib.sha256((local/name).read_bytes()).hexdigest()}  {name}\n' for name in names if name != manifest))
            remote = root / "remote"; shutil.copytree(local, remote)
            bin_dir = root / "bin"; bin_dir.mkdir()
            gh = bin_dir / "gh"
            gh.write_text('''#!/usr/bin/env python3
import json, os, pathlib, sys
root = pathlib.Path(os.environ['HTMLCUT_RELEASE_FIXTURE'])
args = sys.argv[1:]
if args[:2] == ['release','view']:
    if '--jq' in args: print('false')
    else:
        assets = [dict(name=p.name,size=p.stat().st_size) for p in (root/'remote').iterdir()]
        print(json.dumps(dict(tagName='v15.0.0',name='v15.0.0',body='source notes',isDraft=False,isPrerelease=False,assets=assets)))
elif args[:2] == ['release','download']:
    assert '--allow-escape-sequences' in args
    name = args[args.index('--pattern')+1]
    sys.stdout.buffer.write((root/'remote'/name).read_bytes())
else:
    (root/'unexpected-mutation').write_text(' '.join(args))
    sys.exit(99)
''')
            gh.chmod(0o755)
            env = {**os.environ, "PATH": str(bin_dir)+os.pathsep+os.environ["PATH"], "GH_TOKEN": "fixture", "HTMLCUT_RELEASE_FIXTURE": str(root)}
            script = 'source "$1"; htmlcut_repo_root_from_script_dir() { printf "%s\\n" "$HTMLCUT_RELEASE_FIXTURE"; }; main v15.0.0'
            def publish(): return subprocess.run(["bash", "-c", script, "fixture", str(project / "scripts/publish-github-release.sh")], cwd=root, env=env, capture_output=True, timeout=60)
            result = publish()
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn(b"no edit performed", result.stdout)
            self.assertFalse((root / "unexpected-mutation").exists())
            path = remote / next(name for name in names if name != manifest)
            old = path.read_bytes(); path.write_bytes(b"X" + old[1:])
            result = publish()
            self.assertNotEqual(result.returncode, 0)
            self.assertIn(b"asset bytes differ", result.stderr)
            self.assertFalse((root / "unexpected-mutation").exists())

    def test_bounded_file_digest(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "asset"; path.write_bytes(b"abc")
            self.assertEqual(module.digest(path), (3, hashlib.sha256(b"abc").hexdigest()))
            path.write_bytes(b"")
            with self.assertRaises(ValueError): module.digest(path)
