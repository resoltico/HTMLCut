# Contributing

Install rustup and the native linker for your OS. Cargo reads `rust-toolchain.toml`.
Python 3.11+ is needed for release checks. Install the tools listed by
`source scripts/contributor-rust-tools.sh; htmlcut_contributor_cargo_tool_inventory`
with `cargo install NAME --version VERSION --locked`.

```sh
./check.sh
```

The script runs direct formatting, clippy, semantic tests, explicitly selected large
resource acceptance, rustdoc, focused release tests and one advisory/license check.
No coverage score, mutation campaign or fuzz campaign is required. For optional full evaluation scripts install `evaluation/requirements.txt` in an isolated environment.
Missing tools produce their normal command errors; install the named pinned tool.

Retained unsafe dependency safeguards also require targeted Miri:

```sh
source scripts/contributor-rust-tools.sh
rustup toolchain install "$HTMLCUT_CONTRIBUTOR_RUST_NIGHTLY_TOOLCHAIN" --component miri --component rust-src --profile minimal
MIRIFLAGS=-Zmiri-strict-provenance cargo +"$HTMLCUT_CONTRIBUTOR_RUST_NIGHTLY_TOOLCHAIN" miri test --locked -p htmlcut-core --lib tests::selector_and_reading_contract_remain_miri_sound -- --exact
MIRIFLAGS=-Zmiri-strict-provenance cargo +"$HTMLCUT_CONTRIBUTOR_RUST_NIGHTLY_TOOLCHAIN" miri test --locked -p htmlcut-servo-arc --lib
MIRIFLAGS=-Zmiri-strict-provenance cargo +"$HTMLCUT_CONTRIBUTOR_RUST_NIGHTLY_TOOLCHAIN" miri test --locked -p htmlcut-tendril --lib subtendril
```

Check the public core against the published baseline:

```sh
source scripts/contributor-rust-tools.sh
cargo semver-checks check-release -p htmlcut-core --baseline-rev "$HTMLCUT_PUBLISHED_BASELINE_SOURCE"
```

Cargo materializes the immutable source outside the checkout; do not commit baseline
copies. Let package versions determine allowed change level; never force major to
hide a patch deletion. Native CI extracts packages and executes file/pipe/error and
Unicode controls on all four supported targets. Keep the required `Check` status.
See [release protocol](docs/release-protocol.md) for external rollout.
