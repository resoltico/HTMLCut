---
afad: "4.0"
version: "20.0.0"
domain: OPERATIONS
updated: "2026-10-05"
route:
  keywords: [operations, extract, run, inspect, describe, schema]
  questions: ["What operations does the CLI expose?"]
---

# Operations

Use standard native help for current commands: `extract`, `inspect`, `replay` and `schema`.

Extract compiles current queries and reads complete strings/records from caller-owned file/stdin HTML. Inspect surveys repeated groups or, with select, counts and samples explicit nodes. Replay recomputes and verifies a closed source/query/configuration bundle. Schema retrieves one optional exhaustive named shape. No describe catalog or command aliases remain.

Cardinality, readings and assumptions are explicit. Acquisition, rendering, business transformations and comparison belong to callers. Failed assumptions never become alternate successful representations; extraction never truncates success. Bounded observation omissions are labeled. See [CLI](cli.md), [Core](core.md) and [Schemas](schema.md).
