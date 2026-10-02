#!/usr/bin/env python3
"""Independent complete-value conformance cases against an actual candidate executable."""
import argparse
import hashlib
import json
from pathlib import Path
import socket
import subprocess
import tempfile
import threading
import time


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    binary = args.binary.resolve()
    before = hashlib.sha256(binary.read_bytes()).hexdigest()
    rows = []

    def run(label, arguments, data=None, values=None, raw=None, failure=None, cause=None):
        start = time.monotonic()
        expected = 0 if failure is None else failure
        try:
            result = subprocess.run([str(binary), *arguments], input=data, capture_output=True, timeout=20)
        except subprocess.TimeoutExpired as error:
            rows.append({"case": label, "arguments": arguments, "expected_exit": expected,
                         "actual_exit": None, "classification": "process_timeout", "passed": False,
                         "stdout": (error.stdout or b"").decode("utf-8", errors="replace"),
                         "stderr": (error.stderr or b"").decode("utf-8", errors="replace"),
                         "seconds": time.monotonic() - start})
            return
        passed = result.returncode == expected
        parsed = None
        try:
            if failure is not None:
                parsed = json.loads(result.stderr)
                passed = passed and not result.stdout and parsed["version"] == 2 and bool(parsed["code"])
                if cause:
                    passed = passed and parsed["cause"]["kind"] == cause
                passed = passed and b"SYNTHETIC_SECRET" not in result.stderr
            elif raw is not None:
                passed = passed and result.stdout == raw and not result.stderr
            else:
                parsed = json.loads(result.stdout)
                passed = passed and parsed["version"] == 2 and parsed["values"] == values and not result.stderr
        except (ValueError, KeyError):
            passed = False
        rows.append({"case": label, "arguments": arguments, "input_sha256": hashlib.sha256(data).hexdigest() if data is not None else None,
                     "expected_exit": expected, "actual_exit": result.returncode, "expected_values": values,
                     "expected_raw_hex": raw.hex() if raw is not None else None, "expected_cause": cause,
                     "stdout": result.stdout.decode("utf-8", errors="replace"), "stderr": result.stderr.decode("utf-8", errors="replace"),
                     "seconds": time.monotonic() - start, "passed": passed})

    with tempfile.TemporaryDirectory(prefix="htmlcut-native-matrix-") as temporary:
        root = Path(temporary)
        for label, html, css, projection, expected in [
            ("literal-hidden-template", "<p>A<b hidden>B</b><template>C</template></p>", "p", "dom_text", "ABC"),
            ("unicode", "<p>Āžu € 日本 😀</p>", "p", "dom_text", "Āžu € 日本 😀"),
            ("selected-row", "<table><tr><td>Taxi</td><td>180</td></tr></table>", "tr", "document_text", "[cell]Taxi[/cell] | [cell]180[/cell]"),
            ("literal-cell-delimiters", "<table><tr><td>A | [cell]</td></tr></table>", "tr", "document_text", "[cell]A \\| \\[cell\\][/cell]"),
            ("pre-fragment", "<pre>OUT<code id='target'>x\n</code>AFTER</pre>", "#target", "document_text", "```\nx\n\n```"),
            ("ordered-fragment", "<ol start='7'><li>A</li><li id='target'>B</li></ol>", "#target", "document_text", "8. B"),
            ("reversed-fragment", "<ol reversed><li>A</li><li id='target'>B</li><li>C</li></ol>", "#target", "document_text", "2. B"),
            ("definition-blocks", "<dl><dt>Name</dt><dd>Alice</dd><dt>Amount</dt><dd>180</dd></dl>", "dl", "document_text", "Name\nAlice\nAmount\n180"),
            ("details-blocks", "<details><summary>Title</summary><div>A</div><div>B</div></details>", "details", "document_text", "Title\nA\nB"),
            ("foreign-table-names", "<svg><caption>Title</caption><tr><td>A</td><td>B</td></tr></svg>", "svg", "document_text", "TitleAB"),
        ]:
            run(label, ["extract", "--stdin", "--css", css, "--projection", projection], html.encode(), values=[expected])
        run("raw-exact", ["extract", "--stdin", "--start", "A", "--end", "B", "--raw"], "Aé\r\nB".encode(), raw="é\r\n".encode())
        for label, arguments, data, code in [
            ("ambiguous", ["extract", "--stdin", "--css", "p"], b"<p>A</p><p>B</p>", 3),
            ("missing", ["extract", "--stdin", "--css", "p"], b"<b>A</b>", 3),
            ("invalid-css", ["extract", "--stdin", "--css", "["], b"<p>A</p>", 2),
            ("invalid-bytes", ["extract", "--stdin", "--css", "p"], b"<p>\xff</p>", 5),
            ("unknown-option-redacted", ["--SYNTHETIC_SECRET"], None, 2),
        ]:
            run(label, arguments, data, failure=code)
        plan = {"schema": "htmlcut.extraction.plan", "version": 2, "strategy": {"kind": "css", "selector": "p"}, "selection": {"kind": "single"}, "projection": {"kind": "dom_text"}}
        plan_path = root / "plan.json"
        plan_path.write_text(json.dumps(plan))
        run("saved-plan-valid", ["extract", "--stdin", "--plan", str(plan_path)], b"<p>A</p>", values=["A"])
        for label, field in [("single-unknown", "selection"), ("projection-unknown", "projection")]:
            malformed = json.loads(json.dumps(plan))
            malformed[field]["unexpected_field"] = "SYNTHETIC_SECRET"
            plan_path.write_text(json.dumps(malformed))
            run(label, ["extract", "--stdin", "--plan", str(plan_path)], b"<p>A</p>", failure=2)
        old = dict(plan, version=1)
        plan_path.write_text(json.dumps(old))
        run("old-wire-refused", ["extract", "--stdin", "--plan", str(plan_path)], b"<p>A</p>", failure=2)
        run_path = root / "run.json"
        saved = {"schema": "htmlcut.run", "version": 2, "source": {"kind": "stdin"}, "plan": plan}
        run_path.write_text(json.dumps(saved))
        run("replay-valid", ["run", str(run_path)], b"<p>A</p>", values=["A"])
        saved["source"]["path"] = "SYNTHETIC_SECRET"
        run_path.write_text(json.dumps(saved))
        run("stdin-unknown-path", ["run", str(run_path)], b"<p>A</p>", failure=2)
        source = root / "Āžu 日本.html"
        source.write_text("<p>File €</p>", encoding="utf-8")
        run("unicode-file", ["extract", "--file", str(source), "--css", "p", "--raw"], raw="File €".encode())
        destination = root / "result.txt"
        run("atomic-create", ["extract", "--file", str(source), "--css", "p", "--raw", "--output", str(destination)], raw=b"")
        if destination.read_bytes() != "File €".encode():
            rows[-1]["passed"] = False
        run("no-clobber", ["extract", "--file", str(source), "--css", "p", "--output", str(destination)], failure=5)

        body = "<p>Café €</p>".encode()
        final = b"HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: " + str(len(body)).encode() + b"\r\nConnection: close\r\n\r\n" + body
        wires = [
            ("informational-headers", b"HTTP/1.1 100 Continue\r\nX-Note: yes\r\n\r\nHTTP/1.1 103 Early Hints\r\nLink: </style>\r\n\r\n" + final, None, None),
            ("quoted-fake-charset", final.replace(b"charset=utf-8", b'note="x;charset=windows-1252"; charset=utf-8'), None, None),
            ("informational-count-limit", b"HTTP/1.1 100 Continue\r\n\r\n" * 33 + final, 4, None),
            ("partial-206", final.replace(b"200 OK", b"206 Partial Content"), 5, "partial_response"),
            ("partial-metadata", final.replace(b"Content-Type:", b"Content-Range: bytes 0-7/200\r\nContent-Type:"), 5, "partial_response"),
            ("truncated-body", b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\nConnection: close\r\n\r\n<p>A</p>", 5, "transport"),
        ]
        for label, wire, failure, cause in wires:
            listener = socket.socket()
            listener.bind(("127.0.0.1", 0))
            listener.listen(1)
            listener.settimeout(5)
            port = listener.getsockname()[1]
            errors = []
            def serve():
                try:
                    with listener, listener.accept()[0] as connection:
                        connection.settimeout(3)
                        request = b""
                        while b"\r\n\r\n" not in request:
                            block = connection.recv(4096)
                            if not block or len(request) + len(block) > 16384:
                                raise ValueError("Incomplete or excessive fixture request")
                            request += block
                        connection.sendall(wire)
                except Exception as error:
                    errors.append(type(error).__name__)
            worker = threading.Thread(target=serve)
            worker.start()
            run(label, ["extract", "--url", f"http://127.0.0.1:{port}/fixture?token=SYNTHETIC_SECRET", "--css", "p"], values=["Café €"] if failure is None else None, failure=failure, cause=cause)
            worker.join(timeout=6)
            if worker.is_alive() or errors:
                rows[-1]["passed"] = False
                rows[-1]["fixture_errors"] = errors or ["worker-timeout"]
    unchanged = hashlib.sha256(binary.read_bytes()).hexdigest() == before
    evidence = {"schema": "htmlcut.native-reliability-matrix", "version": 2, "binary_sha256": before, "binary_unchanged": unchanged, "rows": rows, "passed": unchanged and all(row["passed"] for row in rows)}
    args.output.write_text(json.dumps(evidence, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"cases": len(rows), "passed": sum(row["passed"] for row in rows), "binary_unchanged": unchanged}))
    if not evidence["passed"]:
        raise SystemExit("Native conformance assertions failed; inspect retained matrix")


if __name__ == "__main__":
    main()
