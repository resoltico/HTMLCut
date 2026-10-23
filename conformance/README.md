# Conformance corpus

The executable specification of HTMLCut. The public-core adapter and the black-box CLI adapter use the development-only `htmlcut-conformance` crate for discovery, typed options and independent literal oracles. `HTMLCUT_TEST_BINARY` selects a delivered executable.

Cases live below `contract/` area directories (including `read/text`, `read/attr-url` and `read/html`) and `outline/`. A case is a directory containing `page.html`; intermediate areas contain only directories. Discovery rejects empty areas/groups and compares its result with an independent recursive page count. Both adapters print every case identity and the executed total; use `-- --nocapture` to see successful-case output.

| File | Meaning |
|---|---|
| `page.html` | The document as exact bytes; some cases deliberately use malformed or non-UTF-8 input. |
| `options.json` | Optional `base_url`, `encoding`, `max_input_bytes`, `max_nodes`, `max_reading_bytes`, `max_output_bytes`, `raw` and `partial_error_facts`. |
| `contract.json` | Extraction contract. |
| `expected.json` | Complete canonical compact JSON plus LF on success, compared as values and exact bytes. |
| `expected.raw` | Complete raw export bytes when requested, including an empty optional-null export. |
| `error.json` | Literal error facts. Remove `message` before comparison. A nonempty string `error` discriminant is mandatory, including for partial facts. Duplicate top-level fact keys refuse without restricting the set of fact names. Objects must otherwise be equal unless `partial_error_facts` is true, in which case every authored fact must match. |
| `outline.json` | Outline options: `select`, `depth`, `max_lines`, `attrs`. |
| `expected.txt` | Complete canonical outline bytes. |

Expected values and error facts are authored independently of the engine. Error oracles use complete fact comparison by default; `partial_error_facts` selects an explicit subset.

Review fixture changes as semantic specification changes. Rejection controls exercise empty/incomplete discovery and incorrect oracles. Passing runs report executed case identities and counts.

The helper crate has no engine dependency. Discovery rejects redirected directories, nonregular files, unexpected entries, incomplete or conflicting oracles and malformed metadata. Contract text remains opaque so duplicates, malformed JSON and exact numeric token spellings reach the compiler. `README.md` and `PROVENANCE.md` are permitted case documentation.

Core compares values and exact bytes. CLI commands set stdin explicitly and compare stdout bytes or diagnostic facts plus exit status. Empty discovery, incomplete cases and deliberately wrong oracles have rejection controls. See the [contract specification](../docs/contract.md), [core adapter](../crates/htmlcut-core/tests/conformance.rs), [CLI adapter](../crates/htmlcut/tests/conformance.rs) and [input tests](../crates/htmlcut/tests/input_contract_boundaries.rs).

## Provenance

The corpus includes synthetic authored pages, generated stress inputs and minimized regressions. The authored rendered example is synthetic, not a browser capture. The offer family combines encoding, first-base resolution, URL queries, filtering, optional fields and captures; navigation/template duplicates and script content remain inert. Report examples combine prose, exclusions, pre/code, tables, definition structures and outline output. The static-source example preserves CSS-hidden source text without executing scripts.
