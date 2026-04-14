pub(crate) const ROOT_LONG_ABOUT: &str = "\
HTMLCut has five operator-facing entry points:

  catalog  Print the capability catalog with stable operation IDs.
  schema   Export validator-grade JSON schemas for HTMLCut's public JSON contracts.
  inspect  Explore a source or preview a request before committing to a final extraction.
  select   Extract final values from CSS selector matches.
  slice    Extract final values between literal or regex boundaries in raw source text.

Discovery flow:
  1. catalog lists stable operation IDs plus the CLI/core, request/result contract refs, usage, typed defaults, command constraints, modes, parameters, notes, and examples for each operation.
  2. schema exports the validator-grade JSON Schema documents behind those contract refs.
  3. inspect source learns document shape, headings, links, classes, and effective base URL.
  4. inspect select or inspect slice previews matches in structured JSON before extraction.
  5. select or slice emits the final payload once you trust the request.

Inputs:
  <INPUT> may be a local file path, an http:// or https:// URL, or - for stdin.
  Bundle directories are created automatically when you use --bundle.

Output model:
  select and slice separate the value you extract from how stdout is rendered:
    --value   what each match produces
    --output  how stdout is emitted

  inspect defaults to JSON so agents can reason about the source and preview report.
  Use --output text for a compact human summary.

URL resolution:
  --rewrite-urls resolves relative links with the effective base URL.
  The effective base comes from --base-url when supplied, the input URL for URL sources,
  and any document <base href> when one is present.
  When no effective base can be resolved, HTMLCut leaves relative URLs unchanged and reports a warning.

Failure model:
  Human output modes print the primary failure to stderr.
  JSON output modes emit structured JSON to stdout and still exit non-zero.";
pub(crate) const ROOT_AFTER_HELP: &str = "\
Examples:
  htmlcut catalog --output json
  htmlcut schema --output json
  htmlcut select ./page.html --css article --match single
  htmlcut select ./page.html --css 'article a.more' --value attribute --attribute href --rewrite-urls
  htmlcut slice ./page.html --from '<article>' --to '</article>'
  htmlcut inspect source ./page.html
  htmlcut inspect select ./page.html --css '.card' --match all";
pub(crate) const CATALOG_LONG_ABOUT: &str = "\
Print HTMLCut's capability catalog.

Use this command to discover stable operation IDs, the command and core surfaces that expose each
operation, the public request/result contracts tied to that operation, and the machine-readable CLI
command contract when one exists, including parameter inventory, typed defaults, command
constraints, and schema references.

Use --output json when an agent or script wants machine-readable capability introspection.";
pub(crate) const CATALOG_AFTER_HELP: &str = "\
Examples:
  htmlcut catalog
  htmlcut catalog --output json
  htmlcut catalog --operation source.inspect
  htmlcut catalog --operation slice.extract --output json";
pub(crate) const SCHEMA_LONG_ABOUT: &str = "\
Export HTMLCut's validator-grade JSON schema registry.

Use this command when a downstream tool needs the actual JSON Schema documents for HTMLCut's public
JSON contracts instead of descriptive capability text.

The registry includes:
  - htmlcut-core request/result schemas
  - htmlcut-cli report schemas
  - the frozen interop schemas shipped by htmlcut_core::interop::v1

Use --name to select one schema family and --schema-version to pin one exact version.";
pub(crate) const SCHEMA_AFTER_HELP: &str = "\
Examples:
  htmlcut schema
  htmlcut schema --output json
  htmlcut schema --name htmlcut.extraction_result --output json
  htmlcut schema --name htmlcut.result --schema-version 1 --output json";
pub(crate) const SELECT_LONG_ABOUT: &str = "\
Extract values from CSS selector matches.

Use inspect source first when you need to learn the document shape, then inspect select
to preview matches before emitting the final payload.

The selector is required. Use --value to choose what each selected match produces:
  text        plain text derived from the matched node
  inner-html  inner HTML of the matched node
  outer-html  outer HTML of the matched node
  attribute   one attribute value from the matched node
  structured  a metadata-rich JSON object for each match

Use --match single|first|nth|all to decide how many matches survive.
Use --output to choose how stdout is emitted. JSON output emits the full structured report.
Structured extraction only supports --output json or --output none.
When --rewrite-urls is requested but no effective base can be resolved, HTMLCut leaves relative URLs unchanged and reports a warning.";
pub(crate) const SELECT_AFTER_HELP: &str = "\
Examples:
  htmlcut select ./page.html --css article --match single
  htmlcut select ./page.html --css '.card' --match all --value outer-html
  htmlcut select ./page.html --css 'article a.more' --value attribute --attribute href --rewrite-urls";
pub(crate) const SLICE_LONG_ABOUT: &str = "\
Extract values between start and end boundaries in the raw source text.

Use --pattern literal for plain substring boundaries or --pattern regex for regex boundaries.
Literal matching is raw substring matching, not tag-aware: `<a` also matches `<article>`.
Boundary matches are consumed exactly as matched.
By default, the selected fragment excludes both matched boundaries.
Use --include-start and/or --include-end when the boundary text itself must remain inside the fragment.
For --value inner-html, HTMLCut returns the selected fragment as HTML.
For --value outer-html, HTMLCut returns the full outer matched range including both boundaries.
When extracting --value attribute from sliced HTML, use --include-start when the opening tag lives in the start boundary.
Use inspect slice to confirm the exact ranges or choose stricter boundaries when you need tag-like behavior.
Use inspect slice first when you want structured previews of the candidate ranges.";
pub(crate) const SLICE_AFTER_HELP: &str = "\
Examples:
  htmlcut slice ./page.html --from '<article>' --to '</article>'
  htmlcut slice ./page.html --from 'START::' --to '::END' --pattern regex --match all --output json
  htmlcut slice ./page.html --from '<a ' --to '</a>' --include-start --include-end --value attribute --attribute href";
pub(crate) const INSPECT_LONG_ABOUT: &str = "\
Inspect a source or preview a request before extracting the final payload.

inspect source    summarizes the parsed document, headings, links, classes, and base URL behavior.
inspect select    previews selector matches using structured per-match metadata.
inspect slice     previews literal or regex slices using structured range metadata.";
pub(crate) const INSPECT_SOURCE_LONG_ABOUT: &str = "\
Inspect the parsed document itself.

This command summarizes title, counts, headings, link previews, top tags, top classes,
document base behavior, and optional source text. It is designed to help you choose
selectors or confirm how URL rewriting will behave before extracting data.";
pub(crate) const INSPECT_SOURCE_AFTER_HELP: &str = "\
Examples:
  htmlcut inspect source ./page.html
  htmlcut inspect source ./page.html --output text --include-source-text --preview-chars 200";
pub(crate) const INSPECT_SELECT_LONG_ABOUT: &str = "\
Preview selector matches without committing to a final extraction payload.

This command always inspects matches in structured form and defaults to JSON output.
When --rewrite-urls is requested but no effective base can be resolved, the preview keeps relative URLs and reports a warning.";
pub(crate) const INSPECT_SELECT_AFTER_HELP: &str = "\
Examples:
  htmlcut inspect select ./page.html --css article --match single
  htmlcut inspect select ./page.html --css '.card' --match all --output text";
pub(crate) const INSPECT_SLICE_LONG_ABOUT: &str = "\
Preview literal or regex slices without committing to a final extraction payload.

This command always inspects slices in structured form and defaults to JSON output.
Boundary matches are consumed exactly as matched, so default previews exclude both matched boundaries.
If the matched --from pattern already includes the payload you wanted, narrow the boundary or switch on --include-start.
Text output shows the selected text and, when it adds signal, a fragment snippet so boundary-consumption mistakes are easier to spot.";
pub(crate) const INSPECT_SLICE_AFTER_HELP: &str = "\
Examples:
  htmlcut inspect slice ./page.html --from '<article>' --to '</article>'
  htmlcut inspect slice ./page.html --from 'START::' --to '::END' --pattern regex --output text";
