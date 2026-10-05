---
afad: "4.0"
version: "19.0.0"
domain: SCHEMA
updated: "2026-10-05"
route:
  keywords: [schemas, extraction plans, results, errors, discovery]
  questions: ["What are the current named schemas?"]
---

# Schemas

Retrieve one current named schema at a time. Types own schema generation; runtime validation owns cross-field applicability, grammar and byte/work bounds. Inputs reject duplicate keys, unknown fields/variants and obsolete versions before execution. Only the current extraction contract is supported.

```sh
htmlcut schema htmlcut.extraction.plan
```

Individually retrievable schemas:

- `htmlcut.extraction.plan`
- `htmlcut.extraction.data`
- `htmlcut.extraction.receipt`
- `htmlcut.extraction.error`
- `htmlcut.inspection`
- `htmlcut.bundle`

Other emitted roles: `htmlcut.operations`, `htmlcut.operation`, and maintainer `htmlcut.gate_run`.

Extraction wire and semantics versions are both 5. The bundle manifest transport family is version 1 and embeds the current receipt/configuration. Bare success data is an array of strings or records, without a role/version envelope. Typed execution and receipt `data_kind` preserve the interpretation of `[]`; the generic data schema deliberately allows that shared shape. Record strings, optional `null`, and all-valued arrays have distinct meanings. Schema shape alone cannot establish field relationships or source-dependent success.

Receipts are execution/integrity facts rather than provenance authentication or delivery proof. Bundles contain real source bytes and configuration, never external source references or executable programs. [Core](core.md) defines cardinality, representation and identities; [CLI](cli.md) defines delivery and replay boundaries.
