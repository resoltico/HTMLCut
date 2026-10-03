"""Checked, randomized process measurements; elapsed time and peak RSS stay separate."""
import platform
import random
import re
import statistics
import subprocess
import time
from pathlib import Path


def peak_rss_bytes(text, system):
    patterns = {"Darwin": r"(?m)^\s*(\d+)\s+maximum resident set size$",
                "Linux": r"Maximum resident set size \(kbytes\):\s*(\d+)"}
    if system not in patterns:
        raise ValueError("Unsupported peak-RSS units")
    matches = re.findall(patterns[system], text)
    if len(matches) != 1:
        raise ValueError("Missing/ambiguous peak RSS")
    return int(matches[0]) * (1024 if system == "Linux" else 1)


def paired(commands, expected, decode, *, data=None, warmups=3, repeats=15, rss_directory=None):
    samples = {name: [] for name in commands}
    outputs = {}
    order = random.Random(1703)
    for trial in range(warmups + repeats):
        names = list(commands)
        order.shuffle(names)
        for name in names:
            start = time.perf_counter_ns()
            result = subprocess.run(commands[name], input=data, capture_output=True, timeout=60, check=True)
            elapsed = time.perf_counter_ns() - start
            if result.stderr:
                raise ValueError(f"Unexpected successful-command diagnostics for {name}: {result.stderr!r}")
            if decode(name, result.stdout) != expected:
                raise ValueError(f"Incomplete or incorrect result from {name}")
            outputs[name] = result.stdout
            if trial >= warmups:
                samples[name].append(elapsed)
    result = {name: dict(samples_ns=values, median_ns=statistics.median(values), warmups=warmups,
                         repeats=repeats, scope="fresh process; includes startup and parent capture")
              for name, values in samples.items()}
    if rss_directory is not None:
        directory = Path(rss_directory)
        directory.mkdir(parents=True, exist_ok=True)
        system = platform.system()
        flags = {"Darwin": ["-l"], "Linux": ["-v"]}.get(system)
        if flags is None:
            raise ValueError("Peak-RSS measurement requires the maintained macOS or Linux time command")
        rss = {name: [] for name in commands}
        for trial in range(warmups + repeats):
            names = list(commands)
            order.shuffle(names)
            for name in names:
                path = directory / f"{trial:02d}-{name}.txt"
                command = ["/usr/bin/time", *flags, "-o", str(path), *commands[name]]
                process = subprocess.run(command, input=data, capture_output=True, timeout=60, check=True)
                if process.stderr or decode(name, process.stdout) != expected:
                    raise ValueError(f"RSS launch did not return the complete correct result: {name}")
                text = path.read_text()
                value = peak_rss_bytes(text, system)
                if trial >= warmups:
                    rss[name].append(value)
        for name, values in rss.items():
            result[name]["rss"] = dict(samples_bytes=values, median_bytes=statistics.median(values),
                                       scope="separate checked launches under /usr/bin/time", raw_directory=str(directory))
    return result, outputs


def warm(operation, expected, *, warmups=3, repeats=100):
    for _ in range(warmups):
        if operation() != expected:
            raise ValueError("Incorrect warmup result")
    samples = []
    for _ in range(repeats):
        start = time.perf_counter_ns()
        value = operation()
        samples.append(time.perf_counter_ns() - start)
        if value != expected:
            raise ValueError("Incorrect warm result")
    return dict(samples_ns=samples, median_ns=statistics.median(samples), warmups=warmups, repeats=repeats)
