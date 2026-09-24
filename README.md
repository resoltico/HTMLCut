# HTMLCut

HTMLCut extracts a chosen part of an HTML document from a file, an HTTP(S) URL, explicit stdin, or HTML supplied on the command line. Use CSS selectors for parsed elements and their values, or source boundaries when the bytes between two markers matter. Inspect matches before extracting, then save the extraction as a request file for later runs.

HTMLCut works on the HTML it receives or fetches. It does not run page JavaScript or read a browser's live DOM.

## Get started

Download a published package from [GitHub Releases](https://github.com/resoltico/HTMLCut/releases), or install the binary from this checkout with the repository's pinned Rust toolchain:

```bash
cargo install --path crates/htmlcut-cli --locked
```

Try an extraction without creating a file:

```bash
htmlcut select --input-html '<article><h1>Guide</h1><p>Hello from HTMLCut.</p></article>' --css article
```

The default value is rendered text, so the result is:

```text
# Guide

Hello from HTMLCut.
```

For installation options and a longer walkthrough, see [Getting Started](docs/getting-started.md).

## Find, preview, extract

Create a page to use with the commands below:

```bash
cat > page.html <<'HTML'
<article>
  <h1>Field guide</h1>
  <p>Read the <a class="more" href="/guide">guide</a>.</p>
</article>
HTML
```

Inspect the source, preview a selector, and extract the link destination:

```bash
htmlcut inspect source page.html
htmlcut inspect select page.html --css 'article a.more' --match single
htmlcut select page.html --css 'article a.more' --match single --value attribute --attribute href
```

The final command prints `/guide`. `--match single` makes an absent or ambiguous match an error. When a selector cannot express the desired source region, `slice` selects between literal or regex boundaries in the raw HTML; its output can then be rendered as text or returned as HTML.

| Task | Command |
| --- | --- |
| Examine a document and likely content roots | `htmlcut inspect source` |
| Preview CSS or source-boundary matches | `htmlcut inspect select`, `htmlcut inspect slice` |
| Extract a value from parsed elements | `htmlcut select` |
| Extract between source boundaries | `htmlcut slice` |
| Explore elements or resolve a target from a saved HTML snapshot | `htmlcut inspect elements`, `htmlcut inspect propose` |
| Discover operations and JSON schemas | `htmlcut catalog`, `htmlcut schema` |

Extraction commands also accept `https://` URLs, `-` for stdin, and `--input-html` for inline HTML. Choose text, HTML, or JSON output as supported by the value mode; use `--output-file` to save a payload or `--bundle` to save the result with diagnostic files. See the [CLI guide](docs/cli.md) for the exact modes, URL handling, output rules, and limits.

## Save an extraction

Once the selector and output are right, write a request file and run it again without repeating the options:

```bash
htmlcut select page.html --css 'article a.more' --match single --value attribute --attribute href --emit-request-file guide.request.json
htmlcut select --request-file guide.request.json
```

Both commands print `/guide`. The request stores the source and extraction settings, so rerunning it reads the source again; a changed page can produce a changed result. Existing request files require `--overwrite` to replace them.

## Use HTMLCut from Rust

The `htmlcut-core` crate exposes extraction primitives and the versioned `htmlcut_core::interop::v2` API for downstream Rust applications. In the interop API, the application supplies decoded HTML, compiles a typed plan, prepares a document snapshot, and executes one or more plans against it. The same snapshot supports bounded element exploration and target resolution. The application remains responsible for fetching and for keeping a browser target aligned with the supplied HTML. See the [Interop v2 guide](docs/interop-v2.md) for the API and its contracts, or the [Core guide](docs/core.md) for lower-level entry points.

## Project references

- [Documentation index](docs/README.md)
- [Quality gates](docs/quality-gates.md)
- [Changelog](changelog.md)
- [MIT License](LICENSE), [Notice](NOTICE), and [Patents](PATENTS.md)
