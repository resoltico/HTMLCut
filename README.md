# HTMLCut

Strict extraction from saved or rendered UTF-8 HTML, with a reusable Rust core.
HTMLCut checks declared selectors, counts and fields and returns complete values or
source-free structured errors. It does not fetch pages or execute JavaScript.

```sh
htmlcut extract --file page.html --select h1
htmlcut inspect --file page.html
htmlcut extract --file books.html --select article.product_pod --all --field title 'h3 a' attr:title --field price .price_color text
```

A JSON plan supplies richer bounds and assumptions:

```json
{"version":7,"select":"article","match":"all","fields":{"title":{"select":"h2"},"summary":{"select":".summary","match":"optional"}}}
```

Run `htmlcut extract --file page.html --plan query.json`. Output is a bare JSON array
plus one LF. `--raw` writes exactly one scalar without LF; `--output FILE` atomically
creates a file, and `--overwrite` explicitly permits replacement. Queries can read
text, literal text, Markdown, parsed HTML, attributes and URLs with an explicit base.
Logical resource limits do not establish process CPU/RSS isolation. A selector can
meet every guard and still identify the wrong business entity.

From a clean checkout install Rust through rustup and a native linker (Xcode Command
Line Tools on macOS, a C compiler on Linux, Visual Studio C++ tools on Windows), then:

```sh
cargo build --release --locked -p htmlcut-cli
cargo install --path crates/htmlcut-cli --locked
```

Rust is pinned in `rust-toolchain.toml`. Source installation is supported; crates.io
installation is pending upstream pointer-safety proof and registry publication.
Native packages for Apple ARM/Intel, Linux x64 musl and Windows x64 MSVC are listed on
[GitHub Releases](https://github.com/resoltico/HTMLCut/releases). This working source
prepares 21.0.0 with query/semantics 7; it does not establish release availability.
Receipts, bundles, replay and identity APIs are retired without compatibility layers.

See the [contract reference](docs/core.md), [CLI](docs/cli.md),
[ordinary-files reproduction](docs/operations.md), and [contributor checks](CONTRIBUTING.md).
