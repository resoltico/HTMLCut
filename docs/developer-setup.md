---
afad: "4.0"
version: "10.1.0"
domain: SETUP
updated: "2026-05-16"
route:
  keywords: [developer setup, devcontainer, host native, fresh machine, rustup, shellcheck, cargo-nextest, cargo-llvm-cov, cargo-fuzz, cargo-miri, macOS clang, CC override, artifact hygiene]
  questions: ["how do I set up a fresh machine for HTMLCut?", "which tools does HTMLCut need locally?", "how do I run the HTMLCut strict-provenance selector-safety Miri proof?", "why does cargo install fail with a missing Homebrew clang path?", "where do HTMLCut build artifacts live on disk?", "do I need Rust installed on the host if I use the HTMLCut devcontainer?"]
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
branch-coverage gate, the maintained strict-provenance selector-safety Miri proof, and live `cargo-fuzz`
campaigns. The maintainer workflow also depends on Rust-native QA commands plus `shellcheck` for
shell-script checks.

The workspace manifest carries the published compatibility floor through
`[workspace.package] rust-version = "1.95"`, while `rust-toolchain.toml` owns the exact
day-to-day repository pin (currently `1.95.0`).

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
  resolves to `1.95.0`.
- the workspace manifest carries the published compatibility floor separately through
  `[workspace.package] rust-version = "1.95"`.
- `nightly` exists because `cargo +nightly llvm-cov --branch` is still required for the maintained
  coverage gate, because `cargo xtask miri` now proves the selector-safety path under strict
  provenance, and because `cargo-fuzz` needs nightly for real fuzzing runs.
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
- The script uses each tool's checked-in lockfile and keeps the QA commands in the same
  Rust-managed toolchain path as `cargo` itself.
- `pkgconf` plus `openssl@3` provide the native metadata needed by the pinned cargo-tool graph on
  the maintained macOS path, especially `cargo-outdated`.
- `CC=clang CXX=clang++` protects fresh macOS machines from stale shell overrides that point at a
  removed Homebrew LLVM install.
- `cargo-fuzz` is not part of the default maintainer gate, but HTMLCut keeps checked-in fuzz
  targets and seed corpora, so contributors should have the runner available for local smoke
  campaigns and incident reproduction.

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
stay on the LLVM toolchain. The strict-provenance selector-safety Miri proof does not need that compiler override,
but it does require the nightly `miri` plus `rust-src` components. Keep `clang` and `clang++`
available on `PATH` on any host where you plan to run the maintained coverage or fuzz commands.

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
shellcheck --version
```

Then run one maintained gate entrypoint:

```bash
./check.sh
```

`cargo xtask check` is the equivalent direct invocation if you want to bypass the shell wrapper.
The curated cross-platform CI Rust lane runs `cargo xtask ci-rust-gate`, which comes from the
same `xtask` plan instead of duplicating a second command inventory in GitHub Actions.
For the maintained selector-safety proof in isolation, use `cargo xtask miri`.
For the maintained dependency-freshness gate in isolation, use `cargo xtask outdated-check`.
For a short live libFuzzer pass that keeps the checked-in seed corpora clean, use
`cargo xtask fuzz-smoke`. That command also preflights the nightly toolchain plus `cargo-fuzz`,
then enables the real `fuzzing` harness mode explicitly before it launches, so missing fuzz
prerequisites fail fast with one actionable message and broad default Cargo test loops stay
finite.
The main `cargo xtask check` gate likewise preflights the exact stable pin from
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
