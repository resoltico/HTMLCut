---
afad: "4.0"
version: "16.0.0"
domain: DEPENDENCY
updated: "2026-10-01"
route:
  keywords: [dependency refresh, parser forks, Rust pins, registry resolution, release verification]
  questions: ["Which dependencies changed before 15.0.0?", "Why is stable Rust unchanged?", "Has the refreshed candidate passed release gates?"]
---

# Dependency refresh for 15.0.0

The user requested a complete dependency/documentation refresh, allowing breaking upstream
versions, before the release process. At that handoff, build and quality gates were explicitly
deferred; only dependency resolution, metadata reads and formatting had run. Release verification
now validates the resulting source rather than reusing preceding green evidence. See
[implementation status](extraction-contract-status.md) and [release protocol](release-protocol.md).

## Runtime and owned forks

Published versions were read from the [crates.io API](https://crates.io/data-access), selecting
the highest non-yanked stable semantic version, including releases with build metadata. The
owned path packages keep explicit HTMLCut suffixes and their applicable local safeguards.

| Component | Previous upstream base | Refreshed upstream base | Owned package |
|---|---|---|---|
| selectors | 0.38.0 | 0.41.0 | `htmlcut-selectors` 0.41.0-htmlcut.1 |
| servo_arc | 0.4.3 | 0.5.0 | `htmlcut-servo-arc` 0.5.0-htmlcut.1 |
| html5ever | 0.39.0 | 0.40.1 | `htmlcut-html5ever` 0.40.1-htmlcut.1 |
| markup5ever | 0.39.0 | 0.40.0 | `htmlcut-markup5ever` 0.40.0-htmlcut.1 |
| tendril | 0.5.0 | 0.5.1 | `htmlcut-tendril` 0.5.1-htmlcut.1 |
| scraper | 0.27.0 | 0.27.0, already current | `htmlcut-scraper` 0.27.0-htmlcut.8, adapted for the new stack |
| sha2 | 0.11.0 | 0.11.0, already current | `htmlcut-sha2` 0.11.0-htmlcut.1, ARM64 correction retained |
| encoding_rs | 0.8.41 | 0.8.42 | Registry dependency in the adapter and optional fork feature |

The selector/parser rebases include upstream runtime changes rather than only changing manifest
labels. Existing selector work accounting, parser stop/construction hooks, filtered serialization
and strict-provenance corrections remain in the owned sources. The newer selector errors discard
borrowed token payloads; scraper follows that API while retaining qualified-name source locations.
These preservation statements describe the edits, not a new Miri or fidelity proof.

Owned registry requirements were refreshed to current stable releases, including smallvec 1.16.2,
cfg-if 1.0.5, digest 0.11.3, hex-literal 1.1.0, cpufeatures 0.3.1 and criterion 0.8.2. Other direct
workspace dependencies, including clap, serde, serde_json, syn, regex, schemars, toml and ureq,
were already at the current stable releases. For example toml's current published release is
`1.1.6+spec-1.1.0`; the Cargo requirement remains `1.1.6` because build metadata is not a newer
compatibility line.

`cargo update` refreshed the active root lockfile to the newest releases allowed by upstream
dependency requirements and the repository compiler floor. Registry changes include cc 1.5.1,
find-msvc-tools 0.1.14, rustls-platform-verifier 0.7.1 and its Android carrier 0.2.0, siphasher
1.0.4, wasm-bindgen 0.2.129 with js-sys/web-sys 0.3.106, and zerocopy 0.8.59. Transitive version
requirements remain owned by their upstream parents; no unverified global override forces a
different major API underneath them. The inherited inert nested upstream lockfiles were removed
from the active forks, leaving root Cargo.lock as the maintained workspace authority. The frozen
published SemVer baseline remains unchanged.

## Rust and contributor infrastructure

The authoritative [stable manifest](https://static.rust-lang.org/dist/channel-rust-stable.toml)
still publishes Rust **1.98.1**, dated 3 September. The development pin and workspace floor already
match it; there is no newer stable release to invent. The owned SHA-2 carrier's manifest floor
now also matches 1.98.1 so workspace resolution does not unnecessarily select older dependencies
for its former upstream 1.85 floor.

The QA nightly advances from `nightly-2026-08-25` to **`nightly-2026-09-30`** (Rust 1.101.0-nightly).
The [dated manifest](https://static.rust-lang.org/dist/2026-09-30/channel-rust-nightly.toml)
advertises Miri and LLVM tools for both maintained Miri hosts, plus rust-src. The canonical script,
tooling messages, test fixtures and active docs now agree on that pin. Its actual repository proofs are part of the release verification record.

The eight pinned Cargo contributor tools were already at their latest stable published releases:
nextest 0.9.146, audit 0.22.2, deny 0.20.2, semver-checks 0.50.0, outdated 0.19.0,
llvm-cov 0.9.1, fuzz 0.13.2 and mutants 27.1.0. Existing verified Nextest checksums remain valid.

The CI install action advances to **v2.87.22**, pinned to its release commit. Checkout,
upload/download-artifact, attest-build-provenance and rust-cache pins already match their latest
stable releases. The npm devcontainer CLI advances from **0.88.0 to 0.89.0** (Node >=20).
The Docker CLI helper advances from **29.7.2 to 29.8.2**, pinned to the verified
multi-platform image digest. The supported Ubuntu 24.04 devcontainer image's current index digest
already matches the repository pin; the supported OS/runner matrix remains defined by the
platform and contributor guides.

Evaluation dependencies are recorded in [evaluation/requirements.txt](../evaluation/requirements.txt):
Beautiful Soup 4.15.0, lxml 6.1.3 and tiktoken 0.14.0, all current published releases. Historical
measurements retain their recorded tool versions and exact binary/source identities.

## Release handoff

README, contributor/workspace/quality guides, release replay commands and the implementation
status describe the v15 contract. The authorized release finalization records a dated 15.0.0
changelog entry; publication is still controlled by the release workflow and actual public object. The release process must validate the final refreshed source, new
packages and dependency graph. See [implementation status](extraction-contract-status.md),
[release preflight](release-preflight.md) and [patch provenance](../patches/README.md).
