---
afad: "4.0"
version: "15.0.0"
domain: SCHEMA
updated: "2026-10-01"
route:
  keywords: [schemas, extraction plans, results, errors, discovery]
  questions: ["What are the current named schemas?"]
---

# Schemas

Retrieve one named schema at a time. Types own schema generation; runtime validation owns cross-field compatibility, grammar and byte/work budgets. Inputs reject unknown fields/versions, duplicate keys and invalid enums/bounds; no old envelope conversion exists.

```sh
htmlcut schema htmlcut.extraction.plan
```

Individually retrievable schemas:

- `htmlcut.extraction.plan`
- `htmlcut.extraction.result`
- `htmlcut.extraction.error`
- `htmlcut.inspection`
- `htmlcut.preview`
- `htmlcut.element_descriptor`
- `htmlcut.selector.proposal`
- `htmlcut.run`

Additional emitted document roles (not individually retrievable schemas):

- `htmlcut.operations`
- `htmlcut.operation`
- `htmlcut.gate_run`

The extraction wire family version and semantics version are both initially 1 and have independent meanings. A saved run contains adapter-owned source configuration and the same extraction plan. Success values contain no alternate projections, source URL or plan copy. Errors are one typed family with only available identity/count evidence; hashes are not authentication.
