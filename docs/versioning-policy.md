# Versioning

Cargo.toml workspace.package.version is the release authority. Stable publication
uses an immutable annotated tag. Current query and semantics versions are 7;
unknown versions are rejected without adapters. Breaking API/CLI/semantic changes
require a major package release. Published versions, dates, tags and artifacts remain
intact. Semver checks explicitly select htmlcut-core against immutable published
v20.0.0 source pinned in `scripts/contributor-rust-tools.sh` in a disposable materialization.
The tool infers permitted change from versions; patch deletions must fail.
