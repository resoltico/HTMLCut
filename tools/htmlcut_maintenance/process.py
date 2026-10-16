#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Run a QA command with finite time/log limits and a sampled owned-disk budget.

This watchdog is not a filesystem quota: a writer may exceed the disk budget
between samples. It never deletes files; callers own their temporary directories.
"""

import argparse
import errno
import json
import math
import os
from pathlib import Path
import signal
import stat
import subprocess
import sys
import threading
import time


def vanished_entry(error):
    if isinstance(error, FileNotFoundError):
        return True
    # Windows can report access denied between a removal's disposition and
    # close. Only a fresh missing-name result distinguishes that from denial.
    if getattr(error, "winerror", None) == 5 and error.filename:
        try:
            os.lstat(error.filename)
        except FileNotFoundError:
            return True
        except OSError:
            pass
    return False


def disk_bytes(root):
    """Conservatively count file/directory allocation without following links."""
    info = os.lstat(root)
    total = max(info.st_size, getattr(info, "st_blocks", 0) * 512)
    reparse = getattr(stat, "FILE_ATTRIBUTE_REPARSE_POINT", 0x400)

    def descend(info):
        # A Windows junction need not be a symlink. Its non-following metadata
        # must decide both accounting and descent, just as for ordinary links.
        return stat.S_ISDIR(info.st_mode) and not (
            getattr(info, "st_file_attributes", 0) & reparse
        )

    def entries(directory):
        try:
            with os.scandir(directory) as found:
                yield from found
        except OSError as error:
            # Compilers remove products during enumeration. Permission and
            # accounting errors still refuse; absence alone permits omission.
            if not vanished_entry(error):
                raise

    pending = [os.fspath(root)] if descend(info) else []
    while pending:
        for entry in entries(pending.pop()):
            try:
                info = entry.stat(follow_symlinks=False)
            except OSError as error:
                if vanished_entry(error):
                    continue
                raise
            total += max(info.st_size, getattr(info, "st_blocks", 0) * 512)
            if descend(info):
                pending.append(entry.path)
    return total


def check_process_ownership():
    """Require an unreaped POSIX child identity until group termination."""
    if os.name == "nt":
        return
    if any(
        not hasattr(os, name)
        for name in ("waitid", "P_PID", "WEXITED", "WNOHANG", "WNOWAIT")
    ):
        raise ValueError(
            "bounded POSIX QA requires waitid/WNOWAIT; use Python 3.13+ on macOS or a supported Python 3.11+ on Linux"
        )
    if signal.getsignal(signal.SIGCHLD) != signal.SIG_DFL:
        raise ValueError(
            "bounded POSIX QA requires default SIGCHLD handling; ignored/custom handlers may reap the owned child"
        )


def child_exited(process):
    if os.name == "nt":
        return process.poll() is not None
    # WNOWAIT keeps an exited leader waitable, reserving its PID/PGID until
    # cleanup. ECHILD means ownership was lost: never signal a possibly reused ID.
    return (
        os.waitid(os.P_PID, process.pid, os.WEXITED | os.WNOHANG | os.WNOWAIT)
        is not None
    )


def command_exit_status(returncode):
    # Windows child statuses are DWORDs. MSYS shells retain only eight bits,
    # so a nonzero multiple of 256 must not become a successful wrapper exit.
    if returncode > 255:
        return 1
    return returncode if returncode >= 0 else 128 - returncode


def stop_process_tree(process):
    if os.name == "nt":
        result = subprocess.run(
            ["taskkill", "/PID", str(process.pid), "/T", "/F"],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            check=False,
            timeout=5,
        )
        if result.returncode and process.poll() is None:
            raise OSError(f"taskkill failed with exit {result.returncode}")
    else:
        cooperation_timed_out = False
        if not child_exited(process):
            # Popen.send_signal/poll/wait can reap. Signal the reserved PID
            # directly and use only nonreaping observations before killpg.
            try:
                os.kill(process.pid, signal.SIGINT)
            except ProcessLookupError:
                pass
            deadline = time.monotonic() + 1
            while not child_exited(process):
                remaining = deadline - time.monotonic()
                if remaining <= 0:
                    cooperation_timed_out = True
                    break
                time.sleep(min(remaining, 0.01))
        try:
            os.killpg(process.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        except PermissionError:
            if sys.platform != "darwin" or not child_exited(process):
                raise
            listing = subprocess.check_output(
                ["ps", "-axo", "pgid=,stat="], text=True, timeout=2
            )
            for line in listing.splitlines():
                group, state = line.split()
                if int(group) == process.pid and not state.startswith("Z"):
                    raise PermissionError("live process group cleanup was denied")
        if cooperation_timed_out:
            raise TimeoutError(
                "cooperative cleanup exceeded grace; escaped descendant cleanup is unconfirmed"
            )


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--disk-limit-mib", type=int, required=True)
    parser.add_argument("--timeout", type=float, required=True)
    parser.add_argument("--log-limit-mib", type=int, default=8)
    parser.add_argument(
        "--poll-seconds",
        type=float,
        default=1,
        help="sampling interval, capped at one second and the remaining deadline",
    )
    parser.add_argument("--label", required=True)
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args(argv)
    cancelled = threading.Event()
    previous = {
        sig: signal.signal(sig, lambda *_: cancelled.set())
        for sig in (signal.SIGINT, signal.SIGTERM)
    }
    try:
        try:
            result = run(
                args.command,
                args.root,
                disk_limit_mib=args.disk_limit_mib,
                timeout=args.timeout,
                label=args.label,
                log_limit_mib=args.log_limit_mib,
                poll_seconds=args.poll_seconds,
                cancelled=cancelled,
            )
        except ValueError as error:
            parser.error(str(error))
        return status(result)
    finally:
        for sig, handler in previous.items():
            signal.signal(sig, handler)


def status(result):
    return 125 if result["refusal"] else command_exit_status(result["returncode"])


def run(
    command,
    root,
    *,
    disk_limit_mib,
    timeout,
    label,
    log_limit_mib=8,
    poll_seconds=1,
    env=None,
    cwd=None,
    cancelled=None,
):
    """Own a command, cooperatively handling main-thread signals when no owner event was supplied."""
    options = dict(
        disk_limit_mib=disk_limit_mib,
        timeout=timeout,
        label=label,
        log_limit_mib=log_limit_mib,
        poll_seconds=poll_seconds,
        env=env,
        cwd=cwd,
    )
    if cancelled is not None:
        return _run(command, root, cancelled=cancelled, **options)
    if threading.current_thread() is not threading.main_thread():
        raise ValueError(
            "worker process ownership requires an owner-supplied cancellation event"
        )
    cancelled = threading.Event()
    previous = {
        sig: signal.signal(sig, lambda *_: cancelled.set())
        for sig in (signal.SIGINT, signal.SIGTERM)
    }
    try:
        return _run(command, root, cancelled=cancelled, **options)
    finally:
        for sig, handler in previous.items():
            signal.signal(sig, handler)


def _run(
    command,
    root,
    *,
    disk_limit_mib,
    timeout,
    label,
    log_limit_mib=8,
    poll_seconds=1,
    env=None,
    cwd=None,
    cancelled=None,
):
    """Own one bounded child and return its recorded result after cleanup."""
    from types import SimpleNamespace

    args = SimpleNamespace(
        command=list(command),
        root=Path(root),
        disk_limit_mib=disk_limit_mib,
        timeout=timeout,
        label=label,
        log_limit_mib=log_limit_mib,
        poll_seconds=poll_seconds,
    )
    command = args.command[1:] if args.command[:1] == ["--"] else args.command
    if (
        not command
        or min(args.disk_limit_mib, args.timeout, args.log_limit_mib, args.poll_seconds)
        <= 0
        or not math.isfinite(args.timeout)
        or not math.isfinite(args.poll_seconds)
    ):
        raise ValueError("command and positive resource limits are required")
    if not args.label.replace("-", "").replace("_", "").isalnum():
        raise ValueError(
            "label must contain only letters, numbers, hyphens or underscores"
        )
    try:
        check_process_ownership()
    except ValueError as error:
        raise ValueError(str(error))
    root = args.root.resolve(strict=True)
    if not root.is_dir():
        raise ValueError("root must be an existing task-owned directory")
    log_path = root / (args.label + ".log")
    report_path = root / (args.label + ".resources.json")
    # Refuse to overwrite evidence from any previous run.
    if report_path.exists():
        raise ValueError("resource report already exists")
    cancelled = cancelled if cancelled is not None else threading.Event()
    with log_path.open("xb") as log:
        started = time.monotonic()
        process = subprocess.Popen(
            command,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            start_new_session=os.name != "nt",
            env=env,
            cwd=cwd,
        )
        log_bytes = 0
        log_exceeded = threading.Event()
        reader_failed = []

        def read_output():
            nonlocal log_bytes
            try:
                while chunk := process.stdout.read1(65536):
                    remaining = args.log_limit_mib * 1024 * 1024 - log_bytes
                    kept = chunk[: max(0, remaining)]
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
        child_identity_lost = False
        sample_count = 0
        completed_samples = 0
        sample_seconds = 0.0
        scan_retries = 0
        access_errors = 0
        access_error_witnesses = []

        def sample(phase):
            nonlocal sample_count, completed_samples, sample_seconds
            nonlocal scan_retries, access_errors
            retry_deadline = None
            exit_confirmation = False
            last_denial = None
            for attempt in range(21):
                if attempt:
                    if stop_reason() or time.monotonic() >= retry_deadline:
                        raise last_denial
                    scan_retries += 1
                sample_count += 1
                sampled = time.monotonic()
                try:
                    try:
                        usage = disk_bytes(root)
                    finally:
                        elapsed = time.monotonic() - sampled
                        sample_seconds += elapsed
                except OSError as error:
                    if error.errno != errno.EACCES:
                        raise
                    last_denial = error
                    access_errors += 1
                    if len(access_error_witnesses) < 16:
                        text = str(error)
                        access_error_witnesses.append({
                            "phase": phase,
                            "attempt": attempt + 1,
                            "error": text[:512],
                            "error_truncated": len(text) > 512,
                            "scan_seconds": round(elapsed, 6),
                        })
                    # A fresh complete walk may recover a changing namespace or
                    # test-owned permission window. Never accept a partial sum.
                    if attempt == 20 or stop_reason():
                        raise
                    if retry_deadline is None:
                        retry_deadline = time.monotonic() + min(args.poll_seconds, 1.0)
                    if phase == "final" or process.returncode is not None or child_exited(process):
                        # Final accounting never queries child identity again.
                        # Allow only one immediate confirmation after cleanup/exit.
                        if exit_confirmation:
                            raise
                        exit_confirmation = True
                    elif attempt:
                        cancelled.wait(max(0, min(0.05, retry_deadline - time.monotonic())))
                else:
                    completed_samples += 1
                    return usage

        def stop_reason():
            if cancelled.is_set():
                return "interrupted"
            if reader_failed:
                return "output-reader-failed: " + reader_failed[0]
            if log_exceeded.is_set():
                return "log-budget-exceeded"
            if time.monotonic() - started >= args.timeout:
                return "wall-time-exceeded"
            return None

        next_sample = started
        try:
            while True:
                # The first sample is mandatory, including for immediate exits.
                # Subsequent cheap observations do not repeatedly walk products.
                if sample_count:
                    refusal = stop_reason()
                    if refusal or child_exited(process):
                        break
                if time.monotonic() >= next_sample:
                    try:
                        usage = sample("monitor")
                    except ChildProcessError:
                        raise
                    except OSError as error:
                        accounting_errors.append(
                            {"phase": "monitor", "error": str(error)}
                        )
                        refusal = stop_reason() or "disk-accounting-failed: " + str(error)
                        break
                    peak = max(peak, usage)
                    if usage > args.disk_limit_mib * 1024 * 1024:
                        refusal = "disk-budget-exceeded"
                    # Never perform catch-up scans after a slow metadata call.
                    next_sample = time.monotonic() + min(args.poll_seconds, 1.0)
                refusal = refusal or stop_reason()
                if refusal or child_exited(process):
                    break
                remaining = args.timeout - (time.monotonic() - started)
                # This bounds idle waits, not the duration of a blocking scan.
                # Event waits also observe an owner cancellation immediately.
                cancelled.wait(
                    max(0, min(0.05, next_sample - time.monotonic(), remaining))
                )
        except Exception as error:
            child_identity_lost = isinstance(error, ChildProcessError)
            refusal = "monitor-failed: " + type(error).__name__ + ": " + str(error)
        finally:
            # Keep the POSIX leader unreaped until its last group signal.
            if not child_identity_lost:
                try:
                    stop_process_tree(process)
                except Exception as error:
                    child_identity_lost = isinstance(error, ChildProcessError)
                    cleanup_errors.append(type(error).__name__ + ": " + str(error))
                    if not child_identity_lost:
                        try:
                            process.kill()
                        except ProcessLookupError:
                            pass
                        except OSError as kill_error:
                            cleanup_errors.append("leader kill: " + str(kill_error))
            if child_identity_lost:
                cleanup_errors.append(
                    "child identity lost; no further PID/group signals or waits attempted"
                )
                # Disarm Popen's destructor polling too. The report retains an
                # unknown returncode, not this internal already-finished marker.
                process.returncode = 0
                returncode = None
            else:
                try:
                    returncode = process.wait(timeout=2)
                except subprocess.TimeoutExpired:
                    returncode = None
                    cleanup_errors.append("leader did not exit within cleanup grace")
            reader.join(timeout=5)
            if not reader.is_alive():
                try:
                    process.stdout.close()
                except OSError as error:
                    reader_failed.append("pipe close: " + str(error))
        if child_identity_lost:
            returncode = None
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
            peak = max(peak, sample("final"))
        except Exception as error:
            accounting_errors.append({"phase": "final", "error": str(error)})
            refusal = refusal or stop_reason() or "disk-accounting-failed: " + str(error)
        if peak > args.disk_limit_mib * 1024 * 1024:
            refusal = refusal or "disk-budget-exceeded"
        # Child exit is not host completion: an interruption during owned
        # cleanup/final accounting must not launch another successful phase.
        if cancelled.is_set():
            refusal = refusal or "interrupted"
        result = {
            "command": command,
            "returncode": returncode,
            "refusal": refusal,
            "elapsed_seconds": round(time.monotonic() - started, 3),
            "peak_sampled_disk_bytes": peak,
            "disk_limit_bytes": args.disk_limit_mib * 1024 * 1024,
            "log_bytes": log_bytes,
            "log_limit_bytes": args.log_limit_mib * 1024 * 1024,
            "disk_accounting_errors": accounting_errors,
            "disk_samples": sample_count,
            "disk_completed_samples": completed_samples,
            "disk_scan_seconds": round(sample_seconds, 6),
            "disk_scan_retries": scan_retries,
            "disk_scan_access_errors": access_errors,
            "disk_scan_access_error_witnesses": access_error_witnesses,
            "disk_scan_access_error_witnesses_omitted": access_errors - len(access_error_witnesses),
            "cleanup_errors": cleanup_errors,
            "reader_errors": reader_failed,
            "disk_enforcement": "sampled watchdog; not a filesystem quota",
        }
        with report_path.open("x", encoding="utf-8") as report:
            json.dump(result, report, indent=2)
            report.write("\n")
        print(json.dumps(result), file=sys.stderr)
        return result


if __name__ == "__main__":
    sys.exit(main())


def capture(command, *, maximum_mib=1, timeout=30, cwd=None):
    """Bound a small command's combined output and return bytes after owned cleanup."""
    import tempfile
    import shutil

    root = Path(
        tempfile.mkdtemp(prefix="htmlcut-command-", dir=os.environ.get("RUNNER_TEMP"))
    )
    try:
        result = run(
            command,
            root,
            disk_limit_mib=maximum_mib + 4,
            timeout=timeout,
            log_limit_mib=maximum_mib,
            label="capture",
            cwd=cwd,
        )
        if status(result):
            raise ValueError(f"Bounded command refused; retained evidence at {root}")
        output = (root / "capture.log").read_bytes()
    except BaseException:
        print(f"Command evidence retained at {root}", file=sys.stderr)
        raise
    else:
        shutil.rmtree(root)
        return output
