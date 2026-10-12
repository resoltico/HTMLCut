# Schemas

`schema NAME` generates one closed contract schema from the current Rust types.
Canonical names are `htmlcut.extraction.plan`, `htmlcut.extraction.data`,
`htmlcut.extraction.error` and `htmlcut.inspection`.
Plans require integer version 8 and a nonempty select member. Unknown or duplicate
members, explicit null in place of omitted default members, retired readings and
following_siblings are refused. There is no version-7 reader or survey schema.
Runtime validation also enforces cross-member rules such as cardinality, predicate
compatibility and aggregate limits; a generated schema alone does not establish
that a query will execute successfully.
