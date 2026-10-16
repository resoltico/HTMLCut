# Release operations

Publication requires an existing annotated stable tag, version agreement with its immutable source manifest, main ancestry and successful latest eligible main push CI for that exact source. [The release workflow](../.github/workflows/release.yml) resolves one SHA and carries it through source archives, all native packages, checksums, notes, provenance and public-byte verification. [The target registry](../tools/release-targets.toml) derives the matrix and asset names. Credentials are not persisted in checkouts; publication is serialized across tags.

Before creating a release candidate, run the full [contributor gate](../CONTRIBUTING.md), the 10,000-run two-target fuzz smoke and full product mutation campaign. Review all survivors and timeouts against the current source. Focused native runtime/CLI campaigns do not establish full workspace review. CI does not mechanically establish human survivor review.

Inspect and build all publishable crates together with the pinned Cargo, in a clean immutable snapshot:

```sh
cargo package --list -p htmlcut-core
cargo package --list -p htmlcut-runtime
cargo package --list -p htmlcut-cli
cargo +1.99.0 package --locked -p htmlcut-core -p htmlcut-runtime -p htmlcut-cli
```

Coordinated packaging verifies unpublished siblings through Cargo's temporary registry. `--no-verify` is not package-build evidence. Source archives include runtime, maintenance sources/config/tests, literal fixtures and licenses. Native archives contain only the executable, target guide and legal files and require no Python at runtime. Original attribution and exact locked native/runtime terms remain in NOTICE.

Canonical operations:

```sh
python3 -m tools.htmlcut_maintenance release identity vMAJOR.MINOR.PATCH --require-ci
python3 -m tools.htmlcut_maintenance release source SOURCE_SHA --output /owned/source-archives
python3 -m tools.htmlcut_maintenance release package TARGET --output /owned/native-packages
python3 -m tools.htmlcut_maintenance release accept --archive ARCHIVE --target TARGET --version VERSION
python3 -m tools.htmlcut_maintenance release checksums --version VERSION --dist /owned/complete-assets
```

Native package builds require a clean source checkout and pinned cargo-about. Acceptance validates exact layout, modes and legal material, verifies Apple signatures before execution, runs real signature rejection controls, then executes the common Rust suite against the unpacked binary, including ignored native/performance controls. The standalone executable is exercised outside the checkout. Missing required ACL/label/pipe/signature capabilities fail acceptance.

Creating tags, publishing, repository-setting changes, merging and registry publication require separate authorization. Registry publication order is **core → runtime → CLI**. Local package builds do not establish external release success.

When separately authorized, `release publish TAG --dist DIR` converges draft assets only after checking exact metadata, notes, inventory, size and bytes. A matching published retry performs no upload/edit; mismatched or missing published bytes refuse. Draft publication cannot downgrade a newer stable release. `release verify TAG --target TARGET --output NEW_DIR` verifies published metadata, downloads public bytes anonymously, checks the complete checksum inventory and exact archive digest, verifies source-bound GitHub attestation, and executes that downloaded native binary. It never rebuilds a replacement. All four native lanes and exact-source/public-byte evidence remain authoritative external gates.
