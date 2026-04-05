# Quality Gates

HTMLCut uses `cargo xtask` as the maintainer gate surface.

## Toolchain

Install the local maintainer toolchain:

```bash
rustup toolchain install stable --profile minimal
rustup toolchain install nightly --profile minimal --component llvm-tools-preview
cargo install cargo-nextest cargo-audit cargo-deny cargo-semver-checks cargo-outdated cargo-llvm-cov --locked
```

Stable remains the default HTMLCut toolchain. Nightly is installed alongside it only for the
coverage gate because `cargo +nightly llvm-cov --branch` is currently required for true branch
coverage.

Install `shellcheck` from your system package manager, for example:

```bash
brew install shellcheck
```

## Commands

Run the full maintainer gate:

```bash
cargo xtask check
```

Run only coverage:

```bash
cargo xtask coverage
```

Refresh the semver baseline only after a release has landed and future semver checks should compare
against that released surface:

```bash
cargo xtask refresh-semver-baseline
```

## What `cargo xtask check` Enforces

- shell script syntax and `shellcheck`
- `cargo fmt --check`
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
- direct workspace dependency freshness
- RustSec advisory auditing with warnings denied
- dependency policy checks for advisories, bans, licenses, registries, yanked crates, and unmaintained crates
- semver regression checks for `htmlcut-core` against the checked-in baseline
- workspace tests through `cargo nextest`
- workspace doc tests
- compiler-enforced rustdoc coverage for the public `htmlcut-core`, `htmlcut-cli`, and `xtask` surfaces
- distribution-profile CLI build-and-launch smoke
- 100% executable-line coverage and 100% branch coverage across the maintained Rust sources, including the `xtask` library, from a clean `cargo llvm-cov` workspace with duplicate branch spans deduplicated before scoring
- CLI/core parity checks through a matrix-driven integration suite that compares CLI JSON reports with direct `htmlcut-core` results

The coverage command starts from a clean `cargo llvm-cov` workspace and deduplicates duplicate
branch spans emitted by Rust lowering before enforcing the 100% line and branch bar.

The maintained public artifact path does not ship from plain Cargo `release`. HTMLCut uses a
dedicated `dist` profile that inherits `release` and then hardens it for shipped binaries with:

- `lto = "thin"`
- `codegen-units = 1`
- `strip = "symbols"`
- `panic = "abort"`

Local maintainer smoke stays host-native. The full public standalone artifact matrix is built by
the release workflow as defined in [platform-support.md](platform-support.md).

GitHub CI also runs a release-target smoke matrix across the public standalone targets before the
aggregate required check reports success.
