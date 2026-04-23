<!--
AFAD:
  afad: "3.5"
  version: "4.4.0"
  domain: PRODUCT
  updated: "2026-04-23"
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

Build from source with the repo-pinned Rust `1.95.0` toolchain:

```bash
rustup toolchain install 1.95.0 --profile minimal
source "$HOME/.cargo/env"
cargo build --locked -p htmlcut-cli --bin htmlcut
./target/debug/htmlcut --help
```

`rust-toolchain.toml` is the canonical exact repo toolchain pin, and `Cargo.toml`
`[workspace.package] rust-version` mirrors the published compiler requirement.

Install into Cargo's bin directory:

```bash
source "$HOME/.cargo/env"
cargo install --path crates/htmlcut-cli --locked
htmlcut --help
```

Install a prebuilt standalone release package on macOS or Linux:

```bash
VERSION=4.4.0
TARGET=aarch64-apple-darwin # or x86_64-apple-darwin / x86_64-unknown-linux-musl
curl -fsSLO "https://github.com/resoltico/HTMLCut/releases/download/v${VERSION}/htmlcut-${VERSION}-${TARGET}.tar.gz"
curl -fsSLO "https://github.com/resoltico/HTMLCut/releases/download/v${VERSION}/htmlcut-${VERSION}-checksums.txt"
EXPECTED="$(grep "  htmlcut-${VERSION}-${TARGET}.tar.gz$" "htmlcut-${VERSION}-checksums.txt" | awk '{print $1}')"
if command -v sha256sum >/dev/null 2>&1; then
  ACTUAL="$(sha256sum "htmlcut-${VERSION}-${TARGET}.tar.gz" | awk '{print $1}')"
else
  ACTUAL="$(shasum -a 256 "htmlcut-${VERSION}-${TARGET}.tar.gz" | awk '{print $1}')"
fi
if [ "$ACTUAL" != "$EXPECTED" ]; then
  printf 'checksum mismatch for %s\n' "htmlcut-${VERSION}-${TARGET}.tar.gz" >&2
  exit 1
fi
tar -xzf "htmlcut-${VERSION}-${TARGET}.tar.gz"
mkdir -p "$HOME/.local/bin"
install "htmlcut-${VERSION}-${TARGET}/htmlcut" "$HOME/.local/bin/htmlcut"
export PATH="$HOME/.local/bin:$PATH"
htmlcut --help
```

Install a prebuilt standalone release package on Windows PowerShell:

```powershell
$Version = "4.4.0"
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

```text
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

If you want a reproducible local sample on a POSIX shell, create the demo page used by the
commands below:

```bash
cat > ./page.html <<'HTML'
<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <title>HTMLCut README Fixture</title>
</head>
<body>
  <main>
    <article>
      <h1>Guide</h1>
      <div class="card">Card alpha</div>
      <div class="card">Card beta</div>
      <p><a class="more" href="../guide.html">Read more</a></p>
      <pre>START::Regex slice payload::END</pre>
    </article>
  </main>
</body>
</html>
HTML
```

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

Save the normalized extraction-definition JSON while you prototype inline flags:

```bash
htmlcut select ./page.html \
  --css 'article a.more' \
  --value attribute \
  --attribute href \
  --emit-request-file ./article-links.json
```

Run the saved reusable extraction-definition JSON file:

```bash
htmlcut select --request-file ./article-links.json
```

Write only the stdout payload to one file:

```bash
htmlcut select ./page.html \
  --css article \
  --output-file ./article.txt
```

## Behavior Notes

Output and artifacts:

- `select` and `slice` separate extraction value (`--value`) from stdout rendering (`--output`).
- `text` and `attribute` values default to text output, `inner-html` and `outer-html` default to HTML output, and `structured` defaults to JSON output.
- `--value inner-html` returns the selected fragment; `--value outer-html` returns the full matched outer range.
- `--output html` is valid only with `--value inner-html` or `--value outer-html`.
- `--output none` is valid only with `--bundle`.
- `--output-file` writes exactly the stdout payload to one file without creating a bundle directory.
- `--bundle` writes `selection.html`, `selection.txt`, and `report.json`.
- HTMLCut creates parent directories automatically for `--bundle`, `--output-file`, and `--emit-request-file`.

Discovery and reusable definitions:

- `catalog` is the capability-discovery surface, and `schema` exports the validator-grade JSON contracts behind maintained public JSON outputs.
- `catalog --output text` and `schema --output text` begin with a short registry summary and an exact follow-up JSON command for deeper inspection.
- `catalog` and `schema` accept `--output-file`, and verbose runs confirm successful file writes on stderr.
- `select`, `slice`, `inspect select`, and `inspect slice` accept extraction-definition JSON files through `--request-file`; inline source and strategy flags then become mutually exclusive with that file.
- `--emit-request-file` writes the normalized extraction-definition JSON for `select`, `slice`, `inspect select`, and `inspect slice`, so inline discovery can be promoted into a reusable JSON file without hand-authoring the contract.
- Invalid extraction-definition files point back to `htmlcut schema --name htmlcut.extraction_definition --output json` and the matching `htmlcut catalog --operation <id> --output json` entry for recovery.
- Extraction-definition shape failures include the failing JSON path, and unknown operation/schema lookups suggest the closest registered names or available schema versions.

Runtime behavior and diagnostics:

- `inspect` defaults to JSON.
- `--max-bytes` accepts raw bytes or KB/MB/GB values only when they resolve to a whole positive byte count after unit scaling.
- URL inputs use HEAD-first preflight by default to reject obvious non-HTML or oversize resources earlier, with automatic fallback to GET when a server rejects or breaks HEAD. `--fetch-preflight get-only` skips the HEAD probe.
- Successful and failed URL-backed inspections and extractions preserve the source-load trace, including HEAD-to-GET fallback details, in verbose output and structured reports.
- `inspect slice --output text` shows the exact matched start and end boundary text alongside the selected ranges and fragment preview.
- Slice previews and extractions warn with `SLICE_SPLITS_MARKUP` when the selected range appears to start or end inside HTML markup.
- `slice` works on raw source text, not parsed HTML nodes.
- `--quiet` suppresses non-fatal stderr diagnostics on successful runs.
- `--version` prints the tool version plus engine identity, schema profile, and repository metadata for bug reports.
- `catalog`, `schema`, rendered help, recovery guidance, and the maintained concrete `htmlcut ...` examples are linted against the same core-owned operation, command-contract, help-document, diagnostic-code, and schema registries.
- The root README quick-start, request-file, and output-file CLI flows are also exercised by integration tests so these concrete examples stay runnable instead of only parsing.

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
Run a short non-mutating libFuzzer smoke with `cargo xtask fuzz-smoke`.
The command preflights nightly plus `cargo-fuzz` before it launches and stages each checked-in
seed corpus into temporary scratch so local smoke runs do not mutate repository-owned corpora.

---

## Legal

HTMLCut itself is MIT-licensed.

Third-party dependency attribution lives in [NOTICE](NOTICE). Allowed dependency-license families
are enforced by `deny.toml`, and the patent-posture summary for those license families lives in
[PATENTS.md](PATENTS.md).

[LICENSE](LICENSE) | [NOTICE](NOTICE) | [PATENTS.md](PATENTS.md)
