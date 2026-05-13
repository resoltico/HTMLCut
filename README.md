[![HTMLCut Art](images/HTMLCut.png)](https://github.com/resoltico/HTMLCut)

[![Release](https://img.shields.io/github/v/release/resoltico/HTMLCut?label=release)](https://github.com/resoltico/HTMLCut/releases)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Linux%20%7C%20Windows-lightgrey.svg)](docs/platform-support.md)

# HTMLCut — repeatable HTML extraction from files, URLs, and stdin

HTMLCut extracts a specific value or fragment from an HTML file, a web page, or stdin.
Use a CSS selector when the content is in the parsed document, or use literal and regex boundaries
when you need to cut raw source text.

You can save an extraction definition as a request file and rerun it later without restating the
selector, slice boundaries, or output settings.

- Extract text, links, attributes, HTML fragments, or structured match data
- Cut raw source text between literal strings or regex boundaries
- Preview a source or an extraction before committing to final output
- Save reusable request files and replay them unchanged
- Write outputs or forensic bundles to disk

[Download releases](https://github.com/resoltico/HTMLCut/releases) ·
[Getting started](docs/getting-started.md) ·
[Command guide](docs/cli.md)

## Save and Reuse an Extraction

```bash
htmlcut select ./page.html \
  --css 'article a.more' \
  --value attribute \
  --attribute href \
  --emit-request-file ./article-link.request.json \
  --overwrite

htmlcut select --request-file ./article-link.request.json
```

The first command writes a reusable extraction definition. The second command reruns that saved
definition, so you get the same selector and output settings without repeating the inline flags.

## Documentation Index

The complete index of Markdown documentation under `docs/` lives in [docs/README.md](docs/README.md).

- [Getting Started](docs/getting-started.md)
- [CLI Developer Guide](docs/cli.md)
- [Core Developer Guide](docs/core.md)
- [Release Protocol Overview](docs/release-protocol.md)

## Legal

HTMLCut is released under the [MIT License](LICENSE). See [NOTICE](NOTICE) and
[PATENTS](PATENTS.md) for the remaining legal files.
