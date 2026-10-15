# Contributing

Start at the repository root with rustup, a native linker, Bash and Python 3.11 or newer (3.13 or newer for bounded QA on macOS); [platform support](docs/platform-support.md) lists native prerequisites. [rust-toolchain.toml](rust-toolchain.toml) pins development, [Cargo.toml](Cargo.toml) declares the minimum tested compiler, and [Cargo.lock](Cargo.lock) pins dependency resolution.

macOS unit tests also use Apple's `/usr/bin/leaks` to check native ACL allocation release. The test first requires detection of intentionally retained allocations, then requires zero leaks from ordinary drops. Detector output and stack-logging files stay in bounded owned temporary directories; missing or unusable detection fails the check.

Linux authorization tests require mounted `/proc`, temporary storage supporting POSIX ACL extended attributes, and an account without DAC permission bypass. Run them as a regular user so the directory-listing denial control is effective. These tests use native APIs and need no ACL command-line tools.

Install `cargo-deny` at the version declared in [the tool inventory](scripts/contributor-rust-tools.sh), then run the contributor gate:

```bash
source ./scripts/contributor-rust-tools.sh
cargo_deny_version=$(htmlcut_contributor_cargo_tool_inventory | awk '$1 == "cargo-deny" {print $2}')
cargo install cargo-deny --version "$cargo_deny_version" --locked
./check.sh
```

The gate checks formatting, denies lint and rustdoc warnings, runs library, CLI, conformance, resource and release-maintenance tests, checks dependency policy and enforces the [frozen contract corpus](docs/design.md#stability). CI also tests the minimum compiler and all native targets. Native package smoke tests use the unpacked executable through `HTMLCUT_TEST_BINARY` and include the ignored positional performance check.

Build output stays in `target/` unless `CARGO_TARGET_DIR` selects another location. Use `cargo clean` to remove the checkout's build output.

Conformance expectations are independent specification inputs. Report a questionable case and reasoning before editing it. Preserve the append-only contract corpus within a major version. Keep default values and limits authoritative in code and derive or check dependent help and schema documentation.

Packaging requires `cargo-about`; install its inventory version with `cargo install`, `--locked` and `--features cli`. Fuzzing requires the inventory version of `cargo-fuzz` and the pinned nightly with `rust-src`. [Fuzzing](fuzz/README.md) describes the bounded commands, seed controls and output ownership. CI runs a 250-execution preflight per target with mutation startup required; the local default is 10,000 executions per target.

Before a release, install the inventory version of `cargo-mutants` with `--locked`, fetch locked dependencies with `cargo fetch --locked`, then run:

```sh
python3 scripts/mutation-audit.py --output /path/outside/checkout/mutation-results
```

The runner uses disposable build roots, offline builds and finite resource budgets. See [QA resource limits](docs/qa-resources.md) for prerequisites, focused runs and retained results. Complete the full workspace review and report every survivor and timeout in the release pull request, explaining either the unresolved defect or why it changes nothing observable.
