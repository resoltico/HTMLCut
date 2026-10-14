# Contributing

Start at the repository root with rustup and a native linker; [platform support](docs/platform-support.md) lists prerequisites. [rust-toolchain.toml](rust-toolchain.toml) pins development, while [Cargo.toml](Cargo.toml) declares the minimum tested compiler. Cargo.lock pins dependency resolution. Build output stays in `target/`; `cargo clean` removes it, and a different location is set with `CARGO_TARGET_DIR`, not in repository configuration.

Install the versions of cargo-deny, cargo-about, cargo-fuzz and cargo-mutants declared in [contributor-rust-tools.sh](scripts/contributor-rust-tools.sh) using Cargo's install command with --version and --locked. Install the script's pinned nightly for fuzz instrumentation. Python 3.11 or newer is required for release maintenance tests.

```sh
./check.sh
```

The gate formats, lints with warnings denied, runs the library and CLI corpus and resource tests, builds rustdoc with warnings denied, runs release-maintenance tests and checks dependency policy. CI also tests the minimum compiler and all native targets. The ignored positional performance test runs in the required native package smoke with `HTMLCUT_TEST_BINARY` pointing at the unpacked shipped executable; unit and integration tests do not build a second binary. See [the frozen contract](docs/design.md#stability).

The two fuzz targets read contract bytes and document bytes paired with a contract. [focused-smoke.sh](scripts/focused-smoke.sh) builds both harnesses once in disposable owned scratch and runs them concurrently, with finite disk, log and time budgets. PR CI runs `--runs 250 --require-mutations`: all seeds must initialize and the subsequent mutation loop must execute. This is a short preflight, not equivalent coverage to the separately authorized 10,000-iteration maintainer/release smoke (the local default). `--runs 100` verifies seed/harness plumbing and may perform no mutations. See [fuzz/README.md](fuzz/README.md) for resource allowances and evidence ownership.

Conformance expectations are independent specification inputs. Report a questionable case and reasoning before editing it. Preserve the append-only contract corpus within a major version. Keep default values and limits authoritative in code and derive or check dependent help/schema documentation.

Before a release, fetch the locked dependencies with `cargo fetch --locked`, then run `python3 scripts/mutation-audit.py --output /path/outside/checkout/new-audit`. The runner uses the pinned cargo-mutants inventory, disposable build roots, a shared CPU jobserver and finite time/log budgets; it retains results and removes only its own rebuildable scratch. Builds run offline and do not copy existing targets or use shared Cargo build overrides. See [QA resource limits](docs/qa-resources.md) for disk accounting, focused runs and incomplete-review handling. Report every survivor and timeout, with the reason it changes nothing observable or its unresolved defect, in the release pull request.
