# HTMLCut

HTMLCut is a CLI for extracting and inspecting HTML from files, URLs, and stdin.

It supports:

- CSS selector extraction
- literal and regex slicing
- text, HTML, outer-HTML, attribute, and structured outputs
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
htmlcut select <INPUT> --css <SELECTOR> [options]
htmlcut slice <INPUT> --from <PATTERN> --to <PATTERN> [options]
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

## Notes

- `select` and `slice` separate extraction value with `--value` from stdout rendering with `--output`.
- `inspect` defaults to JSON.
- `--output none` is valid only with `--bundle`.
- `slice` works on raw source text, not parsed HTML nodes.
- `catalog` is the machine-readable capability surface.
- `schema` exports the validator-grade JSON contracts behind maintained public JSON outputs.

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
