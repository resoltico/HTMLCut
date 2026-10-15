# SPDX-License-Identifier: MPL-2.0
"""Mutation evidence, source binding and disposable ownership controls."""
import copy
import importlib.util
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("mutation_audit", ROOT / "scripts/mutation-audit.py")
audit = importlib.util.module_from_spec(spec)
spec.loader.exec_module(audit)
VERSION = "27.1.0"


def phases(verdict):
    def phase(name, status):
        return {"phase": name, "process_status": status, "duration": 0.01, "argv": ["cargo", "test"]}
    if verdict == "Unviable":
        return [phase("Build", {"Failure": 101})]
    if verdict == "Timeout":
        return [phase("Build", "Success"), phase("Test", "Timeout")]
    return [phase("Build", "Success"), phase("Test", {"Failure": 101} if verdict == "CaughtMutant" else "Success")]


def evidence(output, verdicts=("CaughtMutant", "CaughtMutant")):
    root = output / "mutants.out"
    (root / "log").mkdir(parents=True)
    (root / "diff").mkdir()
    (root / "log/baseline.log").write_text("baseline evidence")
    outcomes = [{"scenario": "Baseline", "summary": "Success", "log_path": "log/baseline.log",
                 "diff_path": None, "phase_results": phases("Success")}]
    plan = []
    counts = {"caught": 0, "missed": 0, "timeout": 0, "unviable": 0, "success": 0}
    for index, verdict in enumerate(verdicts):
        identity = {"name": f"src/lib.rs:1:{index}: replace flag with {index}", "file": "src/lib.rs",
                    "package": "tiny", "replacement": str(index), "genre": "FnValue", "function": None,
                    "span": {"start": {"line": 1, "column": index}, "end": {"line": 1, "column": index + 1}}}
        plan.append({**identity, "diff": f"literal diff {index}"})
        (root / f"log/{index}.log").write_text("scenario evidence")
        (root / f"diff/{index}.diff").write_text(f"literal diff {index}")
        outcomes.append({"scenario": {"Mutant": identity}, "summary": verdict, "log_path": f"log/{index}.log",
                         "diff_path": f"diff/{index}.diff", "phase_results": phases(verdict)})
        counts[{"CaughtMutant": "caught", "MissedMutant": "missed", "Timeout": "timeout", "Unviable": "unviable"}[verdict]] += 1
    result = {**counts, "outcomes": outcomes, "total_mutants": len(plan), "cargo_mutants_version": VERSION, "end_time": "2026-10-15T00:00:00Z"}
    (root / "mutants.json").write_text(json.dumps(plan))
    (root / "outcomes.json").write_text(json.dumps(result))
    return root, plan, result


class OutcomeTests(unittest.TestCase):
    def test_complete_survivors_and_timeouts_keep_their_counts(self):
        for verdicts, status in [(('CaughtMutant', 'Unviable'), 0), (('CaughtMutant', 'MissedMutant'), 2), (('MissedMutant', 'Timeout'), 3)]:
            with self.subTest(status=status), tempfile.TemporaryDirectory() as directory:
                output = Path(directory)
                evidence(output, verdicts)
                result = audit.complete_outcomes(output, status, VERSION)
                self.assertEqual(result['tested_mutants'], 2)
                self.assertEqual(sum(result['counts'].values()), 2)
                self.assertEqual(result['counts']['timeout'], verdicts.count('Timeout'))
                self.assertEqual(result['counts']['missed'], verdicts.count('MissedMutant'))

    def test_malformed_or_incomplete_evidence_is_rejected(self):
        mutations = {
            'duplicate outcome': lambda plan, result: result['outcomes'].__setitem__(2, copy.deepcopy(result['outcomes'][1])),
            'missing outcome': lambda plan, result: result['outcomes'].pop(),
            'empty plan': lambda plan, result: plan.clear(),
            'duplicate plan': lambda plan, result: plan.__setitem__(1, copy.deepcopy(plan[0])),
            'missing baseline': lambda plan, result: result['outcomes'].pop(0),
            'false baseline': lambda plan, result: result['outcomes'][0].__setitem__('phase_results', phases('CaughtMutant')),
            'false caught verdict': lambda plan, result: result['outcomes'][1].__setitem__('phase_results', phases('Success')),
            'wrong counter': lambda plan, result: result.__setitem__('caught', 1),
            'wrong total': lambda plan, result: result.__setitem__('total_mutants', 1),
            'incomplete completion marker': lambda plan, result: result.__setitem__('end_time', None),
            'changed mutant': lambda plan, result: result['outcomes'][1]['scenario']['Mutant'].__setitem__('replacement', 'unexpected'),
            'invalid verdict': lambda plan, result: result['outcomes'][1].__setitem__('summary', 'Success'),
        }
        for name, mutate in mutations.items():
            with self.subTest(name=name), tempfile.TemporaryDirectory() as directory:
                output = Path(directory)
                root, plan, result = evidence(output)
                mutate(plan, result)
                (root / 'mutants.json').write_text(json.dumps(plan))
                (root / 'outcomes.json').write_text(json.dumps(result))
                with self.assertRaises(ValueError):
                    audit.complete_outcomes(output, 0, VERSION)

    def test_missing_log_or_diff_and_false_exit_are_rejected(self):
        for missing in ['log/baseline.log', 'log/0.log', 'diff/0.diff', None]:
            with self.subTest(missing=missing), tempfile.TemporaryDirectory() as directory:
                output = Path(directory)
                root, _, _ = evidence(output)
                if missing:
                    (root / missing).unlink()
                with self.assertRaises(ValueError):
                    audit.complete_outcomes(output, 2 if missing is None else 0, VERSION)


class RunnerTests(unittest.TestCase):
    def run_main(self, directory, status=0, resources=True, cleanup_errors=None, source_changed=False, output_inside=False, extra=(), interrupted=False, empty_plan=False, probe_status=0, probe_version=VERSION, probe_resources=True, probe_cleanup_errors=None, probe_interrupted=False):
        base = Path(directory)
        source = base / 'workspace'
        source.mkdir()
        (source / 'Cargo.toml').write_text('[workspace]\n')
        crate = source / 'crate'
        crate.mkdir()
        (crate / 'Cargo.toml').write_text('[package]\nname="tiny"\nversion="0.1.0"\n')
        output = source / 'output' if output_inside else base / 'results'
        before = {'git_commit': 'commit', 'sha256': 'before', 'files': {'src/lib.rs': {'sha256': 'before'}}}
        after = {'git_commit': 'commit', 'sha256': 'after', 'files': {'src/lib.rs': {'sha256': 'after'}}} if source_changed else before

        def tools(command, **kwargs):
            if command[:2] == ['cargo', 'metadata']:
                return subprocess.CompletedProcess(command, 0, json.dumps({'workspace_root': str(source)}), '')
            if command[0] == audit.bash_program():
                return subprocess.CompletedProcess(command, 0, VERSION + '\n', '')
            if command[:3] == ['cargo', 'mutants', '--version']:
                raise AssertionError('pin query must use the bounded owned phase')
            raise AssertionError(command)

        phases_started = []
        def process(command, **kwargs):
            label=command[command.index('--label')+1]
            phases_started.append(label)
            probe=label=='version'
            phase_root=Path(command[command.index('--root')+1])
            phase_status=probe_status if probe else status
            phase_interrupted=probe_interrupted if probe else interrupted
            if probe:
                (phase_root/'version.log').write_text('cargo-mutants '+probe_version+'\n')
            else:
                evidence(output, ('CaughtMutant', 'Timeout' if status == 3 else 'MissedMutant' if status == 2 else 'CaughtMutant'))
                if empty_plan:
                    (output/'mutants.out/mutants.json').write_text('[]')
            if probe_resources if probe else resources:
                (phase_root/(label+'.resources.json')).write_text(json.dumps({'returncode':70 if phase_interrupted else phase_status,'refusal':'interrupted' if phase_interrupted else 'wall-time-exceeded' if phase_status==125 else None,'cleanup_errors':(probe_cleanup_errors if probe else cleanup_errors) or []}))
            class Process:
                pid=123
                waited=False
                def wait(self,timeout=None):
                    if phase_interrupted and not self.waited:
                        self.waited=True
                        raise KeyboardInterrupt
                    return phase_status
                def poll(self):
                    return phase_status
            return Process()

        argv = ['mutation-audit.py', '--dir', str(crate), '--output', str(output), '--jobs', '4', '--disk-limit-mib', '1', *extra]
        environment = {key: value for key, value in os.environ.items() if key not in {'CARGO_TARGET_DIR', 'CARGO_BUILD_TARGET_DIR', 'CARGO_BUILD_BUILD_DIR'}}
        with patch.dict(os.environ, environment, clear=True), patch.object(sys, 'argv', argv), patch.object(audit.os, 'cpu_count', return_value=8), \
                patch.object(audit.subprocess, 'run', side_effect=tools), patch.object(audit.subprocess, 'Popen', side_effect=process) as launched, \
                patch.object(audit, 'source_identity', side_effect=[before, after]), \
                patch.object(audit.signal, 'signal'), patch.object(audit.platform, 'platform', return_value='test-host'):
            missing_diff = '--in-diff' in extra and not Path(extra[extra.index('--in-diff') + 1]).is_file()
            if output_inside or missing_diff:
                with self.assertRaises(SystemExit) as error:
                    audit.main()
                self.assertEqual(error.exception.code, 2)
                self.assertFalse(output.exists())
                launched.assert_not_called()
                return None, output
            actual = audit.main()
            if probe_status != 0 or probe_version != VERSION or not probe_resources or probe_cleanup_errors or probe_interrupted:
                self.assertEqual(phases_started,['version'],'invalid pin phase must not launch a campaign')
        return actual, output

    def test_exclusions_alone_are_focused_and_forward_exact_patterns(self):
        patterns = ['crates/htmlcut-cli/src/authorization/windows.rs', 'crates/htmlcut-cli/src/authorization/windows/**']
        with tempfile.TemporaryDirectory() as directory:
            actual, output = self.run_main(directory, extra=tuple(part for pattern in patterns for part in ('--exclude-file', pattern)))
            result = json.loads((output / 'audit.json').read_text())
            self.assertEqual(actual, 0)
            self.assertEqual(result['scope'], 'focused')
            self.assertEqual(result['exclude_files'], patterns)
            command = result['command']
            self.assertEqual([command[index+1] for index,value in enumerate(command) if value == '--exclude'], patterns)
            self.assertNotIn('--exclude-file', command)
            self.assertIn('--workspace', command)
            self.assertIn('--all-features', command)
            self.assertEqual(command[command.index('--test-workspace')+1], 'true')

    def test_exclusion_only_empty_plan_cannot_claim_completion(self):
        with tempfile.TemporaryDirectory() as directory:
            actual, output = self.run_main(directory, extra=('--exclude-file', '**'), empty_plan=True)
            result = json.loads((output / 'audit.json').read_text())
            self.assertEqual(actual, 125)
            self.assertEqual(result['scope'], 'focused')
            self.assertEqual(result['exclude_files'], ['**'])
            self.assertFalse(result['execution_complete'])
            self.assertEqual(result['tool_returncode'], 0)
            self.assertIn('empty mutation plan', result['evidence_error'])

    def test_test_threads_default_to_one_and_respect_explicit_setting(self):
        for configured, expected in [(None, '1'), ('3', '3')]:
            environment = {key:value for key,value in os.environ.items() if key != 'RUST_TEST_THREADS'}
            if configured is not None:
                environment['RUST_TEST_THREADS'] = configured
            with self.subTest(configured=configured), patch.dict(os.environ,environment,clear=True), tempfile.TemporaryDirectory() as directory:
                actual, output = self.run_main(directory)
                result=json.loads((output/'audit.json').read_text())
                self.assertEqual(actual,0)
                self.assertNotIn('--cargo-test-arg=--fail-fast',result['command'])
                self.assertEqual(result['execution_environment']['RUST_TEST_THREADS'],expected)
                self.assertIn('--workspace',result['command'])
                self.assertEqual(result['command'][result['command'].index('--test-workspace')+1],'true')

    def test_invalid_or_refused_pin_probe_is_retained_without_mutation_completion(self):
        for options in [dict(probe_version='0.0.0'),dict(probe_status=2),dict(probe_status=125),dict(probe_resources=False),dict(probe_cleanup_errors=['pin child unconfirmed']),dict(probe_cleanup_errors=[1])]:
            with self.subTest(options=options), tempfile.TemporaryDirectory() as directory:
                actual,output=self.run_main(directory,**options)
                report=json.loads((output/'audit.json').read_text())
                self.assertEqual(actual,125)
                self.assertFalse(report['execution_complete'])
                self.assertTrue(report['source_stable'])
                self.assertIsNone(report['tool_returncode'])
                self.assertFalse((output/'mutants.out').exists())
                self.assertEqual(report['tool_version']['command'],['cargo','mutants','--version'])
                self.assertTrue((output/'tool-version/version.log').is_file())
                if 'probe_version' in options:
                    self.assertIn('install the pinned tool',report['tool_version_error'])
                if options.get('probe_status')==2:
                    self.assertEqual(report['tool_version']['tool_returncode'],2)
                if options.get('probe_resources') is False or options.get('probe_cleanup_errors'):
                    self.assertTrue((output/'scratch').is_dir())
                    self.assertIn(str((output/'tool-version').resolve()),report['retained_phase_roots'])

    def test_interrupted_pin_probe_records_own_status_and_skips_campaign(self):
        with tempfile.TemporaryDirectory() as directory:
            actual,output=self.run_main(directory,probe_status=125,probe_interrupted=True)
            report=json.loads((output/'audit.json').read_text())
            self.assertEqual(actual,130)
            self.assertEqual(report['tool_version']['resource_wrapper_returncode'],125)
            self.assertEqual(report['tool_version']['tool_returncode'],70)
            self.assertIsNone(report['tool_returncode'])
            self.assertFalse(report['execution_complete'])

    def test_git_bash_absolute_path_precedes_windows_launcher(self):
        with tempfile.TemporaryDirectory() as directory:
            program_files = Path(directory) / 'Program Files'
            git_bash = program_files / 'Git/bin/bash.exe'
            git_bash.parent.mkdir(parents=True)
            git_bash.write_bytes(b'Git Bash control')
            with patch.dict(os.environ, {'ProgramFiles': str(program_files)}, clear=True):
                self.assertEqual(audit.bash_program(), str(git_bash))
            with patch.dict(os.environ, {}, clear=True):
                self.assertEqual(audit.bash_program(), 'bash')

    def test_shared_build_overrides_refused_before_any_tool_launch(self):
        for key in ('CARGO_TARGET_DIR', 'CARGO_BUILD_TARGET_DIR', 'CARGO_BUILD_BUILD_DIR'):
            with self.subTest(key=key), tempfile.TemporaryDirectory() as directory:
                output = Path(directory) / 'output'
                with patch.dict(os.environ, {key: str(Path(directory) / 'owner-build')}), \
                        patch.object(sys, 'argv', ['mutation-audit.py', '--output', str(output)]), \
                        patch.object(audit.subprocess, 'run') as launched:
                    with self.assertRaises(SystemExit) as error:
                        audit.main()
                    self.assertEqual(error.exception.code, 2)
                    self.assertFalse(output.exists())
                    launched.assert_not_called()

    def test_nonzero_completed_outcomes_preserve_status_and_review_requirement(self):
        for status in [0, 2, 3]:
            with self.subTest(status=status), tempfile.TemporaryDirectory() as directory:
                actual, output = self.run_main(directory, status=status)
                result = json.loads((output / 'audit.json').read_text())
                self.assertEqual(actual, status)
                self.assertEqual(result['tool_returncode'], status)
                self.assertTrue(result['execution_complete'])
                self.assertFalse(result['full_review_complete'])
                self.assertTrue(result['review_required'])
                self.assertEqual(result['jobs'], 4)
                self.assertEqual(result['scope'], 'full workspace')
                self.assertEqual(result['exclude_files'], [])
                self.assertNotIn('--exclude', result['command'])
                self.assertFalse((output / 'scratch').exists())

    def test_interrupted_runner_preserves_distinct_helper_and_tool_status(self):
        with tempfile.TemporaryDirectory() as directory:
            actual, output = self.run_main(directory, status=125, interrupted=True)
            result = json.loads((output / 'audit.json').read_text())
            self.assertEqual(actual, 130)
            self.assertEqual(result['resource_wrapper_returncode'], 125)
            self.assertEqual(result['tool_returncode'], 70)
            self.assertFalse(result['execution_complete'])
            self.assertFalse((output / 'scratch').exists())

    def test_source_change_invalidates_completed_execution(self):
        with tempfile.TemporaryDirectory() as directory:
            actual, output = self.run_main(directory, source_changed=True)
            result = json.loads((output / 'audit.json').read_text())
            self.assertEqual(actual, 125)
            self.assertFalse(result['execution_complete'])
            self.assertFalse(result['source_stable'])
            self.assertEqual(result['changed_source_files'], ['src/lib.rs'])
            self.assertEqual(result['tool_returncode'], 0)

    def test_unconfirmed_cleanup_keeps_owned_scratch(self):
        for resources in [False, True]:
            with self.subTest(resources=resources), tempfile.TemporaryDirectory() as directory:
                actual, output = self.run_main(directory, resources=resources, cleanup_errors=['child still alive'])
                result = json.loads((output / 'audit.json').read_text())
                self.assertEqual(actual, 125)
                self.assertFalse(result['execution_complete'])
                self.assertTrue((output / 'scratch').is_dir())
                self.assertEqual(result['retained_scratch'], str((output / 'scratch').resolve()))
                self.assertTrue(result['cleanup_errors'])

    def test_subcrate_does_not_allow_output_inside_workspace(self):
        with tempfile.TemporaryDirectory() as directory:
            self.run_main(directory, output_inside=True)

    def test_missing_diff_is_rejected_before_owned_output_creation(self):
        with tempfile.TemporaryDirectory() as directory:
            self.run_main(directory, extra=('--in-diff', str(Path(directory) / 'absent.diff')))

    def test_diff_selection_is_staged_and_bound(self):
        with tempfile.TemporaryDirectory() as directory:
            diff = Path(directory) / 'input.diff'
            contents = b'--- a/src/lib.rs\n+++ b/src/lib.rs\n'
            diff.write_bytes(contents)
            actual, output = self.run_main(directory, extra=('--in-diff', str(diff)))
            result = json.loads((output / 'audit.json').read_text())
            self.assertEqual(actual, 0)
            self.assertEqual(result['scope'], 'focused')
            self.assertEqual((output / 'selection.diff').read_bytes(), contents)
            selected = result['command'][result['command'].index('--in-diff') + 1]
            self.assertEqual(Path(selected), (output / 'selection.diff').resolve())
            self.assertEqual(result['selection_diff']['sha256'], audit.file_digest(diff))

    def test_windows_interrupt_uses_active_tree_stop_and_bounded_wait(self):
        process = type('Process', (), {'pid': 123, 'poll': lambda self: None, 'wait': lambda self, **kwargs: None})()
        with patch.object(audit.os, 'name', 'nt'), patch.object(audit.signal, 'signal') as signals, \
                patch.object(audit.subprocess, 'run') as stopped:
            self.assertEqual(audit.interrupt_process(process), [])
            stopped.assert_called_once_with(['taskkill', '/PID', '123', '/T', '/F'], capture_output=True, check=True, timeout=10)
            signals.assert_any_call(signal.SIGINT, signal.SIG_IGN)
            signals.assert_any_call(signal.SIGTERM, signal.SIG_IGN)

    def test_failed_interrupted_reap_stays_explicit(self):
        process = type('Process', (), {'poll': lambda self: 0, 'wait': lambda self, **kwargs: (_ for _ in ()).throw(subprocess.TimeoutExpired('wrapper', 10))})()
        with patch.object(audit.signal, 'signal'):
            errors = audit.interrupt_process(process)
        self.assertIn('interrupted wrapper cleanup', errors[0])


class SourceTests(unittest.TestCase):
    def test_actual_source_bytes_are_bound_and_build_products_ignored(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'Cargo.toml').write_text('[workspace]\n')
            before = audit.source_identity(root)
            (root / 'target').mkdir()
            (root / 'target/build-output').write_bytes(b'compile product')
            self.assertEqual(audit.source_identity(root), before)
            (root / 'Cargo.toml').write_text('[workspace]\nmembers=[]\n')
            self.assertNotEqual(audit.source_identity(root), before)
