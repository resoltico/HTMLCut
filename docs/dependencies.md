# Dependency responsibilities and maintenance

The selected graph aligns html5ever/markup5ever 0.40, web_atoms/string_cache types,
selectors 0.41, servo_arc 0.5 and tendril 0.5.1. Actual owned package versions and
edges are declared in Cargo.toml/Cargo.lock and each carrier manifest. There is no
root-only patch: path, Git and registry consumers must receive the same corrections.

| Responsibility | Current owner | Replacement outcome |
| --- | --- | --- |
| DOM allocation/depth/reparenting admission | core `dom/parser.rs`, `dom/tree_sink.rs` | Implemented via public TreeSink over upstream ego-tree; scraper retired. |
| Static CSS grammar, inner navigation/predicate accounting, pass caches | core `dom/selector.rs`, `budget.rs` | Public Element callbacks verified with scope, nth/:has/ancestor/sibling/negation and cached/cold controls; internal engine budgets removed. |
| Filtered immutable HTML | core `dom/serialization.rs` | Upstream public Serialize boundary; obsolete scraper clone/mutation helpers removed. |
| Current-token parser stopping | htmlcut-html5ever | Upstream 0.40.1 still panics on `<b><p>X</b>Y`, elements=5, before the outer guard can stop it. |
| Stop trait and atom/tendril routing | htmlcut-markup5ever | Required by the parser hooks and corrected transitive tendril; no independent mature parser body change. |
| Safe relative traversal and refusal with cached nth | htmlcut-selectors | Upstream recursive depth-4000 search overflows; its extra debug nth recomputation panics after refusal. Two narrow engine corrections plus the Arc edge remain. |
| Tagged heap/shared string provenance | htmlcut-tendril | A single provenance-preserving NonNull tag restores 16-byte ARM64 layout; upstream integer reconstruction fails the specified strict Miri proof. |
| Tail/tag/borrow ownership | htmlcut-servo-arc | Tail pointer remains unconditional; independent upstream tail and borrowed-clone ownership controls fail, and tagged casts fail strict provenance. |

The retained parser/selector bodies are mature upstream sources with narrow declared
deltas, not replacement implementations. Carrier authorship and original license
texts/notices remain with their sources. Core adaptations retain ISC attribution.
Maintaining these five packages means reviewing upstream changes, keeping the type
graph aligned and rerunning the corresponding stopping/ownership/refusal controls.
No version-number comparison alone establishes safe removal.

Upstream pointer proof failures are precise evidence, not a claim of exploitability;
Stacked Borrows is experimental. One failed parser or pointer path does not halt
independent work. The product boundaries above were separately implemented and
challenged. Full registry-only upstream adoption remains unresolved while these
specific responsibilities require carriers.

Registry distribution is prepared and verified through an isolated dependency-
complete Cargo source registry, including package compilation and installation with
workspace path edges unavailable. This does not establish real registry ownership,
availability or external publication. See [release protocol](release-protocol.md)
for ordered packages, official dry-run prerequisites and exact operator actions.
