---
afad: "4.0"
version: "20.0.0"
domain: CLI
updated: "2026-10-05"
route:
  keywords: [cli, extract, replay, inspect, schema, snapshot bundles, raw output]
  questions: ["what commands does htmlcut-cli expose?", "what does htmlcut schema include?", "how do current queries and raw output work?"]
---

# CLI

Commands are `extract`, `inspect`, `replay` and `schema`, plus standard `--help`/`--version`. Inline flags, current JSON and Rust constructors use one compiler. There are no old command/flag aliases or wire conversions.

```sh
htmlcut extract --help
htmlcut extract --file page.html --select h1
htmlcut extract --stdin --select 'article a' --all --read attr:href
htmlcut extract --file page.html --select article --nth 1 --read outer-html --raw
htmlcut inspect --file page.html
htmlcut inspect --file page.html --select article --samples 3
htmlcut schema htmlcut.extraction.plan
```

Exactly one `--file FILE` or intentional `--stdin` supplies strict UTF-8. Accepted BOM/CRLF/NUL are preserved in the full snapshot; parsed DOM follows HTML rules. Callers own acquisition, charset conversion and browser rendering. Explicit `--base-url` accepts absolute HTTP(S) metadata without userinfo; HTML base elements and environment substitution are unsupported.

Default selection requires exactly one node. `--all` defaults to min=1 and permits `--min`/`--max`; `--nth N` is positive and one-based. Complete candidates are still counted. Scalar `--read` defaults to `text`; alternatives are `literal`, `markdown`, `resolved-markdown`, `inner-html`, `outer-html`, `attr:NAME` and `url:NAME`. Static text retains hidden/noscript content and parsed pre edges, inserts structural boundaries and omits inert payloads. Literal/HTML readings are parsed representations, not exact byte cutting.

Repeated field triples are `--field NAME SELECT READ` (one), `--optional-field NAME SELECT READ` (zero-or-one) and `--many-field NAME SELECT READ` (all/min=1). Optional absence returns null; ambiguity and missing requested attributes fail. Fields are keyed by validated names and run in lexical order. Root read/exclude cannot be combined with records. `--field-exclude NAME SELECT` removes matching descendants inside a declared field. `--following-siblings N` adds exactly N element siblings (0–63) to each row forest, failing on shortage or overlap with any root candidate. `:scope` remains the original anchor.

Use JSON for min=0/bounded/nth fields and richer expectations. `--plan FILE` and `--plan-json JSON` conflict with inline query flags. `--plan -` intentionally reads query stdin; source and query cannot both claim stdin. Regular plan/source/bundle paths validate their opened handles, with regular input symlinks supported.

```sh
htmlcut extract --file quotes.html --plan-json '{"version":6,"select":".quote","match":"all","fields":{"author":{"select":".author","match":"optional"},"tags":{"select":".tag","match":"all","min":0},"text":{"select":".text"}}}'
htmlcut extract --file table.html --select 'tbody tr' --all --many-field cells td text --expect-text 'thead tr' 'Name Price'
htmlcut extract --file page.html --select 'script[type="application/ld+json"]' --read literal --raw
```

`--exclude SELECT` applies to scalar descendants. `--expect-text SELECT TEXT` checks exactly one document node with the complete structural text before exclusions. JSON predicates may use literal text or required attributes; count-only expectations use CSS for attribute presence. A late field/expectation failure rejects the whole output. There is no partial-success extraction, business typing/filtering/joining or grid reconstruction.

For discovery, no-selector `inspect` surveys repeated HTML siblings/table shapes. It defaults to four groups (`--limit` 1–16); `--within SELECT` requires exactly one scope. With `--select`, inspect instead returns a complete selector count and defaults to three samples (`--samples` 1–10). Mode-inapplicable flags fail before input. Samples combine bounded exact id/classes, attribute names and shared structural text, with independent completeness labels. Oversized identifiers are omitted, never shortened into fake usable selectors. Detailed attributes/field discovery use explicit extraction, commonly one representative row's outer HTML. There is no recursive or arbitrary attribute-value sampling.

Survey hints must match exact original-node membership/order after bounded CSS escaping. Null means no hint was proved. Table headers identify reading `text`; only complete values are suitable for exact guards. Header uniqueness, direct cell ranges and spans are facts, not inferred columns. Survey excludes head, script, style, template, pre, code, noscript and foreign-namespace subtrees. Group/signature/work bounds remain finite (10 million survey work, 1,024 signatures per parent). Both inspection modes bind accepted-source SHA-256 and have a 16 KiB encoded response cap. Observations do not prove future-page stability or authenticity.

Default success stdout is only canonical strings/records JSON plus one LF, identical for TTY and pipe. `--raw` emits exactly one scalar string without LF, including a valid empty string. Records or statically impossible all bounds reject raw before source; otherwise actual result cardinality is checked. File-only/empty-raw output does not require an unused stdout descriptor.

`--receipt FILE` requests bounded execution evidence from the result's actual remaining work. `--bundle FILE` instead saves a self-contained closed USTAR source/query/manifest archive; the options are mutually exclusive. `replay BUNDLE` recomputes and compares current evidence without original paths, network, environment, unpacking, nested replay or source/query overrides. Bundles contain real source and explicit caller metadata. Receipts identify canonical JSON data even under raw delivery and establish execution/integrity, not authenticity or delivery.

`--output FILE` publishes data instead of stdout. Destinations reject collisions, symlinks and special files; replacement requires `--overwrite`. All artifacts stage before the first commit, with sidecars committed before data delivery. Each file is atomic, but several files/stdout are not a transaction. Later failure can leave a sidecar or delivered pipe bytes and returns nonzero. No power-loss durability guarantee is inferred.

Errors are bounded and source-free: validated field/member names, selected-output row indexes, completed counts and actual exhausted counter/bound facts identify repairs without echoing selectors, predicate values, paths or transport chains. Early exhaustion uses lower bounds only where established. Exit classes are 0 success/help, 2 invalid request, 3 unmet assumption/representation/replay, 4 resource, 5 real input/UTF-8/publication I/O and 6 invariant. Malformed bytes cannot mask underlying read/seek errors.

Byte-boundary slicing, its flags/constructors and range receipts are removed. Use guarded caller byte/string code for exact cutting. See [Core](core.md) and [Schemas](schema.md).
