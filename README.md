# HTMLCut

HTMLCut provides functionality to extract and inspect HTML from files, URLs, and stdin with CSS selectors, literal or regex slicing, and structured reports.

It supports:

- CSS selector extraction
- literal and regex slicing
- text, `inner-html`, `outer-html`, attribute, and structured outputs
- source and extraction inspection before final extraction
- machine-readable `catalog` and `schema` discovery for automation and agents

## Install

Build from source:

```bash
rustup toolchain install stable --profile minimal
cargo build -p htmlcut-cli --bin htmlcut
./target/debug/htmlcut --help
```

Install into Cargo's bin directory:

```bash
cargo install --path crates/htmlcut-cli --locked
htmlcut --help
```

## Command Surface

```bash
htmlcut catalog [--output text|json] [--operation <ID>]
htmlcut schema [--output text|json] [--name <SCHEMA_NAME>] [--schema-version <SCHEMA_VERSION>]
htmlcut select [INPUT] --css <SELECTOR> [options]
htmlcut slice [INPUT] --from <PATTERN> --to <PATTERN> [options]
htmlcut inspect <source|select|slice> ...
```

`<INPUT>` may be:

- a local file path
- an `http://` or `https://` URL
- `-` for stdin

## Quick Start

Extract readable text from the first article:

```bash
htmlcut select ./page.html --css article
```

Require exactly one match:

```bash
htmlcut select ./page.html --css article --match single
```

Extract the matched node as inner HTML:

```bash
htmlcut select ./page.html --css article --value inner-html
```

Extract every card as outer HTML:

```bash
htmlcut select ./page.html --css '.card' --match all --value outer-html
```

Rewrite a relative link against a base URL:

```bash
htmlcut select ./page.html \
  --css 'article a.more' \
  --value attribute \
  --attribute href \
  --rewrite-urls \
  --base-url https://example.com/docs/start.html
```

Slice raw source between literal boundaries:

```bash
htmlcut slice ./page.html --from '<article>' --to '</article>'
```

Slice raw source between regex boundaries:

```bash
htmlcut slice ./page.html \
  --from 'START::' \
  --to '::END' \
  --pattern regex \
  --match all \
  --output json
```

Inspect a source before choosing selectors:

```bash
htmlcut inspect source ./page.html --output text
```

Preview selector matches before final extraction:

```bash
htmlcut inspect select ./page.html --css '.card' --match all
```

Preview slice matches before final extraction:

```bash
htmlcut inspect slice ./page.html --from '<article>' --to '</article>'
```

Run a reusable extraction definition from JSON:

```bash
htmlcut select --request-file ./article-links.json
```

Write only the stdout payload to one file:

```bash
htmlcut select ./page.html \
  --css article \
  --output-file ./article.txt
```

## Notes

- `select` and `slice` separate extraction value with `--value` from stdout rendering with `--output`.
- `select`, `slice`, `inspect select`, and `inspect slice` can load a first-class JSON definition through `--request-file`; inline source and strategy flags then become mutually exclusive with that file.
- `--value inner-html` returns the selected fragment; `--value outer-html` returns the full matched outer range.
- `inspect` defaults to JSON.
- `--output none` is valid only with `--bundle`.
- `--output-file` writes exactly the stdout payload to one file without creating a bundle directory.
- URL inputs use HEAD-first preflight by default to reject obvious non-HTML or oversize resources earlier; HTMLCut now falls back to GET when a server explicitly rejects HEAD or breaks the HEAD exchange, and `--fetch-preflight get-only` remains available for servers that still mishandle HEAD badly.
- `--quiet` suppresses non-fatal stderr diagnostics on successful runs.
- `--version` prints the tool version plus engine identity, schema profile, and repository metadata for bug reports.
- `slice` works on raw source text, not parsed HTML nodes.
- `catalog` is the machine-readable capability surface.
- `schema` exports the validator-grade JSON contracts behind maintained public JSON outputs.

## Embedding And Interop

Embed HTMLCut in Rust through `htmlcut_core::interop::v1`, the frozen `htmlcut-v1` downstream
interop profile. The maintained contract docs live in [docs/interop-v1.md](docs/interop-v1.md)
and [docs/schema.md](docs/schema.md).

For embeddable core callers, the stable high-level API stays at the crate root while detailed
request/result contract types are grouped under `htmlcut_core::request` and
`htmlcut_core::result`. Reusable request files are modeled by
`htmlcut_core::ExtractionDefinition`; see
[`crates/htmlcut-core/examples/reusable_extraction_definition.rs`](crates/htmlcut-core/examples/reusable_extraction_definition.rs).

## Release Assets

The release workflow publishes source archives plus standalone binaries and checksum files for:

- `htmlcut-X.Y.Z.zip`
- `htmlcut-X.Y.Z.tar.gz`
- `htmlcut-aarch64-apple-darwin`
- `htmlcut-aarch64-apple-darwin.sha256`
- `htmlcut-x86_64-apple-darwin`
- `htmlcut-x86_64-apple-darwin.sha256`
- `htmlcut-x86_64-unknown-linux-musl`
- `htmlcut-x86_64-unknown-linux-musl.sha256`
- `htmlcut-x86_64-pc-windows-msvc.exe`
- `htmlcut-x86_64-pc-windows-msvc.exe.sha256`

## Developer And Maintainer Docs

All developer-facing and maintainer-facing documentation lives under [docs/](docs/README.md).
The checked-in fuzz target inventory lives in [fuzz/README.md](fuzz/README.md).
Contributor workflow lives in [CONTRIBUTING.md](CONTRIBUTING.md), and maintainer contract policy
lives in [docs/versioning-policy.md](docs/versioning-policy.md).

---

## Legal

HTMLCut is MIT-licensed. The compiled binary includes Rust crates under MIT,
Apache-2.0, MPL-2.0 (cssparser, selectors, servo_arc — Servo project), ISC
(ring and cryptographic dependencies), Unicode-3.0 (ICU data crates), and
CDLA-Permissive-2.0 (webpki-root-certs CA data). See [NOTICE](NOTICE) for
attribution details and [PATENTS.md](PATENTS.md) for patent considerations.

[LICENSE](LICENSE) | [NOTICE](NOTICE) | [PATENTS.md](PATENTS.md)
