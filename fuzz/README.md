<!--
AFAD:
  afad: "3.5"
  version: "4.2.0"
  domain: QUALITY
  updated: "2026-04-20"
RETRIEVAL_HINTS:
  keywords: [fuzz, cargo-fuzz, libfuzzer, seed corpus, selector parsing, slice boundaries, interop builder]
  questions: [which fuzz targets does HTMLCut keep?, how do I run the checked-in fuzz targets?, where are the seed corpora?]
  related: [../docs/quality-gates.md, ../docs/developer-setup.md, ../README.md]
-->

# HTMLCut Fuzz Inventory

The checked-in fuzz package lives in [`Cargo.toml`](Cargo.toml). It is intentionally separate from
the main workspace so the normal maintainer flow stays stable-first, while fuzzing can use the
tooling that best suits libFuzzer.

## Targets

- `parse_document_bytes`: feeds arbitrary decoded byte streams through the public document parse and source inspection surfaces.
- `selector_parsing`: builds selector extraction requests from arbitrary HTML, selectors, value modes, and selection policies.
- `slice_boundaries`: drives literal and regex slice extraction with arbitrary boundaries, inclusion flags, and output modes.
- `extraction_request_building`: exercises the frozen `htmlcut_core::interop::v1` plan builder and executor with arbitrary selector and delimiter strategies.

## Seed Corpora

Checked-in seed corpora live under `fuzz/corpus/<target>/`.

The intent is:

- keep only a few balanced seeds per target
- cover both full-document and fragment HTML shapes
- cover both selector and delimiter workflows
- avoid huge or highly repetitive corpora that would skew local smoke runs

These seeds are not treated as a replacement for longer fuzzing campaigns. They are there to make
short reproducible smoke runs and crash repro loops start from known meaningful inputs.

By default, `cargo fuzz` keeps its own build cache under `fuzz/target/`, separate from the
workspace root `target/` tree.

## Run

Install the fuzz driver once:

```bash
cargo install cargo-fuzz --locked
```

Run one target:

```bash
cargo fuzz run --manifest-path fuzz/Cargo.toml selector_parsing fuzz/corpus/selector_parsing
```

Build every target without starting a fuzzing campaign:

```bash
cargo check --manifest-path fuzz/Cargo.toml --bins --locked
```

This compile-smoke is part of the normal maintainer gate. Full fuzzing campaigns are not.

Run a short local smoke campaign from the checked-in seeds:

```bash
cargo fuzz run --manifest-path fuzz/Cargo.toml parse_document_bytes fuzz/corpus/parse_document_bytes -- -runs=200
cargo fuzz run --manifest-path fuzz/Cargo.toml selector_parsing fuzz/corpus/selector_parsing -- -runs=200
cargo fuzz run --manifest-path fuzz/Cargo.toml slice_boundaries fuzz/corpus/slice_boundaries -- -runs=200
cargo fuzz run --manifest-path fuzz/Cargo.toml extraction_request_building fuzz/corpus/extraction_request_building -- -runs=200
```
