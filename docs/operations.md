# Ordinary-files reproduction

Keep the exact UTF-8 `source.html`, normalized `query.json`, explicit base URL and
preparation policy in ordinary files, `output.json`, and the exact executable plus
its `--version` (or an immutable release source/package reference). The CLI uses
`PreparationLimits::default()`; Rust callers should serialize their actual policy.
Obtain normalized query bytes through `CompiledPlan::normalized_json()`.

```sh
htmlcut --version > executable-version.txt
htmlcut extract --file source.html --plan query.json --base-url https://example.test/ --output output.json
htmlcut extract --file source.html --plan query.json --base-url https://example.test/ > recomputed.json
cmp output.json recomputed.json
```

Retain the explicit base argument with those files; omit it only when no base was used.
Ordinary tools can copy the files and compare complete output. This loses one-command
self-contained replay and proves neither authenticity nor cross-version equivalence.
No custom archive or manifest protocol is supplied.
