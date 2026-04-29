//! HTMLCut's embeddable extraction and inspection engine.
#![deny(missing_docs)]

mod catalog;
mod cli_choice;
/// Canonical CLI command contracts, help documents, and discovery helpers owned by `htmlcut-core`.
pub mod cli_contract;
mod contracts;
mod diagnostics;
#[cfg(any(test, doctest))]
mod doctests;
mod document;
mod extract;
mod inspect;
pub mod interop;
mod schema;
mod source;
#[cfg(test)]
mod tests;

/// Typed request-side contracts for embeddable HTMLCut callers.
pub mod request {
    pub use crate::contracts::{
        AttributeName, ContractValueError, ExtractionDefinition, ExtractionRequest, ExtractionSpec,
        ExtractionStrategy, FetchPreflightMode, InspectionOptions, NormalizationOptions,
        OutputOptions, PatternMode, RuntimeOptions, SelectionSpec, SelectorQuery, SliceBoundary,
        SlicePatternSpec, SliceSpec, SourceInput, SourceKind, SourceRequest, ValueSpec, ValueType,
        WhitespaceMode,
    };
}

/// Typed result-side contracts for embeddable HTMLCut callers.
pub mod result {
    pub use crate::contracts::{
        DelimiterPairMatchMetadata, Diagnostic, DiagnosticLevel, DocumentInspection,
        ExtractionMatch, ExtractionMatchMetadata, ExtractionResult, ExtractionStats,
        HeadingInspection, InspectionCount, LinkInspection, ParseDocumentResult, ParsedDocument,
        Range, SelectorMatchMetadata, SourceInspectionResult, SourceLoadAction, SourceLoadOutcome,
        SourceLoadStep, SourceMetadata,
    };
}

pub use catalog::{
    OPERATION_CATALOG, OperationContract, OperationDescriptor, OperationId, OperationIdParseError,
    operation_catalog, operation_descriptor,
};
pub use cli_choice::CliChoice;
pub use contracts::{
    AttributeName, CORE_RESULT_SCHEMA_NAME, CORE_RESULT_SCHEMA_VERSION,
    CORE_SOURCE_INSPECTION_SCHEMA_NAME, CORE_SOURCE_INSPECTION_SCHEMA_VERSION, CORE_SPEC_VERSION,
    ContractValueError, DEFAULT_FETCH_CONNECT_TIMEOUT_MS, DEFAULT_FETCH_PREFLIGHT_MODE,
    DEFAULT_FETCH_TIMEOUT_MS, DEFAULT_INSPECTION_SAMPLE_LIMIT, DEFAULT_MAX_BYTES,
    DEFAULT_PREVIEW_CHARS, Diagnostic, DiagnosticLevel, ExtractionDefinition, ExtractionRequest,
    ExtractionResult, ExtractionSpec, ExtractionStrategy, FetchPreflightMode, InspectionOptions,
    NormalizationOptions, OutputOptions, ParseDocumentResult, PatternMode, RuntimeOptions,
    SelectionSpec, SelectorQuery, SliceBoundary, SlicePatternSpec, SliceSpec, SourceInput,
    SourceInspectionResult, SourceKind, SourceLoadAction, SourceLoadOutcome, SourceLoadStep,
    SourceMetadata, SourceRequest, ValueSpec, ValueType, WhitespaceMode, format_byte_size,
};
pub use diagnostics::{DiagnosticCode, DiagnosticCodeParseError};
#[cfg(test)]
pub(crate) use document::{
    render_document_body_as_text, rewrite_urls_in_document_with_node_ids_for_tests,
};
pub use extract::{extract, inspect_source, parse_document, preview_extraction};
pub use schema::{
    CORE_REQUEST_SCHEMA_VERSION, EXTRACTION_DEFINITION_SCHEMA_NAME,
    EXTRACTION_DEFINITION_SCHEMA_VERSION, EXTRACTION_REQUEST_SCHEMA_NAME,
    HTMLCUT_JSON_SCHEMA_PROFILE, INSPECTION_OPTIONS_SCHEMA_NAME, RUNTIME_OPTIONS_SCHEMA_NAME,
    SOURCE_REQUEST_SCHEMA_NAME, SchemaDescriptor, SchemaExportError, SchemaRef, SchemaStability,
    schema_catalog, schema_descriptor,
};
