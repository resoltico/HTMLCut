<!--
AFAD:
  afad: "4.0"
  version: "17.0.0"
  domain: QUALITY
  updated: "2026-10-03"
RETRIEVAL_HINTS:
  keywords: [fuzz, cargo-fuzz, libfuzzer, seed corpus, selector parsing, slice boundaries, closed plans, prepared snapshots]
  questions: [which fuzz targets does HTMLCut keep?, how do I run the checked-in fuzz targets?, where are the seed corpora?]
  related: [../docs/quality-gates.md, ../docs/developer-setup.md, ../README.md]
-->

# HTMLCut Fuzz Inventory

The checked-in fuzz package lives in [`Cargo.toml`](Cargo.toml) and is a normal member of the main
workspace. That keeps the maintained libFuzzer targets on the shared lockfile, lint policy, and
dependency floor while still letting live fuzzing use nightly through `cargo-fuzz`.

## Targets

- `parse_document_bytes`: accepts bounded valid UTF-8 byte streams, prepares immutable snapshots and checks repeated DOM execution determinism.
- `selector_parsing`: compiles CSS extraction plans from bounded HTML, selectors, projections and selection policies.
- `slice_boundaries`: drives literal and regex slice extraction with arbitrary boundaries, inclusion flags, and source-only projection.
- `extraction_request_building`: reads the closed extraction-plan JSON, verifies canonical round-trip equality and executes valid CSS/slice plans on a prepared snapshot.
- `selector_inspection`: exercises bounded document preparation, inspection pagination, stale/malformed cursor rejection and same-snapshot suggestions with arbitrary text and resource limits.
- `relational_selector_budget`: exercises `:has(...)` work exhaustion and fallible propagation through the maintained selector and scraper forks.
- `cli_parse_error_surface`: feeds unknown arguments to source-included private CLI modules and asserts exit 2, empty stdout and a typed JSON error on stderr.

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
source "$HOME/.cargo/env"
./scripts/install-contributor-cargo-tools.sh cargo-fuzz
```

On macOS, keep the maintained `CC=clang CXX=clang++` override from
[`docs/developer-setup.md`](../docs/developer-setup.md) when installing `cargo-fuzz`.

The smoke command itself runs `cargo +nightly-2026-09-30 fuzz run --features fuzzing ...` with
`CC=clang CXX=clang++`, so keep `clang` and `clang++` available on `PATH` on any host where you
use `cargo xtask fuzz-smoke`.

Run one maintained target without mutating the checked-in seed corpus:

```bash
cargo xtask fuzz-smoke --target selector_parsing
```

Build every target without starting a fuzzing campaign:

```bash
cargo check -p htmlcut-fuzz --bins --features fuzzing --locked
```

This compile-smoke is part of the normal maintainer gate. Full fuzzing campaigns are not. The
default `cargo test -p htmlcut-fuzz --all-targets --locked` loop stays finite because the checked-
in bins only enter libFuzzer mode when the explicit `fuzzing` feature is enabled.

Run the full short local smoke inventory:

```bash
cargo xtask fuzz-smoke
```

Tune the libFuzzer iteration budget when you want a longer or shorter smoke pass:

```bash
cargo xtask fuzz-smoke --runs 500
```

`cargo xtask fuzz-smoke` preflights nightly plus `cargo-fuzz`, then stages each checked-in corpus
into a temporary directory before calling `cargo +nightly-2026-09-30 fuzz run --features fuzzing ...`, so
the checked-in seed inventory stays clean after local smoke runs.

The current inventory covers selector-scoped inspection, independent CommonMark row/cell payload decoding, direct record relationships, and closed USTAR replay parsing/receipt recomputation. HTTP framing/media parsing is not an extraction product surface. Bounded fuzz properties supplement actual native/OS and independent complete-value tests; they do not establish universal conformance or OS isolation. The release campaign runs at least10,000 executions per maintained target with staged seed corpora.
