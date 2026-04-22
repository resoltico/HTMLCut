---
afad: "3.5"
version: "4.3.0"
domain: QUALITY
updated: "2026-04-22"
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

Rust `1.95.0` is the pinned HTMLCut repository toolchain. Nightly is installed alongside it for
the coverage gate and for live `cargo-fuzz` campaigns because `cargo +nightly llvm-cov --branch`
and `cargo +nightly fuzz ...` both need nightly. The workspace manifest mirrors that compiler
contract through `[workspace.package] rust-version`.

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
- recursive Markdown docs-contract lint for the maintained public docs tree, including required AFAD metadata fields, version drift, ISO-date formatting, required retrieval `keywords` and `questions`, broken local links, stale canonical schema-name or operation-ID references, completeness drift in the maintained schema/operation inventory docs, release-target and release-asset drift against the canonical shell registry, and non-parsing concrete `htmlcut ...` examples inside fenced code blocks
- targeted contract-lint tests that fail when rendered help text, operation examples, parser enums, catalog/schema summaries, or representative recovery errors drift away from the canonical core-owned registries
- clap-surface contract-lint that parses the real CLI command tree and fails if command names or applied default values drift away from the canonical core-owned CLI contract
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
- `cargo fmt --check --manifest-path fuzz/Cargo.toml`
- `cargo clippy --manifest-path fuzz/Cargo.toml --bins --locked -- -D warnings`
- direct dependency freshness across the workspace manifests
- direct dependency freshness across `fuzz/Cargo.toml`
- RustSec advisory auditing with warnings denied
- RustSec advisory auditing with warnings denied for `fuzz/Cargo.lock`
- dependency policy checks through `cargo deny` with the repository's configured advisory, yanked, unmaintained, ban, license, and source rules
- semver regression checks for `htmlcut-core` against the checked-in baseline
- compile-smoke of the checked-in libFuzzer targets through `cargo check --manifest-path fuzz/Cargo.toml --bins --locked`
- workspace tests through `cargo nextest`
- workspace doc tests
- compiler-enforced `missing_docs` coverage for the public `htmlcut-core`, `htmlcut-cli`, and `xtask` library surfaces
- distribution-profile CLI build-and-launch smoke
- 100% executable-line coverage and 100% branch coverage across the maintained Rust sources, including the `xtask` library, from a clean `cargo llvm-cov` workspace with duplicate branch spans deduplicated before scoring
- CLI/core parity checks through a matrix-driven integration suite that compares CLI JSON reports with direct `htmlcut-core` results

Before any of those gate steps begin, `cargo xtask check` preflights the exact repository
toolchain declared in `rust-toolchain.toml`. If the pinned compiler itself is missing, if its
required `clippy`/`rustfmt` components are absent, or if those binaries are still not runnable
despite rustup claiming the components are installed, the gate stops immediately with the exact
`rustup` repair command instead of failing later inside `cargo clippy` or `cargo fmt`.

The coverage command fails before any coverage build starts if the nightly toolchain or
`llvm-tools-preview` component is missing. Once the preflight passes, it starts from a clean
`cargo llvm-cov` workspace, deduplicates duplicate branch spans emitted by Rust lowering, and then
enforces the 100% line and branch bar across the maintained executable module set. That bar is
intentional: HTMLCut treats those tracked files as contract-critical logic, not aspirational best
effort. The tracked set is derived from the live `htmlcut-core`, `htmlcut-cli`, and `xtask`
source roots, with an explicit exclusion list only for declarative-only modules and internal
test-only source trees. That keeps the gate aligned automatically when the maintained CLI/core
seams split into new executable modules.

The gate also treats the heaviest scratch directories as disposable:

- coverage work under `target/llvm-cov-target` is cleaned again after scoring completes
- semver scratch under `target/semver-checks` is pruned before and after the semver step

Persistent `target/` growth should therefore come mostly from normal developer build caches rather
than stale gate-specific scratch trees.

Short live libFuzzer smoke is intentionally a separate maintainer step rather than part of
`cargo xtask check`:

```bash
cargo xtask fuzz-smoke
```

That workflow stages each checked-in seed corpus into temporary scratch before launching
`cargo +nightly fuzz run ...`, which keeps the repository-owned fuzz corpora stable after local
smoke runs. It also preflights nightly plus `cargo-fuzz` before launching so missing fuzz tooling
fails early with one actionable message. Use `--target <name>` to focus one maintained target or
`--runs <count>` to adjust the libFuzzer iteration budget.

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

GitHub CI also runs a release-target smoke matrix across the public standalone targets, unpacking
the built release packages and executing the packaged binaries before the aggregate required check
reports success.
