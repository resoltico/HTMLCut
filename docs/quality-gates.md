---
afad: "4.0"
version: "7.0.0"
domain: QUALITY
updated: "2026-05-01"
route:
  keywords: [quality gates, cargo xtask, coverage, semver baseline, nextest, clippy, cargo deny, fuzz, devcontainer, devcontainer check]
  questions: ["what does cargo xtask check enforce?", "how do I run the HTMLCut maintainer gate?", "when should I refresh the semver baseline from a release tag?", "how do I validate the HTMLCut contributor devcontainer?", "how do I run the maintainer gate through the contributor devcontainer from the host?"]
---

# Quality Gates

HTMLCut uses `cargo xtask` as the maintainer gate surface.

Contributor workflow lives in [../CONTRIBUTING.md](../CONTRIBUTING.md). Contract-versioning policy
lives in [versioning-policy.md](versioning-policy.md).

## Toolchain

Use [developer-setup.md](developer-setup.md) as the canonical machine bootstrap guide. It owns the
exact install commands for `rustup`, the cargo QA tools, `shellcheck`, and the macOS
compiler-override safeguard for native crate builds.
Use [developer-devcontainer.md](developer-devcontainer.md) for the preferred contributor-container
workflow on Ubuntu `26.04`.

Rust `1.95.0` is the pinned HTMLCut repository toolchain. Nightly is installed alongside it for
the coverage gate and for live `cargo-fuzz` campaigns because `cargo +nightly llvm-cov --branch`
and `cargo +nightly fuzz ...` both need nightly. The workspace manifest mirrors that compiler
contract through `[workspace.package] rust-version`.

The LLVM-backed `cargo xtask coverage` and `cargo xtask fuzz-smoke` commands both launch Cargo
with `CC=clang CXX=clang++`. Keep `clang` and `clang++` available on `PATH` when you use those
maintained flows on any platform.

## Commands

Run the full maintainer gate through the repo wrapper:

```bash
./check.sh
```

The wrapper delegates to `cargo xtask check`. Running xtask directly is equivalent:

```bash
cargo xtask check
```

Run only coverage:

```bash
cargo xtask coverage
```

Validate the committed contributor devcontainer:

```bash
./scripts/validate-devcontainer.sh
```

Run the full maintainer gate through the committed contributor container from the host:

```bash
./scripts/devcontainer-check.sh
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
- the full `xtask` library test suite, including docs-contract checks, gate-plan invariants, coverage-scoring invariants, release-target/asset doc drift against `scripts/release-targets.sh`, and workspace `rust-version` manifest enforcement
- recursive Markdown docs-contract lint for the maintained public docs set except `changelog.md`, including required AFAD metadata fields, version drift, ISO-date formatting, required retrieval `keywords` and `questions`, broken local links, stale canonical schema-name or operation-ID references, completeness drift in the maintained schema/operation inventory docs, release-target and release-asset drift against the canonical shell registry, `PATENTS.md` license-family drift against `deny.toml`, and concrete fenced `htmlcut ...` examples that no longer parse or run in a fixture-backed sandbox
- targeted contract-lint tests that fail when rendered help text, operation examples, parser enums, catalog/schema summaries, or representative recovery errors drift away from the canonical registries
- clap-surface contract-lint that parses the real CLI command tree and fails if command names or applied default values drift away from the canonical `htmlcut_cli::contract` registry
- `cargo clippy -p htmlcut-core --lib --tests --locked -- -D warnings` on the published
  default-feature core surface so cfg-specific warnings cannot hide behind the all-features
  workspace build
- `htmlcut-core` lib tests with default features disabled so fetch-free embeddings stay supported and
  URL requests fail cleanly unless the `http-client` feature is explicitly enabled
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
- direct dependency freshness across the workspace manifests
- RustSec advisory auditing with warnings denied
- dependency policy checks through `cargo deny` with warnings denied across the shipped standalone release-target graphs, using the canonical `scripts/release-targets.sh` registry for the target list plus the repository's configured advisory, yanked, unmaintained, ban, license, and source rules
- semver regression checks for `htmlcut-core` against the checked-in baseline
- compile-smoke of the checked-in libFuzzer targets through `cargo check -p htmlcut-fuzz --bins --features fuzzing --locked`
- workspace library and integration tests through `cargo nextest`
- workspace doc tests, including the maintained external Rust examples in `docs/architecture.md`, `docs/core.md`, `docs/interop-v1.md`, and `docs/schema.md` through `htmlcut-core` doctest harnesses
- compiler-enforced `missing_docs` coverage for the public `htmlcut-core`, `htmlcut-cli`, and `xtask` library surfaces
- distribution-profile CLI build-and-launch smoke
- 100% executable-line coverage and 100% branch coverage across the maintained tracked executable module set for `htmlcut-core`, `htmlcut-cli`, and `xtask`, with duplicate branch spans deduplicated before scoring
- CLI/core parity checks through a matrix-driven integration suite that compares CLI JSON reports with direct `htmlcut-core` results

Before any of those gate steps begin, `cargo xtask check` preflights the exact repository
toolchain declared in `rust-toolchain.toml`. If the pinned compiler itself is missing, if its
required `clippy`/`rustfmt` components are absent, or if those binaries are still not runnable
despite rustup claiming the components are installed, the gate stops immediately with the exact
`rustup` repair command instead of failing later inside `cargo clippy` or `cargo fmt`.

The coverage command fails before any coverage build starts if the nightly toolchain or
`llvm-tools-preview` component is missing. Once the preflight passes, it starts from a clean
`cargo llvm-cov` scratch tree, runs coverage against the maintained `htmlcut-core`,
`htmlcut-cli`, and `xtask` packages directly, deduplicates duplicate branch spans emitted by Rust
lowering, and then enforces the 100% line and branch bar across the maintained executable module
set. That bar is intentional: HTMLCut treats those tracked files as contract-critical logic, not
aspirational best effort. The tracked set is derived from the maintained non-ignored worktree
inventory under the `htmlcut-core`, `htmlcut-cli`, and `xtask` source roots, with explicit
exclusions for declarative/report-model modules, thin binary entrypoints, and internal test-only
source trees. That keeps the gate aligned automatically when the maintained CLI/core seams split
into new executable modules while keeping ignored scratch files and libFuzzer binaries out of the
coverage runner.

The gate also treats the heaviest scratch directories as disposable:

- coverage work under `target/llvm-cov-target` is cleaned again after scoring completes
- semver scratch under `target/semver-checks` is pruned before and after the semver step

Persistent `target/` growth should therefore come mostly from normal developer build caches rather
than stale gate-specific scratch trees.

Default repository search also stays focused on maintained live code: `.ignore` excludes the frozen
`semver-baseline/` snapshot from normal `rg`/`fd` discovery so symbol search does not mix the live
tree with the published compatibility snapshot. Use an explicit path or `--no-ignore` when you
intentionally need to audit the baseline copy.

Short live libFuzzer smoke is intentionally a separate maintainer step rather than part of
`cargo xtask check`:

```bash
cargo xtask fuzz-smoke
```

That workflow stages each checked-in seed corpus into temporary scratch before launching
`cargo +nightly fuzz run --features fuzzing ...`, which keeps the repository-owned fuzz corpora
stable after local smoke runs while still building the real libFuzzer harnesses explicitly. It
also preflights nightly plus `cargo-fuzz` before launching so missing fuzz tooling fails early
with one actionable message. Use `--target <name>` to focus one maintained target or `--runs
<count>` to adjust the libFuzzer iteration budget.

The maintained public artifact path does not ship from plain Cargo `release`. HTMLCut uses a
dedicated `dist` profile that inherits `release` and then hardens it for shipped binaries with:

- `lto = "thin"`
- `codegen-units = 1`
- `strip = "symbols"`
- `panic = "abort"`

Local maintainer smoke stays host-native. The full public standalone artifact matrix is built by
the release workflow as defined in [platform-support.md](platform-support.md).

The repo-root `./check.sh` wrapper is shell-linted alongside the `scripts/` directory so the
documented maintainer entrypoint cannot silently diverge from the actual Rust gate.

Contributor-container validation is a separate companion gate on purpose. It builds the committed
Ubuntu `26.04` contributor image, validates the committed devcontainer JSON contract, repairs and
bootstraps the named Rust/Cargo cache volumes, proves the repo commands start from inside that raw
container surface, and then proves a real devcontainer client can materialize the committed spec
plus run `devcontainer exec`. Use `./scripts/devcontainer-check.sh` when you want the full
maintainer gate to run through the same contributor image, named volume contract, lifecycle
scripts, and repo-root `./check.sh` entrypoint from the host side. Run both host commands when you
change `.devcontainer/`, the devcontainer lifecycle scripts, or the contributor-container docs.

GitHub CI also runs a release-target smoke matrix across the public standalone targets, unpacking
the built release packages, checking that the packaged README stays package-specific, and
executing one real extraction-plus-request-replay flow before the aggregate required check reports
success.

GitHub CI runs the Linux maintainer gate through the committed contributor devcontainer, alongside
the separate cross-platform Rust jobs and the release-target smoke matrix, before the aggregate
required `Check` result reports success.
