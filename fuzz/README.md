# HTMLCut Fuzz Inventory

The checked-in fuzz package lives in [`fuzz/Cargo.toml`](/Users/erst/Tools/HTMLCut/fuzz/Cargo.toml).
It is intentionally separate from the main workspace so the normal maintainer flow stays stable-first,
while fuzzing can use the tooling that best suits libFuzzer.

## Targets

- `parse_document_bytes`: feeds arbitrary decoded byte streams through the public document parse and source inspection surfaces.
- `selector_parsing`: builds selector extraction requests from arbitrary HTML, selectors, value modes, and selection policies.
- `slice_boundaries`: drives literal and regex slice extraction with arbitrary boundaries, inclusion flags, and output modes.
- `extraction_request_building`: exercises the frozen `htmlcut_core::interop::v1` plan builder and executor with arbitrary selector and delimiter strategies.

## Run

Install the fuzz driver once:

```bash
cargo install cargo-fuzz --locked
```

Run one target:

```bash
cargo fuzz run --manifest-path fuzz/Cargo.toml selector_parsing
```

Build every target without starting a fuzzing campaign:

```bash
cargo check --manifest-path fuzz/Cargo.toml --bins --locked
```
