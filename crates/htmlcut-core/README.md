# htmlcut-core

A pure engine for caller-owned HTML bytes. Compile a JSON `Contract` or typed `QuickQuery`, admit a `Document` with `PreparationPolicy`, then evaluate with an independent `ExecutionPolicy` and `ExecutionContext`. Share the same context across all phases of a job; it owns one absolute deadline. Documents retain semantic state only and safely implement `Send + Sync` without unsafe trait implementations.

`Document::extract` returns complete owned lexical values. `Value::export` implements canonical JSON and raw semantics. `Document::outline` returns bounded grouped facts in `OutlineReport`, whose renderer produces canonical text. Canonical materialization admission includes final LF for JSON, schema and outline. Raw exports also obey canonical JSON admission and their own complete byte bound. Per-source transformed reading admission remains separate from output admission.

The core forbids unsafe code, starts no threads and reads or writes no files or streams. Logical limits and cooperative checks do not establish physical containment. Application frontends use [htmlcut-runtime](https://github.com/resoltico/HTMLCut/tree/main/crates/htmlcut-runtime); low-level consumers can use the [packaged example](examples/contract.rs).

Child sibling ranges stay inside inherited membership. Structural readings without a selector inherit the whole range; attribute and URL readings use its anchor. See [the language](https://github.com/resoltico/HTMLCut/blob/main/docs/contract.md) and [execution guarantees](https://github.com/resoltico/HTMLCut/blob/main/docs/design.md).
