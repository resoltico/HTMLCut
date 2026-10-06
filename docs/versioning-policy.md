---
afad: "4.0"
version: "20.0.0"
domain: MAINTAINER
updated: "2026-10-06"
route:
  keywords: [versioning, extraction schema, semantics, semver baseline]
  questions: ["How are package, schema and semantics versions maintained?"]
---

# Versioning policy

The workspace package version in Cargo.toml is the sole release-version authority. Manifests, lockfiles, docs and native packages follow it. Stable publication uses an immutable annotated release tag after explicit authorization.

The current extraction wire family has schema version 6; extraction semantics independently have version 6. Incompatible wire shapes require a schema change, and selection/projection meaning changes require a semantics change. Packaging alone does not change extraction identity. Unknown versions/fields/enums and obsolete envelopes are rejected, without adapters or migration shims.

The checked-in API baseline identifies its published tag in [BASELINE.toml](../semver-baseline/htmlcut-core/BASELINE.toml). Future refreshes come only from an actual immutable release, using the maintained mechanism. Never refresh from a worktree to hide an API change.

The semver gate explicitly selects `htmlcut-core`, which is distributed in source archives rather than the Cargo registry; implicit package selection can skip `publish = false` packages.

Numeric major increases permit major changes; minor increases permit minor; patch increases and equal versions enforce patch protection. Downgrades, malformed versions and prerelease/build metadata on stable publication are rejected. Future patch/minor releases cannot bypass protection through an unconditional major override.

See [Schemas](schema.md) and [Release Protocol](release-protocol.md).
