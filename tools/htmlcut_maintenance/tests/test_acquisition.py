# SPDX-License-Identifier: MPL-2.0
"""External caller recipe: transport completion must precede extraction."""

import gzip
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import threading
import unittest
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from tools.htmlcut_maintenance.config import ROOT


class AcquisitionTests(unittest.TestCase):
    def test_real_transfers_and_explicit_production_binary(self):
        binary = os.environ.get("HTMLCUT_TEST_BINARY")
        if not binary:
            self.skipTest(
                "supply HTMLCUT_TEST_BINARY explicitly for acquisition integration"
            )
        self.assertTrue(
            Path(binary).is_file(), "explicit production binary is unavailable"
        )
        self.assertIsNotNone(
            shutil.which("curl"), "acquisition integration requires curl"
        )
        spec = importlib.util.spec_from_file_location(
            "acquisition", ROOT / "examples/acquire_snapshot.py"
        )
        recipe = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(recipe)

        class Handler(BaseHTTPRequestHandler):
            def log_message(self, *args):
                pass

            def do_GET(self):
                body = b"<h1>complete</h1>"
                content_type = "text/html; charset=utf-8"
                if self.path == "/redirect":
                    self.send_response(302)
                    self.send_header("Location", "/moved/page")
                    self.end_headers()
                    return
                if self.path == "/moved/page":
                    body = b'<a href="child">link</a>'
                if self.path == "/latin1":
                    body = b"<h1>Caf\xe9</h1>"
                    content_type = "text/html; charset=windows-1252"
                if self.path == "/javascript":
                    body = b'<h1>Shell</h1><script>document.body.innerHTML="Rendered"</script>'
                if self.path == "/gzip":
                    body = gzip.compress(body)
                self.send_response(
                    500
                    if self.path == "/error"
                    else 404
                    if self.path == "/not-found"
                    else 200
                )
                self.send_header("Content-Type", content_type)
                if self.path == "/gzip":
                    self.send_header("Content-Encoding", "gzip")
                self.send_header(
                    "Content-Length",
                    str(len(body) + (100 if self.path == "/truncated" else 0)),
                )
                self.end_headers()
                self.wfile.write(body)

        server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        worker = threading.Thread(target=server.serve_forever)
        worker.start()
        endpoint = f"http://127.0.0.1:{server.server_port}"
        try:
            with tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                contract = root / "contract.json"
                destination = root / "result.json"
                for route in [
                    "/error",
                    "/not-found",
                    "/truncated",
                    "/gzip",
                    "/redirect",
                    "/latin1",
                    "/javascript",
                ]:
                    with self.subTest(route=route):
                        destination.write_bytes(b"old")
                        contract.write_text(
                            '{"select":"a","read":"url:href"}'
                            if route == "/redirect"
                            else '"h1"'
                        )
                        status, report = recipe.acquire_and_extract(
                            Path(binary), endpoint + route, contract, destination
                        )
                        if route in ["/error", "/not-found", "/truncated"]:
                            self.assertNotEqual(status, 0)
                            self.assertFalse(report["extract_invoked"])
                            self.assertEqual(destination.read_bytes(), b"old")
                        else:
                            expected = {
                                "/gzip": "complete",
                                "/redirect": endpoint + "/moved/child",
                                "/latin1": "Café",
                                "/javascript": "Shell",
                            }[route]
                            self.assertEqual(status, 0, report)
                            self.assertTrue(report["extract_invoked"])
                            self.assertEqual(
                                json.loads(destination.read_bytes()), expected
                            )
                loose = subprocess.run(
                    [binary, "run", '"h1"'],
                    input=b"<h1>complete</h1>",
                    capture_output=True,
                    timeout=5,
                )
                self.assertEqual(loose.returncode, 0)
                self.assertEqual(json.loads(loose.stdout), "complete")
        finally:
            server.shutdown()
            server.server_close()
            worker.join(timeout=2)
        self.assertFalse(worker.is_alive())
