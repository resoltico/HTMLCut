# Release process

Release publication requires an existing annotated stable tag and green main CI for that exact source commit. The [release workflow](../.github/workflows/release.yml) validates the tag against the workspace version and main ancestry, then binds checkout, native packages, source archives and verification to one immutable source commit. The [target policy](../scripts/release-targets.sh) supplies the complete asset matrix.

Native packages are unpacked and run against [the conformance corpus](../conformance/README.md). Apple signatures, complete dependency attribution, checksums and provenance are verified before publication. After publication, the workflow downloads package bytes anonymously, verifies them and runs the native corpus again.

The workflow publishes GitHub Releases. Creating tags, merging, changing repository settings and publishing crates.io packages are separate operator actions. Publish htmlcut-core before htmlcut-cli for registry releases. The core package includes its examples and uses registry dependencies; regression fixtures remain in the repository test suite.

The [frozen-corpus policy](design.md#stability) rejects edits and removals in existing contract cases against the latest released tag of the current major. New cases can be added. Outline format may change in a minor version.

Before publication, complete the [full workspace mutation review](qa-resources.md) and list every survivor and timeout with its reason in the release pull request. Focused selections, resource refusals and interrupted runs are incomplete. Investigate observable behavior changes and add regression tests or remove unreachable code before publication.
