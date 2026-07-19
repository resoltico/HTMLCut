# HTMLCut — repeatable HTML extraction from files, URLs, and stdin

HTMLCut extracts a specific value or fragment from an HTML file, a web page, or stdin.
Use a CSS selector when the content is in the parsed document, or use literal and regex boundaries
when you need to cut raw source text.

You can save an extraction definition as a request file and rerun it later without restating the
selector, slice boundaries, or output settings.

- Extract semantic rendered text, direct DOM descendant text, links, attributes, HTML fragments, or structured match data
- Cut raw source text between literal strings or regex boundaries
- Preview a source or an extraction before committing to final output
- Save reusable request files and replay them unchanged
- Write outputs or forensic bundles to disk

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

## Embed Deterministic Extraction

Rust applications can use `htmlcut_core::interop::v1` to prepare typed extraction plans, execute them against application-owned HTML, and persist deterministic result and error documents. Use `text` when HTML-aware rendered structure is part of the value; use CSS-only `plain_text` when the value is the selected element's direct descendant text after its declared whitespace policy. See the [Interop v1 Guide](docs/interop-v1.md) for the contract boundary and integration API.

## Documentation Index

The complete index of Markdown documentation under `docs/` lives in [docs/README.md](docs/README.md).

- [Getting Started](docs/getting-started.md)
- [CLI Developer Guide](docs/cli.md)
- [Core Developer Guide](docs/core.md)
- [Release Protocol Overview](docs/release-protocol.md)

## Legal

HTMLCut is released under the [MIT License](LICENSE). See [NOTICE](NOTICE) and
[PATENTS](PATENTS.md) for the remaining legal files.
