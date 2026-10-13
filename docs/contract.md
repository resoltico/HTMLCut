# Extraction contracts

## 2. Contract language

A contract is one JSON document: a **node**. Members are a closed set. Unknown members,
duplicate member names, wrong types and JSON `null` in place of an omitted member are
refused with `invalid_contract`. There is no version member; see section 11.

A node written as a JSON string `S` means `{"select": S}`.

| Member | Type | Meaning |
|---|---|---|
| `select` | CSS selector list | Elements matched within the scope. Omitted: the scope's anchor itself. |
| `where` | array of predicates | Keeps a match only when every predicate holds for it. Requires `select`. |
| `match` | `"one"` (default), `"optional"`, `"all"` | How many matches the contract accepts. |
| `min`, `max` | integers ≥ 0 | Bounds for `"all"`; default `min` 1 and no `max`. Only with `"all"`. |
| `nth` | integer ≥ 1 | Takes the nth match; at least that many must exist. Only with `"one"`, requires `select`. |
| `until` | CSS selector | Extends each match over its following siblings up to that boundary. Requires `select`. |
| `fields` | object: name → node | Makes the node a record. Names are 1–64 characters of `A-Z a-z 0-9 _ -`. |
| `read` | reading | Value nodes only; default `"text"`. |
| `exclude` | array of CSS selectors | Subtrees left out of `text`, `text-content` and `outer-html`. Value nodes only. |
| `equals` | string | The value must equal it. Value nodes only; exclusive with `pattern`. |
| `pattern` | regular expression | The value must match (search). A group named `value` narrows the value to that group. |
| `expect` | array of predicates | Must hold in the scope where this node is evaluated. |

A node with `fields` is a **record node**; any other node is a **value node**.

A **predicate** is a selector string or an object with only `select`, `read`
(`text`, `text-content` or `attr:NAME`; default `text`), `equals` or `pattern`, `min` and
`max` (both default 1). A string `S` means `{"select": S}`.

Grammar bounds, all refused at compile time with a message naming the bound: contract
text 256 KiB; 512 nodes; records nested 8 deep; 64 fields per record; 32 entries in each
`where`, `expect` and `exclude`; selector 8 KiB with the existing syntax-nesting and
matching-depth admission (64); pattern 8 KiB with the existing compiled-size limit;
`min`, `max` and `nth` at most 1,000,000.

### 2.1 Evaluation

A **scope** is an anchor element plus an **extent**: an ordered run of sibling nodes that
starts with the anchor. The root scope's anchor is the document element and its extent
is that element alone.

Evaluating a node in a scope:

1. **Expect.** Each `expect` predicate, in order, must hold in the scope. Otherwise
   refuse with `expectation_failed`.
2. **Select.** With `select`: every element that is an extent member or a descendant of
   one, and that matches the selector, in document order. The selector is matched against
   the whole document, so it may name ancestors outside the scope; `:scope` matches the
   anchor. Without `select`: the anchor alone.
3. **Filter.** With `where`: keep a candidate only if every predicate holds in the scope
   whose anchor and sole extent member is that candidate.
4. **Count.** `one`: exactly 1, or with `nth` N at least N and the Nth is taken.
   `optional`: 0 or 1. `all`: between `min` and `max`. Otherwise refuse with
   `count_mismatch`.
5. **Read.** For each chosen element, in order, form its extent: the element alone, or
   with `until` the element followed by its following sibling nodes up to, and excluding,
   the first following sibling element that matches `until` (matched with `:scope` bound
   to the chosen element). Then:
   - record node: an object whose members are the fields, each evaluated in the scope
     (chosen element, its extent), in byte order of field name;
   - value node: the reading of the extent with `exclude` applied, then `equals` or
     `pattern`. A mismatch, or a `value` group that did not participate, refuses with
     `value_mismatch`. A participating `value` group replaces the value.
6. **Result.** `all`: a JSON array. `optional` with no match: `null`. Otherwise the single
   value or record.

A predicate **holds** in a scope when the elements it selects there (step 2 with the
predicate's `select`) number between its `min` and `max`, and, if it has `equals` or
`pattern`, the reading of every selected element satisfies it. A predicate with an `attr:NAME` reading requires every selected element to have that
attribute, even without `equals` or `pattern`. A missing attribute makes a predicate not
hold; it is not a reading error. An expectation that fails for this reason carries no
`found`, `expected` or `value` facts.

Evaluation is depth-first, in document order, fields in byte order of name. The first
failure is reported and discards the whole result.

Consequences worth stating:

- A page-level expectation belongs on the root node, whatever the root selects.
- A row-level expectation belongs on a node evaluated inside the row, such as a field.
- A field without `select` reads the row's anchor; `{"read": "attr:id"}` reads its id.
- To read a row's whole extent as one string, re-select it:
  `{"select": ":scope", "until": "…"}`.
- Inside an extent, `:scope ~ *` selects the members after the anchor.

### 2.2 Readings

`attr:NAME` and `url:NAME` read the anchor of the extent. NAME is compared exactly with
the parsed attribute name, is 1–256 characters, and contains no white space or any of
`" ' < > / =`. A missing attribute refuses with `missing_attribute`.

`url:NAME` accepts any attribute. The value is stripped of ASCII white space and resolved
as the HTML standard defines the document base URL: the first `base` element with an
`href`, resolved against `--base-url`, else `--base-url`. A relative value with no usable
base refuses with `unresolvable_url`. Resolution and serialization follow the WHATWG URL
standard as implemented by the `url` crate.

`text-content` is the concatenation of all descendant text of the extent members, as DOM
`textContent`, without comments and without excluded subtrees.

`outer-html` is the html5ever serialization of each extent member in order, without
excluded subtrees.

`text` is the rendered structure of the extent, in the spirit of DOM `innerText` under
the default style sheet. It is deterministic and does not consult CSS.

1. If the extent is a single `script`, `style` or `template` element, the value is its
   `text-content`.
2. Otherwise traverse the members depth-first. Skip comments, doctypes, excluded
   subtrees, and descendant `script`, `style`, `template` and `head` elements in the HTML
   namespace and `script` and `style` in the SVG namespace.
3. Text accumulates into lines; a line is a sequence of cells.
4. A block element ends the current line before its first child and after its last.
   Block elements are, in the HTML namespace: `address article aside blockquote body
   caption center dd details dialog dir div dl dt fieldset figcaption figure footer form
   h1 h2 h3 h4 h5 h6 header hgroup hr html legend li listing main menu nav ol optgroup
   option p plaintext pre search section summary table tbody tfoot thead tr ul xmp`.
5. `br` ends the current line.
6. A `td` or `th` that is not the single extent member and has a preceding sibling `td`
   or `th` that is not excluded starts a new cell.
7. Inside `pre`, `listing`, `plaintext`, `xmp` and `textarea`, text is verbatim: each LF
   ends the line, and a line ended there is kept even when blank.
8. A line containing verbatim text is emitted unchanged, cells joined by U+0009.
9. In any other line each cell has every run of Unicode `White_Space` collapsed to one
   U+0020 and is trimmed; cells are joined by U+0009; a line whose cells are all empty is
   dropped.
10. Lines are joined by U+000A, without an added framing LF; a preserved final blank line can contribute a separator.

### 2.3 Results

Values are JSON strings. Records are JSON objects with members in byte order of name.
Encoding is compact UTF-8 JSON without escaping of non-ASCII characters, followed by one
LF on the command line.

Count bounds are the defence against cleanly truncated documents: HTML permits omitted closing tags. Use declared row counts and page expectations, as in [the guarded rows case](../conformance/contract/08-rows-with-page-expectations/contract.json).

The compiled regex allowance is 8 MiB across the contract, partitioned between patterns and their DFA caches. The schema derives structural bounds from the same constants; CSS/regex syntax and byte bounds remain engine checks.
