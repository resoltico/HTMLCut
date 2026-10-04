

# HTMLCut

HTMLCut executes declared extraction contracts over immutable UTF-8 HTML snapshots. Its Rust core and native CLI return requested values or records, reject unmet assumptions, and bound preparation, matching and output work.

This workspace uses extraction wire version 4 and semantics 4. Only the current contract is supported: old APIs, flags and formats are rejected. The [implementation status](docs/extraction-contract-status.md) identifies verification and publication authorities.

Download and verify an exact native package using [Getting Started](docs/getting-started.md), then run:

```sh
htmlcut describe
htmlcut describe extract
htmlcut extract --file page.html --css article --read markdown --raw
htmlcut extract --file page.html --css 'article a' --read attribute:href --match all
htmlcut inspect --file page.html --css article
```

Default stdout is compact JSON containing only an array of requested strings or records, followed by one LF. Default selection requires exactly one node; all and positive one-based nth selection are explicit. Missing attributes fail; present empty strings are values. `--raw` emits exactly one flat string without an extra LF.

Repeated `--field NAME CSS READ` declares named single-valued fields. `--following-siblings N` adds exactly N element siblings to each record scope. Plan files provide optional or all-valued fields, original-DOM guards, exclusions and explicit transforms. One records execution preserves field relationships without serializing and reparsing row HTML. Numeric conversion, filtering, business meaning and comparison remain caller code.

Literal `dom_text` includes hidden/script/style/template text without invented separators. `markdown` is a declared CommonMark reading convention: normalized prose, block boundaries, links/images, literal source ordinals in bullets, tables as nested row/cell lists, and protected code. It excludes script/style/template payloads and does not infer visibility or boilerplate. Inner/outer HTML serialize the parsed DOM. Source slicing alone preserves exact accepted UTF-8 bytes and never reparses them implicitly.

Files and intentional stdin must be UTF-8. Callers own network acquisition, charset conversion, browser rendering and authentication; supply a completed snapshot. HTMLCut neither executes JavaScript nor fetches or crawls URLs. Base metadata is explicit.

`--receipt FILE` writes execution identities, complete counts and integrity evidence separately. `--bundle FILE` saves a deterministic, self-contained uncompressed archive containing accepted source bytes, normalized plan and replay configuration/evidence. `htmlcut run FILE` recomputes and verifies it without accessing original paths. Receipts establish execution/integrity, not source authenticity or delivery. Output artifacts use bounded staging and atomic single-file publication; replacement requires `--overwrite`. Several files/stdout are not a transaction.

The only supported Rust API is `htmlcut-core`: immutable snapshots, reusable compiled plans, lazy prepared documents and fresh shared execution budgets. The CLI is a binary with no library API. Logical bounds do not promise OS CPU/RAM isolation or sanitization.

Distribution uses GitHub native/source archives. Rust consumers use git/path dependencies; owned package registry publication is disabled. See [Getting Started](docs/getting-started.md), [CLI](docs/cli.md), [Core](docs/core.md), [Schemas](docs/schema.md), [Documentation Index](docs/README.md), [Quality Gates](docs/quality-gates.md), [Changelog](changelog.md), [License](LICENSE), [Notice](NOTICE), and [Patents](PATENTS.md).

Original HTMLCut source from v18 is licensed under MPL-2.0. Vendored dependencies and earlier published material retain their terms; see [License](LICENSE), [Notice](NOTICE) and [Patent Notes](PATENTS.md).
