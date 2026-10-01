# HTMLCut

HTMLCut is a bounded native extraction engine over immutable HTML snapshots, with a thin CLI for files, stdin and HTTP(S) GET acquisition. It preserves explicitly requested values, fails when declared assumptions are unmet, and keeps browser execution, domain mapping and comparison policy in caller code.

HTMLCut 15.0.0 replaces the v14 CLI and Rust contracts; old commands and saved requests are rejected. See the [implementation status](docs/extraction-contract-status.md) for the conformance and release-verification record.

Install from this checkout with its pinned Rust toolchain:

```sh
cargo install --path crates/htmlcut-cli --locked
htmlcut --version
```

Use the compact index and one operation description, then extract an explicit selector:

```sh
htmlcut describe
htmlcut describe extract
htmlcut extract --file page.html --css article
htmlcut extract --file page.html --css 'article a.more' --projection attribute --attribute href --raw
htmlcut inspect --file page.html --page-size 5
```

Default output is compact JSON containing input/plan identities, complete counts and requested string values. Default selection is exactly one; zero or multiple matches fail. Request `--match all` for all matches, or `--match nth --index 1` for an explicitly positional first match. Missing nodes/attributes are errors; present empty strings are valid.

The default `dom_text` concatenates parsed text literally, including hidden, script, style and template content. `document_text` explicitly renders headings, links, image alternatives, lists, tables and preformatted text; it excludes script/style/template payloads. Neither mode guesses boilerplate or browser visibility. Plan files supply explicit exclusions, context guards and transformations.

Raw output requires one value and adds no LF. Source slicing preserves accepted UTF-8 bytes and never automatically reparses a fragment. Inner/outer HTML are parsed-DOM serialization, not original source bytes. Semantic failures emit a typed JSON error on stderr with empty stdout. Files use bounded staging and atomic publication; replacing an existing output requires `--overwrite`.

Saved runs contain separate source and plan members and replay through `htmlcut run`. Automatic URL persistence uses `--url-env NAME --save-run FILE`, storing the environment-variable name rather than its runtime value. HTTP convenience is bounded GET acquisition with strict decoding; HTMLCut does not execute JavaScript, authenticate browser sessions or crawl links. Supply a caller-rendered snapshot when the required data needs a browser.

The only supported Rust API is `htmlcut-core`: immutable snapshots, reusable compiled plans and lazy prepared documents with fresh per-execution budgets. The CLI is a binary, with no separate Rust library API. Digests identify inputs and plans; guards enforce declared literal assumptions, while domain mapping and meaning comparison remain caller-owned.

Distribution uses GitHub native/source archives. Rust callers use the repository or extracted
source as a git/path dependency; Cargo registry publication is disabled for the owned package chain.

See [Getting Started](docs/getting-started.md), [CLI](docs/cli.md), [Core](docs/core.md), [Schemas](docs/schema.md), [Documentation Index](docs/README.md), [Quality Gates](docs/quality-gates.md), [Changelog](changelog.md), [License](LICENSE), [Notice](NOTICE), and [Patents](PATENTS.md).
