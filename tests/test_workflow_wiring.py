# SPDX-License-Identifier: MPL-2.0
"""Workflow handoff must preserve source, inventory and credential boundaries."""
from pathlib import Path
import re
import runpy
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
        coordinator = ROOT / "scripts/focused-smoke.py"
        shell_targets = set(runpy.run_path(str(coordinator))["fuzz_targets"]())
        self.assertIn('exec python3 scripts/focused-smoke.py "$@"', script)
        # Target declarations describe what each harness feeds. Other code
        # identifiers in the resource documentation are not target names.
        documented = set(re.findall(r"`([a-z][a-z0-9_]*)`\s+feeds\b", (ROOT / "fuzz/README.md").read_text()))
        self.assertEqual(targets, shell_targets)
        self.assertEqual(targets, documented)
        self.assertNotIn("--features", coordinator.read_text())
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
        native = text.split("      - name: Execute active native CLI selection; survivors and timeouts require review\n", 1)[1].split("      - name: Preserve outcomes", 1)[0]
        native_body = native.split("        run: |\n", 1)[1]
        native_command = "\n".join(line[10:] for line in native_body.splitlines())
        for runner_os, patterns in [
            ("Linux", ["crates/htmlcut-cli/src/authorization/windows.rs", "crates/htmlcut-cli/src/authorization/windows/**"]),
            ("Windows", ["crates/htmlcut-cli/src/authorization/unix.rs"]),
            ("macOS", None),
        ]:
            with self.subTest(runner_os=runner_os), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                python = root / "python3"
                python.write_text("#!"+sys.executable+"\nimport json,os,sys\nfrom pathlib import Path\nif sys.argv[1:2]==['-c']:\n exec(sys.argv[2])\nelse:\n Path(os.environ['ARGUMENT_LOG']).write_text(json.dumps(sys.argv[1:]))\n")
                python.chmod(0o755)
                log = root / "arguments.json"
                result = subprocess.run(["bash", "-euc", native_command], cwd=ROOT, env={**os.environ,"PATH":str(root)+os.pathsep+os.environ["PATH"],"RUNNER_OS":runner_os,"RUNNER_TEMP":str(root),"ARGUMENT_LOG":str(log)},capture_output=True,text=True)
                if patterns is None:
                    self.assertEqual(result.returncode,2,result.stderr)
                    self.assertFalse(log.exists())
                else:
                    self.assertEqual(result.returncode,0,result.stderr)
                    arguments=json.loads(log.read_text())
                    self.assertEqual([arguments[index+1] for index,value in enumerate(arguments) if value=='--exclude-file'],patterns)
                    self.assertEqual(arguments[arguments.index('--file')+1],'crates/htmlcut-cli/src/**')
        upload = text.split("path: |", 1)[1]
        self.assertIn("mutation-outcomes/mutants.out/", upload)
        self.assertIn("mutation-outcomes/tool-version/", upload)
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


    def test_manual_mutation_cache_saves_only_the_verified_current_main_snapshot(self):
        workflow = (ROOT / ".github/workflows/mutation.yml").read_text()
        action = (ROOT / ".github/actions/cargo-tool/action.yml").read_text()
        eligibility = workflow.split("      - name: Check current main cache eligibility\n", 1)[1].split("      - name: Retain verified current main mutation tool\n", 1)[0]
        command = "\n".join(line[10:] for line in eligibility.split("        run: |\n", 1)[1].splitlines())
        save = workflow.split("      - name: Retain verified current main mutation tool\n", 1)[1].split("      - name: Execute active native CLI selection", 1)[0]
        self.assertIn("if: steps.cache-source.outputs.eligible == 'true' && steps.mutation-tool.outputs.cache-hit != 'true'", save)
        self.assertIn("uses: actions/cache/save@55cc8345863c7cc4c66a329aec7e433d2d1c52a9", save)
        self.assertIn("path: ${{ steps.mutation-tool.outputs.cache-path }}", save)
        self.assertIn("key: ${{ steps.mutation-tool.outputs.cache-key }}", save)
        self.assertNotIn("always()", save)
        self.assertNotIn("failure()", save)
        self.assertNotIn("restore-keys:", save)
        self.assertLess(workflow.index("id: mutation-tool"), workflow.index("id: cache-source"))
        self.assertLess(workflow.index("Retain verified current main mutation tool"), workflow.index("Execute active native CLI selection"))
        for value in ["steps.inventory.outputs.key", "steps.inventory.outputs.cache_path",
                      "steps.trusted-cache.outputs.cache-hit || steps.restore-cache.outputs.cache-hit"]:
            self.assertIn("value: ${{ " + value + " }}", action)
        for name, expression in [("EVENT_NAME", "github.event_name"), ("WORKFLOW_REF", "github.ref"),
                                 ("WORKFLOW_SHA", "github.sha"),
                                 ("SOURCE_SHA", "needs.platform-matrix.outputs.source_sha")]:
            self.assertIn(name + ": ${{ " + expression + " }}", eligibility)

        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            environment = {**os.environ, "GIT_AUTHOR_NAME": "Cache control", "GIT_AUTHOR_EMAIL": "cache@example.test",
                           "GIT_COMMITTER_NAME": "Cache control", "GIT_COMMITTER_EMAIL": "cache@example.test"}

            def git(*arguments):
                return subprocess.check_output(["git", *arguments], cwd=root, env=environment,
                                               stderr=subprocess.PIPE, text=True).strip()

            git("init", "-q")
            commits = []
            for index in range(3):
                (root / "source").write_text(str(index))
                git("add", "source")
                git("commit", "-qm", "Source snapshot")
                commits.append(git("rev-parse", "HEAD"))
            previous, current, advanced = commits
            defaults = {"EVENT_NAME": "workflow_dispatch", "WORKFLOW_REF": "refs/heads/main",
                        "WORKFLOW_SHA": current, "SOURCE_SHA": current}
            cases = [
                ("current-main", {}, current, current, True),
                ("push", {"EVENT_NAME": "push"}, current, current, False),
                ("pull-request", {"EVENT_NAME": "pull_request"}, current, current, False),
                ("branch-dispatch", {"WORKFLOW_REF": "refs/heads/topic"}, current, current, False),
                ("tag-dispatch", {"WORKFLOW_REF": "refs/tags/v22.0.0"}, current, current, False),
                ("historical-source", {"SOURCE_SHA": previous}, previous, current, False),
                ("wrong-workflow-source", {"WORKFLOW_SHA": previous}, current, current, False),
                ("wrong-checkout", {}, previous, current, False),
                ("main-advanced", {}, current, advanced, False),
                ("missing-main-ref", {}, current, None, False),
            ]
            for name, values, head, remote, expected in cases:
                with self.subTest(case=name):
                    git("checkout", "--detach", "-q", head)
                    if remote is None:
                        git("update-ref", "-d", "refs/remotes/origin/main")
                    else:
                        git("update-ref", "refs/remotes/origin/main", remote)
                    output = root / "output"
                    output.unlink(missing_ok=True)
                    result = subprocess.run(["bash", "-c", command], cwd=root,
                                            env={**environment, **defaults, **values, "GITHUB_OUTPUT": str(output)},
                                            capture_output=True, text=True, timeout=5)
                    self.assertEqual(result.returncode, 0, result.stderr)
                    self.assertEqual(output.read_text(), f"eligible={str(expected).lower()}\n")


class ContributorToolCacheTest(unittest.TestCase):
    def test_cleanup_failure_refuses_without_errexit_and_preserves_primary_failures(self):
        shell = "/bin/bash" if Path("/bin/bash").is_file() else "bash"
        for cached, install_status, query_status, expected in [
            (False, 0, 0, 125),
            (True, 0, 0, 125),
            (False, 17, 0, 17),
            (True, 0, 9, 9),
        ]:
            for flags in ["-uc", "-euc"]:
                with self.subTest(cached=cached, install_status=install_status,
                                  query_status=query_status, flags=flags), tempfile.TemporaryDirectory() as directory:
                    root = Path(directory)
                    tools = root / "bin"
                    tools.mkdir()
                    (tools / "python3").symlink_to(sys.executable)
                    marker = root / "cargo-started"
                    removal = tools / "rm"
                    removal.write_text('#!/usr/bin/env bash\nprintf "%s\\n" "$@" >> "$REMOVE_LOG"\nexit 19\n')
                    removal.chmod(0o755)
                    version_program = '#!/usr/bin/env bash\nprintf "cargo-mutants 27.1.0\\n"\nexit "${QUERY_STATUS:-0}"\n'
                    cargo = tools / "cargo"
                    cargo.write_text("""#!/usr/bin/env bash
set -eu
printf 'started' > "$INSTALL_MARKER"
while [[ "$1" != --root ]]; do shift; done
mkdir -p "$2/bin" "$CARGO_TARGET_DIR" "$CARGO_BUILD_BUILD_DIR"
printf 'compiler product' > "$CARGO_TARGET_DIR/product"
printf '%s' "$VERSION_PROGRAM" > "$2/bin/cargo-mutants"
chmod +x "$2/bin/cargo-mutants"
exit "$INSTALL_STATUS"
""")
                    cargo.chmod(0o755)
                    scratch = root / "scratch"
                    scratch.mkdir()
                    destination = root / "tool"
                    if cached:
                        (destination / "bin").mkdir(parents=True)
                        executable = destination / "bin/cargo-mutants"
                        executable.write_text(version_program)
                        executable.chmod(0o755)
                    owner = root / "owner"
                    owner.mkdir()
                    sentinel = owner / "preserve"
                    sentinel.write_bytes(b"owner cache")
                    environment = {**os.environ, "PATH": str(tools) + os.pathsep + os.environ["PATH"],
                                   "RUNNER_TEMP": str(scratch), "REMOVE_LOG": str(root / "removals"),
                                   "INSTALL_MARKER": str(marker), "VERSION_PROGRAM": version_program,
                                   "INSTALL_STATUS": str(install_status), "QUERY_STATUS": str(query_status),
                                   "CARGO_TARGET_DIR": str(owner), "CARGO_BUILD_BUILD_DIR": str(owner)}
                    result = subprocess.run([shell, flags,
                        'source scripts/contributor-rust-tools.sh; htmlcut_install_contributor_cargo_tool cargo-mutants "$1"',
                        "installer", str(destination)], cwd=ROOT, env=environment,
                        capture_output=True, text=True, timeout=6)
                    self.assertEqual(result.returncode, expected, result.stderr)
                    retained, = scratch.iterdir()
                    self.assertIn(str(retained), result.stderr)
                    self.assertIn("cleanup", result.stderr)
                    self.assertTrue((root / "removals").is_file())
                    self.assertEqual(marker.exists(), not cached)
                    self.assertEqual((destination / "bin/cargo-mutants").read_text(), version_program)
                    report = retained / ("version/cargo-mutants-version.resources.json" if cached
                                         else "cargo-mutants-install.resources.json")
                    resources = json.loads(report.read_text())
                    self.assertEqual(resources["cleanup_errors"], [])
                    self.assertEqual(resources["returncode"], query_status if cached else install_status)
                    self.assertEqual(sentinel.read_bytes(), b"owner cache")
                    self.assertEqual(list(owner.iterdir()), [sentinel])

    def test_mutants_protocol_and_postinstall_verification_preserve_failure_evidence(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            tools = root / "bin"
            tools.mkdir()
            (tools / "python3").symlink_to(sys.executable)
            cargo = tools / "cargo"
            cargo.write_text("""#!/usr/bin/env bash
set -euo pipefail
printf '%s\\n' "$@" > "$INSTALL_ARGUMENTS"
while [[ "$1" != --root ]]; do shift; done
mkdir -p "$2/bin"
cat > "$2/bin/cargo-mutants" <<'TOOL'
#!/usr/bin/env bash
if [[ "$#" != 2 || "$1" != mutants || "$2" != --version ]]; then
    printf 'unexpected direct executable arguments\\n' >&2
    exit 41
fi
printf '%s\\n' "${TOOL_VERSION:-cargo-mutants 27.1.0}"
exit "${QUERY_STATUS:-0}"
TOOL
chmod +x "$2/bin/cargo-mutants"
""")
            cargo.chmod(0o755)
            scratch = root / "scratch"
            scratch.mkdir()
            arguments = root / "arguments"
            env = {**os.environ, "PATH": str(tools) + os.pathsep + os.environ["PATH"],
                   "RUNNER_TEMP": str(scratch), "INSTALL_ARGUMENTS": str(arguments)}

            def install(path, **overrides):
                return subprocess.run(["bash", "-euc",
                    'source scripts/contributor-rust-tools.sh; htmlcut_install_contributor_cargo_tool cargo-mutants "$1"',
                    "installer", str(path)], cwd=ROOT, env={**env, **overrides}, capture_output=True, text=True)

            destination = root / "installed"
            first = install(destination)
            self.assertEqual(first.returncode, 0, first.stderr)
            cold_phases = [json.loads(line) for line in (first.stdout + first.stderr).splitlines() if line.startswith('{')]
            self.assertEqual(len(cold_phases), 2, first.stdout + first.stderr)
            self.assertIn('install', cold_phases[0]['command'])
            self.assertEqual(cold_phases[1]['command'][1:], ['mutants', '--version'])
            self.assertIn("cargo-mutants", arguments.read_text().splitlines())
            self.assertEqual(list(scratch.iterdir()), [])
            arguments.unlink()
            cached = install(destination)
            self.assertEqual(cached.returncode, 0, cached.stderr)
            cached_phases = [json.loads(line) for line in (cached.stdout + cached.stderr).splitlines() if line.startswith('{')]
            self.assertEqual(len(cached_phases), 1, cached.stdout + cached.stderr)
            self.assertEqual(cached_phases[0]['command'][1:], ['mutants', '--version'])
            self.assertFalse(arguments.exists(), "A verified cached tool must not reinstall")
            mismatch = install(destination, TOOL_VERSION="cargo-mutants 0.0.0")
            self.assertEqual(mismatch.returncode, 1, mismatch.stderr)
            self.assertIn("version mismatch", mismatch.stderr)
            self.assertFalse(arguments.exists())
            for name, overrides, status, text in [
                ("wrong-version", {"TOOL_VERSION": "cargo-mutants 0.0.0"}, 1, "cargo-mutants 0.0.0"),
                ("failed-query", {"QUERY_STATUS": "9"}, 9, "cargo-mutants 27.1.0"),
            ]:
                before = set(scratch.iterdir())
                failed = install(root / name, **overrides)
                self.assertEqual(failed.returncode, status, failed.stderr)
                evidence, = set(scratch.iterdir()) - before
                self.assertIn(str(evidence), failed.stderr)
                self.assertEqual((evidence / "version/cargo-mutants-version.log").read_text().strip(), text)
                resources = json.loads((evidence / "cargo-mutants-install.resources.json").read_text())
                self.assertEqual(resources["returncode"], 0)
                self.assertEqual(resources["cleanup_errors"], [])
                version = json.loads((evidence / "version/cargo-mutants-version.resources.json").read_text())
                self.assertEqual(version["returncode"], 9 if name == "failed-query" else 0)
                self.assertEqual(version["cleanup_errors"], [])
                self.assertTrue((evidence / "cargo-mutants-install.log").is_file())
                self.assertFalse((evidence / "target").exists())
                self.assertFalse((evidence / "build").exists())

    def test_cached_version_hang_and_flood_are_bounded_without_reinstall_or_cache_cleanup(self):
        original = (ROOT / "scripts/contributor-rust-tools.sh").read_text()
        self.assertIn("--disk-limit-mib 4 --timeout 10 --log-limit-mib 1", original)
        self.assertIn("--poll-seconds 0.05", original)
        for mode in ["hang", "flood"]:
            with self.subTest(mode=mode), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                tools = root / "bin"
                tools.mkdir()
                (tools / "python3").symlink_to(sys.executable)
                marker = root / "cargo-started"
                cargo = tools / "cargo"
                cargo.write_text("#!/usr/bin/env bash\nprintf unexpected > \"$INSTALL_MARKER\"\nexit 97\n")
                cargo.chmod(0o755)
                scratch = root / "scratch"
                scratch.mkdir()
                cache = root / "cached/bin"
                cache.mkdir(parents=True)
                executable = cache / "cargo-mutants"
                body = "import time; time.sleep(10)" if mode == "hang" else "import sys,time; sys.stdout.buffer.write(b'x'*(2*1024*1024)); sys.stdout.flush(); time.sleep(10)"
                executable.write_text("#!" + sys.executable + "\n" + body + "\n")
                executable.chmod(0o755)
                before = executable.read_bytes()
                owner = root / "owner-target"
                owner.mkdir()
                sentinel = owner / "preserve"
                sentinel.write_bytes(b"owner cache")
                helper = root / "helper.sh"
                # The production 10s/1MiB probe is checked above. Shorten time
                # only; the smallest supported log budget is one MiB.
                helper.write_text(original.replace("--timeout 10 --log-limit-mib 1", "--timeout " + ("0.15" if mode == "hang" else "2") + " --log-limit-mib 1"))
                result = subprocess.run(["bash", "-euc", 'source "$1"; htmlcut_install_contributor_cargo_tool cargo-mutants "$2"', "installer", str(helper), str(cache.parent)], cwd=ROOT,
                    env={**os.environ,"PATH":str(tools)+os.pathsep+os.environ["PATH"],"RUNNER_TEMP":str(scratch),"INSTALL_MARKER":str(marker),"CARGO_TARGET_DIR":str(owner),"CARGO_BUILD_BUILD_DIR":str(owner)},capture_output=True,text=True,timeout=8)
                self.assertEqual(result.returncode,125,result.stderr)
                evidence, = scratch.iterdir()
                report = json.loads((evidence / "version/cargo-mutants-version.resources.json").read_text())
                self.assertEqual(report["refusal"], "wall-time-exceeded" if mode == "hang" else "log-budget-exceeded")
                self.assertEqual(report["cleanup_errors"], [])
                self.assertLess(report["elapsed_seconds"], 4)
                self.assertLessEqual((evidence / "version/cargo-mutants-version.log").stat().st_size,1024*1024)
                self.assertFalse((evidence / "cargo-mutants-install.resources.json").exists())
                self.assertFalse(marker.exists(), "Cached version refusal must never reinstall")
                self.assertEqual(executable.read_bytes(),before)
                self.assertEqual(sentinel.read_bytes(),b"owner cache")
                self.assertEqual(list(owner.iterdir()),[sentinel])
                self.assertIn(str(evidence),result.stderr)

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
            command = ["bash", "-euc", 'source "$1"; htmlcut_install_contributor_cargo_tool cargo-about "$2"',
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
            result = subprocess.run(["bash", "-euc", command], cwd=ROOT,
                env={**os.environ, "PATH": str(bin_dir) + os.pathsep + os.environ["PATH"],
                     "INSTALL_ROOT": str(install_root), "ARGUMENT_LOG": str(log)}, capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            pinned = tomllib.loads((ROOT / "rust-toolchain.toml").read_text())["toolchain"]["channel"]
            self.assertEqual(log.read_text().splitlines(), ["+" + pinned, "install", "cargo-about", "--version", "0.9.2", "--locked", "--root", str(install_root), "--features", "cli"])
            log.unlink()
            self.assertEqual(subprocess.run(["bash", "-euc", command], cwd=ROOT,
                env={**os.environ, "PATH": str(bin_dir) + os.pathsep + os.environ["PATH"], "INSTALL_ROOT": str(install_root)}, capture_output=True).returncode, 0)
            self.assertFalse(log.exists(), "Valid cached executable must not reinstall")
            (install_root / "bin/cargo-about").write_text('#!/usr/bin/env bash\nprintf "cargo-about 0.1.0\\n"\n')
            rejected = subprocess.run(["bash", "-euc", command], cwd=ROOT,
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
        patterns = [line[10:] for line in action.split("path: |\n", 1)[1].splitlines() if line.strip()]
        self.assertEqual(set(patterns), {
            "${{ runner.temp }}/htmlcut-cargo-install.*/*.log",
            "${{ runner.temp }}/htmlcut-cargo-install.*/*.resources.json",
            "${{ runner.temp }}/htmlcut-cargo-install.*/version/*.log",
            "${{ runner.temp }}/htmlcut-cargo-install.*/version/*.resources.json",
        })
        self.assertNotIn("htmlcut-cargo-install.*/target", action)
        self.assertNotIn(".htmlcut-artifacts", action)
        self.assertNotIn("dist/", action)
        ci = (ROOT / ".github/workflows/ci.yml").read_text()
        self.assertEqual(ci.count("uses: ./.github/actions/cargo-tool"), 3)
        self.assertIn("needs: [release-target-matrix, linux-maintainer, focused-smoke, release-target-smoke, minimum-rust]", ci)
        release = (ROOT / ".github/workflows/release.yml").read_text()
        self.assertEqual(release.count("uses: ./.github/actions/cargo-tool"), 1)
