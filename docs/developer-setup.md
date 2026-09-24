---
afad: "4.0"
version: "14.0.0"
domain: SETUP
updated: "2026-09-24"
route:
  keywords: [developer setup, devcontainer, host native, fresh machine, rustup, shellcheck, cargo-nextest, cargo-llvm-cov, cargo-fuzz, cargo-mutants, cargo-miri, macOS clang, artifact hygiene]
  questions: ["how do I set up a fresh machine for HTMLCut?", "which tools does HTMLCut need locally?", "how do I run HTMLCut mutation testing?", "how do I run the HTMLCut strict-provenance selector-and-slice Miri proof?", "why does cargo install fail with a missing Homebrew clang path?", "where do HTMLCut build artifacts live on disk?"]
---

# Developer Setup

**Purpose**: bootstrap a fresh machine into the maintained HTMLCut contributor state.
**Prerequisites**: network access and a working C toolchain such as macOS Command Line Tools.

## Overview

HTMLCut's preferred contributor path is the committed devcontainer on Ubuntu `24.04`.
If you use that path, the host needs only Docker plus a devcontainer-spec-aware client.
Use [developer-devcontainer.md](developer-devcontainer.md) for that workflow.

The rest of this document is the host-native Rust path.

HTMLCut pins one exact stable toolchain through `rust-toolchain.toml` and installs nightly for the
branch-coverage gate, the maintained strict-provenance selector-and-slice Miri proof, and live `cargo-fuzz`
campaigns. The maintainer workflow also depends on Rust-native QA commands plus `shellcheck` for
shell-script checks.

The workspace manifest carries the published compatibility floor through
`[workspace.package] rust-version = "1.98.1"`, while `rust-toolchain.toml` owns the exact
day-to-day repository pin (currently `1.98.1`).

Use `rustup` directly for Rust instead of Homebrew Rust. HTMLCut needs explicit control over
stable, nightly, and per-toolchain components, which is exactly what `rustup` is designed to
manage. Use your system package manager for `shellcheck` because it is an external non-Cargo tool.

## Install The Host-Native Rust Toolchains

If `xcode-select -p` fails on macOS, install the Apple command-line tools first:

```bash
xcode-select --install
```

Then install Rust and the HTMLCut toolchains:

```bash
curl --proto '=https' --tlsv1.2 https://sh.rustup.rs -sSf | sh -s -- -y --profile minimal
source "$HOME/.cargo/env"
source ./scripts/contributor-rust-tools.sh
rustup toolchain install "${HTMLCUT_CONTRIBUTOR_RUST_STABLE_TOOLCHAIN}" --profile minimal
htmlcut_contributor_install_nightly_toolchain
htmlcut_contributor_install_stable_toolchain_components
```

Why this shape:

- `./scripts/contributor-rust-tools.sh` is the canonical owner for the exact stable/nightly
  bootstrap values shared by docs, bootstrap scripts, and CI.
- `rust-toolchain.toml` owns the exact stable repository pin for day-to-day work. Right now that
  resolves to `1.98.1`.
- the workspace manifest carries the published compatibility floor separately through
  `[workspace.package] rust-version = "1.98.1"`.
- `nightly` exists because `cargo +nightly llvm-cov --branch` is still required for the maintained
  coverage gate, because `cargo xtask miri` now proves the selector and delimiter-slice paths
  under strict provenance, and because `cargo-fuzz` needs nightly for real fuzzing runs.
- The `minimal` profile keeps the base install smaller, then HTMLCut adds only the components it
  actually uses.

## Install The Host-Native Rust QA Commands

On the maintained macOS path, install the pinned cargo subcommands with the system compiler
explicitly:

```bash
source "$HOME/.cargo/env"
brew install pkgconf openssl@3
CC=clang CXX=clang++ ./scripts/install-contributor-cargo-tools.sh
```

Why this shape:

- `./scripts/install-contributor-cargo-tools.sh` installs the repo-owned pinned contributor tool
  inventory instead of whichever helper versions crates.io happens to serve on that day.
- The script pins each tool's released version. It installs Nextest from the official archive
  after checking a pinned SHA-256 digest, because Nextest requires source installs to use its
  upstream lockfile and that lockfile can retain yanked crates. Deny builds from its clean
  published lockfile because its profile names packages in that graph. The other tools build
  with the workspace's explicit Rust toolchain and resolve available transitive dependencies.
- `pkgconf` plus `openssl@3` provide the native metadata needed by the pinned cargo-tool graph on
  the maintained macOS path, especially `cargo-outdated`.
- `CC=clang CXX=clang++` protects fresh macOS machines from stale shell overrides that point at a
  removed Homebrew LLVM install.
- `cargo-fuzz` is installed with the default contributor inventory because HTMLCut keeps checked-in
  fuzz targets and seed corpora. `cargo-mutants` is also installed by default because the full
  maintainer gate executes a real disposable mutation-workspace integration test; full mutation
  campaigns remain separate scheduled or manual work.

The installer also preflights those macOS native prerequisites now. If `pkg-config` cannot see
OpenSSL metadata, it stops immediately with the exact Homebrew repair command instead of failing
midway through a long `cargo install`.

If you are not on macOS, keep the same tool list but omit the `CC=clang CXX=clang++` override and
use the platform's normal C toolchain instead:

```bash
source "$HOME/.cargo/env"
./scripts/install-contributor-cargo-tools.sh
```

LLVM-backed maintainer flows are a separate concern: `cargo xtask coverage` and
`cargo xtask fuzz-smoke` both launch Cargo with `CC=clang CXX=clang++` so coverage and libFuzzer
stay on the LLVM toolchain. The strict-provenance selector-and-slice Miri proof does not need that compiler override,
but it does require the nightly `miri` plus `rust-src` components. Keep `clang` and `clang++`
available on `PATH` on any host where you plan to run the maintained coverage or fuzz commands.

The CI mutation workflow installs the same pinned `cargo-mutants` release independently for its
sharded campaigns.

The pinned `cargo-semver-checks` `0.50.0` release includes Rustdoc-v60 support for the
maintained Rust `1.98.1` semver gate.

## Install Host-Native ShellCheck

Install `shellcheck` from Homebrew on macOS:

```bash
brew install shellcheck
```

Why this shape:

- `shellcheck` is a system tool, not a Cargo crate.
- Homebrew is the documented macOS install path for ShellCheck and keeps the binary managed outside
  the Rust toolchain.

## Fix Stale Compiler Overrides

If `cargo install` or another native Rust build fails with an error like:

```text
failed to find tool "/opt/homebrew/opt/llvm/bin/clang"
```

your shell is exporting a stale `CC` override for a Homebrew LLVM install that is no longer
present. Fix the shell config so it only exports that path when LLVM actually exists, or rerun the
pinned contributor-tool install with:

```bash
CC=clang CXX=clang++ ./scripts/install-contributor-cargo-tools.sh <tool>
```

Repository-local Cargo work is already guarded by [../.cargo/config.toml](../.cargo/config.toml),
which provides the `cargo xtask` alias but no longer forces a global compiler override across the
whole workspace.

One more macOS footgun: long-lived desktop app shells can inherit a stale `CC` value even after
your `~/.zshrc` has been fixed. If local `cargo build`, `cargo run`, or `cargo test` behavior looks
impossible or newly built binaries fail to launch, check `echo $CC` in the current shell and clear
it for the session before debugging HTMLCut itself:

```bash
unset CC
unset CXX
unset LDFLAGS
unset CFLAGS
unset CXXFLAGS
```

## Verify The Host-Native Setup

Verify the toolchain first:

```bash
source "$HOME/.cargo/env"
rustc --version
cargo --version
cargo nextest --version
cargo audit --version
cargo deny --version
cargo semver-checks --version
cargo outdated --version
cargo llvm-cov --version
cargo +nightly miri --version
cargo fuzz --version
cargo mutants --version
shellcheck --version
```

Then run one maintained gate entrypoint:

```bash
./check.sh
```

The stable repo-owned maintainer launcher is `./scripts/xtask.sh`. Use it when you want the
supported gate path outside Cargo's mutable artifact roots:

```bash
./scripts/xtask.sh ci-rust-gate
./scripts/xtask.sh miri
./scripts/xtask.sh outdated-check
./scripts/xtask.sh fuzz-smoke
# Full mutation campaigns are separate from the normal maintainer gate.
./scripts/xtask.sh mutants
```

The curated cross-platform CI Rust lane runs `./scripts/xtask.sh ci-rust-gate`, which still comes
from the same `xtask` plan instead of duplicating a second command inventory in GitHub Actions.
Direct `cargo xtask ...` remains useful for interactive local work on `xtask` itself.
For a short live libFuzzer pass that keeps the checked-in seed corpora clean, use
`./scripts/xtask.sh fuzz-smoke`. That command also preflights the nightly toolchain plus `cargo-fuzz`,
then enables the real `fuzzing` harness mode explicitly before it launches, so missing fuzz
prerequisites fail fast with one actionable message and broad default Cargo test loops stay
finite.
For mutation testing, use `./scripts/xtask.sh mutants`; it materializes a bounded number of
disposable complete source workspaces, assigns deterministic package-local partitions to those
lanes, runs cargo-mutants in place only inside the copies, and reconciles the resulting `mutants.out`
tree under `../.htmlcut-artifacts/mutation-runs`. Outer sharding alone controls concurrency:
cargo-mutants' sequential in-place worker mode receives a proportionate Cargo build budget and never
inherits an outer GNU Make jobserver. Each lane uses a private offline Cargo home and lock domain,
with its already-cached registry and Git trees linked read-only, so concurrent baselines do not
contend on the contributor's global Cargo cache. Within a copied worker, only Rust source files are
writable; configuration, documentation, and other non-Rust inputs stay read-only. A lane may execute later partitions only after a
content fingerprint proves cargo-mutants restored its copied source tree; it then reuses that lane's
temporary Cargo target. Local execution requests at most four lanes, then derives a lower safe count
from the free space on the temporary-workspace volume, reserving 12 GiB for the host and 10 GiB per
lane; insufficient space fails before any worker is staged. Before each later partition in an
already-staged lane, the runner also requires 2 GiB of runtime headroom: it stops before launching
that partition rather than allowing unrelated host activity to exhaust the filesystem mid-campaign.
Small selections retain at least
sixty-four planned mutants per partition to avoid duplicate cold baselines, so local mutation testing protects a dirty checkout without
recompiling the workspace from scratch for every mutant. The canonical aggregate records the exact
planned inventory. As each worker progresses, it retains runner and baseline diagnostics plus
per-mutant evidence only for missed or timed-out mutants; caught and unviable outcomes remain in the
aggregate documents and compact lists without retaining their non-actionable log and diff trees.
The scheduled CI workflow is the only maintained caller that applies `--in-place` to its checkout,
because that checkout is disposable.

To test only mutations in a reviewed source diff, pass a unified diff file:

```bash
git diff --no-ext-diff --unified=0 origin/main...HEAD > /tmp/htmlcut-mutants.diff
./scripts/xtask.sh mutants --in-diff /tmp/htmlcut-mutants.diff
```
The command reads the diff before hygiene cleanup, stages it under the managed mutation evidence
root for cargo-mutants, and removes that staged copy afterward. Repository-local `tmp/` diff paths
are therefore supported even though hygiene treats that directory as disposable scratch.
The main `xtask check` gate likewise preflights the exact stable pin from
`rust-toolchain.toml`, its required `clippy`/`rustfmt` components, the nightly Miri prerequisites,
and the nightly coverage prerequisites before the Rust gate starts, including direct probes that
verify the tool binaries are actually runnable.

If the gate fails, treat the first real failure as the next missing prerequisite and fix that root
cause before rerunning.

If you are using the committed devcontainer instead of host-native Rust, use
[developer-devcontainer.md](developer-devcontainer.md) and verify the host-side container path with
`./scripts/validate-devcontainer.sh` plus `./scripts/devcontainer-check.sh` from the host shell.

## Disk Usage

HTMLCut now routes normal Cargo work outside the repo root through the committed
[../.cargo/config.toml](../.cargo/config.toml):

- final build artifacts live in `../.htmlcut-artifacts/target`
- intermediate build cache lives in `../.htmlcut-artifacts/build`

The maintainer coverage gate uses sibling disposable coverage roots
(`../.htmlcut-artifacts/coverage-target` and `../.htmlcut-artifacts/coverage-build`), and the
semver gate scratch stays disposable as well. `cargo llvm-cov` then places its nested
`llvm-cov-target` worktrees inside those managed coverage roots, and HTMLCut tags those nested
worktrees as disposable too.

That means a huge repo-local `target/` tree is a legacy spillover condition, not the intended live
layout.

Start with the maintained report:

```bash
source "$HOME/.cargo/env"
cargo xtask hygiene report
```

If you need to reclaim space without deleting the main managed caches:

```bash
source "$HOME/.cargo/env"
cargo xtask hygiene clean --mode safe
```

If you need to reclaim every rebuildable artifact root:

```bash
source "$HOME/.cargo/env"
cargo xtask hygiene clean --mode rebuildable
```

Use [hygiene.md](hygiene.md) for the full artifact-lifecycle contract and the maintained policy.
