//! HTMLCut's embeddable extraction and inspection engine.
#![deny(missing_docs)]

mod catalog;
mod contracts;
mod diagnostics;
mod document;
mod extract;
mod inspect;
pub mod interop;
mod schema;
mod source;
#[cfg(test)]
mod tests;

pub use catalog::{
    OPERATION_CATALOG, OperationContract, OperationDescriptor, OperationId, OperationIdParseError,
    operation_catalog, operation_descriptor,
};
pub use contracts::{
    AttributeName, CORE_RESULT_SCHEMA_NAME, CORE_RESULT_SCHEMA_VERSION,
    CORE_SOURCE_INSPECTION_SCHEMA_NAME, CORE_SOURCE_INSPECTION_SCHEMA_VERSION, CORE_SPEC_VERSION,
    ContractValueError, DEFAULT_FETCH_TIMEOUT_MS, DEFAULT_INSPECTION_SAMPLE_LIMIT,
    DEFAULT_MAX_BYTES, DEFAULT_PREVIEW_CHARS, DEFAULT_REGEX_FLAGS, DelimiterPairMatchMetadata,
    Diagnostic, DiagnosticLevel, DocumentInspection, ExtractionMatch, ExtractionMatchMetadata,
    ExtractionRequest, ExtractionResult, ExtractionSpec, ExtractionStats, ExtractionStrategy,
    HeadingInspection, InspectionCount, InspectionOptions, LinkInspection, NormalizationOptions,
    OutputOptions, ParseDocumentResult, ParsedDocument, PatternMode, Range, RuntimeOptions,
    SelectionSpec, SelectorMatchMetadata, SelectorQuery, SliceBoundary, SlicePatternSpec,
    SliceSpec, SourceInput, SourceInspectionResult, SourceKind, SourceMetadata, SourceRequest,
    ValueSpec, ValueType, WhitespaceMode, format_byte_size,
};
pub use extract::{extract, inspect_source, parse_document, preview_extraction};
pub use schema::{
    CORE_REQUEST_SCHEMA_VERSION, EXTRACTION_REQUEST_SCHEMA_NAME, HTMLCUT_JSON_SCHEMA_PROFILE,
    INSPECTION_OPTIONS_SCHEMA_NAME, RUNTIME_OPTIONS_SCHEMA_NAME, SOURCE_REQUEST_SCHEMA_NAME,
    SchemaDescriptor, SchemaRef, SchemaStability, schema_catalog, schema_descriptor,
};

#[cfg(test)]
pub(crate) use diagnostics::{error_diagnostic, has_errors, warning_diagnostic};
#[cfg(test)]
pub(crate) use document::{
    apply_whitespace_mode, build_node_path, build_preview, collapse_inline_whitespace,
    element_attributes, first_body, first_body_child_element, first_fragment_attributes,
    looks_like_full_document, needs_space, parse_document_node, parse_wrapped_fragment,
    push_newline, render_html_as_text, render_node, resolve_document_base_url, resolve_url,
    rewrite_html_urls, rewrite_urls_in_document, select_first, serialize_children,
    serialize_document,
};
#[cfg(test)]
pub(crate) use extract::{
    build_finder, build_regex, build_selector_match, build_slice_match, extract_slice_candidates,
    run_selector_extraction, run_slice_extraction, select_candidates, validate_request,
};
#[cfg(test)]
pub(crate) use source::{
    LoadedSource, build_http_agent, load_source, read_file_source, read_limited_to_string,
    read_url_source, source_metadata,
};
