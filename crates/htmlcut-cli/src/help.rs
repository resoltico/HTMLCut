use std::sync::LazyLock;

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
  6. --emit-request-file saves the normalized extraction definition you can reuse with --request-file.

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
static CATALOG_AFTER_HELP: LazyLock<String> = LazyLock::new(|| {
    format!(
        "Examples:\n  htmlcut catalog\n  htmlcut catalog --output json\n  htmlcut catalog --operation {}\n  htmlcut catalog --operation {} --output json",
        htmlcut_core::OperationId::SourceInspect.as_str(),
        htmlcut_core::OperationId::SliceExtract.as_str(),
    )
});
pub(crate) const SCHEMA_LONG_ABOUT: &str = "\
Export HTMLCut's validator-grade JSON schema registry.

Use this command when a downstream tool needs the actual JSON Schema documents for HTMLCut's public
JSON contracts instead of descriptive capability text.

The registry includes:
  - htmlcut-core request/result schemas
  - htmlcut-cli report schemas
  - the frozen interop schemas shipped by htmlcut_core::interop::v1

Use --name to select one schema family and --schema-version to pin one exact version.";
static SCHEMA_AFTER_HELP: LazyLock<String> = LazyLock::new(|| {
    format!(
        "Examples:\n  htmlcut schema\n  htmlcut schema --output json\n  htmlcut schema --name {} --output json\n  htmlcut schema --name {} --schema-version 1 --output json",
        htmlcut_core::CORE_RESULT_SCHEMA_NAME,
        htmlcut_core::interop::v1::RESULT_SCHEMA_NAME,
    )
});
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
Use --emit-request-file to save the normalized extraction definition you can rerun later with --request-file.
When --rewrite-urls is requested but no effective base can be resolved, HTMLCut leaves relative URLs unchanged and reports a warning.";
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
Use --emit-request-file to save the normalized extraction definition you can rerun later with --request-file.
Use inspect slice first when you want structured previews of the candidate ranges.";
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
pub(crate) const INSPECT_SELECT_LONG_ABOUT: &str = "\
Preview selector matches without committing to a final extraction payload.

This command always inspects matches in structured form and defaults to JSON output.
Use --emit-request-file when you want to capture the normalized preview definition while iterating on inline flags.
When --rewrite-urls is requested but no effective base can be resolved, the preview keeps relative URLs and reports a warning.";
pub(crate) const INSPECT_SLICE_LONG_ABOUT: &str = "\
Preview literal or regex slices without committing to a final extraction payload.

This command always inspects slices in structured form and defaults to JSON output.
Boundary matches are consumed exactly as matched, so default previews exclude both matched boundaries.
If the matched --from pattern already includes the payload you wanted, narrow the boundary or switch on --include-start.
Use --emit-request-file when you want to capture the normalized preview definition while iterating on inline flags.
Text output shows the selected text and, when it adds signal, a fragment snippet so boundary-consumption mistakes are easier to spot.";

pub(crate) fn catalog_after_help() -> &'static str {
    CATALOG_AFTER_HELP.as_str()
}

pub(crate) fn schema_after_help() -> &'static str {
    SCHEMA_AFTER_HELP.as_str()
}

static SELECT_AFTER_HELP: LazyLock<String> =
    LazyLock::new(|| operation_examples_after_help(htmlcut_core::OperationId::SelectExtract));
static SLICE_AFTER_HELP: LazyLock<String> =
    LazyLock::new(|| operation_examples_after_help(htmlcut_core::OperationId::SliceExtract));
static INSPECT_SOURCE_AFTER_HELP: LazyLock<String> =
    LazyLock::new(|| operation_examples_after_help(htmlcut_core::OperationId::SourceInspect));
static INSPECT_SELECT_AFTER_HELP: LazyLock<String> =
    LazyLock::new(|| operation_examples_after_help(htmlcut_core::OperationId::SelectPreview));
static INSPECT_SLICE_AFTER_HELP: LazyLock<String> =
    LazyLock::new(|| operation_examples_after_help(htmlcut_core::OperationId::SlicePreview));

pub(crate) fn select_after_help() -> &'static str {
    SELECT_AFTER_HELP.as_str()
}

pub(crate) fn slice_after_help() -> &'static str {
    SLICE_AFTER_HELP.as_str()
}

pub(crate) fn inspect_source_after_help() -> &'static str {
    INSPECT_SOURCE_AFTER_HELP.as_str()
}

pub(crate) fn inspect_select_after_help() -> &'static str {
    INSPECT_SELECT_AFTER_HELP.as_str()
}

pub(crate) fn inspect_slice_after_help() -> &'static str {
    INSPECT_SLICE_AFTER_HELP.as_str()
}

fn operation_examples_after_help(operation_id: htmlcut_core::OperationId) -> String {
    let contract =
        htmlcut_core::cli_operation_contract(operation_id).expect("CLI-visible operation");
    format!("Examples:\n  {}", contract.examples.join("\n  "))
}
