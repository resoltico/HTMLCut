# Extraction contracts

## Contract language

A contract is one JSON document: a **node**. Members are a closed set. Unknown members,
duplicate member names, wrong types and JSON `null` in place of an omitted member are
refused with `invalid_contract`. There is no version member; see [Stability](design.md#stability).

A node written as a JSON string `S` means `{"select": S}`.

| Member | Type | Meaning |
|---|---|---|
| `select` | CSS selector list | Elements matched within the scope. Omitted: the scope's anchor itself. |
| `where` | array of predicates | Keeps a match only when every predicate holds for it. Requires `select`. |
| `match` | `"one"` (default), `"optional"`, `"all"` | How many matches the contract accepts. |
| `min`, `max` | integers ≥ 0 | Bounds for `"all"`; default `min` 1 and no `max`. Only with `"all"`. |
| `nth` | integer ≥ 1 | Takes the nth match; at least that many must exist. Only with `"one"`, requires `select`. |
| `until` | CSS selector | Extends each match over its following siblings up to that boundary. Requires `select`. |
| `fields` | object: name → node | Makes the node a record with 1–64 fields. Names are 1–64 characters of `A-Z a-z 0-9 _ -`. |
| `read` | reading | Value nodes only; default `"text"`. |
| `exclude` | array of CSS selectors | Subtrees left out of `text`, `text-content` and `outer-html`. Value nodes only, and refused beside `attr:` or `url:` readings. |
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

### Evaluation

A **scope** is an anchor element plus an **extent**: an ordered run of sibling nodes that
starts with the anchor. The root scope's anchor is the document element and its extent
is that element alone.

Evaluating a node in a scope:

1. **Expect.** Each `expect` predicate, in order, must hold in the scope. Otherwise
   refuse with `expectation_failed`.
2. **Select.** With `select`: every element that is an extent member or a descendant of
   one, and that matches the selector, in document order. Template contents are outside
   the document and never selection candidates. The selector is matched against
   the whole document, including outside ancestors/siblings, :has and positional facts; returned candidates remain members of the inherited range; `:scope` matches the
   anchor. Without `select`: the anchor alone.
3. **Filter.** With `where`: keep a candidate only if every predicate holds in the scope
   whose anchor and sole extent member is that candidate.
4. **Count.** `one`: exactly 1, or with `nth` N at least N and the Nth is taken.
   `optional`: 0 or 1. `all`: between `min` and `max`. Otherwise refuse with
   `count_mismatch`.
5. **Read.** For each chosen element, in order, form its extent: the element alone, or
   with `until` the element followed by its following sibling nodes up to, and excluding,
   the first following sibling element that matches `until` (matched with `:scope` bound
   to the chosen element). A final section without a stop naturally ends at that inherited boundary. Without `select`, structural readings inherit the whole range; attribute/URL readings use its anchor. Explicit `:scope` without `until` reads only the anchor. Then:
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

Template contents are never candidates for `select`, `where`, `expect`, `exclude` or
`until`. Reading a selected template itself preserves its contents; `outer-html`
serialization includes template contents, which exclusions cannot select.

Evaluation is depth-first, in document order, fields in byte order of name. The first
failure is reported and discards the whole result.

Consequences worth stating:

- A page-level expectation belongs on the root node, whatever the root selects.
- A row-level expectation belongs on a node evaluated inside the row, such as a field.
- A field without `select` inherits the row's whole range; `{"read": "attr:id"}` reads only its anchor id.
- To read only the heading, use `{"select": ":scope"}`.
- Where a row range includes a required footer, a body field can use `{"expect":[{"select":"footer","min":1,"max":1}],"select":":scope","until":"footer"}`. Its expectation checks the inherited row before the footer is excluded; a footer outside that row cannot satisfy it.
- Inside an extent, `:scope ~ *` selects the members after the anchor.

### Readings

`attr:NAME` and `url:NAME` read the anchor of the extent. NAME is compared exactly with
the parsed attribute name, is 1–256 characters, and contains no white space or any of
`" ' < > / =`. A missing attribute refuses with `missing_attribute`.

`url:NAME` accepts any attribute. The value is stripped of ASCII white space and resolved
as the HTML standard defines the document base URL: the first `base` element with an
`href`, resolved against `--base-url`, else `--base-url`. Invalid, `data:` and `javascript:` document bases retain the fallback, and later bases do not override the first. Direct absolute attribute schemes remain available. A relative value with no usable
base refuses with `unresolvable_url`. Resolution and serialization follow the WHATWG URL
standard as implemented by the `url` crate.

`text-content` is the concatenation of all descendant text of the extent members, as DOM
`textContent`, without comments and without excluded subtrees. Descendant template contents are
skipped unless the sole extent member is that template; nested templates remain skipped.

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

### Selectors and patterns

**Accepted selectors.** These examples pin the supported forms in the conformance corpus:

- Type, universal, id and class selectors: `main p`, `#u1 > *`, `#s1`, `.Card`,
  `section.card.featured#s1`.
- Attribute presence, equality, whitespace-word, language-prefix, prefix, suffix and
  substring operators, plus the ASCII-insensitive i flag:
  `[disabled]`, `[lang="fr"]`, `[data-kind~="two"]`, `section[lang|="en"]`,
  `a[href^="https:"]`, `a[href$=".pdf"]`, `[data-kind*="a-o"]`,
  `[data-kind="ALPHA-ONE" i]`, `[name=agree]`.
- Descendant, child, next-sibling and later-sibling combinators and selector lists:
  `main p`, `main > section > p`, `h2 + p`, `h2 ~ a`, `h2, aside p`.
- Logical pseudo-classes :not(), :is() and :where(): `section:not(.featured)`,
  `main > :not(section, #nothing)`, `:is(aside, ul) > *`,
  `:where(section, aside)[id$="1"]`.
- Relative :has(), without nesting another :has(): `section:has(> a)`,
  `main :has(.on)`, `h2:has(+ p.lead)`, `section:has(input, span)`.
- Positional pseudo-classes with an+b, even and odd: `li:nth-child(2n+1)`,
  `li:nth-last-child(2)`, `section:nth-of-type(2)`, `section:nth-last-of-type(1)`,
  `li:nth-child(even), li:nth-child(odd)`.
- First, last and only child/of-type forms: `li:first-child`, `li:last-child`,
  `aside > :only-child`, `section > p:first-of-type`, `section > p:last-of-type`,
  `section > span:only-of-type`.
- Tree and scope pseudo-classes: `p:empty`, `:root > body > main`,
  `:scope > body > main`.

**Case.** HTML element and attribute names match in any ASCII case. Parsed SVG/foreign names retain their case: `linearGradient` and `viewBox` are exact; `lineargradient` and `viewbox` do not match them. Ids and classes match
exactly. Attribute values match exactly except values HTML defines as insensitive,
such as `type`.

**Not available.** Pseudo-elements; state and linguistic pseudo-classes such as
`:checked`, `:disabled`, `:link` and `:lang()`; `:nth-child(… of S)`; `:contains()`;
and a nested `:has()`. They are refused as `invalid_selector`. Use attributes such as
`[checked]`, `[disabled]` and `a[href]`, and predicates for text. A later minor version
may accept more forms; refusals are not frozen by conformance cases.

**Template.** A template has no document children and matches `:empty`.

**Patterns.** Rust's `regex` syntax, with Unicode-aware search matching. `\d`, `\w`
and `\s` match Unicode classes; use `[0-9]` for ASCII digits. `^` and `$` match the
start and end of the value. `(?m)` enables line anchors, `(?s)` lets `.` match LF, and
`(?i)` ignores case. Both `(?<value>…)` and `(?P<value>…)` name the narrowing group.
Look-around and backreferences are not available.

## Results

Values are JSON strings. Records are JSON objects with members in byte order of name.
Encoding is compact UTF-8 JSON without escaping of non-ASCII characters, followed by one
LF on the command line.

Count bounds are the defence against cleanly truncated documents: HTML permits omitted closing tags. Use declared row counts and page expectations, as in [the guarded rows case](../conformance/contract/08-rows-with-page-expectations/contract.json).

The compiled regex allowance is 8 MiB across the contract, partitioned between patterns and their DFA caches. The schema derives structural bounds from the same constants; CSS/regex syntax and byte bounds remain engine checks.


### Fetch before extraction

FILE is a local snapshot, not a URL. Use the [two-phase acquisition example](../examples/acquire_snapshot.py):

```sh
python3 examples/acquire_snapshot.py htmlcut https://example.test/catalogue contract.json result.json --report acquisition.json
```

It fetches into owned temporary storage with curl fail-status, redirects, decompression
and a transport timeout, with a 64 MiB download cap. Only successful complete 2xx HTML acquisition launches extraction,
with the final effective URL and declared HTTP charset. A failed/truncated transfer never
updates the result. BOM precedence remains the engine's decoding rule. A loose extraction
can succeed on plausible partial HTML, so piping an unchecked fetch into HTMLCut—even with
shell pipefail—cannot undo an already published result. Counts/business expectations remain
necessary for entity/completeness checks; JavaScript snapshots stay inert. Network/session
policy, authentication, pagination and rendering belong to the caller.

JSON Schema checks authoring structure, not complete compilation. Exact unsigned-integer
tokens are required: `1` is valid, `1.0`, `1e0`, `-0` and rounded/underflowing decimals are
refused. A validator over parsed mathematical values cannot preserve token spelling or
recover duplicate keys. CSS/regex compilation, global limits and cross-count relations
remain compiler-authoritative.
