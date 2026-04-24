---
afad: "4.0"
version: "5.0.0"
domain: MAINTAINER
updated: "2026-04-24"
route:
  keywords: [tempdir, htmlcut-tempdir, htmlcut_tempdir, TempDir, temporary directory, cleanup on drop, test helper]
  questions: ["what is htmlcut-tempdir for?", "how do HTMLCut tests create disposable temp directories?", "what does the htmlcut_tempdir crate export?"]
---

# Tempdir Helper Guide

`htmlcut-tempdir` is the workspace's small internal temporary-directory helper crate.

It exists so tests and maintainer tooling can share one tiny disposable-directory surface instead
of each crate carrying its own ad hoc helper or a duplicated third-party dependency.

This crate is a maintained workspace member, but it is not published.

## Naming Rule

Use the hyphenated name `htmlcut-tempdir` in Cargo manifests.
Use the underscored path `htmlcut_tempdir` in Rust code.

## Public API

The crate exports only three maintained entry points:

- `tempdir() -> io::Result<TempDir>`
- `TempDir::new() -> io::Result<TempDir>`
- `TempDir::path(&self) -> &Path`

Behavior:

- `tempdir()` is the convenience entry point and simply creates one fresh `TempDir`.
- `TempDir::new()` creates one unique directory under the system temp root.
- `TempDir::path()` returns the filesystem path of that directory.
- Dropping `TempDir` recursively deletes the directory tree best-effort.

## Minimal Example

```rust
use htmlcut_tempdir::tempdir;

let scratch = tempdir().unwrap();
let fixture = scratch.path().join("fixture.txt");
std::fs::write(&fixture, "fixture").unwrap();
assert!(fixture.is_file());
```

## Intended Use

Use this helper for short-lived scratch space in:

- crate tests
- CLI integration fixtures
- `xtask` maintenance flows such as docs-contract sandboxes, fuzz-corpus staging, and release tests

Prefer `tempdir()` unless the explicit constructor form is materially clearer in the local code.

## Boundary

This crate intentionally stays tiny.

It does not provide:

- persistent or opt-out cleanup semantics
- named-file helpers
- archive extraction helpers
- public compatibility guarantees outside the HTMLCut workspace

For the full workspace-member map, including where this crate sits relative to `htmlcut-core`,
`htmlcut-cli`, `fuzz`, and `xtask`, use [workspace-layout.md](workspace-layout.md).
