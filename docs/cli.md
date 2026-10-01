---
afad: "4.0"
version: "16.0.0"
domain: CLI
updated: "2026-10-01"
route:
  keywords: [cli, extract, run, inspect, describe, schema, saved runs, raw output]
  questions: ["what commands does htmlcut-cli expose?", "what does htmlcut schema include?", "how do extraction and source slicing outputs work?"]
---

# CLI

The binary commands are `extract`, `run`, `inspect`, `describe`, and `schema`. No CLI Rust library is supported. Inline flags compile to the same core plan as plan files.

```sh
htmlcut describe
htmlcut describe extract
htmlcut extract --file fixture.html --css '#amount'
htmlcut extract --file fixture.html --css '#amount' --raw
htmlcut extract --file fixture.html --css 'a.item' --projection attribute --attribute href --match all
htmlcut extract --stdin --plan amount.plan.json
htmlcut schema htmlcut.extraction.plan
```

Default single selection rejects zero/multiple candidates. `--match nth --index 1` requests first explicitly; `--match all --min 0` explicitly permits an empty array. `--raw` requires one resulting value, including a present empty value, and adds no LF. Default JSON is identical for terminals and pipes. Errors go to stderr with empty stdout before publication.

Files/stdin default to strict UTF-8; `--encoding` is explicit. HTTP uses GET only, caller encoding then HTTP charset then UTF-8, bounded redirects/transfer/decompression/decoding and timeouts. Unsupported/malformed encodings and inconsistent BOMs fail. Query parameters reach the request; operational errors never echo URL values. No HTML charset/base sniffing occurs.

Automatic URL persistence requires `--url-env NAME`; saved runs store the name rather than runtime URL values. Authored public URL literals remain caller-owned. Run file paths resolve relative to the run's directory. A run is data, with no shell substitution, nested run inclusion or automatic environment serialization.

Output files use bounded staging and atomic create/replace; overwrite requires explicit consent. Input/result/evidence destinations cannot collide. Audit fields are explicit and bounded, and audit preparation failure fails the operation. Multiple files are not an OS transaction. An I/O error on a pipe may leave bytes already delivered and returns failure.

Exit classes: 0 success/help; 2 input/options/plan/schema; 3 unmet expectations; 4 resource limits; 5 acquisition/decoding/publication I/O; 6 internal invariants.

## Acquisition and delivery boundaries

File-path sources, plan files and saved runs require regular opened files; intentional streams use stdin. Nonregular paths fail promptly rather than waiting for a FIFO writer. Regular symlink targets remain supported.

HTTP consumes complete informational headers before final metadata, with finite per-hop informational/header limits. Ordinary full-source GET does not accept unsolicited partial representations. Media parameters use quoted-string and quoted-pair grammar; text inside an unrelated quoted parameter cannot become a charset declaration. Explicit encoding takes precedence; invalid explicit encoding is a configuration error.

Safe causes distinguish known option constraints, I/O roles, received HTTP status, protocol/metadata/charset/compression and typed transport categories without echoing source, URL, header, path or supplied-value text. Opaque lower-level errors remain honestly generic.

The real stream adapter acquires handles lazily. Nonempty stdout delivery cannot succeed after an invalid/read-only descriptor write. File-only output and empty raw values do not require an unused stdout. Partial bytes already delivered to a pipe cannot be rolled back; later failure remains nonzero.
