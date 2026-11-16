# HTMLCut licensing and source

Copyright © 2026 Ervins Strauhmanis. Contributors retain copyright in their contributions.

Original HTMLCut source, tests, tooling and accompanying documentation in this checkout are
licensed under the GNU Affero General Public License, version 3 only (`AGPL-3.0-only`), unless
separate terms are identified for a file. See the unmodified license text in [LICENSE](LICENSE).
You may copy, modify and convey this work under those terms. It comes WITHOUT ANY WARRANTY,
including the implied warranties of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE;
see sections 15–16. Third-party components retain their own licenses and notices. This notice
neither relicenses third-party material nor grants exceptions to its terms.


Adapted selector bindings and serialization portions in `htmlcut-core` retain their ISC
terms and attribution in `crates/htmlcut-core/LICENSE-ISC` and
`crates/htmlcut-core/NOTICE`. Registry dependencies and other third-party components keep
their own terms. This grant does not revoke permissions already received under other licenses.

The generated release workflow retains the upstream cargo-dist template's MIT-or-Apache-2.0
terms; HTMLCut's qualification overlay uses AGPL-3.0-only. Its source attribution and full
permissive license texts are in [legal/cargo-dist](legal/cargo-dist/NOTICE).

Commercial use is permitted by the AGPL. Charging money, internal business use or offering a
SaaS service does not, by itself, require a paid license. Sections 4–6 govern conveyance of
covered source and object code. Under section 13, a modified version that supports remote
network interaction must prominently offer all users interacting with it an opportunity to
receive its Corresponding Source, without charge, through a standard means of copying software.
The required scope follows the license's definitions; it is not automatically every unrelated
system, service or item of business data. Read the license for the actual conditions.

## Source for this build

Release packaging produces `SOURCE.tar.gz` alongside the executable archives. Obtain the
source archive for the same build as the binaries, from its qualified artifact set or
[release assets](https://github.com/resoltico/HTMLCut/releases). It contains tracked project
sources, build and installer inputs, the workspace lockfile and Cargo's vendored locked
dependency sources. Cargo configuration in that archive restores the dependencies without a
registry download. Retain the matching source archive with binaries you convey. The Rust
toolchain and unmodified general-purpose development tools are not included; prerequisites
and build commands are in [CONTRIBUTING.md](CONTRIBUTING.md). Inspect the complete
Corresponding Source requirements for your work.

When conveying modified binaries or deploying a modified work under section 13, replace the
archive and affected notices, include any additional required source/build/install inputs,
and ensure the recipients or interacting users can obtain the corresponding source. An upstream release page, this notice or an unchanged archive does not
satisfy obligations for modifications it omits. Do not include private runtime configuration,
credentials, customer records or database backups in source publications. The archive mechanism
is a delivery aid, not a determination that every deployment meets every license obligation.

Upstream: https://github.com/resoltico/HTMLCut.
For the separate commercial licensing status, see [LICENSE-COMMERCIAL.md](LICENSE-COMMERCIAL.md).

## Contribution terms

By submitting a contribution (code, documentation, tests or other material) to this
repository, you:

- confirm that you wrote it or otherwise have the right to submit it on these terms;
- license it to everyone under this repository's public license, `AGPL-3.0-only`; and
- also grant Ervins Strauhmanis a perpetual, worldwide, non-exclusive, royalty-free,
  irrevocable license to use, reproduce, modify, distribute and sublicense it, and to license
  it under other terms, including the commercial terms described in
  [LICENSE-COMMERCIAL.md](LICENSE-COMMERCIAL.md).

You keep the copyright in your contribution. Do not submit material you cannot license on
these terms; third-party components keep their own licenses. Each pull request confirms these
terms with the checkbox in its template.
