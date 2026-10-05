---
afad: "4.0"
version: "20.0.0"
domain: SCHEMA
updated: "2026-10-05"
route:
  keywords: [schemas, extraction plans, results, errors, discovery]
  questions: ["What are the current named schemas?"]
---

# Schemas

Retrieve one current named shape at a time; exhaustive schema retrieval is optional for ordinary use. Types own generation; the compiler owns cross-member applicability, CSS/regex syntax, byte/work bounds and source-dependent success. Duplicate keys, unknown members/variants, supplied nulls, coercions and retired versions are rejected before execution.

```sh
htmlcut schema htmlcut.extraction.plan
```

Named schemas are `htmlcut.extraction.plan`, `htmlcut.extraction.data`, `htmlcut.extraction.receipt`, `htmlcut.extraction.error`, `htmlcut.inspection`, `htmlcut.survey` and `htmlcut.bundle`. Maintainer reports use `htmlcut.gate_run`. There is no operations/describe wire family or separate identifier inspection shape.

Query/receipt wire and extraction semantics are version 6. The query requires integer `version:6` and nonempty `select`. Root members are select/match/min/max/index/read/exclude/fields/expect/following_siblings/limits. Match defaults one and permits all/nth; fields additionally permit optional. Scalar reading defaults text. Field maps contain 1–64 validated ASCII names, processed in lexical order; each field permits select/read/match/min/max/index/exclude. Omission requests defaults; supplied null never substitutes for omission. Default-equivalent requests normalize to the same typed canonical bytes, preserving meaningful explicit maxima.

Expectations permit select/scope/min/max/read/equals/pattern. Counts default 1/1 and document scope. Count-only expectations reject supplied read; predicate reads default text and permit text/literal/attr. At most one of equals/pattern may be supplied. Execution limits belong to the normalized query; preparation and explicit metadata belong to the actual snapshot. There are no strategy/projection wrappers, schema role strings, transforms, boundary source variants or old-version conversions.

Bare data is an array of strings or records without role/version envelope. Empty Values and Records both encode as `[]`; typed results and receipt data_kind retain the distinction. Record strings, explicit null and many-valued arrays have different meanings. Schemas do not prove that a selector identifies the intended business entity.

Receipts contain current wire/semantics, four identities, complete root counts and field-name aggregate maps. Source ranges are removed. The closed USTAR transport remains version 1 and embeds current query/receipt plus actual preparation/metadata and exact accepted source. It contains no original path or executable callback. Recomputed evidence is integrity/execution proof, not authentication or delivery.

[Core](core.md) defines readings, cardinality and identities; [CLI](cli.md) defines framing, discovery and native delivery.
