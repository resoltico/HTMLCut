# htmlcut-core

A deterministic engine for caller-owned HTML document bytes. Compile a `Contract` from the closed JSON contract language, prepare a `Document` with `Source` and `Limits`, then reuse `Document::extract` and `Document::outline`. Results are owned lexical values; a failure includes repair facts. The packaged [example](examples/contract.rs) shows the public API.

The library starts no threads and reads or writes no files or streams. Preparation decodes without replacement and admits retained nodes and transient depth before freezing an opaque document. Each preparation, extraction and outline call has a fresh cooperative deadline. `max_output_bytes` bounds encoded extraction results and outline text; `max_reading_bytes` separately bounds every transformed scalar, including complete predicate and capture sources. These logical limits do not provide physical process or memory containment.

Child sibling ranges stay inside inherited membership. Structural readings without a selector inherit the whole range; attribute and URL readings use its anchor.

See [the language](https://github.com/resoltico/HTMLCut/blob/main/docs/contract.md), [execution guarantees](https://github.com/resoltico/HTMLCut/blob/main/docs/design.md) and [conformance cases](https://github.com/resoltico/HTMLCut/tree/main/conformance).
