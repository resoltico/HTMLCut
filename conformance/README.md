# Conformance corpus

The executable specification of HTMLCut. Every case is a directory. Core, runtime and the actual CLI consume the same independent literal inputs and oracles; `HTMLCUT_TEST_BINARY` selects an unpacked production binary for delivered acceptance.

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

`fixture.rs` is a repository-only Rust path module shared by all three adapters. Discovery rejects empty extraction or outline groups, redirected directories, nonregular files, unexpected entries, missing/conflicting/irrelevant oracles and invalid metadata. Options and outline metadata use closed types: unknown/duplicate keys, null and out-of-range unsigned limits refuse. Missing optional metadata alone selects product defaults. Contract text remains opaque, preserving malformed JSON, duplicates and numeric token spelling for the compiler. `README.md` and `PROVENANCE.md` are permitted human documentation in case directories. Ordinary Cargo tests require no Python or runtime compiler invocation.

Core uses one execution context through compilation, export admission, preparation, evaluation and export. Core and runtime compare extracted values with `expected.json`; all three adapters compare complete output with the selected byte oracle. Runtime also checks fresh export and named Save bytes. CLI invocations use bounded capture and require confirmed child cleanup. Native I/O, authorization, cancellation and commit tests remain separate.

## Provenance and coverage

The corpus contains offline authored examples and realistic pages, generated stress inputs and minimized regressions. The `authored-*` pages are synthetic, including `authored-rendered`; its name does not indicate a browser capture.

The authored offer family combines caller encoding, conflicting meta, relative first base, legacy versus UTF-8 URL queries, conjunctive filtering, optional fields and captures, with decoding and semantic refusals. Navigation/template duplicates and script contents stay inert. The report family combines prose, exclusions, pre/code, table and definition structures and an outline over the same bytes. The static-source example preserves CSS-hidden source text and excludes script text without executing it.

Focused public-core tables independently exhaust the declared five reading families × three cardinalities × zero/one/two matches and seven numeric positions × ten raw tokens. Compiler/schema tests cover expressible grammar restrictions without claiming schema can validate duplicate keys, raw numeric spellings or selector/regex compilation. Runtime and CLI carrier tables cover their finite supported source combinations, while representation/destination tests check complete admission, preservation on refusal and retry. See [the contract reference](../docs/contract.md), [runtime export tests](../crates/htmlcut-runtime/tests/byte_exports.rs), [carrier and preflight tests](../crates/htmlcut-cli/tests/input_contract_boundaries.rs) and [native QA boundaries](../docs/qa-resources.md).
