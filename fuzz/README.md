# Optional fuzz harnesses

Harnesses cover retained parser/query/selector/Markdown/record and CLI surfaces.
Compile with `cargo check --locked -p htmlcut-fuzz --all-targets`.
Authored adversarial regression/resource tests remain mandatory. Full GA fuzzing
and mutation campaigns are outside this change's verification scope.

Focused smoke can test a specific parser or selector boundary when needed. Install
cargo-fuzz 0.13.2, use the nightly toolchain named in
`scripts/contributor-rust-tools.sh`, and copy the target corpus outside the checkout
so a run does not modify checked-in seeds. For example:

```sh
source scripts/contributor-rust-tools.sh
cargo +"$HTMLCUT_CONTRIBUTOR_RUST_NIGHTLY_TOOLCHAIN" fuzz run --features fuzzing parse_document_bytes /path/to/copied-corpus -- -runs=1000 -max_len=4096 -timeout=10 -seed=1008
```

The corresponding relational target is `relational_selector_budget`. These bounded
runs supplement direct tests; they do not establish absence of bugs or OS isolation.
Retain crash inputs and diagnostics when a run fails.
