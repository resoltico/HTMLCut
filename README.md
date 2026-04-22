<!--
AFAD:
  afad: "3.5"
  version: "4.2.1"
  domain: PRODUCT
  updated: "2026-04-22"
RETRIEVAL_HINTS:
  keywords: [htmlcut, html extraction, css selector, slice extraction, extraction-definition json, catalog, schema, inspect]
  questions: [what is HTMLCut?, how do I install HTMLCut?, what commands does htmlcut expose?, how do I save a reusable extraction-definition file?]
  related: [docs/cli.md, docs/core.md, docs/schema.md, docs/interop-v1.md, docs/quality-gates.md, CONTRIBUTING.md, fuzz/README.md, PATENTS.md]
-->

# HTMLCut

HTMLCut extracts and inspects HTML from files, URLs, and stdin with CSS selectors, literal or
regex slicing, and structured JSON reports.

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
source "$HOME/.cargo/env"
cargo build --locked -p htmlcut-cli --bin htmlcut
./target/debug/htmlcut --help
```

Install into Cargo's bin directory:

```bash
source "$HOME/.cargo/env"
cargo install --path crates/htmlcut-cli --locked
htmlcut --help
```

Install a prebuilt release package on macOS or Linux:

```bash
VERSION=X.Y.Z
TARGET=aarch64-apple-darwin # or x86_64-apple-darwin / x86_64-unknown-linux-musl
curl -LO "https://github.com/resoltico/HTMLCut/releases/download/v${VERSION}/htmlcut-${VERSION}-${TARGET}.tar.gz"
curl -LO "https://github.com/resoltico/HTMLCut/releases/download/v${VERSION}/htmlcut-${VERSION}-checksums.txt"
grep "  htmlcut-${VERSION}-${TARGET}.tar.gz$" "htmlcut-${VERSION}-checksums.txt" | shasum -a 256 -c
tar -xzf "htmlcut-${VERSION}-${TARGET}.tar.gz"
install "htmlcut-${VERSION}-${TARGET}/htmlcut" "$HOME/.local/bin/htmlcut"
htmlcut --help
```

Install a prebuilt release package on Windows PowerShell:

```powershell
$Version = "X.Y.Z"
$Target = "x86_64-pc-windows-msvc"
Invoke-WebRequest "https://github.com/resoltico/HTMLCut/releases/download/v$Version/htmlcut-$Version-$Target.zip" -OutFile "htmlcut-$Version-$Target.zip"
Invoke-WebRequest "https://github.com/resoltico/HTMLCut/releases/download/v$Version/htmlcut-$Version-checksums.txt" -OutFile "htmlcut-$Version-checksums.txt"
$Expected = ((Select-String -Path "htmlcut-$Version-checksums.txt" -Pattern "  htmlcut-$Version-$Target\.zip$").Line -replace ' .*', '').ToLowerInvariant()
$Actual = (Get-FileHash "htmlcut-$Version-$Target.zip" -Algorithm SHA256).Hash.ToLowerInvariant()
if ($Actual -ne $Expected) { throw "checksum mismatch" }
Expand-Archive "htmlcut-$Version-$Target.zip" -DestinationPath .
New-Item -ItemType Directory -Force "$HOME\bin" | Out-Null
Copy-Item "htmlcut-$Version-$Target\htmlcut*" "$HOME\bin"
$env:Path = "$HOME\bin;$env:Path"
htmlcut --help
```

Each prebuilt release package contains the platform binary plus `README.md`, `LICENSE`, `NOTICE`,
and `PATENTS.md`.

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

HTMLCut has one canonical command surface:

- `catalog`
- `schema`
- `inspect`
- `select`
- `slice`

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

Run a reusable extraction-definition JSON file:

```bash
htmlcut select --request-file ./article-links.json
```

Save the normalized extraction-definition JSON while you prototype inline flags:

```bash
htmlcut select ./page.html \
  --css 'article a.more' \
  --value attribute \
  --attribute href \
  --emit-request-file ./article-links.json
```

Write only the stdout payload to one file:

```bash
htmlcut select ./page.html \
  --css article \
  --output-file ./article.txt
```

## Notes

- `select` and `slice` separate extraction value with `--value` from stdout rendering with `--output`.
- `select` and `slice` default stdout to `text`, except `--value structured`, which defaults stdout to `json`.
- `select`, `slice`, `inspect select`, and `inspect slice` can load a first-class extraction-definition JSON file through `--request-file`; inline source and strategy flags then become mutually exclusive with that file.
- `--emit-request-file` writes the normalized extraction-definition JSON for `select`, `slice`, `inspect select`, and `inspect slice`, so inline discovery can be promoted into a reusable file without hand-authoring JSON.
- `--value inner-html` returns the selected fragment; `--value outer-html` returns the full matched outer range.
- `--output html` is valid only with `--value inner-html` or `--value outer-html`.
- `inspect` defaults to JSON.
- `catalog --output text` now renders every operation in detail; `--output json` remains the machine-readable discovery surface.
- `catalog --output text` and `schema --output text` now start with a short registry summary plus the exact follow-up command to inspect one entry in JSON.
- `catalog` and `schema` also accept `--output-file`, and verbose runs confirm that write on stderr.
- `--output none` is valid only with `--bundle`.
- `--output-file` writes exactly the stdout payload to one file without creating a bundle directory.
- verbose `catalog`, `schema`, extraction, and inspection runs now confirm successful `--output-file` and `--emit-request-file` writes on stderr.
- URL inputs use HEAD-first preflight by default to reject obvious non-HTML or oversize resources earlier; HTMLCut now falls back to GET when a server explicitly rejects HEAD or breaks the HEAD exchange, and `--fetch-preflight get-only` remains available for servers that still mishandle HEAD badly.
- successful URL inspections and extractions now expose the source-load trace in verbose output, including when a HEAD preflight fell back to GET.
- failed URL inspections and extractions now keep that source-load trace too, so human error output and structured reports show the attempted HEAD/GET path that failed.
- `inspect slice --output text` now shows the exact matched start and end boundary text alongside the selected ranges and fragment preview.
- slice previews and extractions now warn with `SLICE_SPLITS_MARKUP` when the selected range appears to start or end inside HTML markup, which makes the classic literal `<a` versus `<article>` footgun much more obvious.
- invalid extraction-definition files now point directly back to `htmlcut schema --name htmlcut.extraction_definition --output json` and the matching `htmlcut catalog --operation <id> --output json` entry for recovery.
- extraction-definition shape failures now include the failing JSON path, and unknown operation/schema lookups now suggest the closest registered names or available schema versions.
- HTMLCut creates parent directories automatically for `--bundle`, `--output-file`, and `--emit-request-file`.
- `--quiet` suppresses non-fatal stderr diagnostics on successful runs.
- `--version` prints the tool version plus engine identity, schema profile, and repository metadata for bug reports.
- `slice` works on raw source text, not parsed HTML nodes.
- `catalog` is the capability discovery surface.
- `schema` exports the validator-grade JSON contracts behind maintained public JSON outputs.
- `catalog`, `schema`, the rendered CLI help/catalog/error-recovery surfaces, and the concrete fenced `htmlcut ...` examples in the maintained docs are linted against the same core-owned operation, command-contract, help-document, diagnostic-code, and schema registries. They are intended to stay in lockstep, not merely tell a similar story.

## Embedding And Interop

Embed HTMLCut in Rust through `htmlcut-core`.

- Use the crate-root execution functions plus the `htmlcut_core::request` and
  `htmlcut_core::result` namespaces for generic in-process extraction and inspection.
- Use `htmlcut_core::interop::v1` only when you need the frozen `htmlcut-v1` downstream
  integration profile.

The maintained contract docs live in [docs/core.md](docs/core.md),
[docs/interop-v1.md](docs/interop-v1.md), and [docs/schema.md](docs/schema.md).

For embeddable core callers, the stable high-level API stays at the crate root while detailed
request/result contract types are grouped under `htmlcut_core::request` and
`htmlcut_core::result`. Reusable extraction-definition files are modeled by
`htmlcut_core::ExtractionDefinition`; see
[`crates/htmlcut-core/examples/reusable_extraction_definition.rs`](crates/htmlcut-core/examples/reusable_extraction_definition.rs).

## Release Assets

The maintained release workflow publishes versioned source archives, versioned standalone packages,
and one checksum manifest:

- `htmlcut-source-X.Y.Z.zip`
- `htmlcut-source-X.Y.Z.tar.gz`
- `htmlcut-X.Y.Z-aarch64-apple-darwin.tar.gz`
- `htmlcut-X.Y.Z-x86_64-apple-darwin.tar.gz`
- `htmlcut-X.Y.Z-x86_64-unknown-linux-musl.tar.gz`
- `htmlcut-X.Y.Z-x86_64-pc-windows-msvc.zip`
- `htmlcut-X.Y.Z-checksums.txt`

The `htmlcut-source-...` assets are release-owned source snapshots. They are intentionally named as
source archives to distinguish them from runnable binary packages.

GitHub will also render `Source code (zip)` and `Source code (tar.gz)` links on the release page.
Those links are GitHub-generated convenience downloads, not part of HTMLCut's maintained release
asset inventory.

## Developer And Maintainer Docs

All developer-facing and maintainer-facing documentation lives under [docs/](docs/README.md).
The checked-in fuzz target inventory lives in [fuzz/README.md](fuzz/README.md).
Contributor workflow lives in [CONTRIBUTING.md](CONTRIBUTING.md), and maintainer contract policy
lives in [docs/versioning-policy.md](docs/versioning-policy.md).

Run the full local maintainer gate with `./check.sh` or `cargo xtask check`.

---

## Legal

HTMLCut itself is MIT-licensed.

Third-party dependency attribution lives in [NOTICE](NOTICE). Allowed dependency-license families
are enforced by `deny.toml`, and the patent-posture summary for those license families lives in
[PATENTS.md](PATENTS.md).

[LICENSE](LICENSE) | [NOTICE](NOTICE) | [PATENTS.md](PATENTS.md)
