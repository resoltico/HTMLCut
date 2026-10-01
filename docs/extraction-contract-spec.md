---
afad: "4.0"
version: "15.0.0"
domain: ENGINEERING
updated: "2026-09-30"
route:
  keywords: [HTMLCut, extraction contracts, snapshots, fidelity, implementation, coverage, release readiness]
  questions: ["What must the HTMLCut 15.0.0 implementation deliver?", "How must the implementing agent resolve failing quality gates?", "What proves that all ten extraction decisions are complete?"]
---

# HTMLCut 15.0.0 — integrated implementation work specification

**Executing agent:** GPT-6 Sol, Medium reasoning.  
**Repository:** `resoltico/HTMLCut`.  
**Work product:** one complete, tested replacement architecture and one release-ready integration PR.  
**First release containing this work:** `15.0.0`; no intervening `14.x` release.  
**Specification status:** design reviewed and revised on 30 September 2026; implementation and implementation verification have not been performed by this document. The companion QA files attest to the 26 September revision only; see §15.

**Use of this document:** the imperative language below defines a future implementation handoff. Reading or reviewing this specification does not itself authorize executing it. The current user request determines whether to review the plan, implement it, or perform a release action.

## 1. Mandate, authority, and terminal goal

Implement **all ten decisions D1–D10 below together**. They replace the old product contract, not merely extend it. Breaking CLI, JSON, and Rust API changes are intentional. Do not retain compatibility shims, aliases, migration tooling, an old execution engine, a legacy feature flag, or two supported architectures. Reuse sound internals, safety patches, and relevant tests rather than rewriting working foundations.

The goal is a small, offline-capable, immutable-snapshot extraction core with optional CLI acquisition; one execution language; explicit faithful projections; strict assumptions; compact output; bounded discovery; and reproducible evidence. Do not add a browser, crawler, scheduler, session server, LLM dependency, mapping DSL, semantic inference, persistence engine, or a new public agent protocol.

“One go” means **one integrated delivery and one release boundary**, not one giant commit or an inability to checkpoint. Use one working branch and one PR; dependency-ordered implementation steps and several commits are expected. Do not deliver a subset as a finished PR and defer the remainder to 15.1.0.

This specification is the implementation authority for the ten accepted decisions. Higher-priority user instructions still govern. Repository instructions remain binding where compatible. A test or document that deliberately encodes a rejected old behavior must be rewritten to the new contract; an unrelated quality requirement must not be weakened.

**Completion is release-ready implementation, not unauthorized publication.** Open or update the single PR, prove the specified gates, prepare 15.0.0 artifacts and release metadata, and report the exact validated revision. Do not merge, push a release tag, publish packages, or publish a GitHub Release without explicit user authorization for that action. Preparing an unpublished 15.0.0 version is authorized. An inaccessible publication credential is consequently not an implementation blocker.

If PR creation is genuinely unavailable after the recovery procedure in §13, deliver the complete source archive plus patch/commit history and verification evidence. This changes the delivery transport, not the implementation scope or truth of its verification status.

## 2. Baseline and repository facts to recheck at execution start

The reviewed release is **14.0.0**, commit `a606a959ebab0b6397b47e704967ec5db549d0e3`. Main was rechecked for this specification at **`b52ddadaab0c03a36a64de86e28b016ab56f6ac1`**. Refresh the checkout and record the actual starting commit; do not reset newer user work to these historical references. Read root and nested agent instructions if present. [S1]

| Existing owner | Verified starting responsibility / required consequence |
|---|---|
| Root `Cargo.toml`, `Cargo.lock`, `scripts/workspace-version.sh` | Workspace release version is authoritative. Set the workspace and required path-dependency constraints coherently to 15.0.0; do not maintain a second version registry. |
| `rust-toolchain.toml`, `scripts/contributor-rust-tools.sh` | Reviewed stable pin: 1.98.1; maintained nightly: nightly-2026-08-25. Use the repository's actual pins and bootstrap scripts, not remembered latest versions. |
| `crates/htmlcut-core/src/source/` | Contains filesystem and HTTP acquisition that must leave the core. `http-client` currently enables optional `ureq` and `encoding_rs`. |
| `crates/htmlcut-core/src/contracts/`, `interop/v2/`, `wire/`, `schema/` | Overlapping public contract/translation surfaces must become one contract. Preserve useful compiled/prepared internals, not the adapter hierarchy. |
| `crates/htmlcut-core/src/document/text/` | Reader filtering and caption-sensitive media suppression must be removed from extraction. |
| `crates/htmlcut-core/src/extract/selector/` | Reuse correct lazy projections and bounded selector traversal. |
| `crates/htmlcut-cli/src/args/`, `contract/`, `model/`, `execute/`, `file_output.rs` | Replace old CLI/report semantics; retain thin input/output adapters and testable internals. |
| `xtask/src/docs/` and `xtask/Cargo.toml` | Directly use `htmlcut_cli::run`, `command`, contract registries, and report constants. Removing the CLI library requires replacing these consumers, not leaving a hidden supported API. |
| `xtask/src/coverage/`, `tooling/rust-source-shape-policy.toml` | Coverage inventory, ownership, allowed dependencies, and source-cohesion budgets must follow the replacement modules without excluding their executable logic. |
| `patches/rust/`, `patches/README.md` | Maintained selector/parser resource and provenance fixes are product safeguards; preserve their guarantees. |
| `.github/workflows/ci.yml`, `mutants.yml`, `scripts/xtask.sh`, `check.sh` | Required verification includes actual execution, not merely job configuration. Keep stable launcher and artifact-root discipline. |
| `semver-baseline/htmlcut-core/`, `xtask/src/plan/semver.rs` | Frozen published API evidence; not a supported runtime. Preserve during implementation and repair release-type classification. |
| `scripts/release-targets.sh`, build/smoke/checksum/publication scripts, `release.yml`, `changelog.md` | Preserve the native artifact matrix and changelog authority; bind artifacts to the intended immutable source revision. |

The reviewed coverage policy is **100% executable-line and 100% branch coverage over its maintained scored modules**, not “100% of every dependency.” It includes executable core, CLI, and xtask modules; support-crate tests run too. Existing narrowly defined declarative/test/entrypoint exclusions do not authorize hiding new runtime work there. [S2]

Historical fidelity and token findings justify the redesign; they are not measurements of the new implementation. The earlier experiment was a small engineering evaluation, not an independent market study or universal benchmark. [S3]

## 3. Common execution and wire contract

### 3.1 Ownership and public surface

The only supported Rust product API is `htmlcut-core`. Publish `SourceSnapshot`, metadata and limits, `ExtractionPlan`, opaque `CompiledPlan`, opaque `PreparedDocument`, `ExtractionResult`, and one typed `ExtractionError` family, plus bounded inspection/proposal types. Names may be organized into responsibility-based modules, but do not create version/phase namespaces such as `v15`, `next`, `legacy`, `phase2`, or `migration`.

All product paths implement the same conceptual flow:

```text
CLI acquisition -> accepted UTF-8 snapshot + explicit metadata
                                      |
plan -> validation/compilation -> core execution -> bounded serialization/publication
                                      |
                         prepared DOM, created lazily at most once
```

Library callers supply snapshots directly. The core may allocate and use interior caching; it does not perform filesystem, network, process, clock, environment, or terminal I/O. Runtime DOM/parser types remain private. A checked-in historical SemVer baseline is evidence, never a runtime dependency or compatibility branch.

Use one supported, closed JSON contract family. New names are `htmlcut.extraction.plan`, `htmlcut.extraction.result`, and `htmlcut.extraction.error`, with one family schema-version constant initially `1`. Discovery and saved-run documents have their own role names, not alternate extraction representations. The extraction-semantics constant initially `1` is independently meaningful: changing output/selection meaning changes it; changing packaging alone does not. Do not reuse an old schema identifier with incompatible semantics.

Reject unknown versions, unknown fields, duplicate object keys, invalid enum values, nonintegral/overflowing bounds, and incompatible options. Schema generation comes from serializable public types; runtime validation remains authoritative for cross-field rules. Constructors and deserialization must meet the same invariants. Materialize defaults before canonical plan hashing. Reject obsolete v14 envelopes; do not silently convert them.

Detect duplicate keys while reading the original JSON, including nested objects, before constructing an intermediate map that could discard duplicates. Apply the same closed-input rules to saved runs and discovery requests. Core constructors and wire validation share the same invariant checks; adapter validation adds only acquisition/publication rules.

### 3.2 Normative plan examples

The following is an executable target-format example, not existing v14 syntax. Store it as a fixture and validate it with generated schema and runtime validation:

```json
{
  "schema": "htmlcut.extraction.plan",
  "version": 1,
  "strategy": {"kind": "css", "selector": "#amount"},
  "selection": {"kind": "single"},
  "projection": {"kind": "dom_text"},
  "exclude": [],
  "guards": [],
  "transforms": []
}
```

One plan performs **one selection and one projection**. It does not contain field mapping, joins, loops, executable expressions, or nested domain-record construction. A Rust caller can compile multiple plans and apply them to one prepared document without reparsing.

CSS projections are `dom_text`, `document_text`, `inner_html`, `outer_html`, and `attribute` with an explicit nonempty attribute name. A slice plan uses `strategy.kind = "slice"`, literal or regex boundaries and inclusion flags, and only `projection.kind = "source"`. DOM exclusions, DOM guards, DOM rendering, and URL/whitespace transformations are invalid on a slice. No hidden slice-to-DOM conversion occurs.

Selection kinds are `single` (default), `all`, and `nth` with a **1-based positive** index. `nth(1)` expresses first explicitly; do not retain a separate first-match default. `all` takes explicit optional minimum/maximum count bounds; default minimum is one and the effective maximum is finite. `single` admits exactly one. Positional selection guarantees position, not identity.

Guards are a small list of separately selected checks, not another extraction pipeline. A guard has `scope` (`document` or `selected`), a CSS selector, cardinality constraints, a `dom_text` or `attribute` read, and one exact or regex predicate. `selected` evaluates independently within each selected root, with explicit `:scope` support. Required-attribute checks may apply to selected nodes without a value predicate. Guards operate on the original DOM with literal values, before exclusions or transformations. Regex matching is explicitly search matching; callers use anchors for a full-value match. All guard work consumes the operation's shared budgets.

Each guard defaults to a minimum of one match and a finite effective maximum. After cardinality succeeds, **every** matched guard value must satisfy its predicate; there is no implicit any-match predicate. A missing requested attribute fails even when the predicate would match an empty string. An explicit minimum of zero permits an empty guard set and a vacuously satisfied value predicate; document this deliberate opt-out. Required attributes on selected nodes are checked on every selected node. Multiple guards are conjunctive and any failure rejects the entire extraction. Freeze this vocabulary in schema examples and exact tests before implementing guard execution.

### 3.3 Result, errors, framing

Successful JSON returns only the family/schema and semantics identifiers; source, normalized-plan, and extraction identities; exact `candidate_count` and `selected_count`; an ordered array of requested string `values`; slice byte ranges when applicable; and necessary bounded diagnostics. Do not emit a copy of the plan, source URL, title, full DOM, preview, and all projections for every value. Each slice range corresponds by array position to its value.

For a semantic/validation/limit error, stdout is empty and stderr contains a bounded error document with a stable code, stage, safe message, and applicable identities/count evidence. CLI acquisition/publication failures use the same error document shape without wrapping a core error in a second result. Do not attach unavailable identities or fabricate exact counts.

Use fixed exit classes: `0` success/help; `2` input/plan/schema/options error; `3` unmet selection/attribute/guard/boundary expectation; `4` resource limit; `5` acquisition/decoding/output I/O failure; `6` internal invariant failure. Preserve more-specific typed machine codes within those classes. A nonzero exit is never a successful extraction, even when diagnostics can be serialized.

Default extraction output is compact JSON, identical for terminals and pipes. Serialize deterministic object keys and document-order arrays. JSON has one final framing LF; `--raw` emits the **exact single value bytes with no added LF** and requires exactly one resulting value. A present empty string is a valid value; missing nodes/attributes are not empty-string success. `all` with an explicitly zero minimum may return an empty array; raw output then fails its single-value requirement.

A detailed evidence artifact is opt-in and field-selected. It is bounded and off stdout. Its failure is an operation failure, not a warning that permits success; publication guarantees are described under D9. Operational timings, host paths, and acquisition logs are not part of deterministic extraction results.

## 4. The ten implementation decisions

### D1. Snapshot input; acquisition belongs to the CLI

**Problem/evidence.** Generic core source loading and caller-owned fetching overlap. Externally rendered snapshots already supplied JavaScript-created data without a browser inside HTMLCut. [S3, S4]

**Implement.** Move source loading, charset decoding, stdin/filesystem operations, and HTTP dependencies to the CLI adapter. Eliminate the core's `http-client` feature and all production I/O call paths. Keep HTTP(S) convenience as bounded GET only: no HEAD preflight, retries that silently change meaning, login workflow, or browser fallback.

The core snapshot is the exact accepted UTF-8 string. It preserves CRLF and source spelling; a DOM parser may apply normal HTML parsing rules, but slices address the original accepted bytes. For file/stdin input, default decoding is strict UTF-8; an explicit encoding may be requested in the adapter. For HTTP, use an explicit caller encoding first, otherwise the supported HTTP charset, otherwise UTF-8. Reject malformed byte sequences, unknown encodings, and inconsistent BOM/declared-encoding combinations instead of replacement-character decoding. Strip a recognized encoding BOM only as a documented acquisition step, never silently inside the core. Charset expansion is bounded while decoding. Do not charset-sniff by parsing HTML.

Effective base URL is explicit `--base-url`/caller metadata, otherwise the final HTTP response URL after redirects, otherwise absent. Reject URL userinfo in base metadata. **Do not infer or apply an HTML `<base>` element.** This preserves lazy, source-only operation and makes equal bytes plus equal metadata route-independent. URL resolution is opt-in; without it, preserve relative references as written. A requested resolution that needs an absent or invalid base is a typed failure.

Preserve query strings during acquisition and resolution. Remove fragments from HTTP requests according to URL semantics, not query parameters. Bound total acquisition time, connection time, redirects, compressed transfer bytes, decompressed bytes, and decoded bytes. Check status and actual streamed size; a misleading/missing Content-Length does not bypass bounds. Keep transport certificate checks and explicit redirect policy intact. Cross-origin redirects must not forward caller credentials; HTTPS-to-HTTP downgrade requires explicit policy rather than accidental permission.

A saved run contains separate `source` and `plan` members. The source may be a local path, stdin, a deliberately authored public URL, or a runtime environment-variable reference to a URL. **Automatic saved-run creation stores environment-variable references for transient URLs and any credential-bearing inputs, not their values.** This allows complete query-bearing URLs to be supplied at runtime without serializing secrets. Reject embedded userinfo and authentication fields in persisted documents. Do not claim arbitrary query strings can be reliably classified as secret-free; authored public source literals remain the author's responsibility. Default diagnostics redact all query values, fragments, userinfo, credentials, and sensitive headers, including values nested in transport error chains or redirect locations. Do not silently redact requested extraction values: those are untrusted source data, not operational logs.

**Acceptance.** The core dependency graph has no HTTP-client dependency under any product feature combination; source-route parity holds for identical accepted bytes/base; no HEAD is sent; non-UTF-8 and query-bearing local HTTP fixtures work under the stated rules; runtime secret sentinels do not appear in generated plans/runs/errors/evidence metadata. [T01–T04]

**Why this design.** It retains one-command convenience while keeping reproducibility, browser choice, authentication, and network policy outside the extraction engine.

### D2. One public execution contract; no translation profile

**Problem/evidence.** Generic, interop, and CLI report surfaces duplicate related representations. The CLI library is also a dependency of xtask's docs machinery. [S4, S5]

**Implement.** Consolidate around §3; delete `interop/v2` as a public adapter and the separately supported CLI Rust API. Rehome its proven internals by responsibility. Compilation validates selectors/regex/options once. Preparation accepts source/metadata/limits and lazily initializes a bounded DOM only for DOM-dependent operations. Cache preparation failures as well as success; repeated execution cannot repeatedly attempt an oversized parse.

Bind cached preparation success/failure to the immutable snapshot and its fixed preparation limits. A compiled plan stores validated syntax/options, not a spent operation budget. Each execution gets fresh counters, shared across its selection, guards, exclusions and projection; repeated calls cannot inherit a prior call's work consumption. Reusing a prepared DOM still charges that operation's traversals. A caller requesting different preparation limits creates a new prepared document; do not silently reuse a DOM prepared under a looser policy. No required thread-safety promise is implied; do not add a global cache or server.

Make the CLI a binary product with internal modules and private test seams. Keep main a genuinely thin entrypoint. Replace xtask's direct `htmlcut_cli` imports with core-owned schemas/contract metadata plus bounded calls to a once-built CLI binary for help, describe, parsing, and fixture execution. Keep clap-tree consistency tests inside CLI unit tests. Do not copy the parser/renderer into xtask or publish another helper crate merely to keep an old test seam.

Update doctest inclusion, schema consumers, fuzz targets, benchmarks, examples, coverage ownership, and source-shape rules when modules move. Remove the old CLI dependency from xtask rather than retaining an undocumented runtime interface.

**Acceptance.** CLI and Rust API produce equal deterministic result/error content for equivalent operations. An instrumented test proves zero DOM parses for repeated slicing, one parse for repeated DOM queries, and no eager rendering of unrequested projections. Unknown/old/duplicate-key plans fail. [T05–T07]

**Why this design.** It reduces contract and validation drift while retaining compile-once/prepare-once value.

### D3. Faithful, explicitly named text projections

**Problem/evidence.** Existing `reference internal`, `policy`, and caption heuristics delete meaningful selected content. [S3, S6]

**Implement.** Remove reader/boilerplate suppression, terminal auxiliary-section cutoffs, neighboring-caption deduplication, and class/ID/phrase-based omission from extraction. Do not replace them with website exceptions or a different heuristic reader.

`dom_text` is the default. Traverse DOM text nodes in document order, including template-content fragments at their host position; concatenate their parsed values with no added separator, trimming, whitespace collapse, or visibility inference. Script/style/template text is obtainable without execution. HTML entity decoding and parser newline normalization are properties of the parsed DOM, not source-byte preservation. This traversal is explicitly not advertised as browser `textContent` compatibility.

`document_text` is an explicit deterministic structural renderer. Preserve headings, paragraphs, ordered/nested lists, table captions/row/cell order/header distinctions and declared spans, link labels and destinations, image alt, and preformatted text. Its only inherent payload-element exclusions are `script`, `style`, and `template`, including when one is the selected root. Hidden/ARIA-hidden/inline-style-hidden elements remain included. CSS layout, generated content, screenshots, and browser visibility are outside the contract.

Use this single rendering convention and freeze exact fixtures before implementing its traversal: Markdown-like headings (`#` through `######`); block boundaries as LF-separated blocks; indented list markers, honoring list start/value/reversed attributes; links as `[label](destination)` with reversible delimiter escaping; image alt as text at image position; fenced preformatted blocks with a fence longer than any payload fence run; table captions as separate blocks and table rows with explicit cell separators, distinguishing headers and retaining row/column-span annotations. Nested tables must remain distinguishable, not collapse into a concatenated cell string. This is a documented structural text format, not a claim of complete CommonMark/GFM conversion. Text payload whitespace is preserved by default; optional normalization applies only as defined in D8. Do not introduce an unconditional final LF into the value itself.

Caller-declared CSS exclusions are applied only to nodes inside each selected subtree; removing a matched element removes that element and its descendants, never ancestors or siblings outside the selected root. Matching uses the original DOM, and the removal set is fixed before projection. A matching selected root can therefore yield an empty value. Exclusions do not mutate the prepared DOM and do not alter candidate selection or guards. Attribute projections reject subtree exclusions as inapplicable. CSS querying follows the documented supported selector semantics; `dom_text`'s special template traversal does not promise browser-style querySelector/textContent parity.

Freeze root and selector-context behavior for guards and exclusions in W1. Scope limits which nodes can be read or removed; it does not implicitly rewrite a CSS selector. Explicit `:scope` addresses the selected root. Document and test whether ancestor-dependent selector matching can consult ancestors outside that root, using the existing selector engine's semantics consistently; such consultation must never authorize reading/removing an outside match. Do not introduce a second selector evaluator or silently clone/reparent the DOM to change matching context. Cover root matches, outside siblings, ancestor combinators, selector lists, and template fragments.

The structural renderer is bounded formatting, not a general HTML-to-Markdown converter. Freeze exact rules for source line breaks at block boundaries, delimiter/backslash escaping, empty labels/destinations, nested lists/tables, invalid list/span attributes, and preformatted fence collisions. Preserve payload rather than normalizing it to make Markdown look cleaner. These formatting decisions are tested implementation details within the convention above, not permission to add reader heuristics or more output modes.

**Acceptance.** Fixtures A–F and full-value/structural fixtures in §7 pass. Irrelevant class/ID changes cannot alter inclusion when selectors/exclusions do not depend on them. Caption insertion cannot remove image labels. Hidden content and template/script distinctions are explicitly tested. [T08–T12]

**Why this design.** A predictable projection plus explicit exclusions is auditable; another implicit cleanup policy would preserve the failure mode.

### D4. Source slicing is not DOM extraction

**Problem/evidence.** Source slicing preserved original spelling but old stdout framing added LF, whereas DOM serialization normalized spelling. [S3]

**Implement.** Keep literal and bounded-regex slicing as a typed strategy over the UTF-8 snapshot. Enumerate non-overlapping pairs: find the next opening boundary, then the earliest closing boundary starting at or after the end of that opening match; resume after the closing match. No implicit nesting. Closing boundaries preceding an opening are ignored. An unmatched opening is an expectation failure even when earlier complete pairs exist. Reject empty literal boundaries at validation and any zero-length regex boundary match at execution; always make forward progress. Capture groups do not replace the selected source.

Both inclusion flags independently choose whether opening/closing boundary bytes belong to the value. Return half-open UTF-8 byte offsets `[start,end)` and the exact substring. All success counts are complete subject to budgets. A caller that wants to parse a slice must explicitly submit its value as a new snapshot, with a new identity and caller-chosen metadata. No title discovery, HTML base inference, text rendering, or DOM preparation runs during slicing.

Inner/outer HTML remain parsed-DOM serialization, explicitly not original source. Default serialization preserves parsed attributes and structure without canonicalization-based stripping. Optional exclusions are represented by a read-only projection/filtered traversal, not mutation of the original DOM.

**Acceptance.** Slice bytes and ranges agree for Unicode, CRLF, capitalization, quote styles, all inclusion combinations, multiple pairs, missing boundaries, and regex edge cases. Raw emits no framing LF. [T13–T15]

**Why this design.** It retains byte-exact extraction without conflating it with HTML parsing and rendering.

### D5. Explicit assumptions; no silent fallback

**Problem/evidence.** A first-match default can select a plausible wrong fact. Unchanged target text does not prove unchanged surrounding meaning. [S3]

**Implement.** Apply §3 selection/guard semantics. Enumerate candidates in document order; deduplicate a selector-list's same element by node identity. On success, report complete counts. A single-selection failure may stop after a second match and report `observed_at_least = 2`; do not label that an exact total. A budget boundary before complete successful enumeration is a limit error, including for nth selection.

Check selected required attributes and context guards before output transforms/exclusions. Match absence, missing attribute, empty attribute/text, failed guard, ambiguous selection, and output limit have distinct typed codes. Any failed selected item or guard fails the whole operation. Do not drop failed items, substitute empty strings, automatically repair selectors, switch projections, increase requested limits, or ask an LLM to reinterpret intent.

Document-scope context checks may deliberately read outside the selected output; selected-scope checks cannot escape their root. Guard selectors/patterns and exclusion matching share work limits rather than each receiving an unbounded fresh allowance. A guard does not claim semantic/business validation beyond its declared literal predicate.

**Acceptance.** Default duplicate/zero matches fail; all-minimum-zero is explicit; positional matches are stable but not advertised as identities; changed label is rejected only when a corresponding guard is present; empty values remain distinguishable. [T16–T19]

**Why this design.** Failures make broken assumptions observable rather than silently guessing another fact.

### D6. Compact requested values and atomic semantic publication

**Problem/evidence.** Historical reports used 39,151 proxy tokens for product subtrees versus 9,939 in the source, and 8,690 for title reports versus 192 for compact titles. These were opt-in formats, not unavoidable costs. [S3]

**Implement.** Use §3's compact result. Compute only the requested projection, identity fields, necessary guards and diagnostics, and explicitly requested evidence. Delete generic structured-DOM reports and automatic per-match parallel text/plain-text/inner/outer HTML copies.

Attribute-only work must not render text, serialize subtrees, compute previews, or discover a document title. Selected values in JSON and raw modes must be exactly equal after JSON decoding; Unicode escaping must not change values. Plan defaults must not depend on terminal detection.

Validate and serialize within explicit bounds before stdout publication. Include JSON escaping and metadata overhead in the serialized-output cap, not just payload string lengths. Audit evidence is generated separately with explicitly selected fields/limits. Neither failed core validation nor artifact preparation may leak a success-shaped partial stdout document.

**Acceptance.** Counted operation tests prove unused projection paths do not execute; the twenty-title fixture has correct compact output without irrelevant representations; stdout/stderr separation, empty values, raw cardinality, and escaping boundaries pass. [T20–T22]

**Why this design.** It saves both computation and model-visible output without requiring another tool to strip a large report.

### D7. Bounded discovery that cannot authorize itself

**Problem/evidence.** Full historical catalog/schema output cost about 71,010 proxy tokens, although narrower discovery was possible. Suggestions are not verified intent. [S3]

**Implement.** Default discovery is a compact operation index. One named operation description and one named schema are retrievable independently. CLI help and discovery are generated from one maintained operation metadata source; core enum/schema vocabulary remains core-owned. Do not reintroduce a full-catalog default or an alternate public CLI library to share it.

Use bounded element descriptors, previews, snapshot-bound handles/cursors, and optional selector proposals. A proposal is marked `suggestion`, not a validated durable identity or confidence percentage. Executing it requires an explicit caller selection of the resulting selector/plan. Preview calls the same projection implementation, but uses a distinct preview result with `complete: false` when shortened and explicit truncation/work information. It is not an extraction success payload.

A cursor/handle binds source identity, effective metadata, preparation/discovery limits, option set, and relevant semantics. Reject stale or altered evidence. Reject live URL plus cursor; instruct the caller to acquire one saved snapshot and inspect that same file with the same explicit base metadata. Do not silently refetch/cache behind the cursor. Terminal pages have no next cursor. Pages must advance monotonically or return a clear resource error, not repeat forever.

Intentional preview-length truncation may produce the distinct incomplete preview document. Source, preparation, selector-work, or other safety-budget exhaustion remains a typed operation error; do not turn it into a truncated preview success. Reaching the descriptor page size returns a next cursor only when the pagination contract can establish forward progress within its budgets.

**Acceptance.** Index plus one description is sufficient for the tested basic extraction; full schemas are not needed. Truncated preview never looks complete. Changed snapshots/options reject cursors, and bounded pagination reaches the tail without silent missing descriptors. [T23–T25]

**Why this design.** It keeps helpful exploration while separating assistance from extraction authority.

### D8. Reproducibility identity; caller-owned comparison

**Problem/evidence.** Whole-source changes can coexist with unchanged selected values. A digest identifies inputs, not correctness or authorship. [S3, S4]

**Implement.** Keep `source_sha256` over exact accepted UTF-8 bytes. Hash normalized, fully defaulted plan JSON with stable key order; preserve array order. Define and document one domain-separated, length-framed hashing format with golden vectors. Extraction identity includes source digest, plan digest, effective base metadata, all effective limits that can affect outcomes, and extraction semantics. It excludes acquisition route/path, runtime timestamps, timings, machine names, and output destination. Do not include the identity field in its own hash input.

For this identity, effective limits mean **core preparation and execution limits**, including core value limits. HTTP timeouts, redirect/transfer/decoding limits, input route, adapter JSON/raw framing and serialized-publication caps are adapter policy and are excluded. They can prevent acquisition/publication, but cannot change a successful core result over equal accepted bytes, metadata, plan and core limits. Keep these policy classes distinct in their registries and golden vectors. The adapter must not silently lower core limits for a requested output format.

Hash the full effective base metadata when present, including its query, without emitting that URL in default results or operational evidence. Redact base/redirect metadata with the same policy as acquisition URLs. Hashes are opaque identifiers, not a secrecy guarantee for guessable inputs; never persist raw normalized hash inputs containing runtime URLs. Requested extraction values remain exact, even when they themselves contain a URL or sensitive source text.

Prepared-document identity for handles also binds source, metadata, and preparation limits. Keep all source and prepared DOM data immutable from the caller's perspective. Removing or changing projection options cannot mutate future results from the same prepared document.

Remove detached-DOM comparison canonicalization and parallel comparison-value fields. Permit only explicit small transformations: ASCII whitespace normalization outside preformatted content, and URL resolution. Whitespace normalization preserves paragraph/list/row separators introduced by document rendering and does not change NBSP, zero-width characters, or Unicode normalization. Track preformatted origin while traversing; do not regex-normalize the finished string.

Apply declared transforms in array order; reject duplicate instances of the same transform rather than accumulating redundant modes. Whitespace normalization acts on source text segments, not URL destinations, attribute URLs, delimiter escapes, or generated structural markers. Define leading/trailing whitespace and collapse behavior across adjacent inline nodes in W1, with full-value goldens. URL resolution changes only the supported URL positions and never the label or surrounding source text. Test combined transforms in both orders and preserve their declared order in the plan digest. `dom_text` with explicit normalization remains an explicit transformed projection, not the default literal path.

URL resolution applies to structural link destinations and explicitly requested single-URL attributes; reject application to token-list attributes such as srcset or arbitrary text, and reject options on incompatible projections. Use explicit/final-response base metadata, never a parsed HTML `<base>`. Keep link labels intact. Unrequested parsed attribute values remain unchanged. Broader normalization, dates, money, history, and monitoring policies belong to callers.

**Acceptance.** Golden identity vectors are stable across equivalent CLI/library runs and source routes; navigation change alters source evidence but not an unaffected value; base/limit/semantics changes alter identity; key order/default omission does not change normalized plan identity; source slices still preserve exact bytes. [T26–T28]

**Why this design.** It supports replay without creating a change-monitoring subsystem or conflating identity with meaning.

### D9. Preserve resource, safety, and deployment guarantees

**Problem/evidence.** Existing bounded selectors, parser patches, and reusable preparation are genuine differentiators. A smaller unbounded replacement would be a regression. [S4, S7]

**Implement.** Carry forward all applicable budgets and hard maxima; remove a budget only with the operation it governed. Bound plan bytes/complexity, selector/regex compilation, source/decoding, DOM construction/depth/node counts, total traversal/work, candidates/selected matches, guards/exclusions, previews, diagnostics, requested evidence, value bytes, and serialized publication. Use checked arithmetic. A finite input limit alone is not a substitute for a DOM/work/output budget.

Known starting defaults include 50 MiB accepted source; 250,000 elements (hard maximum 1,000,000); DOM depth 2,048 (hard maximum 4,096); 1,000,000 selector-work units; 100,000 candidates; 10,000 selected matches; 8 MiB per value; 64 MiB aggregate value bytes; HTTP connect 5,000 ms and total 15,000 ms. Recheck the actual declarations and preserve the stricter applicable bound. New categories need explicit finite defaults/hard maxima in one core/adapter-owned registry, frozen with rationale and boundary tests before their implementation is marked complete. A user-supplied policy can reduce bounds, not bypass hard maxima. [S8]

Enforce each bound before the allocation or operation it governs exceeds it. Check DOM budgets in construction/traversal hooks rather than only after a complete oversized parse. Preserve selector-fork work accounting and fallible propagation. Include excluded nodes, guards, regex searches, templates, serializer escaping, and audit metadata in their relevant aggregate limits. Do not replace a patched parser with an upstream lookalike without proving every maintained guarantee. Bound iterative traversal and avoid introducing recursive stack-exhaustion paths for permitted deep input.

Stage semantic results before publication. For a file, create bounded temporary staging in the destination filesystem, refuse overwrite unless explicit, and use the platform's atomic replace/create mechanism without a check-then-clobber race. Preserve an existing target on pre-publication failure. Clean owned staging on errors. Reject colliding input/result/audit destinations and symlink/path cases that would violate the documented contract. Test no-clobber behavior, not just an `exists()` check.

For result plus audit files, prepare both before publication and report either commit failure; do not promise an impossible cross-file transaction. Publish audit before the successful result indicator. An output I/O failure after writing to a pipe can leave bytes already delivered: report failure, never promise retraction. The core provides logical budgets, not OS-level hard memory/CPU isolation. Extracted HTML is neither sanitized HTML nor prompt-injection-safe text.

**Acceptance.** Boundary tests for every limit (below/at/above), fault-injected read/decode/write/flush/rename failures, parser-count checks, maintained fork tests, Miri and fuzz proofs, and all native package smokes pass. [T29–T32]

**Why this design.** It retains the operational reasons to prefer a maintained engine to a short script.

### D10. Fidelity and useful-task economics govern release readiness

**Problem/evidence.** Old broad keyword checks passed despite missing technical identifiers; equivalent parsers already solve simple extraction tasks. [S3]

**Implement.** Add the full conformance matrix in §7; port valuable old tests to the new contract. Include synthetic structural variants and a small offline, licensed/redistributable corpus with hashes and source/license metadata. Do not depend on expired GitHub artifacts or changing live pages for correctness gates. Reproduce historical failures with the old binary when available; missing historical download access must not prevent running the embedded regression inputs or implementing the fixes. Never invent a baseline result.

Separate fidelity from optional heuristic discovery. Test exact values and structural relationships, not merely exit code, broad keyword presence, token length, or snapshots regenerated from whatever the implementation emitted. Independently review expected fixtures before committing new goldens.

Measure equivalent correct tasks against a competent available parser/script. At minimum: titles/URLs, a faithful technical section, one strict guarded extraction, and a multi-field caller-mapping task. Separate process startup from in-process reuse; use fixed input, warmups, repeated runs and recorded versions. Report corpus/hashes, commands, complete values, observed retries, and token encoding.

For each comparison, distinguish the value payload from the complete default result envelope and from opt-in audit output. Report one-time discovery/setup separately from repeated prepared execution so compact payloads do not hide identity/metadata or integration overhead. Machine-only schema validation is not model-visible discovery cost. Record generated commands/scripts and observed repair steps; describe unmeasured agent work as unmeasured rather than substituting script length for a measured session total. Do not claim actual agent billing, a blind multi-agent study, global superiority, or a universal tokens-per-byte estimate. Token proxies require a real named tokenizer; bytes are not tokens. Use available runner tooling if local tokenization tools are unavailable.

The release gate requires documented correct task results and compact-output evidence, not that HTMLCut wins every comparison. No mandatory numerical performance claim is invented. Remove unnecessary representations rather than deleting values to win a token comparison. Compare optional audit output separately from default results.

Update all maintained docs, examples, schemas, help, release smoke flows, Rust doctests, manifests, lockfiles, source ownership, mutation scope, legal metadata and changelog. Delete obsolete live guides and wrappers rather than presenting two supported products. Historical changelog entries and the frozen baseline remain history. Keep the source/version/change-note authorities singular. [T33–T40]

**Why this design.** It demonstrates the reasons to adopt the redesigned tool without benchmark theatre or scope expansion.

## 5. CLI and adapter integration target

Implement a small command vocabulary: `extract`, `run`, `inspect`, `describe`, and `schema`. Conventional `--help` and `--version` remain. Do not keep old `select`/reader aliases as compatibility routes.

`extract` accepts exactly one source form (`--file`, `--stdin`, `--url`, or `--url-env NAME`) and either `--plan FILE` or inline selection options, not conflicting mixtures. Inline options compile to the same fully specified core plan. Provide `--css`, `--projection`, `--attribute`, `--match`, `--index`, literal/regex boundary options, `--base-url`, explicit `--raw`, `--output`, and explicit overwrite consent where applicable. Provide `--save-run FILE` for source/plan persistence. URL-based automatic persistence requires `--url-env NAME`; combining transient `--url` with `--save-run` is a typed options error explaining how to use a runtime reference. File/stdin sources can be persisted directly under their documented path rules. Nontrivial guards/exclusions/transforms can be supplied through a plan rather than inventing an extensive second flag language.

These target examples must become tested examples:

```sh
htmlcut extract --file fixture.html --css '#amount'
htmlcut extract --file fixture.html --css '#amount' --projection dom_text --raw
htmlcut extract --file fixture.html --css 'a.item' --projection attribute --attribute href --match all
htmlcut extract --stdin --plan amount.plan.json
htmlcut extract --url-env HTMLCUT_SOURCE_URL --plan amount.plan.json --save-run amount.run.json
htmlcut run amount.run.json
htmlcut describe
htmlcut describe extract
htmlcut schema htmlcut.extraction.plan
```

A complete minimal saved run has the following shape. `HTMLCUT_SOURCE_URL` is supplied by the caller at execution, including any query parameters; the file never captures its value:

```json
{
  "schema": "htmlcut.run",
  "version": 1,
  "source": {"kind": "http", "url_env": "HTMLCUT_SOURCE_URL"},
  "plan": {
    "schema": "htmlcut.extraction.plan",
    "version": 1,
    "strategy": {"kind": "css", "selector": "#amount"},
    "selection": {"kind": "single"},
    "projection": {"kind": "dom_text"},
    "exclude": [],
    "guards": [],
    "transforms": []
  }
}
```

Treat a run file as caller-authorized configuration, not instructions obtained from an untrusted webpage. `run` loads an adapter-owned source specification and the same extraction plan; runtime environment values are resolved only in the adapter and not embedded into the plan. A saved file path is relative to the run file's directory, not the process working directory, unless absolute. A run file is data, not a shell script: no command substitution, arbitrary command execution, nested run inclusion, or automatic environment serialization.

Validate plan/options and output-destination conflicts before acquisition where possible. Resolve a CLI source URL once, then execute all work for that invocation against the acquired immutable snapshot. Multi-call exploration has no implicit persisted session; a caller supplies the same saved snapshot.

Help/index/operation descriptions must describe defaults and failure behavior accurately. A library example should prepare once, execute two compiled plans, and show source-only slicing without DOM parsing. No public CLI Rust helper is necessary to demonstrate this.

## 6. Dependency-ordered execution, without staged release scope

The implementing agent performs a short local mapping and design QA before editing product code. Use the selected semantics above; do not restart an open-ended product-design exercise. Resolve internal implementation questions and record decisions without waiting for permission already granted.

Use an early end-to-end slice to prove the architecture before widening behavior: one supplied snapshot, one CSS single/dom_text plan, its compact CLI result, one typed error, and one source-only slice with zero DOM parses. Keep this inside the same integration branch and release scope. Validate schema/runtime agreement, lazy preparation, identity framing, private adapter seams and real coverage ownership on this slice. It is an implementation checkpoint, not a reduced deliverable.

Move each required test family and executable module into coverage/mutation ownership as it is introduced. Close affected coverage gaps in W2–W5 rather than accumulating them for W6. Repair §9 gate/source-identity tooling early enough to trust intermediate evidence; W6 performs the final complete campaign and integration closure. Do not repeatedly run the complete mutation/platform matrix on unchanged intermediate code. Record a finite command timeout, artifact/disk budget, and next diagnostic for expensive campaigns using the maintained tooling. A failure starts a repair loop; it does not justify unbounded identical retries.

| Work unit | Required work and evidence | Exit condition before proceeding |
|---|---|---|
| W1: baseline and contract | Read repo instructions; capture starting SHA, dirty state, actual gates/pins; reproduce A–F where possible; freeze examples, renderer goldens, limits, error table, module ownership, and removal inventory; bump development workspace to 15.0.0 coherently. | A complete D1–D10 map, no known design contradiction, and explicit baseline failures recorded. |
| W2: snapshot/plan foundation | D1/D2 pure-core boundary, typed plans, lazy prepared document, canonical identities, schema generation, compilation and validation budgets; prove the minimal end-to-end slice with a private adapter seam. | Core build/default/all-feature checks, minimal CLI success/error and source-only slice pass; early T01–T07/T26 evidence and affected coverage ownership are recorded. |
| W3: extraction fidelity | D3–D5 selectors, projections, slicing, exclusions, guards and shared budgets. | All normative value/ordering/error fixtures pass; no heuristic filtering path remains. |
| W4: adapters and discovery | D6/D7 compact JSON, exact raw, saved runs, HTTP, bounded publication, descriptors/proposals/cursors; remove CLI API and repair xtask consumers. | CLI/core parity, I/O fault tests, secret redaction, and discovery tests pass. |
| W5: repository integration | Finish D8/D9 identities/resource proofs and §9 gate/release repairs; complete docs/doctests/fuzz/Miri/bench consumers and delete superseded live paths; reconcile the incrementally maintained ownership/coverage/mutation maps. | No active reference to the rejected contract; full source inventory is accounted for; affected coverage and gate-tooling proofs pass before final campaigns. |
| W6: quality closure | D10 matrix, corpus/economics; §8–§10 gate repairs; fix every actionable failure and rerun affected tests. | Complete clean-revision maintainer, platform, mutation, package and conformance evidence. |
| W7: handoff | Prepare one PR, 15.0.0 metadata/artifacts, final traceability and reproduction instructions. | §14 completion checklist is satisfied; no unauthorized release action occurred. |

These units can overlap when dependencies permit. They are not separate product releases or excuses to stop after a “foundation.” Keep the program runnable with the single emerging architecture; temporary local edits are not supported legacy routes.

## 7. Conformance matrix and independent expected values

Every test family below must have maintained test IDs/paths mapped in the completion record. One test may cover several rows, but a claimed row needs actual assertions. Unit, property/metamorphic, integration, fixture and fault-injection tests complement each other; none substitutes for all the others.

| Test IDs | Decisions | Mandatory proof |
|---|---|---|
| T01–T02 | D1,D2 | No core I/O/dependency reachability; equal bytes/base produce equal result across file, stdin, inline library and local HTTP input. |
| T03–T04 | D1,D9 | Strict decoding/BOM and streamed compression/redirect/time limits; GET-only; query preservation; generated-run environment references; default secret redaction and no source clobber. |
| T05 | D2 | Generated schema and runtime plan validation; unknown/old/duplicate fields; unknown versions; invalid bounds/options; CLI and Rust error-code parity. |
| T06–T07 | D2,D6,D9 | Compile once, parse once, cached preparation failure; slicing parses zero times; attribute-only projection never runs renderer/serializer/preview. |
| T08–T09 | D3 | A–F exact dom_text expectations and document_text preservation; no reference/policy/caption heuristic; link label/destination retention. |
| T10–T12 | D3,D8 | Hidden/ARIA/style-hidden inclusion; explicit root/descendant exclusions; script/style/template difference; entity/newline/NBSP/Unicode/preformatted rules; nested lists/tables/captions/spans/alt. |
| T13–T15 | D4 | UTF-8 ranges; all boundary inclusions; literal/regex pairing and progress; unmatched opening; non-overlap; DOM serialization distinguished from exact slices; zero-added-LF raw bytes. |
| T16–T17 | D5 | Single zero/multiple; all bounds including explicit minimum zero; nth 1-based bounds; exact successful counts, lower-bound error counts and over-budget failure. |
| T18–T19 | D5 | Missing versus empty attribute; required attributes; exact/regex guards; document/selected scope; original-DOM evaluation; atomic failure of any selected item; no automatic repair. |
| T20–T22 | D6,D9 | Compact success/error shape, stdout/stderr isolation, JSON/raw value equality; escaped-output limits; staged artifact failures; unused representation absence. |
| T23–T25 | D7 | Narrow metadata/help/schema agreement; explicit suggestion adoption; bounded truncated preview; cursor/handle snapshot/options invalidation; advancing terminal pagination; no URL+cursor refetch. |
| T26–T28 | D8 | Identity golden vectors; omitted/explicit default equivalence; array-order sensitivity; base/limits/semantics sensitivity; navigation-only change/value stability; no DOM mutation between runs. |
| T29–T30 | D9 | Boundary triplets for every budget; construction-time DOM limits; aggregate work across guards/exclusions; numeric overflow; bounded diagnostics and regex; deep/adversarial local fixtures terminate. |
| T31–T32 | D9 | Private I/O fault injection for read/write/flush/rename, no-clobber races and cleanup; no escaped paths; broken pipe truthfulness; native packaged execution and replay. |
| T33–T34 | D10 | Offline corpus licenses/hashes and complete expected content; class/ID/caption/unrelated-sibling metamorphic invariants; supplied rendered DOM without executing script. |
| T35 | D10 | Equivalent-task benchmark correctness, named tokenizer/byte distinction, startup vs in-process separation, versions/hashes and measured retry disclosure. |
| T36 | D2,D10 | Maintained doctests/example execution; docs/enum/schema/help registry consistency; old runtime/adapter removal; real module ownership/coverage accounting. |
| T37 | D10 | SemVer classifier matrix and frozen-baseline integrity; 15.0.0 authorized break, future patch/minor not treated as major. |
| T38 | D9,D10 | Core-only change cannot skip full maintainer coverage; expected required CI jobs cannot be replaced by arbitrary successful/skipped summaries. |
| T39 | D9,D10 | Tag/source-revision packaging test: moving main differs from tag, but archives/binaries/hashes use the intended commit; changelog preview and exact published-version notes cannot select adjacent sections. |
| T40 | D10 | Clean-revision closeout: actual 100% scored line/branch report, mutation accounting, Miri, live fuzz smoke, platform/package checks and explicit no-release state. |

### 7.1 Mandatory historical reproductions, made self-contained

Use exactly-one CSS selection. These inputs are not dependent on a network download. A, C and E were passing controls; B, D and F exposed default-rendering omissions in the evaluated 14.0.0 binary, despite successful command exits. [S3]

| Case | Selector | HTML |
|---|---|---|
| A | `p` | `<p>Before <a href="relative.html">IMPORTANT</a> after.</p>` |
| B | `p` | `<p>Before <a class="reference internal" href="relative.html">IMPORTANT</a> after.</p>` |
| C | `article` | `<article><p>BEGINNING</p><section id="section-one"><h2>Policy terms</h2><p>IMPORTANT POLICY CONTENT</p></section><p>ENDING</p></article>` |
| D | `article` | `<article><p>BEGINNING</p><section id="policy"><h2>Policy terms</h2><p>IMPORTANT POLICY CONTENT</p></section><p>ENDING</p></article>` |
| E | `p` | `<p>Damage: <img src="mirror.jpg" alt="Broken mirror"> end.</p>` |
| F | `article` | `<article><table><caption>Charges</caption><tr><td>EUR 180</td></tr></table><img src="mirror.jpg" alt="Broken mirror"></article>` |

Exact untransformed `dom_text` expectations:

```json
{
  "A": "Before IMPORTANT after.",
  "B": "Before IMPORTANT after.",
  "C": "BEGINNINGPolicy termsIMPORTANT POLICY CONTENTENDING",
  "D": "BEGINNINGPolicy termsIMPORTANT POLICY CONTENTENDING",
  "E": "Damage:  end.",
  "F": "ChargesEUR 180"
}
```

`document_text` must preserve A/B's text and destination, C/D's beginning/heading/content/ending, E's image alt at its original position, and F's caption, amount, and image alt. A/B and C/D have identical document_text values when options match. Freeze full structural goldens independently; do not use just `contains` assertions. **Alt is an attribute, not a DOM text node:** the deliberate absence in E/F dom_text is not a defect.

For a licensed/pinned technical-document fixture, preserve all source-present text including `global interpreter lock`, `Windows`, `macOS`, `--disable-gil`, `PYTHON_GIL`, `sys.version`, `Py_mod_gil`, and `PyUnstable_Module_SetGIL`. Assert the entire selected projection and structural relationships; do not tune a renderer just to those strings. A synthetic equivalent covering linked identifiers is mandatory even when the historical page is unavailable.

### 7.2 Additional exact small oracles

`<p>A<span>B</span>C</p>` produces `ABC`, not `A B C`, in dom_text. `<p>A&amp;B&nbsp;C</p>` produces `A&B\u00a0C` after JSON decoding. `<p hidden>A</p>` still produces `A`. `<p><span class="omit">A</span>B</p>` produces `AB` without exclusions and `B` with `.omit` excluded. An attribute `data-x=""` exists and extracts `""`; missing `data-x` fails.

For snapshot `éSTART\r\n<X a='1'>✓</X>\r\nEND` (the notation `\r\n` here represents actual CRLF), opening `START`, closing `END`, neither included, the payload is `\r\n<X a='1'>✓</X>\r\n` and its UTF-8 range is **[7,27)**. Verify the byte count in code, including the multibyte characters. Raw output must have exactly those 20 payload bytes.

A context fixture contains `<h2 id="label">Repair cost</h2><p id="amount">EUR 180</p>`. Selecting `#amount` without guards returns the same value if only the heading changes. A document-scoped exact dom_text guard selecting `#label` rejects the changed heading. This proves declared validation, not automatic semantic awareness.

## 8. Quality gate policy and the coverage closure loop

Preserve the actual maintained 100% executable-line **and** branch bar and its justified inventory. Do not interpret a failing measurement as permission to stop coding. It creates a concrete test/refactor task. Preserve all relevant existing tests; update only assertions and API wiring made obsolete by this approved contract, recording the reason.

The required closure loop is:

1. Run the smallest useful failing test/gate; retain command, toolchain, source revision, exit status, and full evidence path. Read the real failure rather than merely quoting the aggregate nonzero exit.
2. For uncovered lines/branches, inspect the per-file report and source. Classify each gap as missing behavior test, missing fault-path test, real defect, unreachable design, or a reproducible instrumentation defect.
3. Add behaviorally meaningful assertions and private deterministic seams for clocks/network/readers/writers/filesystem operations where needed. Prefer unit tests for branches and integration tests for composition. Avoid sleeps, public test hooks, brittle OS-failure tricks, and tests that assert only that a function was called.
4. Refactor genuinely unreachable/redundant logic out of production where it improves the design. Do not delete a required branch or supported behavior merely to improve a percentage. Correct an instrumentation/reporting defect only with a minimal reproducer and regression test; do not invent an exclusion or reset the denominator.
5. Rerun targeted tests and fresh affected coverage. Inspect surviving branches. Repeat until the genuine gate is green, then run the complete clean coverage pass.
6. Rerun all affected broader gates and record the final revision. A prior green report is not evidence for code changed afterward.

Forbidden shortcuts: lowering thresholds; adding broad ignore patterns or coverage-off annotations; moving executable logic into excluded `main.rs`, declarative modules, support crates, or build scripts; modifying the scored inventory to omit new code; blanket lint allows; `continue-on-error`; converting failures to warnings; deleting non-obsolete tests; snapshot acceptance without reviewing correctness; and claiming tests ran when they were merely configured.

Coverage demonstrates exercised code, not correctness. Add the fidelity, property, fault-injection, parity, resource and mutation checks independently. All new/moved executable modules must retain their real coverage and mutation ownership. A rename that accidentally evades an exact-path rule is a defect to fix.

Use the repository launcher as the execution owner:

```sh
./scripts/xtask.sh structure check
./scripts/xtask.sh ci-rust-gate
./scripts/xtask.sh coverage
./scripts/xtask.sh miri
./scripts/xtask.sh fuzz-smoke
./scripts/xtask.sh mutants
./check.sh
```

Read each current command's help/policy before invoking options. Do not fabricate an argument from this specification. Incremental runs need not replay the full gate every time. The final report must show the complete maintainer gate on the final implementation revision, plus separately owned live fuzz, mutation, conformance, economics, and platform/package evidence. The maintained full gate is the authoritative combined execution; a list of individually passing snippets is not a replacement. [S2]

Run the required PR mutation path and one complete maintained mutation campaign for the final candidate, locally or on authorized runners. Reconcile exact planned/run mutant sets; repair actionable survivors and infrastructure timeouts. A timeout is not automatically a caught mutant. Equivalent/unviable cases require the maintained policy and concrete reasoning; do not broaden exclusions to obtain success. Iteration mode is useful while writing tests but does not replace the final clean campaign.

Miri must still prove the new selector/slice boundary with strict provenance, not execute only an obsolete API. Update fuzz harnesses/corpora, compile them and run bounded live smoke. Include new plan validation, template traversal, bounds, and serialization paths. Keep large exploration acceptance out of per-mutant duplication while executing it once in the intended gate.

## 9. Required adjacent gate and release repairs

These are part of D10 delivery, not additional optional projects.

### 9.1 Real SemVer classification and baseline discipline

The reviewed `semver_release_type_from_versions` returns minor for equal versions and major for every difference. That could treat 15.0.0 → 15.0.1 as permission for arbitrary API breakage. Replace this with a real SemVer comparison, not string inequality. [S9]

For stable versions with major > 0: increased major permits major; same major/increased minor permits minor; same major/minor/increased patch permits patch. Equal versions permit no incompatible contract change; use the strictest relevant check (patch) for regression testing until the version is intentionally changed. Reject decreases, malformed versions, and release tags with prerelease/build metadata on the stable-publication path. Test numeric ordering, not lexicographic ordering. No unconditional `--release-type major` override remains.

Keep the checked-in **14.0.0** baseline immutable during this implementation. A 15.0.0 major comparison can intentionally remove the old API without adapters. Never refresh a baseline from the worktree merely to hide changes. An appropriate 15.0.0 baseline is produced **only after** the user authorizes and completes that immutable release, using the existing refresh mechanism. The release-ready PR supplies the validated post-release command and provenance expectations; it must not require a nonexistent published tag to pass pre-release gates.

This timing is deliberate: baseline refresh is release bookkeeping, not unfinished product implementation. Historical snapshots and changelog entries do not violate the prohibition on supported legacy paths.

### 9.2 Required Linux coverage is not a devcontainer-path side effect

The reviewed full maintainer run is inside the path-conditional `contributor-devcontainer` job; the other Rust lanes are the curated subset. Do not assume a green summary therefore proves full coverage on a core-only change. [S10]

Provide one required Linux maintainer lane on every relevant PR/push that runs the full gate, independent of whether contributor-environment files changed. Keep the heavier devcontainer validation conditional on actual environment changes; avoid executing the same complete suite twice merely to satisfy both jobs. Cross-platform and release-target smokes remain required as appropriate.

The aggregate must verify each expected mandatory job succeeded. Only explicitly enumerated optional jobs may be skipped. Test failure, cancellation, missing outputs, unexpected skip, and the normal core-only-change path. Keep required status names `Check` and `cargo-mutants pull-request summary` intact unless separately authorized to change branch protection. Job names alone are not evidence: retain actual full-gate reports at the final SHA.

### 9.3 Immutable artifact source

The reviewed release workflow checks out the default branch for build jobs and creates source archives from `HEAD`, while release-tag validation reads the tag manifest. These are distinct identities when main advances. Inspect the entire current path and repair it so one resolved intended source commit owns archive contents, binary build inputs, package version, checksum manifest, smoke results and provenance. [S11]

Publication tooling may be maintained separately, but it must not substitute its own checkout for the release source. Add a disposable local Git fixture with different main/tag contents and versions to prove this. No real remote tag or release is needed for the test. Dry-run/source packaging must use the validated candidate SHA before publication; after publication authorization, every asset must use the immutable release tag's SHA.

Keep native targets from the single `scripts/release-targets.sh` registry: macOS ARM64 and x86-64, Linux x86-64 musl, and Windows x86-64 MSVC at the reviewed baseline. Do not claim cross-platform success from cross-compilation alone: execute the packaged smoke/replay flow on matching runners. Preserve checksums, provenance, package-specific README and source archives. Release notes must come from `changelog.md`, not a second hand-maintained notes tree. The inspected publication script currently invokes GitHub generated notes; replace that path with bounded, tested extraction of the exact version section from the intended source revision. [S12] Refuse absent, duplicate or empty release sections, and test that adjacent versions are not included. Do not append generated summaries as a second authority or rewrite historical releases as part of this work.

Keep development entries in `[Unreleased]`, explicitly intended for 15.0.0. A nonpublishing notes-preview path can select that section explicitly. The separately authorized release finalization promotes those entries to `[15.0.0]` with the actual release date; the publishing path then requires that exact section at the tag. Do not invent a release date or require a published 15.0.0 tag during implementation. This administrative finalization, like the later baseline refresh, is not permission to defer code or test work. Add notes-section tests to T39/T40.

## 10. Documentation, deletion, and architecture accounting

Update the maintained AFAD metadata and workspace-version references using existing repository conventions. Concrete fenced CLI examples must parse and run in the fixture sandbox. Rust examples must compile through maintained doctests. An old example that relies on removed behavior is rewritten, not used as a reason to keep that behavior.

Remove or replace active `docs/interop-v2.md`, `docs/cli-library.md`, old output/canonicalization/reader guides, old schema catalogs and API adapters as their responsibilities disappear. Port their useful concepts to the single current architecture/core/CLI documentation. Do not delete unrelated operational, platform, legal, contributor, or release instructions.

Maintain a deletion/retention ledger: old responsibility, new owner or reason removed, updated consumers, test disposition, and coverage/mutation ownership. This prevents abandoned serializers, public types, enum variants, fixtures, flag aliases, and auxiliary reader vocabularies from remaining reachable.

Module names express responsibility, not the lifecycle of this work. Splitting for source-cohesion budgets must yield coherent modules, not numerical fragments. Update `tooling/rust-source-shape-policy.toml` with actual ownership and allowed dependencies; do not broadly raise budgets to fit monolithic code. In particular, core cannot depend on CLI/xtask, and no renderer/selector semantics are duplicated in adapters.

Carry input licenses and patch provenance forward. Dependency changes needed by the redesign are permitted, but avoid unrelated version churn. Dependency/advisory/freshness failures discovered during verification are actionable: update and test the affected dependency/tooling rather than muting its policy. Do not add a blanket advisory waiver.

## 11. Design QA and implementation review checkpoints

Before product edits, record a compact module-level plan and review it against the following closed decisions. Fix contradictions immediately; do not ask the user to select among already-settled architectural alternatives.

| Review question | Required answer |
|---|---|
| Does anything except the CLI adapter acquire input? | No. |
| Can a source-only operation parse HTML for metadata or identity? | No. |
| Is text literal by default, and is alt handled differently by the two projections? | Yes. |
| Can original-DOM guards be weakened by exclusions or normalization? | No. |
| Can a success misstate counts after early termination? | No. |
| Can incompatible/unknown plans or invalid mode mixtures slip through? | No. |
| Can repeated queries reparse or generate unrequested representations? | No. |
| Can saved-run generation or errors serialize transient secrets? | No. |
| Are limits enforced at the governed boundary, including aggregate/encoding costs? | Yes. |
| Can output validation failure publish values? | No; OS write failures have the explicitly narrower guarantee. |
| Can old CLI/API tooling survive merely because xtask imports it? | No. |
| Are changed/new executable files still scored for coverage and mutation? | Yes. |
| Does code-only CI execute the full maintainer coverage proof? | Yes. |
| Can an authorized major change bypass all future SemVer protection? | No. |
| Can packaging build from a different commit than the requested source? | No. |
| Does the handoff require an unapproved merge/tag/publication? | No. |

At the end, review the actual implementation rather than repeating this table mechanically. Trace representative success/error paths from CLI to core, inspect dependency reachability and deletions, and check that release examples execute the shipped binary. Use another reviewer when genuinely available; do not claim independence or multi-agent testing when review was performed by the same agent.

## 12. Persistent execution and evidence protocol

Place this specification at `docs/extraction-contract-spec.md` as the single in-repository work authority. Maintain **one** concise `docs/extraction-contract-status.md` containing: starting/current commit; current work unit; each D/T family's state; evidence paths and tested revisions; unresolved failures with next repair; and the next concrete command. At closeout it becomes the evidence summary, not another evolving product specification. Follow docs metadata rules for both.

Use `not started`, `in progress`, `implemented/unverified`, `failed verification`, and `verified` accurately. A decision is verified only when its implementation, consumers, deletions and tests are evidenced. Keep full gate logs in the repository's established artifact roots, outside the source tree; record their paths and hashes rather than flooding the status document or model context with them. Preserve enough portable summaries to survive artifact expiration.

After an interruption or context reset: read the spec and status, inspect Git status and the current diff, check which evidence belongs to the current revision, and resume the first unmet requirement. Do not restart a broad audit or ask what the user wanted. Reuse valid artifacts/caches, but invalidate proofs whose source/configuration inputs changed.

Do not respond with just a plan, an implementation outline, a partially wired foundation, or “coverage is below 100%, blocked.” Keep implementing, diagnosing, testing and repairing within available authorized tools. Update the user concisely when a material finding or obstacle changes the path; no fictional background work or promised future completion.

## 13. Recovery procedure; persistence without false success

A failure is initially a diagnosis task, not a terminal state. Execute the following until it is resolved or genuinely requires unavailable external authority:

**Build/behavior failures.** Read full diagnostic context; reproduce with a smaller test; fix the owning code and dependent consumers; rerun. A stale fixture or removed API is a test-update task only when the spec intentionally changed its behavior. Do not repeatedly run the unchanged command hoping it becomes green.

**Missing toolchain/tool.** Use current repository bootstrap/version pins; repair required components. If installation is unavailable locally, use the committed devcontainer or authorized GitHub runner. Do not silently downgrade Rust, remove Miri/fuzz/coverage, or report not-run work as passed.

**Disk/concurrency/time limits.** Inspect actual capacity; use managed artifact cleanup for rebuildable outputs only, preserving user work and evidence. Reduce concurrency or use existing bounded shards/caches. Diagnose hung workers and correct their liveness handling. A timeout is a failed/incomplete execution, not proof of the contract.

**Network/download failure.** Use local captures, checked-in synthetic fixtures and dependency caches; retry transient failures with bounded backoff; try an authorized runner/connector. Do not make correctness depend on live third-party pages. Do not use private data, secrets, or unrelated repositories as a transport workaround.

**Git/PR/CI obstacle.** Inspect actual repository state and permissions; reuse the single task branch; repair workflow/configuration defects in scope; use the available connector/CLI. Preserve unrelated edits, never force-push someone else's branch or bypass protection. Workflow changes use minimal permissions, no secret logging and no unnecessary spending. If PR transport remains unavailable, create the full archive and retain exact verification status.

**Actual external authority/hard limit.** Do not bypass credentials, protections, approvals, or execution/tool limits. Complete independent work and save a reproducible checkpoint, including attempted remedies and the exact missing external action. State what is unfinished and do not call it release-ready. This is an honest incomplete handoff, not fulfillment of the terminal goal, and does not authorize dropping that goal on resume.

No category permits weakened tests, fabricated coverage, unexecuted checks marked green, or an unsupported claim that the implementation is complete. Equally, a correctable first failure is not a reason to stop.

## 14. Completion checklist and final handoff

All of the following are required for **release-ready** status:

- D1–D10 implemented together; T01–T40 mapped to real assertions/evidence; no mandatory behavior deferred.
- One replacement public contract and one code path; obsolete live APIs, flags, schemas, report adapters and contradictory docs removed; useful tests retained/ported.
- Correct exact-value/fidelity, parity, context, source-slice, resource, cursor, secret-redaction and publication behavior demonstrated.
- Actual 100% scored executable-line and branch coverage on the final candidate; complete maintained gate green; source ownership/dependency rules and mutation inventory include moved/new runtime work.
- Required mutation checks and final full campaign accounted for; Miri, compiled fuzz harnesses and live bounded smoke completed; no actionable survivor or infrastructure failure disguised as a pass.
- Required Linux full-gate proof, platform lanes and packaged native smoke/replay proofs complete at the validated revision; intentionally optional skips explained separately.
- Correctness-qualified task-economics report, named tokenizer counts when called tokens, raw measurement data/versions, and no fabricated agent-cost or superiority claim.
- Workspace release metadata consistently 15.0.0; frozen published 14.0.0 baseline unchanged; real major/patch/minor classification tested; no 14.x release created.
- Artifacts/source/checksums/provenance tied to the validated source commit; examples and changelog-derived release notes current; no unapproved merge/tag/release/publication.
- One PR or complete-source fallback archive, cleanly reproducible build/test instructions, final traceability/status document and portable evidence summary. No credentials, cache trees, temporary executables, or third-party font files in the deliverable.

The final agent response states: PR URL or artifact location; source/base commit; what changed by D1–D10; exact tests/gates and measured coverage; platform/mutation outcomes; removed surfaces; release version; and what remains for the separately authorized release action. Keep incomplete verification explicitly distinct from a passing result. Do not ask the user to authorize more implementation work that this specification already mandates.

## 15. Source references and scope of verification

This specification consolidates the complete attached **HTMLCut-architecture-brief.md** and adds execution details and necessary release/gate repairs. The attached earlier evaluation report is historical evidence, not a hidden prerequisite for execution: the key fixtures and facts needed here are embedded above.

References below are supporting evidence, not commands to fetch entire catalogs into agent context. Use repository-local files first and immutable refs to resolve historical claims. Current executing-agent checkout takes precedence for actual file placement, not for silently changing this target behavior.

- **[S1] Main identity checked:** `b52ddadaab0c03a36a64de86e28b016ab56f6ac1`; [repository](https://github.com/resoltico/HTMLCut), [commit](https://github.com/resoltico/HTMLCut/commit/b52ddadaab0c03a36a64de86e28b016ab56f6ac1).
- **[S2] Actual gates and baseline policy:** [quality-gates.md](https://github.com/resoltico/HTMLCut/blob/b52ddadaab0c03a36a64de86e28b016ab56f6ac1/docs/quality-gates.md), [coverage tracking](https://github.com/resoltico/HTMLCut/blob/a606a959ebab0b6397b47e704967ec5db549d0e3/xtask/src/coverage/tracking.rs), [source ownership](https://github.com/resoltico/HTMLCut/blob/a606a959ebab0b6397b47e704967ec5db549d0e3/tooling/rust-source-shape-policy.toml).
- **[S3] Historical measured evaluation and reproducer:** [evaluation README](https://github.com/resoltico/HTMLCut/blob/c732367a4fe6289b48ef622edaf5c328d3e34900/evaluation/README.md), [six-case checker](https://github.com/resoltico/HTMLCut/blob/c732367a4fe6289b48ef622edaf5c328d3e34900/evaluation/check_fidelity.py). The earlier token figures use tiktoken 0.14.0 / o200k_base; they are proxies, not billed session usage.
- **[S4] Existing architecture and prepared execution:** [architecture.md](https://github.com/resoltico/HTMLCut/blob/a606a959ebab0b6397b47e704967ec5db549d0e3/docs/architecture.md), [interop-v2.md](https://github.com/resoltico/HTMLCut/blob/a606a959ebab0b6397b47e704967ec5db549d0e3/docs/interop-v2.md), [core manifest](https://github.com/resoltico/HTMLCut/blob/a606a959ebab0b6397b47e704967ec5db549d0e3/crates/htmlcut-core/Cargo.toml).
- **[S5] CLI/tooling coupling:** [xtask manifest](https://github.com/resoltico/HTMLCut/blob/a606a959ebab0b6397b47e704967ec5db549d0e3/xtask/Cargo.toml), [docs command sandbox](https://github.com/resoltico/HTMLCut/blob/a606a959ebab0b6397b47e704967ec5db549d0e3/xtask/src/docs/commands/sandbox.rs).
- **[S6] Filtering and image policies:** [policy.rs](https://github.com/resoltico/HTMLCut/blob/b52ddadaab0c03a36a64de86e28b016ab56f6ac1/crates/htmlcut-core/src/document/text/policy.rs), [media.rs](https://github.com/resoltico/HTMLCut/blob/a606a959ebab0b6397b47e704967ec5db549d0e3/crates/htmlcut-core/src/document/text/render/media.rs).
- **[S7] Maintained parser/selector guarantees:** [patches README](https://github.com/resoltico/HTMLCut/blob/a606a959ebab0b6397b47e704967ec5db549d0e3/patches/README.md).
- **[S8] Starting limits:** [preparation types](https://github.com/resoltico/HTMLCut/blob/a606a959ebab0b6397b47e704967ec5db549d0e3/crates/htmlcut-core/src/interop/v2/types/prepared.rs), [plan budgets](https://github.com/resoltico/HTMLCut/blob/a606a959ebab0b6397b47e704967ec5db549d0e3/crates/htmlcut-core/src/interop/v2/types/plan/mod.rs), [core constants](https://github.com/resoltico/HTMLCut/blob/a606a959ebab0b6397b47e704967ec5db549d0e3/crates/htmlcut-core/src/contracts/constants.rs).
- **[S9] Inspected release-type defect:** [semver.rs](https://github.com/resoltico/HTMLCut/blob/b52ddadaab0c03a36a64de86e28b016ab56f6ac1/xtask/src/plan/semver.rs). See also [SemVer 2.0.0](https://semver.org/spec/v2.0.0.html) and [cargo-semver-checks](https://github.com/obi1kenobi/cargo-semver-checks) for version/check semantics.
- **[S10] Inspected CI ownership:** [ci.yml](https://github.com/resoltico/HTMLCut/blob/b52ddadaab0c03a36a64de86e28b016ab56f6ac1/.github/workflows/ci.yml).
- **[S11] Release source identity paths:** [release.yml](https://github.com/resoltico/HTMLCut/blob/a606a959ebab0b6397b47e704967ec5db549d0e3/.github/workflows/release.yml), [release-tag.sh](https://github.com/resoltico/HTMLCut/blob/a606a959ebab0b6397b47e704967ec5db549d0e3/scripts/release-tag.sh), [target registry](https://github.com/resoltico/HTMLCut/blob/a606a959ebab0b6397b47e704967ec5db549d0e3/scripts/release-targets.sh).

- **[S12] Inspected release-note source:** [publish-github-release.sh](https://github.com/resoltico/HTMLCut/blob/b52ddadaab0c03a36a64de86e28b016ab56f6ac1/scripts/publish-github-release.sh). The specification changes its generated-notes behavior; it does not claim changelog-derived publication already exists.

**Verification boundary:** the 26 September specification received design QA and mechanical document/example checks. Its companion QA Markdown/JSON and recorded SHA-256 describe that historical revision, not this revised file. The 30 September review used the supplied evaluation, architecture brief, QA record and the local checkout at `b52ddadaab0c03a36a64de86e28b016ab56f6ac1`; it preserves D1–D10 and tightens guard aggregation, selector context, budget reuse, identity-policy separation, transformation/preview rules, benchmark accounting and incremental verification. Structural/JSON/oracle checks were rerun for the revised document, but the historical QA artifacts were not rewritten or represented as current certification. No product code was implemented, built or tested during this plan review. The executing agent must produce the implementation evidence specified here.
