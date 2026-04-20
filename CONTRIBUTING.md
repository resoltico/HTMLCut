<!--
AFAD:
  afad: "3.5"
  version: "4.2.0"
  domain: MAINTAINER
  updated: "2026-04-20"
RETRIEVAL_HINTS:
  keywords: [contributing, maintainer workflow, developer setup, quality gate, docs contract lint, update fixtures, docs sync, release expectations]
  questions: [how do I contribute to HTMLCut?, what checks must pass before merging?, how do I update frozen interop fixtures?, how are Markdown docs linted?]
  related: [docs/developer-setup.md, docs/quality-gates.md, docs/release-protocol.md, docs/versioning-policy.md, docs/interop-v1.md]
-->

# Contributing

HTMLCut prefers coherent current-state design over compatibility scaffolding. Generic public
surfaces may change when the architecture needs to improve; frozen interop profiles may not.

## Setup

Follow [docs/developer-setup.md](docs/developer-setup.md) for the canonical machine bootstrap.
That guide owns the exact `rustup`, cargo QA tool, `shellcheck`, and macOS compiler-override
commands plus the reasoning behind them.

Stable remains the default development toolchain. Nightly exists only for the coverage gate.

## Normal Workflow

1. Read the affected crate, module, tests, and docs before editing.
2. Change code, tests, docs, and changelog together when the public surface changes.
3. Run the full maintainer gate before handing work off:

```bash
./check.sh
```

or directly:

```bash
cargo xtask check
```

The maintained gate definition lives in [docs/quality-gates.md](docs/quality-gates.md).

That gate now includes recursive Markdown docs-contract linting across the maintained public docs
set. It fails on missing AFAD metadata fields, metadata/version drift, ISO-date formatting,
missing retrieval `keywords` or `questions`, broken local links, stale schema-name or
operation-ID references, completeness drift in the maintained schema/operation inventory docs,
and non-parsing concrete `htmlcut ...` examples. Keep repository docs relative-link clean, avoid
machine-specific absolute paths, and use the canonical names exported by the product code.

Dependency updates that affect workspace crates must refresh both `Cargo.lock` and
`fuzz/Cargo.lock`. The fuzz package is checked in and validated with `--locked`, so a
workspace-only lockfile refresh is incomplete.

Cargo Dependabot PRs are intentionally disabled for this reason. Use maintainer-authored
dependency refreshes instead of relying on bot PRs that cannot keep the two lockfiles in sync.

## Contract Rules

- Do not add backwards-compatibility shims, aliases, or migration paths for generic HTMLCut surfaces.
- If a generic JSON contract changes, update the corresponding schema version and docs in the same change.
- Keep schema names product-owned and generic; do not introduce consumer-specific naming.
- Keep one canonical CLI command surface. Do not add undocumented aliases or shadow entrypoints.
- Treat [docs/versioning-policy.md](docs/versioning-policy.md) as the authority for versioning, schema naming, frozen interop policy, and semver-baseline usage.

## Frozen Interop Work

`htmlcut-v1` is frozen.

If you change anything that touches frozen interop plan/result/error documents, digests, or schema
identity:

```bash
cargo test -p htmlcut-core --test v1
cargo test -p htmlcut-core --test v1_properties
cargo test -p htmlcut-core --test v1_acceptance
```

If the frozen fixtures must be deliberately regenerated:

```bash
UPDATE_FIXTURES=1 cargo test -p htmlcut-core -- --ignored update_fixtures
```

Inspect that diff carefully before keeping it. The acceptance test must pass afterwards.

Do not mutate `htmlcut-v1` casually. If downstream requirements exceed the frozen profile, add a
new interop profile instead of changing v1 in place.

## Documentation Sync Loop

Documentation is part of the maintained contract.

When behavior or public contract changes:

- update the relevant guide under `docs/`
- update `README.md` if user-facing behavior changed
- update `docs/README.md` if the maintained doc set changed
- add or revise the public-facing `Unreleased` entry in `changelog.md`

Keep docs current-state only. Historical provenance belongs in `changelog.md`, not in reference
docs.

For docs under `docs/`, keep AFAD metadata current. For special top-level files such as
`README.md`, `CONTRIBUTING.md`, `PATENTS.md`, and `fuzz/README.md`, use HTML-comment metadata
rather than YAML frontmatter.

When docs mention a schema family or operation ID, use the canonical names from `htmlcut schema`
and `htmlcut catalog`. The Markdown docs contract now validates those identifiers directly.

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
