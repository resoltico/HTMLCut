# SPDX-License-Identifier: MPL-2.0
"""Workflow handoff must preserve source, inventory and credential boundaries."""
from pathlib import Path
import re
import json
import os
import subprocess
import sys
import tempfile
import unittest
import tomllib

ROOT = Path(__file__).resolve().parents[1]


class WorkflowWiringTest(unittest.TestCase):
    def test_fuzz_seeds_keep_exact_bytes_in_checkouts(self):
        seeds = sorted(str(path.relative_to(ROOT)) for path in (ROOT / "fuzz/seeds").rglob("*") if path.is_file())
        self.assertTrue(seeds, "retained document seeds must be discovered")
        values = subprocess.check_output(["git", "check-attr", "-z", "text", "diff", "--", *seeds], cwd=ROOT).decode().split("\0")[:-1]
        self.assertEqual(len(values), 6 * len(seeds))
        for path, attribute, value in zip(values[::3], values[1::3], values[2::3]):
            self.assertEqual(value, "unset", f"{path}: {attribute} must preserve byte inputs")

    def test_fuzz_targets_match_the_smoke_and_documentation(self):
        manifest = tomllib.loads((ROOT / "fuzz/Cargo.toml").read_text())
        targets = {entry["name"] for entry in manifest["bin"]}
        script = (ROOT / "scripts/focused-smoke.sh").read_text()
        shell_targets = set(re.search(r"^for target in ([^;\n]+); do", script, re.M).group(1).split())
        # Target declarations describe what each harness feeds. Other code
        # identifiers in the resource documentation are not target names.
        documented = set(re.findall(r"`([a-z][a-z0-9_]*)`\s+feeds\b", (ROOT / "fuzz/README.md").read_text()))
        self.assertEqual(targets, shell_targets)
        self.assertEqual(targets, documented)
        self.assertNotIn("--features", script)
        self.assertIn("document.outline", (ROOT / "fuzz/fuzz_targets/extract.rs").read_text())

    def test_cargo_configuration_keeps_each_checkout_build_isolated(self):
        for name in ["config", "config.toml"]:
            path = ROOT / ".cargo" / name
            if path.exists():
                config = tomllib.loads(path.read_text())
                self.assertFalse({"target-dir", "build-dir"} & config.get("build", {}).keys(),
                                 f"{path}: build routing belongs in personal environment settings")

    def test_ci_exports_complete_matrix_as_one_action_output(self):
        text = (ROOT / ".github/workflows/ci.yml").read_text()
        step = text.split("      - id: matrix\n", 1)[1].split("\n  linux-maintainer:", 1)[0]
        body = step.split("        run: |\n", 1)[1]
        command = "\n".join(line[10:] for line in body.splitlines())
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "output"
            subprocess.run(["bash", "-c", command], cwd=ROOT,
                           env={**os.environ, "GITHUB_OUTPUT": str(output)}, check=True)
            lines = output.read_text().splitlines()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "scripts").mkdir()
            producer = root / "scripts/release-targets.sh"
            producer.write_text("#!/usr/bin/env bash\nexit 23\n")
            producer.chmod(0o755)
            output = root / "output"
            failed = subprocess.run(["bash", "-c", command], cwd=root,
                                    env={**os.environ, "GITHUB_OUTPUT": str(output)})
            self.assertEqual(failed.returncode, 23)
            self.assertNotIn("EOF\n", output.read_text().split("\n", 1)[1])
        self.assertEqual(lines[0], "matrix<<EOF")
        self.assertEqual(lines[-1], "EOF")
        matrix = json.loads("\n".join(lines[1:-1]))
        self.assertEqual(len(matrix["include"]), 4)
        triples = subprocess.check_output([str(ROOT / "scripts/release-targets.sh"), "triples"], text=True).splitlines()
        self.assertEqual([row["target_triple"] for row in matrix["include"]], triples)

    def test_required_smoke_failure_propagates_and_retains_log(self):
        text = (ROOT / ".github/workflows/ci.yml").read_text()
        job = text.split("\n  focused-smoke:\n", 1)[1].split("\n  minimum-rust:\n", 1)[0]
        command = job.split("      - name: Execute both bounded fuzz preflights\n", 1)[1].split("      - name:", 1)[0].split("        run: ", 1)[1].strip()
        self.assertIn('--runs 250 --require-mutations', command)
        self.assertIn('if: always()', job)
        self.assertIn('path: ${{ runner.temp }}/focused-smoke/', job)
        aggregate = text.split("\n  check:\n", 1)[1]
        self.assertIn('focused-smoke,', aggregate)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            bin_dir = root / 'bin'
            bin_dir.mkdir()
            (bin_dir / "python3").symlink_to(sys.executable)
            cargo = bin_dir / 'cargo'
            cargo.write_text('#!/usr/bin/env bash\nprintf "intentional smoke failure\\n" >&2\nexit 23\n')
            cargo.chmod(0o755)
            result = subprocess.run(['bash', '-c', command], cwd=ROOT,
                                    env={**os.environ, 'PATH': str(bin_dir) + os.pathsep + os.environ['PATH'],
                                         'RUNNER_TEMP': str(root)}, capture_output=True, text=True)
            self.assertEqual(result.returncode, 23)
            evidence, = (root / 'focused-smoke').glob('run.*')
            self.assertIn('intentional smoke failure', (evidence / 'build.log').read_text())
            self.assertTrue(any((evidence / 'contract_bytes/corpus').iterdir()))
            self.assertTrue((evidence / 'contract_bytes/crashes').is_dir())
            self.assertEqual(json.loads((evidence / 'build.resources.json').read_text())['returncode'], 23)
            self.assertIn('retention-days: 3', job)

    def test_every_checkout_discards_credentials(self):
        for path in (ROOT / ".github/workflows").glob("*.yml"):
            for step in re.split(r"\n      - ", path.read_text()):
                if "uses: actions/checkout@" in step:
                    self.assertIn("persist-credentials: false", step)
                    self.assertIn("ref:", step)

    def test_release_resolves_once_without_shell_interpolating_dispatch_input(self):
        text = (ROOT / ".github/workflows/release.yml").read_text()
        self.assertIn("cancel-in-progress: false", text)
        self.assertLess(text.index("name: Validate release tag before checkout"), text.index("uses: actions/checkout@"))
        self.assertIn('git cat-file -t "refs/tags/$TAG"', text)
        self.assertEqual(text.count("RELEASE_SOURCE_SHA: ${{ needs.release-target-matrix.outputs.source_sha }}"), 4)
        for line in text.splitlines():
            if "${{ inputs.release_tag" in line:
                self.assertIn("CANDIDATE_TAG:", line)

    def test_published_native_execution_waits_for_publication_and_uses_downloaded_bytes(self):
        text = (ROOT / ".github/workflows/release.yml").read_text()
        body = text.split("\n  published-native:\n", 1)[1]
        self.assertIn("needs: [release-target-matrix, release]", body)
        self.assertIn("matrix: ${{ fromJson(needs.release-target-matrix.outputs.matrix) }}", body)
        self.assertIn("--source-digest \"$RELEASE_SOURCE_SHA\"", body)
        self.assertIn("curl --fail --location", body)
        self.assertIn("scripts/smoke-release-artifact.sh", body)
        self.assertNotIn("scripts/build-release-artifact.sh", body)
        self.assertIn("dist/native-conformance-*", body)
        self.assertIn("dist/native-conformance-*", text)

    def test_release_retains_complete_native_evidence_before_and_after_publication(self):
        text = (ROOT / ".github/workflows/release.yml").read_text()
        for job, next_job in [("build-standalone", "release"), ("published-native", None)]:
            body = text.split(f"\n  {job}:\n", 1)[1]
            if next_job:
                body = body.split(f"\n  {next_job}:\n", 1)[0]
            upload = body.split("uses: actions/upload-artifact@", 1)[1]
            for family in ["conformance", "semantics", "acceptance"]:
                self.assertIn(f"dist/native-{family}-*", upload,
                              f"{job} must retain {family} proof for the actual binary")

    def test_release_linux_acceptance_installs_acl_tools_in_both_native_lanes(self):
        text = (ROOT / ".github/workflows/release.yml").read_text()
        for job, next_job in [("build-standalone", "release"), ("published-native", None)]:
            body = text.split(f"\n  {job}:\n", 1)[1]
            if next_job:
                body = body.split(f"\n  {next_job}:\n", 1)[0]
            package_line, = [line for line in body.splitlines() if "apt-get install" in line]
            self.assertIn("acl", package_line.split(),
                          f"{job}: native authorization requires setfacl/getfacl")

    def test_platform_mutation_collection_is_explicit_and_keeps_raw_failures(self):
        text = (ROOT / ".github/workflows/mutation.yml").read_text()
        self.assertIn("workflow_dispatch:", text)
        self.assertNotIn("pull_request:", text)
        self.assertNotIn("schedule:", text)
        self.assertNotIn("continue-on-error:", text)
        self.assertLess(text.index("Validate exact source before checkout"), text.index("uses: actions/checkout@"))
        self.assertIn('git merge-base --is-ancestor "$SOURCE_SHA" origin/main', text)
        self.assertIn("--file 'crates/htmlcut-cli/src/**'", text)
        self.assertIn("CARGO_BUILD_TARGET: ${{ matrix.target_triple }}", text)
        self.assertIn("RUST_TEST_THREADS: '1'", text)
        self.assertIn("if: always()", text)
        upload = text.split("path: |", 1)[1]
        self.assertIn("mutation-outcomes/mutants.out/", upload)
        self.assertNotIn("scratch", upload)
        step = text.split("      - name: Select complementary native platforms\n", 1)[1].split("\n  collect:", 1)[0]
        body = step.split("        run: |\n", 1)[1]
        command = "\n".join(line[10:] for line in body.splitlines())
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "output"
            subprocess.run(["bash", "-c", command], cwd=ROOT,
                           env={**os.environ, "GITHUB_OUTPUT": str(output)}, check=True)
            rows = json.loads("\n".join(output.read_text().splitlines()[1:-1]))["include"]
        self.assertEqual({row["id"] for row in rows}, {"linux-x64-musl", "windows-x64"})


class ContributorToolCacheTest(unittest.TestCase):
    def test_failed_install_bounds_owned_products_and_preserves_owner_targets(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            bin_dir = root / "bin"
            bin_dir.mkdir()
            (bin_dir / "python3").symlink_to(sys.executable)
            cargo = bin_dir / "cargo"
            cargo.write_text("""#!/usr/bin/env bash
set -euo pipefail
printf '%s\\n' "$CARGO_TARGET_DIR" "$CARGO_BUILD_BUILD_DIR" > "$ENVIRONMENT_LOG"
mkdir -p "$CARGO_TARGET_DIR" "$CARGO_BUILD_BUILD_DIR"
python3 -c 'import os; from pathlib import Path; Path(os.environ["CARGO_TARGET_DIR"], "product").write_bytes(b"x" * (2 * 1024 * 1024))'
exit 17
""")
            cargo.chmod(0o755)
            owned = root / "owned"
            owned.mkdir()
            owner = root / "owner"
            owner.mkdir()
            sentinel = owner / "keep"
            sentinel.write_bytes(b"owner cache")
            helper = root / "tools.sh"
            # Exercise the real installer/watchdog with a tiny limit; no
            # multi-GiB write or actual tool compilation is needed for refusal.
            helper.write_text((ROOT / "scripts/contributor-rust-tools.sh").read_text().replace(
                "--disk-limit-mib 4096", "--disk-limit-mib 1"))
            environment_log = root / "environment"
            command = ["bash", "-c", 'source "$1"; htmlcut_install_contributor_cargo_tool cargo-about "$2"',
                       "installer", str(helper), str(root / "installed")]
            environment = {**os.environ, "PATH": str(bin_dir) + os.pathsep + os.environ["PATH"],
                           "RUNNER_TEMP": str(owned), "ENVIRONMENT_LOG": str(environment_log),
                           "CARGO_TARGET_DIR": str(owner), "CARGO_BUILD_BUILD_DIR": str(owner)}
            result = subprocess.run(command, cwd=ROOT, env=environment,
                capture_output=True, text=True)
            self.assertEqual(result.returncode, 125, result.stderr)
            target, build = map(Path, environment_log.read_text().splitlines())
            self.assertEqual(target.parent, build.parent)
            self.assertEqual(target.parent.parent, owned)
            self.assertFalse(target.exists())
            self.assertFalse(build.exists())
            report = json.loads((target.parent / "cargo-about-install.resources.json").read_text())
            self.assertEqual(report["refusal"], "disk-budget-exceeded")
            self.assertEqual(report["returncode"], 17)
            self.assertIn("bounded log and resource evidence", result.stderr)
            self.assertEqual(sentinel.read_bytes(), b"owner cache")
            self.assertEqual(list(owner.iterdir()), [sentinel])

            # A reported cleanup failure must retain compiler products. The
            # actual child is already reaped; inject only the report uncertainty.
            python = bin_dir / "python3"
            python.unlink()
            python.write_text(f"#!{sys.executable}\n" + """import json, os, pathlib, subprocess, sys
if len(sys.argv) > 1 and sys.argv[1].endswith('run-resource-bounded.py'):
    result = subprocess.run([sys.executable, *sys.argv[1:]])
    root = pathlib.Path(sys.argv[sys.argv.index('--root') + 1])
    report = root / 'cargo-about-install.resources.json'
    value = json.loads(report.read_text())
    value['cleanup_errors'] = ['injected unconfirmed cleanup']
    report.write_text(json.dumps(value))
    sys.exit(result.returncode)
os.execv(sys.executable, [sys.executable, *sys.argv[1:]])
""")
            python.chmod(0o755)
            uncertain = subprocess.run(command, cwd=ROOT, env=environment, capture_output=True, text=True)
            self.assertEqual(uncertain.returncode, 125, uncertain.stderr)
            target, build = map(Path, environment_log.read_text().splitlines())
            self.assertTrue((target / "product").exists())
            self.assertTrue(build.exists())
            self.assertIn("child cleanup was not confirmed", uncertain.stderr)
            self.assertEqual(sentinel.read_bytes(), b"owner cache")

    def test_fingerprint_tracks_selected_tool_installer_and_compiler_only(self):
        original = (ROOT / "scripts/contributor-rust-tools.sh").read_text()
        compiler = (ROOT / "rust-toolchain.toml").read_text()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            script = root / "tools.sh"
            pin = root / "rust-toolchain.toml"

            def fingerprint(source=original, toolchain=compiler, tool="cargo-about"):
                script.write_text(source)
                pin.write_text(toolchain)
                return subprocess.run(
                    ["bash", "-c", 'source "$1"; htmlcut_contributor_cargo_tool_fingerprint "$2"',
                     "fingerprint", str(script), tool], cwd=root,
                    capture_output=True, text=True)

            baseline = fingerprint()
            self.assertEqual(baseline.returncode, 0, baseline.stderr)
            self.assertRegex(baseline.stdout, r"^[0-9a-f]{64}\n$")
            self.assertEqual(fingerprint(original.replace("cargo-fuzz 0.13.2", "cargo-fuzz 0.13.3")).stdout,
                             baseline.stdout)
            for changed in [
                fingerprint(original.replace("cargo-about 0.9.2", "cargo-about 0.9.3")),
                fingerprint(original.replace('features=(--features cli)', 'features=(--features cli --offline)')),
                fingerprint(toolchain=compiler + "\n# changed compiler inputs\n"),
            ]:
                self.assertEqual(changed.returncode, 0, changed.stderr)
                self.assertNotEqual(changed.stdout, baseline.stdout)
            rejected = fingerprint(tool="unknown-tool")
            self.assertNotEqual(rejected.returncode, 0)
            self.assertIn("Unknown contributor Cargo tool", rejected.stderr)

    def test_install_uses_pinned_compiler_locked_version_and_required_feature(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            bin_dir = root / "bin"
            bin_dir.mkdir()
            (bin_dir / "python3").symlink_to(sys.executable)
            cargo = bin_dir / "cargo"
            cargo.write_text("""#!/usr/bin/env bash
set -euo pipefail
printf '%s\\n' "$@" > "$ARGUMENT_LOG"
while [[ "$1" != --root ]]; do shift; done
mkdir -p "$2/bin"
printf '#!/usr/bin/env bash\\nprintf "cargo-about 0.9.2\\\\n"\\n' > "$2/bin/cargo-about"
chmod +x "$2/bin/cargo-about"
""")
            cargo.chmod(0o755)
            install_root = root / "tool"
            log = root / "arguments"
            command = 'source scripts/contributor-rust-tools.sh; htmlcut_install_contributor_cargo_tool cargo-about "$INSTALL_ROOT"'
            result = subprocess.run(["bash", "-c", command], cwd=ROOT,
                env={**os.environ, "PATH": str(bin_dir) + os.pathsep + os.environ["PATH"],
                     "INSTALL_ROOT": str(install_root), "ARGUMENT_LOG": str(log)}, capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            pinned = tomllib.loads((ROOT / "rust-toolchain.toml").read_text())["toolchain"]["channel"]
            self.assertEqual(log.read_text().splitlines(), ["+" + pinned, "install", "cargo-about", "--version", "0.9.2", "--locked", "--root", str(install_root), "--features", "cli"])
            log.unlink()
            self.assertEqual(subprocess.run(["bash", "-c", command], cwd=ROOT,
                env={**os.environ, "PATH": str(bin_dir) + os.pathsep + os.environ["PATH"], "INSTALL_ROOT": str(install_root)}, capture_output=True).returncode, 0)
            self.assertFalse(log.exists(), "Valid cached executable must not reinstall")
            (install_root / "bin/cargo-about").write_text('#!/usr/bin/env bash\nprintf "cargo-about 0.1.0\\n"\n')
            rejected = subprocess.run(["bash", "-c", command], cwd=ROOT,
                env={**os.environ, "PATH": str(bin_dir) + os.pathsep + os.environ["PATH"], "INSTALL_ROOT": str(install_root)}, capture_output=True, text=True)
            self.assertNotEqual(rejected.returncode, 0)
            self.assertIn("version mismatch", rejected.stderr)

    def test_cache_only_saves_trusted_main_tools_and_preserves_all_jobs(self):
        action = (ROOT / ".github/actions/cargo-tool/action.yml").read_text()
        self.assertIn("if: github.event_name == 'push' && github.ref == 'refs/heads/main'", action)
        self.assertIn("if: github.event_name != 'push' || github.ref != 'refs/heads/main'", action)
        self.assertIn("actions/cache/restore@55cc8345863c7cc4c66a329aec7e433d2d1c52a9", action)
        self.assertNotIn("restore-keys:", action)
        self.assertIn("$RUNNER_OS_ID-$RUNNER_ARCH_ID-${ImageOS:-unknown}-${ImageVersion:-unknown}", action)
        self.assertIn('htmlcut_contributor_cargo_tool_fingerprint "$TOOL"', action)
        self.assertIn("if: failure()", action)
        self.assertIn("retention-days: 3", action)
        self.assertIn("${{ runner.temp }}/htmlcut-cargo-install.*/*.log", action)
        self.assertIn("${{ runner.temp }}/htmlcut-cargo-install.*/*.resources.json", action)
        self.assertNotIn("htmlcut-cargo-install.*/target", action)
        self.assertNotIn(".htmlcut-artifacts", action)
        self.assertNotIn("dist/", action)
        ci = (ROOT / ".github/workflows/ci.yml").read_text()
        self.assertEqual(ci.count("uses: ./.github/actions/cargo-tool"), 3)
        self.assertIn("needs: [release-target-matrix, linux-maintainer, focused-smoke, release-target-smoke, minimum-rust]", ci)
        release = (ROOT / ".github/workflows/release.yml").read_text()
        self.assertEqual(release.count("uses: ./.github/actions/cargo-tool"), 1)
