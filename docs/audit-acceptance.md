# Field audit implementation acceptance

The immutable handoff is `/Users/erst/tools/htmlcut-field-audit-2026-10-14.tar.gz`, captured from source `123144b108da2226dfd52a4d779b0c88251c155c`. Candidate results use separate owned storage; the extracted baseline checksum manifest was verified with zero mismatches. Hashes establish artifact identity, not review quality.

The chosen engine replaces the coupled scraper parser/tree with private admitted builder/frozen DOM and maintained html5ever 0.40.1 plus selectors. No parser carrier fork, compatibility backend or migration path remains. Source-node admission precedes allocation; transient depth and the per-source `reading_bytes` budget are intentional API contracts. Exact canonical JSON admission occurs during transformed reading production. Input/output/reading/node limits remain logical bounds, not physical heap guarantees.

## Reviewed expectation changes

| Case | Candidate contract/result | Reason |
|---|---|---|
| Corpus 42 heading | Explicit `select: ":scope"`; literal heading values retained | Omitted selection now inherits the whole section |
| Corpus 43 term | Explicit `select: ":scope"`; literal term values retained | Same inherited-range contract |
| Corpus 44 key | Explicit `select: ":scope"`; literal narrowed keys retained | A key pattern applies to the anchor rather than section body |
| Field extent-sibling-selection title | Explicit `:scope`; original business heading retained | Approved authoring update |
| Field extent-anchor-default-design | Whole inherited section | Approved omitted-selection semantics |
| Field extent-parent-boundary-design | Child body ends inside each inherited row | Approved containment fix |
| Field extent-no-stop-design | Natural inherited end succeeds | The audit's proposed required-stop refusal was explicitly rejected by final review |
| Field SVG lowercase document claims | Count refusal; exact parsed-case controls succeed | Documentation correction, not matching relaxation |
| Field raw-newline-array-design | `unrepresentable_output`, exit 3, no publication | Approved LF-list framing contract; multiline scalar remains valid |
| CLI FIFO positive control | Peer readiness within the invocation deadline | Input waiting now participates in timeout; new unconnected negative controls require exit 4 |
| CLI charset metadata | Successful complete `"ok"` output | Fixed maintained parser, not a caught panic |

`field-semantics.py` has explicit candidate overrides for these field cases. Baseline matrices remain immutable and their mismatches are retained separately. Snapshot outer-HTML literal oracles were captured from the source-built baseline and retain independently fixed bytes after the upstream wrapper is removed. Public parser-admission consumers and malformed-tree literals challenge the actual new sink, including refusal under `panic=abort`.

## Implementation QA and native verification

Separate adversarial review demonstrated two staging failures in intermediate candidates: pathname substitution could publish substituted bytes, and a handle obtained before restriction retained access to later result bytes. Those failing probe records are preserved. The root-cause revision creates Unix result files at mode zero inside a verified directory with inherited access/default ACLs removed, and Windows result files with an atomic protected writer-only DACL, followed by verified empty DACL installation through the retained writer handle. Ordinary/timer cleanup and commit revalidate owned stage identity. Target policy/identity and original path resolution are rechecked; final namespace races remain explicit boundaries.

Adversarial native rechecks rejected actual stage substitution, preserved destination bytes and foreign replacements, and could not acquire a data reader before restriction. A separate inherited-directory listing denial exposed an empty-directory cleanup failure on Darwin; search-only handles fix that case and native acceptance retains the denying control. If creation succeeds but access denial or a failed identity syscall prevents confirming ownership, safe pathname-only deletion is unavailable: the refusal names possible empty residue. This is a cleanup boundary, not successful publication.

Deterministic unit barriers exist only in the unit-test binary; they exercise native files at reservation, creation, restricted write, final policy and either side of commit. Production binaries expose no test modes. Delivered-package `native-acceptance.py` records real lifetime, diagnostic, representation and native access probes, with missing prerequisites classified as unavailable and failing acceptance. GitHub's four native lanes execute these scripts against the extracted signed/package binary and retain hashes and case counts. Cross-compilation is only a compile check.

Windows final ACL reproduction uses one handle-based full-descriptor setter. Native readback showed that `SetSecurityInfo(SE_FILE_OBJECT)` with unprotection added inherited ACEs to a pinned explicit ACL, so ignoring control flags would conceal genuine drift. Microsoft [discourages `SetKernelObjectSecurity` for filesystem objects](https://learn.microsoft.com/en-us/windows/win32/api/securitybaseapi/nf-securitybaseapi-setkernelobjectsecurity) in favor of the inheritance-aware setters; this is an explicit deviation for exact reproduction on an already created regular staging file, which has no child objects. It requests only owner/group/DACL/mandatory-label components, retains exact readback, and requires independent native protected/unprotected parent-rule propagation and actual allowed/denied read controls. No audit-SACL backup or compatibility path is introduced. Native execution must establish this behavior before acceptance.

The full mutation campaign in the handoff is superseded by the user's instruction **not to run full campaigns in this task**. Only tiny mutation controls and very short fuzz verification are authorized here; they do not establish full mutation assurance. [Resource limits](qa-resources.md) distinguish measured peaks and sampled watchdog thresholds from enforceable filesystem quotas. Neither owner caches nor pre-existing archives are cleaned.

## External rollout

No merge, tag or publication is part of this task. Core 22 is unpublished, so a registry-only CLI 22 installation cannot be established yet. Extracted CLI validation uses only a command-scoped override to the exact extracted core archive, without changing production manifests or lockfiles. Publishing core before registry-only CLI resolution is a separate owner action.
