# Contributing

Start at the repository root with rustup and a native linker; [platform support](docs/platform-support.md) lists prerequisites. [rust-toolchain.toml](rust-toolchain.toml) pins development, while [Cargo.toml](Cargo.toml) declares the minimum tested compiler. Cargo.lock pins dependency resolution. Build output stays in `target/`; `cargo clean` removes it, and a different location is set with `CARGO_TARGET_DIR`, not in repository configuration.

Install the versions of cargo-deny, cargo-about and cargo-fuzz declared in [contributor-rust-tools.sh](scripts/contributor-rust-tools.sh) using Cargo's install command with --version and --locked. Install the script's pinned nightly for fuzz instrumentation. Python 3.11 or newer is required for release maintenance tests.

```sh
./check.sh
```

The gate formats, lints with warnings denied, runs the library and CLI corpus and resource tests, builds rustdoc with warnings denied, runs release-maintenance tests and checks dependency policy. CI also tests the minimum compiler and all native targets. The ignored positional performance test runs in the required native package smoke with `HTMLCUT_TEST_BINARY` pointing at the unpacked shipped executable; unit and integration tests do not build a second binary. See [the frozen contract](docs/design.md#stability).

The two fuzz targets read contract bytes and document bytes paired with a contract. [focused-smoke.sh](scripts/focused-smoke.sh) generates seeds from conformance and retained document seeds and keeps mutable corpora and crash evidence outside the checkout. Each target must finish all 10,000 iterations.

Conformance expectations are independent specification inputs. Report a questionable case and reasoning before editing it. Preserve the append-only contract corpus within a major version. Keep default values and limits authoritative in code and derive or check dependent help/schema documentation.

Install the release audit tool with `cargo install cargo-mutants --version 27.1.0 --locked`, then run `cargo mutants --workspace` before a release. Each copy builds in its own checkout; do not share `CARGO_TARGET_DIR` or Cargo build-directory overrides between copies. Report every survivor and the reason it changes nothing observable in the release pull request.
