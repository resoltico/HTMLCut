#!/usr/bin/env python3
"""Prove packaged TLS verification using only a disposable hosted Linux trust store."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import socket
import ssl
import subprocess
import tempfile
import threading
import time


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def command(records, *args):
    run = subprocess.run(args, capture_output=True, text=True, timeout=30)
    records.append({"command": list(args), "exit_code": run.returncode,
                    "stdout": run.stdout, "stderr": run.stderr})
    run.check_returncode()


def certificates(root, records):
    (root / "newcerts").mkdir()
    (root / "index").write_text("")
    (root / "serial").write_text("1000\n")
    command(records, "openssl", "req", "-x509", "-newkey", "rsa:2048", "-nodes",
            "-subj", "/CN=HTMLCut disposable TLS proof", "-days", "2",
            "-addext", "basicConstraints=critical,CA:TRUE",
            "-keyout", str(root / "root.key"), "-out", str(root / "root.crt"))
    for name in ["valid", "wrong-host", "expired", "untrusted"]:
        command(records, "openssl", "req", "-new", "-newkey", "rsa:2048", "-nodes",
                "-subj", f"/CN={name}.fixture", "-keyout", str(root / f"{name}.key"),
                "-out", str(root / f"{name}.csr"))
        if name == "untrusted":
            extensions = root / "untrusted.cnf"
            extensions.write_text("subjectAltName=IP:127.0.0.1\nextendedKeyUsage=serverAuth\nbasicConstraints=critical,CA:FALSE\n")
            command(records, "openssl", "x509", "-req", "-in", str(root / f"{name}.csr"),
                    "-signkey", str(root / f"{name}.key"), "-days", "1",
                    "-extfile", str(extensions),
                    "-out", str(root / f"{name}.crt"))
            continue
        san = "DNS:wrong.invalid" if name == "wrong-host" else "IP:127.0.0.1"
        config = root / f"{name}.cnf"
        config.write_text(f"""[ca]
default_ca=proof
[proof]
database={root}/index
new_certs_dir={root}/newcerts
certificate={root}/root.crt
private_key={root}/root.key
serial={root}/serial
default_md=sha256
default_days=1
policy=policy
x509_extensions=leaf
[policy]
commonName=supplied
[leaf]
basicConstraints=critical,CA:FALSE
keyUsage=critical,digitalSignature,keyEncipherment
extendedKeyUsage=serverAuth
subjectAltName={san}
""")
        dates = ["-startdate", "20200101000000Z", "-enddate", "20200102000000Z"] if name == "expired" else []
        command(records, "openssl", "ca", "-batch", "-notext", "-config", str(config),
                "-in", str(root / f"{name}.csr"), "-out", str(root / f"{name}.crt"), *dates)


def exercise(binary, root, name):
    context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
    context.load_cert_chain(root / f"{name}.crt", root / f"{name}.key")
    listener = socket.socket()
    listener.bind(("127.0.0.1", 0))
    listener.listen(1)
    listener.settimeout(5)
    port = listener.getsockname()[1]
    server = []

    def serve():
        try:
            with listener, listener.accept()[0] as connection:
                connection.settimeout(4)
                with context.wrap_socket(connection, server_side=True) as stream:
                    request = b""
                    while b"\r\n\r\n" not in request:
                        data = stream.recv(4096)
                        if not data or len(request) + len(data) > 16384:
                            raise ValueError("bounded request")
                        request += data
                    body = "<p>Café €</p>".encode()
                    stream.sendall(b"HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: "
                                   + str(len(body)).encode() + b"\r\nConnection: close\r\n\r\n" + body)
                    server.append("http-served")
        except ssl.SSLError:
            server.append("peer-refused-tls")
        except Exception as error:
            server.append(type(error).__name__)

    worker = threading.Thread(target=serve, daemon=True)
    worker.start()
    begin = time.monotonic()
    expected = 0 if name == "valid" else 5
    row = {"case": name, "expected_exit": expected, "certificate_sha256": digest(root / f"{name}.crt"), "passed": False}
    try:
        run = subprocess.run([str(binary), "extract", "--url",
                              f"https://127.0.0.1:{port}/fixture?token=SYNTHETIC_SECRET",
                              "--css", "p", "--raw"], capture_output=True, timeout=20)
        worker.join(timeout=6)
        row.update(actual_exit=run.returncode, stdout=run.stdout.decode(errors="replace"),
                   stderr=run.stderr.decode(errors="replace"), server_outcome=server)
        passed = run.returncode == expected and not worker.is_alive()
        if expected == 0:
            passed = passed and run.stdout == "Café €".encode() and not run.stderr and server == ["http-served"]
        else:
            error = json.loads(run.stderr)
            passed = passed and not run.stdout and error["cause"] == {"kind": "transport", "problem": "tls"}
            passed = passed and b"SYNTHETIC_SECRET" not in run.stderr and server == ["peer-refused-tls"]
        row["passed"] = passed
    except Exception as error:
        row["harness_error"] = type(error).__name__
    finally:
        listener.close()
        worker.join(timeout=6)
        row["seconds"] = time.monotonic() - begin
    return row


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--source-sha", required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--disposable-hosted-runner", action="store_true")
    args = parser.parse_args()
    if not re.fullmatch(r"[0-9a-f]{40}", args.source_sha):
        parser.error("Expected exact source commit")
    if not (args.disposable_hosted_runner and platform.system() == "Linux"
            and os.environ.get("GITHUB_ACTIONS") == "true"
            and os.environ.get("RUNNER_ENVIRONMENT") == "github-hosted"):
        parser.error("Trust-store proof requires explicit disposable hosted Linux runner authorization")
    binary = args.binary.resolve(strict=True)
    before = digest(binary)
    proof = {"source_commit": args.source_sha, "binary_sha256": before,
             "runner_system": platform.platform(), "runner_machine": platform.machine(),
             "trust_scope": "disposable hosted Linux OS store", "commands": [], "rows": [], "passed": False}
    trust = Path("/usr/local/share/ca-certificates") / ("htmlcut-proof-" + args.source_sha + ".crt")
    if trust.exists():
        raise ValueError("Refusing to overwrite existing trust anchor")
    try:
        with tempfile.TemporaryDirectory(prefix="htmlcut-tls-") as directory:
            root = Path(directory)
            certificates(root, proof["commands"])
            proof["ca_sha256"] = digest(root / "root.crt")
            public = args.output.with_suffix(".certificates")
            public.mkdir(exist_ok=False)
            for certificate in root.glob("*.crt"):
                (public / certificate.name).write_bytes(certificate.read_bytes())
            try:
                command(proof["commands"], "sudo", "install", "-m", "0644", str(root / "root.crt"), str(trust))
                command(proof["commands"], "sudo", "update-ca-certificates")
                for name in ["valid", "wrong-host", "expired", "untrusted"]:
                    proof["rows"].append(exercise(binary, root, name))
            finally:
                command(proof["commands"], "sudo", "rm", "-f", str(trust))
                command(proof["commands"], "sudo", "update-ca-certificates", "--fresh")
            proof["passed"] = all(row["passed"] for row in proof["rows"]) and len(proof["rows"]) == 4 and digest(binary) == before
    finally:
        args.output.write_text(json.dumps(proof, ensure_ascii=False, indent=2) + "\n")
    raise SystemExit(0 if proof["passed"] else 1)


if __name__ == "__main__":
    main()
