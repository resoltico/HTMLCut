# HTMLCut

HTMLCut validates an extraction contract against one caller-owned UTF-8 HTML
snapshot. It returns complete strings or row records, or refuses the whole query.
It does not fetch, browse, run JavaScript, infer article content or convert documents.

```sh
htmlcut extract --file snapshot.html --select h1 --raw
htmlcut inspect --file snapshot.html --select article.book --samples 2
htmlcut extract --file snapshot.html --select article.book --all --field title 'h3 a' attr:title --field price .price text --expect-text h1 Catalogue
```

For repeated work, save a plan and run `htmlcut extract --file snapshot.html --plan query.json`:

```json
{"version":8,"select":"article.book","match":"all","min":1,"max":100,"fields":{"title":{"select":"h3 a","read":"attr:title"},"price":{"select":".price"},"stock":{"select":".stock","match":"optional"}},"expect":[{"select":"h1","equals":"Catalogue"}]}
```

Output is a compact JSON array plus LF. `--raw` requires one scalar and omits LF.
`--output FILE` atomically creates a file; `--overwrite` permits replacement.
Readings are structural `text`, parsed `literal`, `attr:NAME`, `url:NAME`, and
normalized `outer-html`. Relative URLs require explicit `--base-url`.
Validate representative values and business identity before reusing a plan:
selectors and guards can all pass on the wrong entity.

The CLI isolates parsing and matching in a private worker with native memory
controls and a supervisor deadline. The Rust core provides logical admission and
result limits; in-process callers must provide their own physical isolation for
adversarial snapshots. See [resource boundaries](docs/architecture.md).

From a clean checkout install rustup and a native linker (Xcode Command Line Tools
on macOS, a C compiler on Linux, Visual Studio C++ tools on Windows), then:

```sh
cargo build --release --locked -p htmlcut-cli
cargo install --path crates/htmlcut-cli --locked
```

Rust is pinned in `rust-toolchain.toml`. Published v21 packages are available on
[GitHub Releases](https://github.com/resoltico/HTMLCut/releases/tag/v21.0.0) and crates.io.
This source prepares the incompatible next major release with plan version 8;
it does not establish publication of that candidate. Plans and consumers must be
rewritten for the removed readings, survey and sibling-forest APIs; no adapter is supplied.

Prefer strict Python/lxml for one-off mappings, browser locators for dynamic pages,
Readability/Trafilatura for unknown articles, Pandoc for conversion, and a JSON
parser/jq for structured data. HTMLCut is useful when a reusable selection contract
must refuse incomplete or structurally changed records.

See [the core contract](docs/core.md), [CLI](docs/cli.md),
[ordinary-files reproduction](docs/operations.md), [evaluation](evaluation/README.md)
and [contributor checks](CONTRIBUTING.md).
