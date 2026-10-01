---
afad: "4.0"
version: "15.0.0"
domain: ENGINEERING
updated: "2026-10-01"
route:
  keywords: [configuration audit, dependency employment, workflow source binding, release integrity]
  questions: ["What changed in the configuration/workflow audit?", "What prevents false release or mutation success?", "Were the refreshed gates executed?"]
---

# Configuration, dependency and workflow audit

A design pass and a separate adversarial design QA pass preceded implementation. Their durable
records are in the maintainer's `Downloads/HTMLCut-15.0.0-config-audit-2026-10-01` directory.
The working baseline included the uncommitted dependency refresh; that work and the frozen
published baseline were preserved. At the audit handoff, runtime gates were deferred by the user. Release verification now executes
those assertions; the audit itself is not substituted for passing runtime evidence.

## Configuration and dependency employment

All twelve Cargo packages, including seven owned runtime forks, are explicit workspace members.
Default-members, the root lockfile, resolver 3, Rust pin owners, resource limits, coverage/structure
budgets, mutation scope and native deployment matrix remain the existing authorities. Explicit
membership fixes the mismatch between Cargo's implicit path membership and freshness tooling's
manifest inventory.

The CLI no longer declares unused scraper, serde_path_to_error, thiserror, assert_cmd, predicates,
regex or shell-words dependencies. Unused workspace aliases were removed. SHA-2's application
dependency disables unused alloc/OID defaults; unused URL serde and schemars URL integration
features are removed. Actual URL parsing, Unicode regex behavior, schema generation, TLS platform
verification, strict charset decoding and compression remain employed through their existing
owners. The core retains zero acquisition I/O.

Freshness tooling now sanitizes owned-carrier dependencies in member and target tables as well
as the root, preserving aliases/features/default-feature flags and copying flat root Rust sources.
The temporary-directory owner remains alive until the subprocess completes, fixing leaked staging
directories. Git/path consumption and GitHub native/source archives are the configured products;
Cargo registry publication is disabled for the unsupported branded package chain.

## Workflows and evidence

- Every checkout selects the intended source and discards persisted credentials. CI and mutation
  planners use the PR head, while shards/summaries use the planner's SHA. Rust caches explicitly
  include the actual sibling target/build roots and only save from main. Bootstrap retries use
  the canonical helpers, and contributor-environment change detection includes Rust/configuration
  and installer changes.
- Mutation planners preserve the complete selected inventory and source/hash binding. Summaries
  verify successful baseline build/test phases, exact per-shard planned/completed identities,
  unique global identity union, source commits, exact counters and no misses/timeouts. Empty PR
  plans require explicit zero evidence and skipped shards. The standalone count summary also
  returns failure for survivors/timeouts; it does not replace the full identity verifier.
- Release tag syntax is checked before checkout through environment data. The producer requires
  annotation, main ancestry and successful same-source main CI/full mutation runs. It emits one
  source/tag/version binding; downstream helpers reject retagging and extract notes from that
  source. Different-tag publications serialize, and active publication is not cancelled by retries.
- Native evidence validates canonical package path, version, target, architecture, clean source,
  output collisions and source/hash stability before/after the actual smoke command. CI and
  release builds both use it; target-specific release proof filenames avoid artifact merge collisions.
- Draft convergence compares existing uploaded bytes to the prepared assets and verifies exact
  checksums/metadata before publication. Wrong existing assets fail without clobber. Matching public
  releases are verification-only; older new drafts cannot downgrade a stable release. Consumer
  verification streams bounded raw asset bytes against published checksums without requiring all
  native packages locally. Attestations remain the separate provenance mechanism.

## Validation handoff

Regression sources cover renamed/target dependencies and flat layouts, scratch cleanup, duplicate
or substituted mutants, failed baselines, wrong source, empty plans, malformed checksum sets,
provider sizes/statuses, actual published retries with mocked GitHub byte streams, native package
mislabeling, moved/invalid tags, latest same-source verification, credential/cache/source wiring
and bootstrap triggers. They are wired into existing unittest discovery and Rust test ownership.
No new runtime crate, alternate extraction API, compatibility engine, relaxed policy or scoring
exclusion was introduced. No gate, CI push, tag, merge or publication was performed in this audit.

Primary references: [Cargo workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html),
[GitHub secure workflow guidance](https://docs.github.com/en/actions/reference/security/secure-use),
and [rust-cache configuration](https://github.com/Swatinem/rust-cache#configuration).
See [dependency refresh](dependency-refresh.md), [implementation status](extraction-contract-status.md),
[quality gates](quality-gates.md) and [release protocol](release-protocol.md).
