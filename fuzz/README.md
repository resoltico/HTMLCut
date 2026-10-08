# Fuzz harnesses

Harnesses cover retained parser/query/selector/Markdown/record and CLI surfaces.
Compile with `cargo check --locked -p htmlcut-fuzz --all-targets`.
Authored adversarial regression/resource tests and the bounded parser and relational
selector smokes are mandatory. The required Linux CI job runs both existing targets
for at most 1,000 iterations each, with finite input, execution time and memory bounds.
Full GA fuzzing and mutation campaigns are outside this change's verification scope.

Run the same smoke locally:

```sh
source scripts/contributor-rust-tools.sh
cargo install cargo-fuzz --version "$(htmlcut_contributor_cargo_tool_inventory | awk '$1 == "cargo-fuzz" {print $2}')" --locked
rustup toolchain install "$HTMLCUT_CONTRIBUTOR_RUST_NIGHTLY_TOOLCHAIN" --component rust-src --profile minimal
./scripts/focused-smoke.sh
```

The script copies checked-in corpora for `parse_document_bytes` and
`relational_selector_budget` to an external temporary directory. An optional first
argument selects an output directory outside the checkout. It prints that location
and preserves logs and crash inputs there. CI uploads these artifacts even if a smoke
fails, and failed, cancelled, missing or skipped smoke jobs fail the required `Check`.

These bounded runs supplement direct tests; they do not establish absence of bugs or
OS isolation. Retain crash inputs and diagnostics when a run fails, turn discoveries
into regression fixtures and fixes, then rerun the affected smoke.
