---
afad: "4.0"
version: "6.0.0"
domain: WORKSPACE
updated: "2026-04-29"
route:
  keywords: [workspace layout, crate map, htmlcut-core, htmlcut-cli, htmlcut-tempdir, htmlcut-fuzz, xtask, package name, crate name]
  questions: ["which Cargo packages are in the HTMLCut workspace?", "what is htmlcut-tempdir used for?", "why do HTMLCut package names use hyphens but Rust paths use underscores?"]
---

# Workspace Layout

HTMLCut's runtime architecture and its Cargo workspace topology are related, but they are not the
same thing.

[architecture.md](architecture.md) explains which runtime surfaces own behavior.
This guide explains which workspace members exist, why they exist, and how their package names map
to Rust paths.

## Workspace Members

| Path | Package / Rust path | Role | Published |
| --- | --- | --- | --- |
| `crates/htmlcut-core` | package `htmlcut-core`, Rust crate `htmlcut_core` | Canonical extraction engine, schema registry, operation catalog, interop surface, and typed request/result contracts. | yes |
| `crates/htmlcut-cli` | package `htmlcut-cli`, Rust crate `htmlcut_cli`, binary `htmlcut` | Operator-facing CLI adapter plus typed CLI report models and clap-command helpers. | yes |
| `crates/htmlcut-tempdir` | package `htmlcut-tempdir`, Rust crate `htmlcut_tempdir` | Small internal temporary-directory helper shared by tests and maintainer tooling. | no |
| `fuzz` | package `htmlcut-fuzz` | Checked-in libFuzzer targets and seed corpora kept on the main workspace lockfile. | no |
| `xtask` | package `xtask` | Maintainer automation for the gate, docs contract, coverage, fuzz smoke, and semver-baseline refresh. | no |

## Naming Rule

Cargo package names are hyphenated:

- `htmlcut-core`
- `htmlcut-cli`
- `htmlcut-tempdir`

Rust crate paths use underscores:

- `htmlcut_core`
- `htmlcut_cli`
- `htmlcut_tempdir`

Use the package spelling in Cargo manifests, install commands, and release assets.
Use the Rust-path spelling in `use` statements, doctests, and library code.

## Dependency Direction

The important dependency direction is:

1. `htmlcut-cli` depends on `htmlcut-core`.
2. `xtask` depends on the maintained workspace crates so the gate can validate their live
   contracts.
3. Tests and maintainer helpers use `htmlcut-tempdir` instead of each crate carrying its own ad
   hoc temp-directory helper.

`htmlcut-tempdir`, `fuzz`, and `xtask` are real maintained workspace members, but they are not
runtime product surfaces in the same sense as `htmlcut-core`, `htmlcut-cli`, and
`htmlcut_core::interop::v1`.

## Trees Outside The Workspace

These paths matter, but they are not normal workspace members:

- `semver-baseline/htmlcut-core` is a checked-in snapshot of the last published `htmlcut-core`
  API used by semver checks. It is intentionally excluded from the live workspace.
- `docs/` is the maintained Markdown contract set.
- `dist/` and `target/` are generated artifact trees, not source-of-truth contract owners.

## Where To Go Next

- Use [architecture.md](architecture.md) for runtime ownership boundaries.
- Use [cli.md](cli.md) for operator-facing command behavior.
- Use [cli-library.md](cli-library.md) for the published `htmlcut_cli` crate API.
- Use [core.md](core.md) for the canonical embeddable engine surface.
- Use [tempdir.md](tempdir.md) for the internal `htmlcut_tempdir` helper crate.
