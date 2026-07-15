---
afad: "4.0"
version: "10.3.0"
domain: OPERATIONS
updated: "2026-07-15"
route:
  keywords: [artifact hygiene, disk usage, cargo target dir, cargo build dir, xtask hygiene, cache cleanup, managed build artifacts]
  questions: ["where do HTMLCut build artifacts live?", "how do I reclaim HTMLCut disk usage?", "what does cargo xtask hygiene do?", "why is repo-local target considered legacy in HTMLCut?", "which artifact roots are managed and disposable?"]
---

# Artifact Hygiene

HTMLCut treats build artifacts as a first-class maintained system, not as an accidental side
effect of running Cargo.

The source tree is the source of truth. Rebuildable Cargo output, coverage scratch, and temporary
investigation artifacts are not.

## Managed Artifact Layout

The committed [../.cargo/config.toml](../.cargo/config.toml) owns the default Cargo artifact
layout:

- final Cargo artifacts go to a sibling `../.htmlcut-artifacts/target` tree outside the repo root
- intermediate Cargo build state goes to a sibling `../.htmlcut-artifacts/build` tree outside the
  repo root

That means routine `cargo build`, `cargo test`, `cargo run`, `cargo xtask ...`, and `./check.sh`
do not grow the repository directory with multi-gigabyte `target/debug` and
`target/debug/incremental` trees.

The maintained coverage gate uses isolated coverage roots derived from those managed paths:

- `../.htmlcut-artifacts/coverage-target`
- `../.htmlcut-artifacts/coverage-build`

Those two coverage roots are the canonical repo-owned parents. `cargo llvm-cov` then creates its
fixed nested Cargo worktrees inside them:

- `../.htmlcut-artifacts/coverage-target/llvm-cov-target`
- `../.htmlcut-artifacts/coverage-build/llvm-cov-target`

Each managed artifact root is tagged with `CACHEDIR.TAG` plus a small
`.htmlcut-artifact.toml` manifest so it is obvious that the directory is disposable.
`cargo xtask hygiene verify` also treats those marker files as part of the maintained contract:
if a managed root or one of the nested `llvm-cov-target` worktrees exists but its markers are
missing, the hygiene gate fails instead of silently accepting an ambiguous cache tree.

## Policy

HTMLCut enforces these hygiene rules:

- repo-local `target/` is legacy spillover, not the intended live Cargo cache root
- repo-local `tmp/` is for temporary investigation artifacts only and must not retain Cargo target
  roots after the maintainer gate finishes
- coverage scratch and semver scratch are disposable and are cleaned automatically by maintained
  `cargo xtask` flows
- the maintainer Rust gates run a hygiene cleanup and a hygiene verification pass before and after
  the command plan

The contributor-container host helpers intentionally export `CARGO_TARGET_DIR` and
`CARGO_BUILD_BUILD_DIR` when they launch Cargo so the container path keeps heavy artifacts on its
mounted cache volume instead of the container filesystem. Those env vars are explicit launch-time
overrides. `cargo xtask` honors them when a caller deliberately supplies them, while the committed
`.cargo/config.toml` remains the default repo-owned layout for ordinary host-native commands and
for hygiene reporting.

## Commands

Inspect the current artifact inventory:

```bash
cargo xtask hygiene report
```

The report's `total_bytes` field counts unique concrete artifact roots once. Aggregate diagnostics
such as `repo-tmp-cargo-targets` are reported separately without inflating the repository total.

Render the same report as JSON:

```bash
cargo xtask hygiene report --format json
```

Fail when the current inventory violates policy:

```bash
cargo xtask hygiene verify
```

Remove the repo-local temporary workspace, legacy repo-local `target/`, coverage scratch, and
other disposable state while keeping the main managed caches:

```bash
cargo xtask hygiene clean --mode safe
```

Remove every rebuildable artifact root, including the managed workspace caches:

```bash
cargo xtask hygiene clean --mode rebuildable
```

## Practical Recovery

If disk usage spikes:

1. run `cargo xtask hygiene report`
2. inspect whether the growth is in the managed caches, repo-local `target/`, or repo-local `tmp/`
3. run `cargo xtask hygiene clean --mode safe` first
4. use `cargo xtask hygiene clean --mode rebuildable` only when you want to reclaim everything
   that Cargo can regenerate

The managed caches are disposable by design. If they are removed, Cargo will rebuild them on the
next command.
