# htmlcut-core

A deterministic engine for caller-owned HTML document bytes. Compile a Contract from closed JSON, prepare a Document with Source and Limits, then reuse Document::extract and Document::outline. Results are owned lexical values; a failure includes repair facts.

The library starts no threads and reads/writes no files or streams. Preparation decodes without replacement and admits retained owned nodes/transient depth during maintained parser callbacks before freezing a private DOM. Each extraction and outline has a fresh cooperative deadline and complete-output policy; `max_reading_bytes` separately bounds complete transformed sources for predicates/captures. Child sibling ranges stay inside inherited membership; omitted structural selection inherits the range. Operators provide any required physical containment.

See [the language](https://github.com/resoltico/HTMLCut/blob/main/docs/contract.md), [guarantees](https://github.com/resoltico/HTMLCut/blob/main/docs/design.md), [conformance](https://github.com/resoltico/HTMLCut/tree/main/conformance) and the packaged [example](examples/contract.rs).
