<!--
AFAD:
  afad: "4.0"
  version: "16.0.0"
  domain: MAINTAINER
  updated: "2026-10-01"
RETRIEVAL_HINTS:
  keywords: [contributing, maintainer workflow, developer setup, devcontainer, quality gate, docs contract lint, update fixtures, docs sync, release expectations]
  questions: [how do I contribute to HTMLCut?, what checks must pass before merging?, how do I update extraction contract fixtures?, how are Markdown docs linted?, what is the preferred contributor environment?]
  related: [docs/developer-setup.md, docs/developer-devcontainer.md, docs/quality-gates.md, docs/release-protocol.md, docs/versioning-policy.md, docs/core.md]
-->

# Contributing

HTMLCut maintains one core execution contract and one binary CLI. Public changes follow the
package/schema/semantics policy; published API evidence is preserved in the frozen baseline.

## Setup

The preferred contributor environment is the committed devcontainer on Ubuntu `24.04`.
Use [docs/developer-devcontainer.md](docs/developer-devcontainer.md) for that path.
Use [docs/developer-setup.md](docs/developer-setup.md) when you explicitly want the host-native
Rust workflow instead.
The setup guide owns the exact `rustup`, cargo QA tool, `shellcheck`, and macOS
compiler-override commands plus the reasoning behind them.
Use [docs/workspace-layout.md](docs/workspace-layout.md) when you need the current workspace-member
map or the package-name versus Rust-path naming rule.

Rust `1.98.1` is the pinned development toolchain through `rust-toolchain.toml`. The workspace
manifest mirrors that compiler contract through `[workspace.package] rust-version`, and nightly
exists separately for the coverage gate plus live `cargo-fuzz` campaigns.

## Normal Workflow

1. Read the affected crate, module, tests, and docs before editing.
2. Change code, tests, docs, and changelog together when the public surface changes.
3. Run the full maintainer gate before handing work off:

```bash
./check.sh
```

or directly:

```bash
./scripts/xtask.sh check
```

The maintained gate definition lives in [docs/quality-gates.md](docs/quality-gates.md).
For a short live libFuzzer pass that stages the checked-in corpora into disposable scratch, use
`./scripts/xtask.sh fuzz-smoke`.
For mutation testing against first-party runtime code and the maintained selector/scraper
resource-boundary modules, use `./scripts/xtask.sh mutants`.
If you changed `.devcontainer/`, the contributor-container lifecycle scripts, or the contributor
container docs, also run `./scripts/validate-devcontainer.sh` plus
`./scripts/devcontainer-check.sh` from the host shell.

That gate includes recursive Markdown docs-contract linting across the maintained public docs
set except `changelog.md`. It fails on missing AFAD metadata fields, metadata/version drift,
ISO-date formatting, missing retrieval `keywords` or `questions`, broken local links, stale
schema-name or operation-ID references, completeness drift in the maintained schema/operation
inventory docs, release-target and release-asset drift against `scripts/release-targets.sh`,
`PATENTS.md` license-family drift against `deny.toml`, and concrete `htmlcut ...` examples that
no longer parse or run. The same `xtask` test suite also enforces the workspace `rust-version`
floor and member-manifest inheritance. Keep repository docs relative-link clean, avoid
machine-specific absolute paths, and use the canonical names exported by the product code.

Dependency updates that affect workspace crates must refresh `Cargo.lock`. The fuzz package is now
a normal workspace member, so the maintainer gate picks it up through the shared `fmt`, `clippy`,
dependency-freshness, audit, and maintained all-targets coverage/test pass, then adds one explicit
`cargo check -p htmlcut-fuzz --bins --features fuzzing --locked` compile-smoke to prove the
maintained libFuzzer targets still build in their explicit harness mode. The same gate resolves `cargo deny` target coverage from the canonical
`scripts/release-targets.sh` registry, so dependency policy always follows the shipped standalone
target matrix instead of drifting onto an ad hoc local graph.

Cargo Dependabot is re-enabled for the workspace root now that there is only one maintained Cargo
lockfile. Maintainer review is still required before merging dependency PRs.

## Contract Rules

- Do not add backwards-compatibility shims, aliases, or migration paths for generic HTMLCut surfaces.
- If a generic JSON contract changes, update the corresponding schema version and docs in the same change.
- Keep schema names product-owned and generic; do not introduce consumer-specific naming.
- Keep one canonical CLI command surface. Do not add undocumented aliases or shadow entrypoints.
- Treat [docs/versioning-policy.md](docs/versioning-policy.md) as the authority for versioning, schema naming, extraction-semantics policy, and semver-baseline usage.

## Extraction Contract Work

Maintain full expected values, structural relationships, source ranges and typed failures when
changing extraction. Do not regenerate goldens from the current output merely to make a test pass.
The core owns plan validation, execution, limits and identities; CLI acquisition/publication are
adapter responsibilities. Compile/prepare reuse and unused-projection instrumentation remain
part of the acceptance evidence.

The historical `htmlcut-v2` runtime is removed. Its published API snapshot is immutable evidence,
not a supported compatibility path. Use [docs/core.md](docs/core.md) and
[docs/schema.md](docs/schema.md) for the current contract; changes to behavior require the relevant
schema or semantics revision and complete validation before publication.

## Documentation Sync Loop

Documentation is part of the maintained contract.

When behavior or public contract changes:

- update the relevant guide under `docs/`
- update `README.md` if user-facing behavior changed
- update `docs/README.md` if the maintained doc set changed
- add or revise the public-facing `Unreleased` entry in `changelog.md`

Keep docs current-state only. Historical provenance belongs in `changelog.md`, not in reference
docs.

For docs under `docs/`, keep AFAD metadata current. The docs-contract code in
`xtask/src/docs/metadata.rs` owns the `afad` format version and validates the required
metadata fields; it does not depend on agent instruction files. For special top-level files such as
`README.md`, `CONTRIBUTING.md`, `PATENTS.md`, and `fuzz/README.md`, use HTML-comment metadata
rather than YAML frontmatter.

When docs mention a schema family or operation ID, use the canonical names from `htmlcut schema`
and `htmlcut describe`. The Markdown docs contract validates those identifiers directly.

Concrete fenced `htmlcut ...` examples are expected to stay runnable under the docs-contract
sandbox, and the maintained public Rust fences in `docs/architecture.md`, `docs/core.md`,
and `docs/schema.md` are exercised through `htmlcut-core` doctests. If you
change those examples, treat them as executable code, not prose.

Default repo search intentionally excludes `semver-baseline/` through `.ignore` so day-to-day
symbol search stays on the live maintained tree. Use an explicit path or `rg --no-ignore` only
when you are deliberately auditing the published baseline snapshot.

## Release Expectations

Releases are maintainer work and are driven through the GitHub CLI and
[docs/release-protocol.md](docs/release-protocol.md), not through the GitHub web UI.

Important rules:

- `[workspace.package] version` in `Cargo.toml` is the single release-version source of truth
- do not refresh the semver baseline during feature work
- refresh the semver baseline only after the corresponding release is published
- the release is not complete until the published assets and checksums are verified

## Pull Request Hygiene

- Keep diffs coherent by theme.
- Do not leave public docs, schemas, and tests describing different contracts.
- Prefer removing obsolete surface area to carrying dead compatibility debt.
- If you touch release, quality, or versioning policy, update the matching maintainer docs in the same change.
- If you touch the contributor-container surface, update the matching devcontainer docs, scripts,
  and CI validation in the same change.
