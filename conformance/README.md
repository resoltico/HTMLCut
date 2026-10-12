# Conformance corpus

The executable specification of HTMLCut. Every case is a directory.

| File | Meaning |
|---|---|
| `page.html` | The document, as exact bytes. Some are deliberately not UTF-8. |
| `options.json` | Optional: `base_url`, `encoding`, `max_nodes`. |
| `contract.json` | The contract to run. |
| `expected.json` | The result on success. Compare as JSON values. |
| `error.json` | The failure. Every member except `message` must equal the produced error. |
| `outline.json` | Outline options instead of a contract: `select`, `depth`, `max_lines`, `attrs`. |
| `expected.txt` | The outline, byte for byte. |

`contract/` is append-only within a major version: changing or removing a case there is
a breaking change. `outline/` pins a reader-facing format and may change in a minor
release.
