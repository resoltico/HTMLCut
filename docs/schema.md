# Schemas

`htmlcut schema NAME` generates the current schema directly from Rust types.
Names: `htmlcut.extraction.plan`, `htmlcut.extraction.data`, `htmlcut.extraction.error`,
`htmlcut.inspection`, `htmlcut.survey`. Ordinary use needs no schema retrieval.
Queries require integer version 7 and select; semantics version is 7. Unknown
members, enum values, versions, null defaults and duplicate JSON keys fail directly.
Cross-field invariants are enforced by the typed compiler. Bare data has no envelope;
empty records and values both encode as `[]`. Their typed enum variants differ.
See [contract](core.md) for cardinality, scopes, readings and limits.
