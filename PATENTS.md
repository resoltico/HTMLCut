<!--
AFAD:
  afad: "4.0"
  version: "18.0.0"
  domain: LEGAL
  updated: "2026-10-04"
RETRIEVAL_HINTS:
  keywords: [patents, patent grant, apache-2.0, mpl-2.0, mit, isc, ncsa, dependency licenses]
  questions: [what is HTMLCut's patent posture?, which dependency license families include explicit patent grants?, where should I look for legal attribution?]
  related: [README.md, NOTICE, deny.toml]
-->

# Patent Notes

HTMLCut's original code from version 18.0.0 is licensed under MPL-2.0.
Section 2.1(b) grants contributor patent rights within the scope defined by
sections 1 and 2.3; section 5.2 provides a patent-litigation termination rule.
These are the standard license terms, not a separate patent covenant.

## Dependency Patent Grants

HTMLCut allows third-party dependency licenses through `deny.toml`, and those SPDX license families
have different patent postures. The exact dependency inventory for a given release lives in
[NOTICE](NOTICE) and `Cargo.lock`; this note explains the policy implications rather than trying to
freeze a crate-by-crate list in prose.

| License family | Explicit patent grant | Notes |
|:---------------|:----------------------|:------|
| MIT | No explicit grant | Applies to permissively licensed dependencies and earlier published HTMLCut material. |
| Apache-2.0 | Yes | Section 3 grants patent rights from contributors to their contributions. |
| MPL-2.0 | Yes, scoped | Section 2.1 grants patent rights within the scope of the covered files. |
| ISC | No explicit grant | Plain permissive grant, no standalone patent clause. |
| BSD-3-Clause | No explicit grant | Plain permissive grant, no standalone patent clause. |
| NCSA | No explicit grant | University of Illinois/NCSA terms grant broad copyright permissions but do not add a standalone patent clause. |
| Unlicense | No explicit grant | Public-domain-like or permissive terms without a dedicated patent clause. |
| Unicode-3.0 | No explicit grant | Data-license terms, not a patent grant. |

Apache-2.0 includes an explicit patent grant in Section 3 from each contributor
to the covered code. MPL-2.0 includes a patent grant in Section 2.1
scoped to the licensed files.

## Repository-Level Patent Posture

This repository does not publish a separate project-level patent license,
retaliation clause, or patent non-assert covenant beyond:

- HTMLCut's own MPL-2.0 license
- whatever patent terms are present in the allowed third-party dependency licenses

Earlier published HTMLCut versions retain their MIT terms. Upstream dependencies
retain their own license/patent terms; the project license does not relabel them.

## Legal Disclaimer

This document is informational only and does not constitute legal advice. For
patent-related concerns, consult qualified legal counsel.
