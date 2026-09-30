# HTMLCut

HTMLCut is a bounded native extraction engine over immutable HTML snapshots, with a thin CLI for files, stdin and HTTP(S) GET acquisition. It preserves explicitly requested values, fails when declared assumptions are unmet, and keeps browser execution, domain mapping and comparison policy in caller code.

The 15.0.0 implementation is in development; release readiness is tracked in [the implementation status](docs/extraction-contract-status.md). Install the checkout with its pinned toolchain:

```sh
cargo install --path crates/htmlcut-cli --locked
```

Use the compact index and one operation description, then extract an explicit selector:

```sh
htmlcut describe
htmlcut describe extract
htmlcut extract --file page.html --css article
htmlcut extract --file page.html --css 'article a.more' --projection attribute --attribute href --raw
htmlcut inspect --file page.html --page-size 5
```

Default output is compact JSON; default selection is exactly one and default projection is literal `dom_text`. `document_text` is explicit structural rendering. Raw output requires one value and adds no LF. Source slicing preserves accepted UTF-8 bytes and never automatically reparses a fragment. Missing nodes/attributes are errors; present empty strings are valid.

Saved runs contain separate source and plan members. Automatic URL persistence uses a runtime environment-variable reference, without storing its value. The core has no acquisition I/O and supports compiled plans and lazy prepared-document reuse through one Rust API.

See [Getting Started](docs/getting-started.md), [CLI](docs/cli.md), [Core](docs/core.md), [Schemas](docs/schema.md), [Documentation Index](docs/README.md), [Quality Gates](docs/quality-gates.md), [Changelog](changelog.md), [License](LICENSE), [Notice](NOTICE), and [Patents](PATENTS.md).
