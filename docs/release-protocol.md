# Release protocol

CI verifies direct stable checks, the published-core API baseline, bounded nightly
libFuzzer smokes and all four native packages. Required `Check` rejects failed,
cancelled, missing or skipped mandatory jobs. Native package evidence must execute
the unpacked binary, worker containment, file/pipe/error, Unicode and signature
controls; cross-compilation cannot establish native enforcement.

There are only two distributable project packages: core and CLI. After separately
authorized publication, publish core first, then CLI once its exact core dependency
is available. Use `cargo publish --locked --dry-run -p NAME`, inspect normalized
archives, then publish without `--no-verify`. Verify official package/owner metadata,
a clean external registry-only core consumer and a fresh locked CLI install.
A local package or registry simulation does not establish official publication.
Previously published fork crates remain historical artifacts; the current graph
has no dependency on them and no fork publishing sequence.

## Reviewed source and native verification

Push a reviewed branch and create a PR against main. Require completed checks at
the exact reviewed head and resolve actionable reviews. Do not bypass branch
protection or infer main verification from a branch whose tree happens to match.
After a separately authorized merge, inspect push-event CI for the resulting main
commit and bind fresh source/native evidence to that exact SHA.

`build-source-archives.sh REF` archives committed source, not a moving working tree.
`build-release-artifact.sh TARGET` requires a clean source/version, generates full
locked target and compiler attribution, and signs staged macOS executables ad hoc.
`native-package-evidence.py` binds hashes to actual unpacked execution. Preserve
logs and failed controls; a checksum establishes identity, not review quality.

## Publication boundary

Tagging and publishing a public release require separate owner authorization.
No redesign PR authorizes either action. Only after exact-main verification and
that authorization create the new annotated version tag once. The Release workflow
resolves one immutable source, checks same-source main CI, serializes publication,
generates attribution/checksums/build provenance and refuses replacing public bytes.
Its native jobs download published packages anonymously, verify provenance and
execute those downloaded bytes on Apple ARM/Intel, Linux musl and Windows MSVC.
Do not move tags, replace public assets or claim post-publication verification before
these native jobs succeed. Retain published release history and license grants.

Ad-hoc macOS signatures check sealed-code integrity; they supply neither Developer
ID identity nor Apple notarization and do not guarantee Gatekeeper acceptance.
See [platform guidance](platform-support.md). Checksums/provenance remain separate
reference checks. Changes to remote protections require their own authorization.
