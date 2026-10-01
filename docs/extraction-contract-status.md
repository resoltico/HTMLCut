---
afad: "4.0"
version: "15.0.0"
domain: ENGINEERING
updated: "2026-10-01"
route:
  keywords: [HTMLCut, implementation status, conformance, live QA, dependency refresh, release candidate]
  questions: ["What remains before HTMLCut 15.0.0 publication?", "Which revision was verified?", "Do previous proofs cover the dependency refresh?"]
---

# Extraction contract implementation status

D1–D10 are implemented together. The implementation authority is
[extraction-contract-spec.md](extraction-contract-spec.md); the supported product is one
immutable-snapshot Rust core and the `htmlcut` binary. The old CLI library, reader heuristics,
interop runtime and command aliases are removed.

Implementation and configuration work are complete. The release process validates the refreshed
source and new packages through the required gates, source-bound mutation accounting and native
proofs. Authoritative publication state is the [public release](https://github.com/resoltico/HTMLCut/releases/tag/v15.0.0);
exact-source verification is retained in [GitHub Actions](https://github.com/resoltico/HTMLCut/actions).
The historical evidence below identifies its tested revision; it does not attest later changes.

## Verified implementation before the refresh

The clean verified revision was `fd96d8e842589b4b5a68b300db275c946599777c`, on the integrated
`codex/extraction-contract-15` branch and PR #91. It completed:

- All 58 maintainer steps, including the genuine 100% scored executable-line and branch coverage
  ledger, private fault injection, parser/selector boundary tests, docs/schema checks and Miri.
- Seven live fuzz targets, 200 executions each, with all 14 fuzz-smoke steps passing.
- Exact full mutation accounting: 3,470 planned/tested, 1,906 caught, 1,564 unviable, zero missed
  and zero timeouts, with all 16 shard baselines successful. PR-diff mutation also passed.
- Native package smoke on macOS arm64/x64, Linux x64 musl and Windows x64, plus required CI.
- Five complete-value equivalent-task comparisons, named-tokenizer evidence and prepared-document
  reuse timing, with caller mapping measured separately.

On 1 October, a freshly built and packaged macOS arm64 binary passed 214 live/adversarial cases
and a second 214-case captured replay. Targets included MDN, Microsoft Learn, Wikipedia, Python
technical documentation and Books to Scrape. Independent full values, preformatted blocks and
href arrays agreed; the eight identifiers omitted by v14 survived both text projections.
Seeded class/ID/hidden variants, exact source slices, context guards, strict decoding, loopback
HTTP faults, pagination, terminal/pipe parity, publication faults and an eight-process no-clobber
race passed. No product remediation was needed in that loop; mistaken harness expectations were
corrected with their failed rounds retained.

The portable live matrix, methodology, native package, binary, captures, scripts and retained
proofs are in the maintainer's `Downloads/HTMLCut-15.0.0-live-qa-2026-10-01` directory. That local
archive is supporting evidence, not a runtime dependency or a public install endpoint.

## Changed candidate and next release work

The [configuration/workflow audit](configuration-audit.md) follows the separate design/QA pass
through dependency employment, complete freshness inventory and scratch ownership, same-source
CI/mutation proof, native evidence and immutable release asset verification. Its regression tests are included in the release gates.

The [dependency refresh record](dependency-refresh.md) identifies the new upstream parser/selector
versions, refreshed registry lockfile, current Rust pins and infrastructure dependencies. The
resource/provenance hooks and diagnostic locations have been carried onto those sources;
their behavior is validated by the release process. Documentation now describes only the
current execution contract and correct package replay commands.

| Decisions | Implementation | Verification after refresh |
|---|---|---|
| D1–D2 | Pure snapshot core, CLI acquisition, one closed execution contract and lazy reuse | Release gates |
| D3–D4 | Literal/structural projections, explicit exclusions and byte-exact source slicing | Release gates and fidelity replay |
| D5–D6 | Strict selection/guards, compact values, bounded staged publication | Release gates and fault replay |
| D7–D8 | Bounded explicit discovery, immutable identities, caller-owned comparison | Release gates and discovery/identity replay |
| D9–D10 | Resource/safety/deployment safeguards, conformance and useful-task economics | Full release verification on the refreshed revision |

The release process must validate the exact final source and new native packages, including the
maintained gates, affected mutation/platform proofs and the live/captured fidelity matrix. The finalized changelog records 15.0.0 under its dated version section. Do not substitute
an old coverage report, executable hash or CI run for the changed candidate.

Use [Release Preflight](release-preflight.md), [Quality Gates](quality-gates.md),
[Release Publishing](release-publishing.md) and [Release Closeout](release-closeout.md).
