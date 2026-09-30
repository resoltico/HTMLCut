---
afad: "4.0"
version: "15.0.0"
domain: ENGINEERING
updated: "2026-09-30"
route:
  keywords: [HTMLCut, implementation status, conformance, extraction, evidence]
  questions: ["What remains for HTMLCut 15.0.0?", "Which extraction-contract proofs have run?"]
---

# Extraction contract implementation status

Authority: [extraction-contract-spec.md](extraction-contract-spec.md).
Starting/current committed revision: `b52ddadaab0c03a36a64de86e28b016ab56f6ac1`.
Branch: `codex/extraction-contract-15`. Starting worktree was clean; origin/main was fetched and equals the starting revision.
Current work unit: W2 foundation checkpoint established; W3/W4 fidelity, adapters and conformance remain in progress. W5 consumer/deletion/verification integration is next.
Release status: not implemented; no merge/tag/publication authorized or performed.

| Decision | State | Owner and next proof |
|---|---|---|
| D1 | in progress | Move source loading/strict decoding to CLI acquisition; core snapshot module has no I/O. |
| D2 | in progress | Replace contracts/interop/wire with plan, snapshot, execution and result modules; CLI binary only; repair xtask callers. |
| D3 | in progress | Projection modules: iterative literal text and bounded structural text; retire reader policy/vocabulary. |
| D4 | in progress | Source-only compiled boundary matching and exact UTF-8 ranges. |
| D5 | in progress | Selection and guard modules, shared per-operation work counters. |
| D6 | in progress | Core compact result; adapter bounded JSON/raw staging and publication. |
| D7 | in progress | Core bounded discovery descriptors/cursors and explicit proposals; CLI operation metadata. |
| D8 | in progress | Domain-separated length-framed identity; immutable prepared snapshot/cache. |
| D9 | in progress | Limits registry; maintained parser construction hooks and fallible selectors; I/O fault paths. |
| D10 | in progress | Offline conformance corpus/economics; docs/gates/release-source repairs and complete final evidence. |

T01–T40: not fully verified. Initial core unit suite: 20 passing tests for closed JSON/schema defaults, exact A–F fidelity in both projections, hidden/template/script distinctions, structural lists/tables/preformatted content, original-DOM guards/exclusions, transforms, complete counts, exact slices, lazy reuse/failure caching, identity sensitivity and pagination/preview boundaries. SemVer classifier: 3 targeted xtask tests passed before switching the core surface. Nine CLI unit tests passed: JSON/raw/error/slice, HTTP/stdin/library parity, GET/query/charset, gzip/chunking, strict BOM/decoding/expansion, no-clobber race, symlink refusal, read/write/flush failures and diagnostic redaction. Four direct scraper construction-boundary tests passed (including deep-tree and malformed/template/foster-parenting cases). The core library passes Clippy with warnings denied. Normal production dependencies under all product features exclude HTTP, decoding and adapter/tooling crates. Core identity tests also pass with serde_json/preserve_order enabled by a downstream consumer. No coverage, mutation, Miri, fuzz or platform proof claimed.

## Module design and retention/deletion map

- Core `snapshot`: immutable accepted UTF-8/base metadata/preparation limits and lazy DOM cache; cache failures too. Replace core acquisition and old prepared-input envelopes.
- Core `plan`: closed serializable plan/selection/projection/guard/transform types, recursive duplicate-key detection, shared validation, bounded compilation; replace old request/interop translations.
- Core `limits`, `identity`, `result`: one typed error family, deterministic compact result, canonical plan and source identities. Retain SHA-256 and schema-generation libraries.
- Core `execution`: fresh shared operation counters, complete candidate enumeration, slice pairing and original-DOM guards. Retain patched selector grammar/work accounting; remove first-selection default and alternate engines.
- Core `projection`: literal template-aware DOM text, structural rendering, filtered HTML serialization, attributes; immutable exclusions and explicit transforms. Retain useful renderer/selector tests and port them; delete heuristic reader signals and detached comparison canonicalization.
- Core `discovery`: bounded descriptors/preview/handles/cursors; retain proven proposal/evidence logic by responsibility and port its tests.
- CLI `acquisition`, `args`, `publication`, `run`, `metadata`: own all source I/O, strict decoding, private deterministic I/O seams and bounded output; remove supported CLI Rust library and old serializers/flags.
- xtask docs/schema/examples: core metadata plus bounded once-built CLI invocation, replacing direct CLI-library imports. Update source-shape, coverage, mutation, doctest, benchmark, fuzz and Miri consumers when moved.
- Retain `patches/rust/` safeguards, native target registry, artifact-root policy, unrelated operational/legal docs and immutable `semver-baseline/htmlcut-core/`.
- Repair SemVer numeric classification, always-required Linux maintainer CI and immutable release-source/changelog notes before final campaigns. Keep all real scored coverage/mutation obligations.

## Design checkpoint

One plan selects/projects once. Default single/dom_text is literal; document_text is opt-in. Slice never prepares DOM. Original-DOM guards are conjunctive and all matched values satisfy their predicate. Exclusions cannot weaken guards. Success counts require complete enumeration. Core identity excludes adapter acquisition/framing policy. Each execution has fresh shared counters; prepared limits are immutable. No unrequested projection, compatibility engine, public parser type, or publication action is introduced.

Selector context: use the maintained engine's original DOM, so ancestor combinators may consult outside ancestors; reads/removals are restricted to the selected subtree. Explicit :scope selects its root. Freeze exact rendering, guard examples and new finite budgets before their implementation closes W1.

## Environment, evidence, and next action

Host: macOS ARM64; stable 1.98.1 and nightly-2026-08-25 installed; approximately 405 GiB free at baseline. Existing launcher owns artifact roots. Required future evidence: retained gate-run reports with tested source revision/input hashes and portable summaries here.
Baseline A–F reproduced using a freshly built 14.0.0 CLI: A/C/E controls preserved content; B omitted IMPORTANT, D omitted the policy section and following text, F omitted Broken mirror; all exited 0. Report: `../.htmlcut-artifacts/gate-runs/implementation-15/baseline/fidelity.json` (binary/input hashes and commands preserved).
Unresolved: the core and binary CLI build and the xtask library checks; old integration tests, examples, fuzz consumers and documentation still need porting. Delete disconnected old code only after accounting for tests. New parser hooks, scoped selectors and streaming serialization require direct fork tests and coverage/mutation ownership updates.
Next: port xtask docs tests and add CLI HTTP/strict decoding/publication fault assertions, then update source ownership and close affected coverage. The CLI Rust library is disabled and xtask production docs consumers now invoke a once-built binary through bounded subprocess capture; Eleven targeted docs-command tests now pass against the once-built binary, including execution of saved runs and verification of output/audit artifacts. Other consumers still need porting.

Frozen new core budgets: plan 256 KiB; patterns 8 KiB; JSON/CSS/regex nesting 64; aggregate regex programs/DFA caches 8 MiB each, partitioned deterministically across the plan's regexes; 32 guards and 32 exclusions; max 2 distinct transforms. Preparation additionally bounds all nodes at 1,000,000 (hard 4,000,000) and construction/attachment work at 10,000,000 (hard 100,000,000). Fixed internal parser sentinel counts in the node budget, not the selected DOM.

Renderer fixtures use single LF block separators, `[label](destination)` with reversible backslash/bracket/parenthesis escaping, no added final LF, explicit `[table]`/`[/table]` nesting and `[header]`/span annotations. Lists use two-space depth indentation; ordered list start/value/reversed rules are explicit. Source whitespace normalization collapses runs to one space without trimming, preserves preformatted origin and never touches destinations. More structural edge fixtures remain mandatory before D3 is verified.

## Checkpoint evidence and outstanding verification

The first foundation checkpoint is deliberately incomplete. The workspace version is 15.0.0, core acquisition dependencies are removed, the new core API is the only supported execution surface, and the CLI is a binary with private seams. Disconnected old source/tests/docs still exist pending the deletion/port ledger; their presence is not a passing inventory or compatibility promise.

Evidence root: `../.htmlcut-artifacts/gate-runs/implementation-15/`. Retained initial logs: `core-tests.stdout.log`, `cli-tests.stdout.log`, `semver-tests.stdout.log` and paired stderr logs; `core-clippy.stderr.log`; `boundary/core-dependencies.txt`; `baseline/fidelity.json`. These are targeted development checks, not the full maintainer gate.

Source ownership: new modules have responsibility-based rules, with existing core cohesion ceilings retained. The two new parser/serializer boundary modules are added to the maintained scored and mutation inventories; fixture inventories and complete coverage still need reconciliation. Prior successful structure report: `../.htmlcut-artifacts/gate-runs/run-1790759779787-53897-0/report.json`; later changes require a fresh final proof.

Remaining priorities: port existing examples, integration tests, fuzz/Miri/benchmark and docs consumers; complete saved-run environment/redirect/deadline/fault and all-budget boundary assertions; account for and delete superseded live code; repair required Linux CI and immutable-source/changelog packaging; close genuine 100% coverage and mutation outcomes; run the complete conformance/economics/native package campaign on the final candidate. Do not mark any D/T family verified solely from this foundation checkpoint.
