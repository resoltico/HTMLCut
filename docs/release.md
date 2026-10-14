# Release process

Release publication requires an existing validated stable tag and green exact-source main CI. The [release workflow](../.github/workflows/release.yml) binds checkout, native packages, source archives and verification to one immutable source commit. The [target policy](../scripts/release-targets.sh) supplies the complete asset matrix.

Native packages are unpacked and run against [the conformance corpus](../conformance/README.md). Apple signing, complete dependency attribution, checksums and provenance are verified before publication. After publication the workflow checks anonymous downloaded package bytes and runs the same corpus again.

Publishing GitHub Releases and crates.io packages, creating tags, merging and changing repository settings are separate operator actions. Publish htmlcut-core before htmlcut-cli if registry publication is authorized. The core package contains its own example and regression fixtures and depends on registry packages.

The [frozen-corpus policy](design.md#stability) rejects changes to existing contract cases against the latest released tag of the current major. With no such tag there is no baseline; new cases can still be added. Outline format may change in a minor version.

Before a release, run the [resource-bounded mutation audit](../CONTRIBUTING.md) for the full workspace and list every survivor and timeout with its reason in the release pull request. A focused selection, resource refusal or interrupted audit does not complete this review. Investigate observable behavior changes and add regression tests or remove unreachable code before publication.
