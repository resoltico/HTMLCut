---
afad: "4.0"
version: "19.0.0"
domain: WORKSPACE
updated: "2026-10-05"
route:
  keywords: [workspace layout, crate map, htmlcut-core, htmlcut-cli, htmlcut-tempdir, htmlcut-fuzz, xtask, devcontainer, package name, crate name, artifacts]
  questions: ["which Cargo packages are in the HTMLCut workspace?", "what is htmlcut-tempdir used for?", "why do HTMLCut package names use hyphens but Rust paths use underscores?", "where do the HTMLCut managed Cargo artifact roots live?", "where does the HTMLCut contributor devcontainer live?"]
---

# Workspace Layout

HTMLCut's runtime architecture and its Cargo workspace topology are related, but they are not the
same thing.

[architecture.md](architecture.md) explains which runtime surfaces own behavior.
This guide explains which workspace members exist, why they exist, and how their package names map
to Rust paths.

## Workspace Members

| Path | Package / Rust path | Role | Cargo registry publication |
| --- | --- | --- | --- |
| `crates/htmlcut-core` | package `htmlcut-core`, Rust crate `htmlcut_core` | Immutable snapshots, compiled plans, lazy prepared documents, requested projections, bare data/receipts/errors and targeted inspection. | disabled |
| `crates/htmlcut-cli` | package `htmlcut-cli`, binary `htmlcut` | Regular file/stdin UTF-8 snapshots, five commands, self-contained bundles, framing and atomic publication; no Rust library API. | disabled |
| `crates/htmlcut-tempdir` | package `htmlcut-tempdir`, Rust crate `htmlcut_tempdir` | Small internal temporary-directory helper shared by tests and maintainer tooling. | no |
| `fuzz` | package `htmlcut-fuzz` | Checked-in libFuzzer targets and seed corpora kept on the main workspace lockfile. | no |
| `xtask` | package `xtask` | Maintainer automation for the gate, docs contract, coverage, fuzz smoke, and semver-baseline refresh. | no |
| `patches/rust/scraper` | package `htmlcut-scraper`, Rust crate `scraper` | Owned bounded DOM, selector adapter and immutable serialization carrier. | disabled |
| `patches/rust/selectors` | package `htmlcut-selectors`, Rust crate `selectors` | Owned CSS selector engine and shared work accounting. | disabled |
| `patches/rust/servo_arc` | package `htmlcut-servo-arc`, Rust crate `servo_arc` | Paired selector allocation/provenance carrier. | disabled |
| `patches/rust/html5ever` | package `htmlcut-html5ever`, Rust crate `html5ever` | Paired tokenizer/tree builder and stop hooks. | disabled |
| `patches/rust/markup5ever` | package `htmlcut-markup5ever`, Rust crate `markup5ever` | Paired parser/serializer interfaces. | disabled |
| `patches/rust/tendril` | package `htmlcut-tendril`, Rust crate `tendril` | Paired text buffering/provenance carrier. | disabled |
| `patches/rust/sha2` | package `htmlcut-sha2`, Rust crate `sha2` | SHA-256 identity backend with ARM64 correction. | disabled |

The supported publication is GitHub native/source archives, with Rust consumption through git/path
dependencies. No complete branded-fork crates.io distribution is configured; `publish = false`
prevents an accidental partial registry publication. All maintained packages are explicit members;
default-members stays limited to core, CLI and the temporary-directory helper.

## Naming Rule

Cargo package names are hyphenated:

- `htmlcut-core`
- `htmlcut-cli`
- `htmlcut-tempdir`

Rust crate paths use underscores:

- `htmlcut_core`
- `htmlcut_tempdir`

Use the package spelling in Cargo manifests, install commands, and release assets.
Use the Rust-path spelling in `use` statements, doctests, and library code.

## Dependency Direction

The important dependency direction is:

1. `htmlcut-cli` depends on `htmlcut-core`.
2. `xtask` depends on core and temporary-directory helpers. It validates CLI behavior through a
   bounded invocation of a once-built binary rather than importing CLI internals.
3. Tests and maintainer helpers use `htmlcut-tempdir` instead of each crate carrying its own ad
   hoc temp-directory helper.

`htmlcut-tempdir`, `fuzz`, and `xtask` are real maintained workspace members, but they are not
runtime product surfaces. The supported products are the core Rust API and the `htmlcut` binary.

## Trees Outside The Workspace

These paths matter, but they are not normal workspace members:

- `semver-baseline/htmlcut-core` is a checked-in snapshot of the last published `htmlcut-core`
  API used by semver checks. It is intentionally excluded from the live workspace; vendored
  dependency manifests stay inert until `xtask` materializes the snapshot in managed scratch.
- `.devcontainer/` owns the committed contributor-container contract and is backed by lifecycle
  scripts plus host-side entrypoints under `scripts/`.
- `docs/` is the maintained Markdown contract set.
- Cargo's managed build artifacts live in the sibling `../.htmlcut-artifacts/` tree, not in the
  repo root. The coverage gate also owns nested `llvm-cov-target` worktrees inside its sibling
  coverage roots. A repo-local `target/` tree is a legacy spillover condition that `cargo xtask
  hygiene` can reclaim.

## Where To Go Next

- Use [architecture.md](architecture.md) for runtime ownership boundaries.
- Use [cli.md](cli.md) for operator-facing command behavior.
- Use [core.md](core.md) for the canonical embeddable engine surface.
- Use [tempdir.md](tempdir.md) for the internal `htmlcut_tempdir` helper crate.
