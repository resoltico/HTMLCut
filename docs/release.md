# Release operations

The [Release workflow](../.github/workflows/release.yml) publishes GitHub source and
native packages for an existing annotated stable `vMAJOR.MINOR.PATCH` tag, supplied by
a tag push or manual dispatch. The tag must match the workspace version in its immutable
source commit, and that commit must be an ancestor of `origin/main`. The latest eligible **main push**
CI run for that exact SHA must have completed successfully. PR, manual CI and local
checks do not satisfy this admission rule.

The tagged source must contain exactly one nonempty changelog section for `VERSION`.
Release notes come from that commit's section, not from `Unreleased` or edits in a
working checkout. The [target registry](../tools/release-targets.toml) defines maintained
native targets and asset names.

Before tagging a release candidate, run the full [contributor gate](../CONTRIBUTING.md),
the 10,000-run two-target fuzz smoke and full product mutation campaign. Review every
survivor and timeout against the release source. Focused native runtime/CLI campaigns
do not establish full workspace review, and CI does not establish human survivor review.

## Workflow and local stages

The workflow resolves one SHA, checks out that source for every job, builds source
archives and native packages, accepts the native executables, generates checksums and
provenance, then publishes. It verifies anonymous public downloads and executes each
downloaded native package afterward. Publication is serialized across tags; checkout
credentials are not persisted.

Full workspace, minimum-Rust and contributor checks belong to the admitted main CI
run. Local `package` and `accept` operations do not rerun that workflow or establish
external release success. Native acceptance executes the CLI tests against the unpacked
binary, including ignored native/performance controls, the shared corpus, signature
rejection controls where applicable and the external acquisition recipe. Missing
required ACL, label, pipe or signature capabilities fail acceptance.

For local stages, use up-to-date `origin/main` and tag refs from the release repository.
Work from the repository root in a clean checkout at the admitted `SOURCE_SHA`.
`identity` requires `HEAD` to equal the resolved tag source; `package`
builds the current `HEAD`, not an arbitrary source argument. `accept` checks source,
version, guide, archive and binary identity through execution.

Install the pinned `cargo-about` tool as described in [contributor setup](../CONTRIBUTING.md).
Cargo uses [the checked-in compiler pin](../rust-toolchain.toml). From the checkout root,
install the selected target with `rustup target add TARGET` and provide its native linker.
The maintained Linux musl lane requires a musl toolchain (`musl-tools` on the workflow's
Ubuntu host); see the [native setup](../.github/workflows/release.yml).
Inspect and verify all publishable crates together:

```sh
cargo package --list -p htmlcut-core
cargo package --list -p htmlcut-runtime
cargo package --list -p htmlcut-cli
cargo package --locked -p htmlcut-core -p htmlcut-runtime -p htmlcut-cli
```

Coordinated packaging verifies unpublished siblings through Cargo's temporary registry.
`--no-verify` is not package-build evidence. Source archives include runtime, maintenance
sources/config/tests, literal fixtures and licenses. Native archives contain only the
executable, target guide and legal files and require no Python at runtime. Original
attribution and exact locked native/runtime terms remain in NOTICE.

Local GitHub operations require `gh` authentication for the release repository. Set
`GITHUB_REPOSITORY` for exact-source CI admission and `GH_REPO` for GitHub CLI operations;
use the same repository for both. CI supplies its scoped job credentials. Admission
needs read access to Actions; publication requires `GH_TOKEN` with release write access,
and attestation verification needs the corresponding read access. Public asset bytes
are downloaded anonymously with `curl`.

Substitute the annotated `TAG`, exact `SOURCE_SHA`, manifest `VERSION`, maintained
`TARGET` triple and actual emitted `ARCHIVE` below. Output directories are caller-owned;
the checksum stage needs the complete configured source/native asset set in one directory.

```sh
export GITHUB_REPOSITORY=resoltico/HTMLCut
export GH_REPO="$GITHUB_REPOSITORY"
python3 -m tools.htmlcut_maintenance release identity TAG --require-ci
python3 -m tools.htmlcut_maintenance release notes --ref SOURCE_SHA --version VERSION
python3 -m tools.htmlcut_maintenance release source SOURCE_SHA --output /owned/source-archives
python3 -m tools.htmlcut_maintenance release package TARGET --output /owned/native-packages
python3 -m tools.htmlcut_maintenance release accept --archive ARCHIVE --target TARGET --version VERSION
python3 -m tools.htmlcut_maintenance release checksums --version VERSION --dist /owned/complete-assets
```

## Publication and verification

`release publish TAG --dist DIR` validates tag/source/version and immutable metadata,
notes, inventory, sizes and bytes before converging draft assets. A matching published
retry performs no upload or edit; missing or mismatched published bytes refuse. Draft
publication cannot downgrade a newer stable release. This standalone command does not
rerun `identity --require-ci`; perform that admission explicitly or use the workflow.

`release verify TAG --target TARGET --output NEW_DIR` verifies published metadata,
downloads public bytes anonymously, checks the complete checksum inventory and exact
archive digest, verifies source-bound GitHub attestation, and executes that downloaded
native binary. It does not rebuild a replacement. Run it from the matching source
checkout on the appropriate native host, with a new output directory.

All maintained native lanes and exact-source/public-byte execution are required release
evidence. Cross-compilation and translated execution do not establish a native-host pass.
Crates.io publication is a separate operation; when publishing the product crates, use
the dependency order **core → runtime → CLI**. The GitHub workflow does not publish them
to a registry.
