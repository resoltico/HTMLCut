# Bounded fuzzing

Use the nightly (with `rust-src`) and cargo-fuzz versions pinned in the top-level environment of [CI](../.github/workflows/ci.yml). From the repository root, with `FUZZ_NIGHTLY` set to that nightly and `FUZZ_TARGET` set to the installed nightly compiler's host triple (`rustc +"$FUZZ_NIGHTLY" -vV`):

```sh
cargo +"$FUZZ_NIGHTLY" fuzz run --target "$FUZZ_TARGET" contract_bytes fuzz/corpus/contract_bytes -- -runs=2000 -max_len=65536 -rss_limit_mb=2048 -timeout=10
cargo +"$FUZZ_NIGHTLY" fuzz run --target "$FUZZ_TARGET" extract fuzz/corpus/extract -- -runs=2000 -max_len=65536 -rss_limit_mb=2048 -timeout=10
```

`contract_bytes` receives raw contract bytes. `extract` receives a mode byte, little-endian u16 contract length, contract bytes and document bytes. Bit 0 selects parser admission with `(mode >> 2) % 33` allowed nodes; bit 1 selects a small reading limit. Semantic mode compiles, parses once, extracts/exports both formats and outlines/renders. Internal errors are findings; normal contract, representation and resource refusals are allowed. Fuzzing does not compare literal conformance oracles.

Checked-in inputs contain conformance cases and parser-admission documents framed by the example generator. Content-hash names deduplicate only identical bytes; collisions are rejected. Rerun the generator when cases change (see [CONTRIBUTING](../CONTRIBUTING.md)). LibFuzzer may add discovered inputs: use copied corpus directories when verification inputs must remain stable. Minimize findings and add independently authored regression oracles.
