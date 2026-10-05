<!--
AFAD:
  afad: "4.0"
  version: "19.1.0"
  domain: LEGAL
  updated: "2026-10-05"
RETRIEVAL_HINTS:
  keywords: [patents, patent grant, apache-2.0, mpl-2.0, mit, isc, ncsa, dependency licenses]
  questions: [what is HTMLCut's patent posture?, which dependency license families include explicit patent grants?, where should I look for legal attribution?]
  related: [README.md, NOTICE, deny.toml]
-->

# Patent Notes

HTMLCut's original code is licensed under MPL-2.0. Section 2.1(b) grants rights
under each Contributor's Patent Claims for its Contributions or Contributor
Version. These terms are defined in section 1; section 2.3 limits the grant.
Section 5.2 provides a patent-litigation termination rule.
These are the standard license terms, not a separate patent covenant.

## Dependency Patent Grants

HTMLCut allows third-party dependency licenses through `deny.toml`, and those SPDX license families
have different patent postures. An allowed family is not proof that a component
using it ships in a package. Each native package’s `NOTICE` contains its generated
dependency attribution and runtime notices. `Cargo.lock` records the workspace
dependency versions; the repository [NOTICE](NOTICE) describes project scope and
source availability, not a complete native dependency inventory.

| License family | Explicit patent grant | Notes |
|:---------------|:----------------------|:------|
| MIT | No explicit grant | The license has no express patent-license clause. |
| Apache-2.0 | Yes | Section 3 grants patent rights from contributors to their contributions. |
| MPL-2.0 | Yes, scoped | Section 2.1(b) grants contributor patent claims, subject to sections 1 and 2.3. |
| ISC | No explicit grant | Plain permissive grant, no standalone patent clause. |
| BSD-3-Clause | No explicit grant | Plain permissive grant, no standalone patent clause. |
| NCSA | No explicit grant | University of Illinois/NCSA terms grant broad copyright permissions but do not add a standalone patent clause. |
| Unlicense | No explicit grant | Public-domain-like or permissive terms without a dedicated patent clause. |
| Unicode-3.0 | No explicit grant | Data-license terms, not a patent grant. |

Apache-2.0 includes an explicit patent grant in Section 3 from each contributor
to the covered code. MPL-2.0 grants contributor patent claims as described above;
its file-level copyright obligations do not imply a blanket patent license.

## Repository-Level Patent Posture

This repository does not publish a separate project-level patent license,
retaliation clause, or patent non-assert covenant beyond:

- HTMLCut's own MPL-2.0 license
- whatever patent terms are present in the allowed third-party dependency licenses

Upstream dependencies retain their own license and patent terms; the project
license does not relabel them.

## Legal Disclaimer

This document is informational only and does not constitute legal advice. For
patent-related concerns, consult qualified legal counsel.
