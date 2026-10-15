#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Run a QA command with finite time/log limits and a sampled owned-disk budget.

This watchdog is not a filesystem quota: a writer may exceed the disk budget
between samples. It never deletes files; callers own their temporary directories.
"""
import argparse
import json
import math
import os
from pathlib import Path
import signal
import subprocess
import sys
import threading
import time


def disk_bytes(root):
    """Conservatively count file/directory allocation without following links."""
    def walk_error(error):
        # Build tools may remove a directory between enumeration and descent.
        # Inaccessible directories cannot be safely omitted from accounting.
        if not isinstance(error, FileNotFoundError):
            raise error

    total = 0
    for directory, subdirectories, files in os.walk(root, followlinks=False, onerror=walk_error):
        paths = [Path(directory)] + [Path(directory) / name for name in files]
        paths += [Path(directory) / name for name in subdirectories if (Path(directory) / name).is_symlink()]
        for path in paths:
            try:
                stat = path.lstat()
            except FileNotFoundError:
                continue
            total += max(stat.st_size, getattr(stat, "st_blocks", 0) * 512)
    return total


def stop_process_tree(process):
    if os.name == "nt":
        result = subprocess.run(["taskkill", "/PID", str(process.pid), "/T", "/F"],
                                stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                                check=False, timeout=5)
        if result.returncode and process.poll() is None:
            raise OSError(f"taskkill failed with exit {result.returncode}")
    else:
        cooperation_timed_out = False
        # cargo-mutants owns separate build/test groups. Give its interrupt
        # handler a bounded chance to stop those before killing the outer group.
        if process.poll() is None:
            try:
                process.send_signal(signal.SIGINT)
            except ProcessLookupError:
                pass
            try:
                process.wait(timeout=1)
            except subprocess.TimeoutExpired:
                cooperation_timed_out = True
        try:
            os.killpg(process.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        except PermissionError:
            # Darwin can return EPERM for an exited group. Do not ignore an
            # actual live-group denial or an unavailable/uncertain inspection.
            if sys.platform != "darwin" or process.poll() is None:
                raise
            listing = subprocess.check_output(["ps", "-axo", "pgid=,stat="],
                                              text=True, timeout=2)
            for line in listing.splitlines():
                group, state = line.split()
                if int(group) == process.pid and not state.startswith("Z"):
                    raise PermissionError("live process group cleanup was denied")
        if cooperation_timed_out:
            raise TimeoutError("cooperative cleanup exceeded grace; escaped descendant cleanup is unconfirmed")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--disk-limit-mib", type=int, required=True)
    parser.add_argument("--timeout", type=float, required=True)
    parser.add_argument("--log-limit-mib", type=int, default=8)
    parser.add_argument("--poll-seconds", type=float, default=1,
                        help="sampling interval, capped at one second and the remaining deadline")
    parser.add_argument("--label", required=True)
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    command = args.command[1:] if args.command[:1] == ["--"] else args.command
    if not command or min(args.disk_limit_mib, args.timeout, args.log_limit_mib, args.poll_seconds) <= 0 or not math.isfinite(args.timeout) or not math.isfinite(args.poll_seconds):
        parser.error("command and positive resource limits are required")
    if not args.label.replace("-", "").replace("_", "").isalnum():
        parser.error("label must contain only letters, numbers, hyphens or underscores")
    root = args.root.resolve(strict=True)
    if not root.is_dir():
        parser.error("root must be an existing task-owned directory")
    log_path = root / (args.label + ".log")
    report_path = root / (args.label + ".resources.json")
    # Refuse to overwrite evidence from any previous run.
    if report_path.exists():
        parser.error("resource report already exists")
    cancelled = threading.Event()
    signal.signal(signal.SIGINT, lambda *_: cancelled.set())
    signal.signal(signal.SIGTERM, lambda *_: cancelled.set())
    with log_path.open("xb") as log:
        started = time.monotonic()
        process = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                                   start_new_session=os.name != "nt")
        log_bytes = 0
        log_exceeded = threading.Event()
        reader_failed = []

        def read_output():
            nonlocal log_bytes
            try:
                while chunk := process.stdout.read1(65536):
                    remaining = args.log_limit_mib * 1024 * 1024 - log_bytes
                    kept = chunk[:max(0, remaining)]
                    log.write(kept)
                    log.flush()
                    log_bytes += len(kept)
                    if len(chunk) > len(kept):
                        log_exceeded.set()
                        # Drain without retaining excess: a cooperative stop
                        # handler must not block writing cleanup diagnostics.
            except Exception as error:
                reader_failed.append(str(error))
                log_exceeded.set()

        reader = threading.Thread(target=read_output, daemon=True)
        reader.start()
        peak = 0
        refusal = None
        accounting_errors = []
        cleanup_errors = []
        try:
            while True:
                try:
                    usage = disk_bytes(root)
                except OSError as error:
                    accounting_errors.append({"phase": "monitor", "error": str(error)})
                    refusal = "disk-accounting-failed: " + str(error)
                    break
                peak = max(peak, usage)
                remaining = args.timeout - (time.monotonic() - started)
                if cancelled.is_set():
                    refusal = "interrupted"
                elif usage > args.disk_limit_mib * 1024 * 1024:
                    refusal = "disk-budget-exceeded"
                elif reader_failed:
                    refusal = "output-reader-failed: " + reader_failed[0]
                elif log_exceeded.is_set():
                    refusal = "log-budget-exceeded"
                elif remaining <= 0:
                    refusal = "wall-time-exceeded"
                if refusal or process.poll() is not None:
                    break
                # Large finite values must not overflow sleep or postpone the
                # deadline or defer the next scan by more than one second.
                time.sleep(min(args.poll_seconds, remaining, 1.0))
        except Exception as error:
            refusal = "monitor-failed: " + type(error).__name__ + ": " + str(error)
        finally:
            # Kill descendants even if their parent has exited; they may hold
            # stdout open or continue writing after apparent success.
            try:
                stop_process_tree(process)
            except Exception as error:
                cleanup_errors.append(type(error).__name__ + ": " + str(error))
                # Still reap the leader where possible; this does not prove
                # descendant cleanup when group termination failed.
                try:
                    process.kill()
                except ProcessLookupError:
                    pass
                except OSError as kill_error:
                    cleanup_errors.append("leader kill: " + str(kill_error))
            try:
                returncode = process.wait(timeout=2)
            except subprocess.TimeoutExpired:
                returncode = None
                cleanup_errors.append("leader did not exit within cleanup grace")
            reader.join(timeout=5)
        if cleanup_errors:
            refusal = refusal or "process-cleanup-failed"
        if reader.is_alive():
            refusal = refusal or "output-reader-did-not-stop"
            cleanup_errors.append("output reader still holds the child pipe")
        if log_exceeded.is_set():
            refusal = refusal or "log-budget-exceeded"
        if reader_failed:
            refusal = refusal or "output-reader-failed: " + reader_failed[0]
        try:
            peak = max(peak, disk_bytes(root))
        except Exception as error:
            accounting_errors.append({"phase": "final", "error": str(error)})
            refusal = refusal or "disk-accounting-failed: " + str(error)
        if peak > args.disk_limit_mib * 1024 * 1024:
            refusal = refusal or "disk-budget-exceeded"
        result = {"command": command, "returncode": returncode, "refusal": refusal,
                  "elapsed_seconds": round(time.monotonic() - started, 3),
                  "peak_sampled_disk_bytes": peak, "disk_limit_bytes": args.disk_limit_mib * 1024 * 1024,
                  "log_bytes": log_bytes, "log_limit_bytes": args.log_limit_mib * 1024 * 1024,
                  "disk_accounting_errors": accounting_errors,
                  "cleanup_errors": cleanup_errors,
                  "reader_errors": reader_failed,
                  "disk_enforcement": "sampled watchdog; not a filesystem quota"}
        with report_path.open("x", encoding="utf-8") as report:
            json.dump(result, report, indent=2)
            report.write("\n")
        print(json.dumps(result), file=sys.stderr)
        return 125 if refusal else returncode if returncode >= 0 else 128 - returncode


if __name__ == "__main__":
    sys.exit(main())
