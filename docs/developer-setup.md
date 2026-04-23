---
afad: "3.5"
version: "4.4.0"
domain: SETUP
updated: "2026-04-23"
route:
  keywords: [developer setup, fresh machine, rustup, shellcheck, cargo-nextest, cargo-llvm-cov, cargo-fuzz, macOS clang, CC override]
  questions: ["how do I set up a fresh machine for HTMLCut?", "which tools does HTMLCut need locally?", "why does cargo install fail with a missing Homebrew clang path?"]
---

# Developer Setup

**Purpose**: bootstrap a fresh machine into the maintained HTMLCut contributor state.
**Prerequisites**: network access and a working C toolchain such as macOS Command Line Tools.

## Overview

HTMLCut pins Rust `1.95.0` as the default repository toolchain and installs nightly for the
branch-coverage gate plus live `cargo-fuzz` campaigns. The maintainer workflow also depends on
Rust-native QA commands plus `shellcheck` for shell-script checks.

The workspace manifest mirrors that compiler contract through `[workspace.package] rust-version`,
so the published crates and the pinned repository toolchain do not silently drift apart.

Use `rustup` directly for Rust instead of Homebrew Rust. HTMLCut needs explicit control over
stable, nightly, and per-toolchain components, which is exactly what `rustup` is designed to
manage. Use your system package manager for `shellcheck` because it is an external non-Cargo tool.

## Install The Rust Toolchains

If `xcode-select -p` fails on macOS, install the Apple command-line tools first:

```bash
xcode-select --install
```

Then install Rust and the HTMLCut toolchains:

```bash
curl --proto '=https' --tlsv1.2 https://sh.rustup.rs -sSf | sh -s -- -y --profile minimal
source "$HOME/.cargo/env"
rustup toolchain install 1.95.0 --profile minimal
rustup toolchain install nightly --profile minimal --component llvm-tools-preview
rustup component add clippy rustfmt --toolchain 1.95.0
```

Why this shape:

- `1.95.0` is the exact toolchain pinned by `rust-toolchain.toml` for day-to-day repository work.
- the workspace manifest mirrors that compiler contract through `[workspace.package] rust-version`.
- `nightly` exists because `cargo +nightly llvm-cov --branch` is still required for the maintained
  coverage gate, and because `cargo-fuzz` needs nightly for real fuzzing runs.
- The `minimal` profile keeps the base install smaller, then HTMLCut adds only the components it
  actually uses.

## Install The Rust QA Commands

On the maintained macOS path, install the cargo subcommands with the system compiler explicitly:

```bash
source "$HOME/.cargo/env"
CC=clang CXX=clang++ cargo install cargo-nextest cargo-audit cargo-deny cargo-semver-checks cargo-outdated cargo-llvm-cov cargo-fuzz --locked
```

Why this shape:

- These commands are Rust-native tools, so `cargo install` keeps them in the same Rust-managed
  toolchain path as `cargo` itself.
- `--locked` uses each tool's checked-in lockfile and avoids drifting dependency resolution during
  bootstrap.
- `CC=clang CXX=clang++` protects fresh macOS machines from stale shell overrides that point at a
  removed Homebrew LLVM install.
- `cargo-fuzz` is not part of the default maintainer gate, but HTMLCut keeps checked-in fuzz
  targets and seed corpora, so contributors should have the runner available for local smoke
  campaigns and incident reproduction.

If you are not on macOS, keep the same tool list but omit the `CC=clang CXX=clang++` override and
use the platform's normal C toolchain instead.

LLVM-backed maintainer flows are a separate concern: `cargo xtask coverage` and
`cargo xtask fuzz-smoke` both launch Cargo with `CC=clang CXX=clang++` so coverage and libFuzzer
stay on the LLVM toolchain. Keep `clang` and `clang++` available on `PATH` on any host where you
plan to run those maintained commands.

## Install ShellCheck

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
global cargo install with:

```bash
CC=clang CXX=clang++ cargo install <tool> --locked
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

## Verify The Setup

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
cargo fuzz --version
shellcheck --version
```

Then run one maintained gate entrypoint:

```bash
./check.sh
```

`cargo xtask check` is the equivalent direct invocation if you want to bypass the shell wrapper.
For a short live libFuzzer pass that keeps the checked-in seed corpora clean, use
`cargo xtask fuzz-smoke`. That command also preflights the nightly toolchain plus `cargo-fuzz`
before it launches, so missing fuzz prerequisites fail fast with one actionable message.
The main `cargo xtask check` gate likewise preflights the exact pinned `1.95.0` toolchain and its
required `clippy`/`rustfmt` components before the Rust gate starts, including direct probes that
verify the tool binaries are actually runnable.

Rust `1.95.0` should remain the pinned repository toolchain. If the gate fails, treat the first
real failure as the next missing prerequisite and fix that root cause before rerunning.

## Disk Usage

The Git repository itself is small. The multi-gigabyte footprint comes from build artifacts under
`target/`, especially coverage workspaces such as `target/llvm-cov-target`, native dependency
builds, semver-check scratch data, and compiled test binaries. Local fuzzing also uses a separate
`fuzz/target/` tree unless you override Cargo's target directory for fuzz runs.

`cargo xtask check` treats the two worst offenders as ephemeral scratch:

- `target/llvm-cov-target` is cleaned again after the coverage step finishes.
- `target/semver-checks` is pruned before and after the semver gate runs.

That means persistent growth should mostly come from normal Cargo developer caches such as
`target/debug` and target-triple build outputs.

If you need to reclaim space after running the maintainer gate:

```bash
source "$HOME/.cargo/env"
cargo llvm-cov clean --workspace
cargo clean
```

If you have been running libFuzzer locally, you may also want to remove `fuzz/target/`.
