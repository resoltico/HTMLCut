---
afad: "4.0"
version: "19.1.0"
domain: OPERATIONS
updated: "2026-10-05"
route:
  keywords: [operations, extract, run, inspect, describe, schema]
  questions: ["What operations does the CLI expose?"]
---

# Operations

The binary exposes `extract`, `run`, `inspect`, `describe`, and `schema` through one maintained command vocabulary.

`extract` validates/compiles an explicit plan and returns only requested strings or records from a file/stdin snapshot. `run` recomputes and verifies a self-contained bundle. `inspect` counts an explicit selector completely and returns bounded samples; `--identifiers` shows only exact `id` and class values with structural text previews for selector authoring. `describe` retrieves the compact index or one operation's defaults; `schema` retrieves one named current shape.

Selection assumptions and data representations are explicit. Acquisition, rendering, business transformation and comparison policy belong to callers. Failures never become alternate successful projections or silently shortened extraction values. Inspection sample abbreviation is separately labeled. See [CLI](cli.md), [Core](core.md) and [Schemas](schema.md).
