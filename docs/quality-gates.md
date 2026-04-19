---
afad: "3.5"
version: "4.1.0"
domain: QUALITY
updated: "2026-04-19"
route:
  keywords: [quality gates, cargo xtask, coverage, semver baseline, nextest, clippy, cargo deny, fuzz]
  questions: ["what does cargo xtask check enforce?", "how do I run the HTMLCut maintainer gate?", "when should I refresh the semver baseline from a release tag?"]
---

# Quality Gates

HTMLCut uses `cargo xtask` as the maintainer gate surface.

Contributor workflow lives in [../CONTRIBUTING.md](../CONTRIBUTING.md). Contract-versioning policy
lives in [versioning-policy.md](versioning-policy.md).

## Toolchain

Use [developer-setup.md](developer-setup.md) as the canonical machine bootstrap guide. It owns the
exact install commands for `rustup`, the cargo QA tools, `shellcheck`, and the macOS
compiler-override safeguard for native crate builds.

Stable remains the default HTMLCut toolchain. Nightly is installed alongside it only for the
coverage gate because `cargo +nightly llvm-cov --branch` is currently required for true branch
coverage.

## Commands

Run the full maintainer gate through the stable repo wrapper:

```bash
./check.sh
```

The wrapper delegates to `cargo xtask check`. Running xtask directly remains valid too:

```bash
cargo xtask check
```

Run only coverage:

```bash
cargo xtask coverage
```

Refresh the semver baseline only after a release has landed and future semver checks should compare
against that released surface. Always point it at the published tag or commit, never at the live
worktree:

```bash
cargo xtask refresh-semver-baseline --git-ref vX.Y.Z
```

## What `cargo xtask check` Enforces

- shell script syntax and `shellcheck`
- `cargo fmt --check`
- targeted contract-lint tests that fail when help text, operation examples, parser enums, or catalog contracts drift away from the canonical core-owned registries
- clap-surface contract-lint that parses the real CLI command tree and fails if command names or applied default values drift away from the canonical core-owned CLI contract
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
- direct workspace dependency freshness
- RustSec advisory auditing with warnings denied
- dependency policy checks for advisories, bans, licenses, registries, yanked crates, and unmaintained crates
- semver regression checks for `htmlcut-core` against the checked-in baseline
- compile-smoke of the checked-in libFuzzer targets through `cargo check --manifest-path fuzz/Cargo.toml --bins --locked`
- workspace tests through `cargo nextest`
- workspace doc tests
- compiler-enforced rustdoc coverage for the public `htmlcut-core`, `htmlcut-cli`, and `xtask` surfaces
- distribution-profile CLI build-and-launch smoke
- 100% executable-line coverage and 100% branch coverage across the maintained Rust sources, including the `xtask` library, from a clean `cargo llvm-cov` workspace with duplicate branch spans deduplicated before scoring
- CLI/core parity checks through a matrix-driven integration suite that compares CLI JSON reports with direct `htmlcut-core` results

The coverage command now fails before any coverage build starts if the nightly toolchain or
`llvm-tools-preview` component is missing. Once the preflight passes, it starts from a clean
`cargo llvm-cov` workspace, deduplicates duplicate branch spans emitted by Rust lowering, and then
enforces the 100% line and branch bar across the curated executable module set. That bar is
intentional: HTMLCut treats those tracked files as contract-critical logic, not aspirational best
effort.

The gate also treats the heaviest scratch directories as disposable:

- coverage work under `target/llvm-cov-target` is cleaned again after scoring completes
- semver scratch under `target/semver-checks` is pruned before and after the semver step

Persistent `target/` growth should therefore come mostly from normal developer build caches rather
than stale gate-specific scratch trees.

The maintained public artifact path does not ship from plain Cargo `release`. HTMLCut uses a
dedicated `dist` profile that inherits `release` and then hardens it for shipped binaries with:

- `lto = "thin"`
- `codegen-units = 1`
- `strip = "symbols"`
- `panic = "abort"`

Local maintainer smoke stays host-native. The full public standalone artifact matrix is built by
the release workflow as defined in [platform-support.md](platform-support.md).

`./check.sh` is intentionally checked in and shell-linted alongside the `scripts/` directory so the
documented maintainer entrypoint cannot silently diverge from the actual Rust gate.

GitHub CI also runs a release-target smoke matrix across the public standalone targets before the
aggregate required check reports success.
