---
afad: "4.0"
version: "15.0.0"
domain: SCHEMA
updated: "2026-09-30"
route:
  keywords: [schema registry, maintainer gate report, htmlcut.gate_run, htmlcut.plan, htmlcut.result, htmlcut.error, htmlcut-json-schema-v2, HtmlInput, plain_text, dom_canonicalization, comparison_text_output, schema inventory]
  questions: ["what schemas does HTMLCut export?", "what is the HTMLCut maintainer gate report schema?", "what are the htmlcut-v2 schema names?", "why is HtmlInput not in the schema registry?", "which interop schema versions carry plain text and DOM canonicalization?"]
---

# Schemas

Retrieve one named schema at a time. Types own schema generation; runtime validation owns cross-field compatibility, grammar and byte/work budgets. Inputs reject unknown fields/versions, duplicate keys and invalid enums/bounds; no old envelope conversion exists.

```sh
htmlcut schema htmlcut.extraction.plan
```

Current roles:

- `htmlcut.extraction.plan`
- `htmlcut.extraction.result`
- `htmlcut.extraction.error`
- `htmlcut.inspection`
- `htmlcut.preview`
- `htmlcut.element_descriptor`
- `htmlcut.selector.proposal`
- `htmlcut.run`
- `htmlcut.operations`
- `htmlcut.operation`
- `htmlcut.gate_run`

The extraction wire family version and semantics version are both initially 1 and have independent meanings. A saved run contains adapter-owned source configuration and the same extraction plan. Success values contain no alternate projections, source URL or plan copy. Errors are one typed family with only available identity/count evidence; hashes are not authentication.
