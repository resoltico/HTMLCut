---
afad: "4.0"
version: "18.0.0"
domain: ENGINEERING
updated: "2026-10-04"
route:
  keywords: [HTMLCut, extraction contracts, snapshots, fidelity, implementation, coverage, release readiness]
  questions: ["What is the current extraction contract?", "What bounds extraction?", "What proves a release is verified?"]
---

# Extraction contract

The current 18.0.0 candidate contract has wire version 4 and semantics 4. Only the current input vocabulary is accepted. This document describes supported product behavior; external implementation handoffs are not repository documentation.

The pure core owns accepted immutable UTF-8 snapshots, bounded lazy preparation, closed plan validation/compilation, original-DOM selection/guards, scalar and record projection, targeted inspection, deterministic data and receipts. The CLI owns regular files/intentional stdin, strict UTF-8, the closed self-contained bundle, framing and staged publication. Callers own acquisition, decoding, browsers, visibility/domain inference, sanitization and comparison policy.

Single/all/nth roots and single/optional/all/nth fields have explicit cardinality and shape. Missing attributes fail even under optional node selection; empty strings remain values. Record candidates remain inside the row while predicates retain charged original context. Guards precede output exclusions/transforms. Compiled selectors/preparation are reused without retaining spent execution budgets. Every semantic failure rejects whole output.

Bare arrays of strings or records are separate from receipts. Receipts bind inputs/policy/semantics/data and complete counts without copied values or external references. Metadata is explicit and normalized; hashes establish integrity rather than origin trust or delivery.

Literal text, conventional Markdown, parsed HTML and exact parse-free source slices have distinct properties. Markdown has normalized prose/blocks, literal source list ordinals, nested-list table rows/cells, protected code and explicit link/image metadata. It does not reconstruct spans/layout, infer boilerplate or use browser visibility. Foreign namespace roles and inherited fragment context remain explicit. See [Core](core.md) for the exact reading and accounting convention.

Bounds cover accepted bytes, parser construction, selector/guard/field/context work, values/cells, compiled patterns, encoded JSON, inspection and replay artifacts. They never authorize truncated successful extraction and do not assert OS CPU/RAM isolation.

Bundles are uncompressed USTAR with exactly three ordered regular members: manifest.json, plan.json, source.html. Raw entry types/names, headers/checksums, declared sizes, padding and full terminal region are validated without unpacking. Source/plan/configuration/receipt are validated, re-executed and compared; no original paths, nested run, environment or network is consulted. Integrity is not a signature. See [CLI](cli.md) for limits, safe errors and delivery boundaries.

All requested artifacts are staged before first commit. Each file is atomically created/replaced, with explicit overwrite; multiple files/stdout are not a transaction. Later failures retain nonzero status and cannot retract already committed evidence or pipe bytes. Real descriptor, file-kind, race and I/O failures must remain observable.

Independent complete-value and parsed-Markdown oracles, constructor/CLI parity, real OS/native packaged controls, strict Miri, live fuzzing, actual scored coverage and exact mutation reconciliation establish assurance only at their executed scope. The [implementation status](extraction-contract-status.md) separates candidate work from immutable published evidence; [Quality Gates](quality-gates.md) defines release verification.
