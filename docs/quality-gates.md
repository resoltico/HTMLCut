---
afad: "4.0"
version: "10.1.0"
domain: QUALITY
updated: "2026-05-16"
route:
  keywords: [quality gates, cargo xtask, coverage, miri, semver baseline, nextest, clippy, cargo deny, fuzz, devcontainer, devcontainer check, hygiene]
  questions: ["what does cargo xtask check enforce?", "how do I run the HTMLCut maintainer gate?", "how do I run the HTMLCut strict-provenance selector-safety Miri proof?", "when should I refresh the semver baseline from a release tag?", "how do I validate the HTMLCut contributor devcontainer?", "how do I run the maintainer gate through the contributor devcontainer from the host?", "which command checks HTMLCut artifact hygiene?"]
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
workflow on Ubuntu `24.04`.

`rust-toolchain.toml` owns the exact HTMLCut repository toolchain pin (currently `1.95.0`).
Nightly is installed alongside it for the strict-provenance selector-safety Miri proof, the
coverage gate, and live `cargo-fuzz` campaigns because `cargo xtask miri`, `cargo +nightly
llvm-cov --branch`, and `cargo +nightly fuzz ...` all need nightly. The workspace manifest carries the
published compatibility floor separately through
`[workspace.package] rust-version = "1.95"`.

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

Run the curated cross-platform CI Rust subset locally:

```bash
cargo xtask ci-rust-gate
```

Run only coverage:

```bash
cargo xtask coverage
```

Run only the strict-provenance selector-safety Miri proof:

```bash
cargo xtask miri
```

Run only the maintained dependency-freshness gate:

```bash
cargo xtask outdated-check
```

Inspect or repair artifact hygiene:

```bash
cargo xtask hygiene report
cargo xtask hygiene clean --mode rebuildable
```

The hygiene contract is repo-owned: `cargo xtask` resolves managed artifact roots from the
committed `.cargo/config.toml`, not from ambient caller overrides. The coverage gate also manages
and tags the nested `llvm-cov-target` worktrees that `cargo llvm-cov` creates inside the sibling
coverage roots.

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
- the final curated coverage pass, which is the canonical execution owner for the maintained
  `xtask`, `htmlcut-core`, `htmlcut-cli`, and `htmlcut-tempdir` package test targets instead of
  replaying those same inventories earlier in `cargo xtask check`
- recursive Markdown docs-contract lint for the maintained public docs set except `changelog.md`, including required AFAD metadata fields, version drift, ISO-date formatting, required retrieval `keywords` and `questions`, broken local links, stale canonical schema-name or operation-ID references, completeness drift in the maintained schema/operation inventory docs, release-target and release-asset drift against the canonical shell registry, `PATENTS.md` license-family drift against `deny.toml`, and concrete fenced `htmlcut ...` examples that no longer parse or run in a fixture-backed sandbox
- targeted contract-lint tests that fail when rendered help text, operation examples, parser enums, catalog/schema summaries, or representative recovery errors drift away from the canonical registries
- clap-surface contract-lint that parses the real CLI command tree and fails if command names or applied default values drift away from the canonical `htmlcut_cli::contract` registry
- `cargo clippy -p htmlcut-core --lib --tests --locked -- -D warnings` on the published
  default-feature core surface so cfg-specific warnings cannot hide behind the all-features
  workspace build
- `htmlcut-core` lib tests with default features disabled so fetch-free embeddings stay supported and
  URL requests fail cleanly unless the `http-client` feature is explicitly enabled
- the maintained selector-validation and selector-execution safety proof through `cargo xtask
  miri`, which runs `cargo +nightly miri test -p htmlcut-core --lib --no-default-features
  --locked tests::extract_api::selector_contract_remains_miri_sound -- --exact` with
  `MIRIFLAGS=-Zmiri-strict-provenance`
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
- direct dependency freshness across the workspace manifests through `cargo xtask outdated-check`,
  which materializes a sanitized temporary workspace so repo-owned local path patches do not break
  the freshness check itself
- RustSec advisory auditing with warnings denied
- dependency policy checks through `cargo deny` with warnings denied across the shipped standalone release-target graphs, using the canonical `scripts/release-targets.sh` registry for the target list plus the repository's configured advisory, yanked, unmaintained, ban, license, and source rules
- semver regression checks for `htmlcut-core` against the checked-in baseline
- compile-smoke of the checked-in libFuzzer targets through `cargo check -p htmlcut-fuzz --bins --features fuzzing --locked`
- workspace doc tests, including the maintained external Rust examples in `docs/architecture.md`, `docs/core.md`, `docs/interop-v1.md`, and `docs/schema.md` through `htmlcut-core` doctest harnesses
- compiler-enforced `missing_docs` coverage for the public `htmlcut-core`, `htmlcut-cli`, and `xtask` library surfaces
- distribution-profile CLI build-and-launch smoke
- 100% executable-line coverage and 100% branch coverage across the maintained tracked executable module set for `htmlcut-core`, `htmlcut-cli`, and `xtask`, with duplicate branch spans deduplicated before scoring
- CLI/core parity checks through a matrix-driven integration suite that compares CLI JSON reports with direct `htmlcut-core` results

Before any of those gate steps begin, `cargo xtask check` preflights the exact repository
toolchain declared in `rust-toolchain.toml`, the nightly Miri prerequisites, and the nightly
coverage prerequisites. If the pinned compiler itself is missing, if its required
`clippy`/`rustfmt` components are absent, if nightly is missing `miri` or `rust-src`, if
`cargo +nightly miri --version` is broken despite rustup reporting the components, or if the
coverage prerequisites are absent, the gate stops immediately with the exact repair command
instead of failing later inside the Rust gate.

The cross-platform CI Rust lane uses `cargo xtask ci-rust-gate`, which is built from the same
`xtask` command-plan module as the local gate instead of maintaining a second hard-coded command
inventory in GitHub Actions.

The same preflight also refuses to run semver checks against a dirty
`semver-baseline/htmlcut-core` tree. That baseline is the frozen last-published API snapshot, so
the maintainer gate stops before command planning if the worktree has edited it instead of
comparing against a moved target. The maintained refresh path records the source Git ref and crate
version for that snapshot in `semver-baseline/htmlcut-core/BASELINE.toml`.

The coverage command fails before any coverage build starts if the nightly toolchain or
`llvm-tools-preview` component is missing. Once the preflight passes, it starts from a clean
`cargo llvm-cov` scratch tree, executes the maintained `htmlcut-core`, `htmlcut-cli`,
`htmlcut-tempdir`, and `xtask` package test targets once under coverage, deduplicates duplicate
branch spans emitted by Rust lowering, and then enforces the 100% line and branch bar across the
maintained executable module set. That bar is intentional: HTMLCut treats those tracked files as
contract-critical logic, not aspirational best effort. The scored tracked set is derived from the
maintained non-ignored worktree inventory under the `htmlcut-core`, `htmlcut-cli`, and `xtask`
source roots, with explicit exclusions for declarative/report-model modules, thin binary
entrypoints, and internal test-only source trees. `htmlcut-tempdir` participates in the canonical
execution pass because it is a maintained support crate, even though its helper-only code does not
add scored tracked modules to the 100% coverage ledger. That keeps the gate aligned automatically
when the maintained CLI/core seams split into new executable modules while keeping ignored scratch
files and libFuzzer binaries out of the coverage runner. The coverage scorer now also classifies
tracked Rust sources by syntax shape, so declarative-only files such as module routers, pure type
surfaces, and constant vocabularies stay in the maintained source inventory without being
misreported as missing executable coverage.

The gate also treats artifact hygiene as a maintained invariant:

- Cargo's default workspace output is routed outside the repo root by the committed
  [../.cargo/config.toml](../.cargo/config.toml)
- the coverage gate uses its own sibling managed coverage roots instead of nesting inside the main
  workspace artifact trees
- semver scratch is pruned before and after the semver step
- `cargo xtask check`, `cargo xtask ci-rust-gate`, and `cargo xtask semver-check` run a safe
  hygiene cleanup plus a hygiene verification pass before and after the command plan
- `cargo xtask coverage`, `cargo xtask miri`, and `cargo xtask fuzz-smoke` also run the same safe
  cleanup and verification passes before and after their maintained execution flow

Use [hygiene.md](hygiene.md) for the artifact-root inventory, cleanup modes, and disk-usage
workflow.

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
Ubuntu `24.04` contributor image, validates the committed devcontainer JSON contract, repairs and
bootstraps the named Rust/Cargo cache volumes, proves the repo commands start from inside that raw
container surface, and then proves a real devcontainer client can materialize the committed spec
plus run `devcontainer exec`. Use `./scripts/devcontainer-check.sh` when you want the full
maintainer gate to run through the same contributor image, named volume contract, lifecycle
scripts, and repo-root `./check.sh` entrypoint from the host side. Run both host commands when you
change `.devcontainer/`, the devcontainer lifecycle scripts, or the contributor-container docs.
GitHub CI keeps the fresh-volume and devcontainer-client proof but sets
`HTMLCUT_DEVCONTAINER_REPO_COMMAND_PROBES=skip` because the same job immediately runs
`./scripts/devcontainer-check.sh`, which provides the stronger raw-image repo-command proof without
paying for duplicate help-surface compiles inside the validator.

**Path-based devcontainer gate theory.** The devcontainer gate validates the contributor
*environment*, not application code. Application code changes are already proven by the
cross-platform Rust gate. Running the full devcontainer gate on every PR regardless of what changed
wastes 40-50 minutes per run proving the same environment twice. The gate therefore fires only when
the environment itself changes — specifically when any of these paths are touched:

- `.devcontainer/` — the Dockerfile and `devcontainer.json`
- `scripts/validate-devcontainer.sh`
- `scripts/devcontainer-check.sh`
- `scripts/devcontainer-prepare-user-home.sh`
- `scripts/devcontainer-bootstrap.sh`
- `scripts/devcontainer-cli-helper.Dockerfile`
- `scripts/common.sh`
- `check.sh` — the script the gate runs inside the container

A `devcontainer-changes` detection job computes a git diff of the PR's changed files against those
paths before the gate is evaluated. When no relevant files changed, `contributor-devcontainer` is
skipped. The aggregate `Check` required-status job uses `if: always()` and explicit failure
detection so that a skipped devcontainer gate does not block merge; only a *failed* or *cancelled*
gate prevents `Check` from succeeding. A skipped result is a correct, intended outcome, not a
coverage gap.

The cross-platform Rust gate assigns separate timeout budgets per runner — `30` minutes for macOS
arm64 and `150` minutes for Windows x64 — so the Windows lane can finish a cold `cargo nextest`
build plus dependency-policy and semver verification without expiring mid-run. The Windows runner
also excludes its `target/` build directory from Windows Defender before any Cargo operations
begin, removing the antivirus overhead that otherwise scans every file write during compilation.
Both runners use `Swatinem/rust-cache` with a per-platform key to persist the Cargo registry and
incremental build artifacts across runs.

GitHub CI also runs a release-target smoke matrix across the public standalone targets, unpacking
the built release packages, checking that the packaged README stays package-specific, and
executing one real extraction-plus-request-replay flow before the aggregate required check reports
success.

GitHub CI runs the Linux maintainer gate through the committed contributor devcontainer, alongside
the separate cross-platform Rust jobs and the release-target smoke matrix, before the aggregate
required `Check` result reports success. `Check` uses `if: always()` with explicit
`${{ toJSON(needs.*.result) }}` inspection so a skipped `contributor-devcontainer` gate — the
correct outcome when no devcontainer-relevant files changed — does not prevent `Check` from being
reported or block merge.
