# SPDX-License-Identifier: MPL-2.0
"""Windows watchdog cancellation at the actual native process-tree boundary."""
import ctypes
import json
import os
from pathlib import Path
import runpy
import shutil
import subprocess
import sys
import tempfile
import time
from types import SimpleNamespace
import unittest
from unittest import mock

ROOT = Path(__file__).resolve().parents[1]


class FuzzProcessCleanupTest(unittest.TestCase):
    def controller(self):
        return runpy.run_path(str(ROOT / 'scripts/focused-smoke.py'))['stop_children']

    def test_windows_stops_live_tree_without_terminating_watchdog_first(self):
        stop = self.controller()
        process = mock.Mock(pid=123)
        process.poll.side_effect = [None, 1]
        children = [process]
        with mock.patch.dict(stop.__globals__, os=SimpleNamespace(name='nt')):
            with mock.patch.object(subprocess, 'run') as taskkill:
                self.assertTrue(stop(children), 'forced watchdog stop cannot confirm its own report')
        taskkill.assert_called_once_with(['taskkill', '/PID', '123', '/T', '/F'],
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=True, timeout=5)
        process.terminate.assert_not_called()
        self.assertEqual(children, [])

    def test_windows_taskkill_failure_stays_uncertain(self):
        stop = self.controller()
        process = mock.Mock(pid=456)
        process.poll.side_effect = [None, 1]
        children = [process]
        with mock.patch.dict(stop.__globals__, os=SimpleNamespace(name='nt')):
            with mock.patch.object(subprocess, 'run', side_effect=subprocess.TimeoutExpired('taskkill', 5)):
                self.assertTrue(stop(children))
        process.terminate.assert_not_called()
        self.assertEqual(children, [])

    def test_completed_windows_watchdog_has_no_numeric_pid_signal(self):
        stop = self.controller()
        process = mock.Mock(pid=789)
        process.poll.return_value = 0
        children = [process]
        with mock.patch.dict(stop.__globals__, os=SimpleNamespace(name='nt')):
            with mock.patch.object(subprocess, 'run') as taskkill:
                self.assertFalse(stop(children))
        taskkill.assert_not_called()
        process.terminate.assert_not_called()
        self.assertEqual(children, [])

    @unittest.skipUnless(os.name == 'nt', 'requires actual Windows taskkill/process handles')
    def test_native_tree_stop_and_old_terminate_counterexample(self):
        # Retain handles to both descendants before the negative control. These
        # reserve their process identities even after their parent is killed.
        from ctypes import wintypes
        kernel = ctypes.WinDLL('kernel32', use_last_error=True)
        kernel.OpenProcess.argtypes = [wintypes.DWORD, wintypes.BOOL, wintypes.DWORD]
        kernel.OpenProcess.restype = wintypes.HANDLE
        kernel.WaitForSingleObject.argtypes = [wintypes.HANDLE, wintypes.DWORD]
        kernel.WaitForSingleObject.restype = wintypes.DWORD
        kernel.GetProcessTimes.argtypes = [wintypes.HANDLE] + [ctypes.POINTER(wintypes.FILETIME)] * 4
        kernel.GetProcessTimes.restype = wintypes.BOOL
        def birth(handle):
            times = [wintypes.FILETIME() for _ in range(4)]
            if not kernel.GetProcessTimes(handle, *(ctypes.byref(value) for value in times)):
                raise ctypes.WinError(ctypes.get_last_error())
            return times[0].dwLowDateTime | (times[0].dwHighDateTime << 32)
        def owned_handle(pid, expected_birth):
            handle = kernel.OpenProcess(0x00101001, False, pid)
            if not handle:
                raise ctypes.WinError(ctypes.get_last_error())
            try:
                if birth(handle) != expected_birth:
                    raise AssertionError('process identity changed before handle acquisition')
            except (OSError, AssertionError):
                kernel.CloseHandle(handle)
                raise
            return handle
        kernel.TerminateProcess.argtypes = [wintypes.HANDLE, wintypes.UINT]
        kernel.TerminateProcess.restype = wintypes.BOOL
        kernel.CloseHandle.argtypes = [wintypes.HANDLE]
        kernel.CloseHandle.restype = wintypes.BOOL
        root = Path(tempfile.mkdtemp(prefix='htmlcut-fuzz-process-control.'))
        controls = []
        facts = []
        confirmed = False
        observations_complete = False
        try:
            for old_terminate in [False, True]:
                owned = root / ('old-terminate' if old_terminate else 'tree-stop')
                owned.mkdir()
                ready, writer_ready, marker, start = (owned / name for name in ['ready.json', 'writer-ready', 'late-write', 'start'])
                writer_code = ('from pathlib import Path; import os,sys,time; '
                    'p=Path(sys.argv[1]); p.with_suffix(".pending").write_text(str(os.getpid())); p.with_suffix(".pending").replace(p); '
                    'deadline=time.monotonic()+15; '
                    '\nwhile not Path(sys.argv[3]).exists() and time.monotonic()<deadline: time.sleep(.01)\n'
                    'if not Path(sys.argv[3]).exists(): sys.exit(2)\n'
                    'Path(sys.argv[2]).write_text("writer survived"); time.sleep(15)')
                child_code = """from pathlib import Path
import ctypes,json,os,subprocess,sys,time
from ctypes import wintypes as W
k=ctypes.WinDLL('kernel32',use_last_error=True)
k.GetCurrentProcess.restype=W.HANDLE
k.GetProcessTimes.argtypes=[W.HANDLE]+[ctypes.POINTER(W.FILETIME)]*4
k.GetProcessTimes.restype=W.BOOL
def birth(handle):
    times=[W.FILETIME() for _ in range(4)]
    if not k.GetProcessTimes(handle,*(ctypes.byref(value) for value in times)):
        raise ctypes.WinError(ctypes.get_last_error())
    return times[0].dwLowDateTime | (times[0].dwHighDateTime<<32)
writer=subprocess.Popen([sys.executable,'-c',sys.argv[1],sys.argv[3],sys.argv[4],sys.argv[5]])
p=Path(sys.argv[2])
p.with_suffix('.pending').write_text(json.dumps({
    'child':{'pid':os.getpid(),'birth':birth(k.GetCurrentProcess())},
    'writer':{'pid':writer.pid,'birth':birth(int(writer._handle))}}))
p.with_suffix('.pending').replace(p)
time.sleep(30)
"""
                process = subprocess.Popen([sys.executable, str(ROOT / 'scripts/run-resource-bounded.py'),
                    '--root', str(owned), '--disk-limit-mib', '8', '--timeout', '30', '--log-limit-mib', '1',
                    '--label', 'control', '--', sys.executable, '-c', child_code,
                    writer_code, str(ready), str(writer_ready), str(marker), str(start)],
                    stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
                control = {'process': process, 'handles': [], 'owned': owned}
                controls.append(control)
                deadline = time.monotonic() + 5
                while (not ready.exists() or not writer_ready.exists()) and time.monotonic() < deadline:
                    time.sleep(.01)
                self.assertTrue(ready.exists() and writer_ready.exists(), 'owned writer did not start')
                pids = json.loads(ready.read_text())
                self.assertEqual(int(writer_ready.read_text()), pids['writer']['pid'])
                for role in ['child', 'writer']:
                    expected = pids[role]
                    # Query creation time before authorizing any PID-based
                    # cleanup; a stale readiness PID may already be reused.
                    handle = owned_handle(expected['pid'], expected['birth'])
                    control['handles'].append((role, expected['pid'], handle))
                with self.assertRaisesRegex(AssertionError, 'identity changed'):
                    owned_handle(pids['writer']['pid'], pids['writer']['birth'] + 1)
                self.assertFalse(marker.exists(), 'writer fired before cancellation control')
                children = [process]
                if old_terminate:
                    # Reproduce the old cancellation primitive: it kills only
                    # the watchdog, before that watchdog can stop its own tree.
                    process.terminate()
                    process.wait(timeout=2)
                    start.write_text('write after watchdog termination')
                    deadline = time.monotonic() + 4
                    while not marker.exists() and time.monotonic() < deadline:
                        time.sleep(.01)
                    self.assertEqual(marker.read_text(), 'writer survived')
                    self.assertEqual(kernel.WaitForSingleObject(control['handles'][1][2], 0), 258)
                    uncertain = True
                else:
                    uncertain = self.controller()(children)
                    start.write_text('write after tree termination')
                    for role, _, handle in control['handles']:
                        self.assertEqual(kernel.WaitForSingleObject(handle, 2000), 0, f'{role} survived tree stop')
                    self.assertFalse(marker.exists())
                    self.assertTrue(uncertain)
                    self.assertEqual(children, [])
                facts.append({'case': 'old-terminate' if old_terminate else 'tree-stop',
                              'late_writer_marker': marker.exists(), 'cleanup_uncertain': uncertain,
                              'watchdog_returncode': process.returncode})
            self.assertEqual([x['late_writer_marker'] for x in facts], [False, True])
            observations_complete = True
        finally:
            failures, cleanup_attempt_errors = [], []
            for control in controls:
                process = control['process']
                try:
                    if process.poll() is None:
                        try:
                            subprocess.run(['taskkill', '/PID', str(process.pid), '/T', '/F'],
                                           stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                                           check=True, timeout=5)
                        except (OSError, subprocess.SubprocessError) as error:
                            cleanup_attempt_errors.append('watchdog tree: ' + str(error))
                            process.kill()
                        process.wait(timeout=2)
                except (OSError, subprocess.SubprocessError) as error:
                    failures.append('watchdog cleanup: ' + str(error))
                if len(control['handles']) != 2:
                    failures.append('descendant handles were not both pinned before failure')
                # Both handles keep exact process identities owned. Stop each
                # still-live process even if another cleanup action failed.
                for role, pid, handle in control['handles']:
                    if kernel.WaitForSingleObject(handle, 0) == 258:
                        try:
                            subprocess.run(['taskkill', '/PID', str(pid), '/T', '/F'],
                                           stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                                           check=True, timeout=5)
                        except (OSError, subprocess.SubprocessError) as error:
                            cleanup_attempt_errors.append(role + ' tree: ' + str(error))
                        if kernel.WaitForSingleObject(handle, 0) == 258:
                            if not kernel.TerminateProcess(handle, 1):
                                cleanup_attempt_errors.append(role + ' handle termination: ' + str(ctypes.get_last_error()))
                for role, _, handle in control['handles']:
                    if kernel.WaitForSingleObject(handle, 2000) != 0:
                        failures.append(f'{role} cleanup not confirmed')
                    if not kernel.CloseHandle(handle):
                        failures.append(role + ' handle close failed')
            confirmed = not failures
            if confirmed and observations_complete:
                shutil.rmtree(root)
            else:
                (root / 'control-failure.json').write_text(json.dumps({
                    'facts': facts, 'cleanup_failures': failures,
                    'cleanup_attempt_errors': cleanup_attempt_errors}, indent=2))
                print(f'Owned process evidence retained at {root}: {failures}', file=sys.stderr)
            if failures:
                self.fail(f'Owned process cleanup unconfirmed at {root}: {failures}')
        self.assertTrue(confirmed)
        print('native Windows fuzz cancellation control: ' + json.dumps(facts, sort_keys=True))
