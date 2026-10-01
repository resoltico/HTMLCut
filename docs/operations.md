---
afad: "4.0"
version: "15.0.0"
domain: OPERATIONS
updated: "2026-09-30"
route:
  keywords: [operations, extract, run, inspect, describe, schema]
  questions: ["What operations does the CLI expose?"]
---

# Operations

| Operation | Purpose |
| --- | --- |
| `extract` | One selection and one requested projection over an acquired snapshot. |
| `run` | Execute an explicit saved acquisition/plan specification. |
| `inspect` | Bounded snapshot-bound descriptors, incomplete previews and labelled selector suggestions. |
| `describe` | Compact index or one named operation's defaults/failure behavior. |
| `schema` | One named schema, without a full-catalog default. |

```sh
htmlcut describe
htmlcut describe extract
htmlcut inspect --file page.html --page-size 5
```

A proposal is a suggestion, requiring explicit caller adoption. Positional selectors promise position, not durable meaning. Cursors/handles bind bytes, metadata, preparation/discovery policy and semantics. Changed snapshots/options reject old evidence. Live URLs cannot be refetched with cursor/handle input; callers supply the same saved snapshot and base metadata. Preview safety-budget failures remain errors.
