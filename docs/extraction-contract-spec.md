---
afad: "4.0"
version: "16.0.0"
domain: ENGINEERING
updated: "2026-09-30"
route:
  keywords: [HTMLCut, extraction contracts, snapshots, fidelity, implementation, coverage, release readiness]
  questions: ["What must the HTMLCut 15.0.0 implementation deliver?", "How must the implementing agent resolve failing quality gates?", "What proves that all ten extraction decisions are complete?"]
---

# Extraction contract

This document describes the supported 16.0.0 candidate contract. Wire-family version is 2 and extraction semantics is 2. Only the current contract is accepted; old envelopes and versions are rejected. The external implementation handoff is not repository documentation.

## Ownership

The pure core accepts immutable UTF-8 snapshots and explicit metadata, validates/compiles plans, prepares a bounded DOM lazily and executes one selection/projection. It performs no filesystem, network, environment, clock, process or terminal I/O. The CLI owns acquisition, decoding, saved-run references, framing and atomic single-file publication. Browser execution, domain mapping, visibility inference and sanitization belong to callers.

## Closed plans and original-DOM expectations

Every tagged object is closed, including variants without data members. Duplicate keys are rejected before map conversion. Required/current versions, type/bounds, semantic applicability, CSS/regex grammar and original-DOM expectations are enforced at their owning boundary. Schema-valid shape alone does not authorize source-dependent success. Invalid configuration does not consume the source or publish artifacts.

Default single selection requires exactly one candidate. All and positive one-based nth are explicit. Guards read original DOM values before exclusions/transforms. Exclusions affect only selected output subtrees and never mutate the prepared document or alter candidate/guard interpretation.

## Projections

Literal dom_text concatenates parsed text without invented separators, including hidden and script/style/template payloads. Structural document_text preserves the declared heading/block/list/link/alt/table/pre convention and excludes only script/style/template payloads. Original pre ancestry protects normalized descendants. Ordered-list ordinals derive from original list membership/start/reversed/value resets; exclusions do not renumber surviving items.

Structural tables have balanced [table], [caption] and [cell] frames. Cell separators occur outside cell frames. Reserved source delimiters outside generated pre fences are escaped; code inside fences stays literal. Selected fragments carry only required context, never outside payload. Definition-list, details/summary and address units have explicit block boundaries. See [Core](core.md) for encoding and [CLI](cli.md) for delivery.

Inner/outer HTML serialize the parsed DOM, not original source spelling. Source slicing is a distinct parse-free strategy: exact accepted UTF-8 substring, half-open byte ranges, nonoverlapping literal/bounded-regex boundary pairs, explicit inclusion flags, forward progress and whole-operation rejection for missing/empty matches.

## Bounds, reuse and identity

Preparation and execution limits are finite and typed. Each execution receives fresh counters shared across selection, guards, exclusions, context and projection. Scoped selector scratch is reused within a pass and discarded; all relevant inner walks/predicates and initialization are charged. Sticky exhaustion is failure rather than a non-match. Source/context/serialization limits do not silently truncate successful values. Logical bounds and finite watchdog observations are not OS CPU/RAM isolation guarantees.

Source, normalized-plan and extraction identities use deterministic domain-separated hashing. Source identity reflects accepted source bytes; plan/extraction/discovery identities reflect their current contracts and semantics. Hashes identify data, not origin authentication or semantic correctness.

## Acquisition and publication

File-path source/plan/run inputs require regular opened files and reject special-file waits; intentional streams use stdin. HTTP is bounded GET with strict complete interim/final interpretation, metadata parameter grammar, explicit encoding precedence and full-representation acceptance. It rejects unsolicited partial/switching/malformed responses and retains actual TLS verification.

Results contain requested values, complete counts and identities. Default JSON has one framing LF; raw is one value with no added LF. Validation/acquisition failures emit no successful stdout. Real output I/O must report delivery failures. Unused stdout is not a prerequisite for file-only or empty-raw output. Bytes already delivered before a pipe failure cannot be retracted. Atomic file replacement/create does not make several outputs one transaction.

Errors have the current closed family, broad code/exit class, stage, bounded safe message and only known cause/evidence facts. They do not expose raw transport chains, supplied values, source, paths, URLs, headers or environment contents.

## Assurance

Independent complete-value and malformed-input oracles, raw protocol/real-OS tests, source-bound native packages, fuzz/Miri, exact mutation reconciliation and genuine scored line/branch coverage establish evidence at their actual scope. Observational capture and integrity hashes alone are not product conformance. See [Quality gates](quality-gates.md), [Schemas](schema.md) and [Versioning policy](versioning-policy.md).
