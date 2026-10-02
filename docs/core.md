---
afad: "4.0"
version: "16.0.0"
domain: CORE
updated: "2026-10-01"
route:
  keywords: [core, immutable snapshots, compiled plans, prepared documents, projections, guards, bounded discovery, identities]
  questions: ["what is the maintained htmlcut-core surface?", "what does the core schema registry cover?", "how should a Rust caller embed htmlcut-core?"]
---

# Core API

Supply accepted UTF-8 bytes and explicit metadata. The snapshot preserves source spelling and CRLF; the parsed DOM follows HTML parsing rules. Prepared documents parse lazily at most once and cache failures. Compile plans once and reuse them.

```rust
use htmlcut_core::{CompiledPlan, ExtractionPlan, PreparationLimits, PreparedDocument,
    Projection, SnapshotMetadata, SourceSnapshot};
let snapshot = SourceSnapshot::new("<p id='amount'>EUR 180</p><a href='next'>Link</a>", SnapshotMetadata::default())?;
let document = PreparedDocument::new(snapshot, PreparationLimits::default())?;
let amount = CompiledPlan::compile(&ExtractionPlan::css("#amount")?)?;
let mut link = ExtractionPlan::css("a")?;
link.projection = Projection::Attribute { name: "href".into() };
let link = CompiledPlan::compile(&link)?;
assert_eq!(document.execute(&amount)?.values, ["EUR 180"]);
assert_eq!(document.execute(&link)?.values, ["next"]);
# Ok::<(), htmlcut_core::ExtractionError>(())
```

Slicing returns exact accepted bytes and requires no DOM parsing:

```rust
use htmlcut_core::{Boundary, CompiledPlan, ExtractionPlan, PreparationLimits,
    PreparedDocument, SnapshotMetadata, SourceSnapshot};
let document = PreparedDocument::new(SourceSnapshot::new("STARTéEND", SnapshotMetadata::default())?, PreparationLimits::default())?;
let plan = ExtractionPlan::slice(Boundary::Literal { value: "START".into() }, Boundary::Literal { value: "END".into() })?;
let result = document.execute(&CompiledPlan::compile(&plan)?)?;
assert_eq!(result.values, ["é"]);
assert_eq!(result.ranges.unwrap()[0].start, 5);
# Ok::<(), htmlcut_core::ExtractionError>(())
```

One plan selects once and projects once. Single is the default; all and one-based nth are explicit. Present empty attributes/text are valid; absence fails. Context guards are conjunctive and every matched value must satisfy the exact or regex-search predicate. Their original-DOM reads cannot be weakened by output exclusions or transforms.

Core preparation/execution limits bind identities. Adapter acquisition/framing limits do not. Source and normalized-plan digests prove identity, not authenticity or business correctness. Callers own browsers, domain mapping, history and comparison policy.

URL processing accepts at most 8 KiB raw URL/base inputs and uses a finite 32 KiB processing cap; the requested value cap is enforced before returning the resolved string. Oversized metadata is an explicit limit error. This does not change untransformed attribute extraction.

Structural list ordinals follow HTML signed-integer prefix parsing, including leading ASCII whitespace. Invalid prefixes use default ordinals; numeric overflow fails. Reversed default counts reflect the explicitly filtered direct list items. An unused terminal next-ordinal overflow does not discard the current value. Fences account for literal text, alternative text, destinations and nested formatting.

## Fragment interpretation and structural encoding

Text normalization consults original preformatted ancestry, preserving significant selected payload newlines and indentation. Structural fragments derive only the context needed to represent their selected content; surrounding payload is not included. Ordered list items retain source-derived ordinals, including start, reversed and prior value resets. Exclusions filter output without renumbering original remaining items.

Structural table output uses balanced `[table]`, `[caption]` and `[cell]` frames. Adjacent cells are separated by ` | ` outside cell frames. Source backslash, brackets, parentheses, pipe and backtick characters outside generated pre fences are escaped with a backslash. Fenced code remains literal, including newlines and delimiter-looking text. Nested table and cell frames are balanced. A selected row containing Taxi and 180 renders `[cell]Taxi[/cell] | [cell]180[/cell]`. This is a deterministic structural convention, not complete DOM reconstruction or universal Markdown conversion.

Each selector/scope pass reuses its own matching scratch while charging actual traversal and predicate work. Prepared reuse gets fresh operation budgets; no global cache supplies free work.

Pre fence separators each own exactly one newline. The closing separator is emitted in addition to any payload trailing newlines, so decoding can preserve their count.

Budgeted selector scratch borrows one immutable document. Scopes and candidates from another document are refused before matching; scope-specific and repeated document passes create fresh caches under fresh work budgets. The maintained fork exposes distinct work-exhaustion and document-mismatch Rust errors.

Table/caption/row/cell framing applies to HTML namespace roles. Foreign SVG/MathML elements sharing those local names retain their text without invented HTML frames. Missing context for a genuine HTML row/cell is an internal invariant failure.
