use std::num::NonZeroUsize;

mod constants;
mod request;
mod results;

pub use constants::{
    CORE_RESULT_SCHEMA_NAME, CORE_RESULT_SCHEMA_VERSION, CORE_SOURCE_INSPECTION_SCHEMA_NAME,
    CORE_SOURCE_INSPECTION_SCHEMA_VERSION, CORE_SPEC_VERSION, DEFAULT_FETCH_PREFLIGHT_MODE,
    DEFAULT_FETCH_TIMEOUT_MS, DEFAULT_INSPECTION_SAMPLE_LIMIT, DEFAULT_MAX_BYTES,
    DEFAULT_PREVIEW_CHARS, DEFAULT_REGEX_FLAGS,
};
pub(crate) use constants::{
    default_fetch_timeout_ms, default_inspection_sample_limit, default_max_bytes,
    default_preview_chars, default_regex_flags, default_spec_version, default_true,
};
pub use request::{
    AttributeName, ContractValueError, ExtractionDefinition, ExtractionRequest, ExtractionSpec,
    ExtractionStrategy, FetchPreflightMode, InspectionOptions, NormalizationOptions, OutputOptions,
    PatternMode, RuntimeOptions, SelectionSpec, SelectorQuery, SliceBoundary, SlicePatternSpec,
    SliceSpec, SourceInput, SourceKind, SourceRequest, ValueSpec, ValueType, WhitespaceMode,
};
pub use results::{
    DelimiterPairMatchMetadata, Diagnostic, DiagnosticLevel, DocumentInspection, ExtractionMatch,
    ExtractionMatchMetadata, ExtractionResult, ExtractionStats, HeadingInspection, InspectionCount,
    LinkInspection, ParseDocumentResult, ParsedDocument, Range, SelectorMatchMetadata,
    SourceInspectionResult, SourceLoadAction, SourceLoadOutcome, SourceLoadStep, SourceMetadata,
};

/// Formats a byte size using friendly binary units when possible.
pub fn format_byte_size(bytes: usize) -> String {
    let kibibyte = 1024usize;
    let mebibyte = kibibyte * kibibyte;
    let gibibyte = mebibyte * kibibyte;

    if bytes >= gibibyte && bytes.is_multiple_of(gibibyte) {
        return format!("{} GB", bytes / gibibyte);
    }

    if bytes >= mebibyte && bytes.is_multiple_of(mebibyte) {
        return format!("{} MB", bytes / mebibyte);
    }

    if bytes >= kibibyte && bytes.is_multiple_of(kibibyte) {
        return format!("{} KB", bytes / kibibyte);
    }

    format!("{bytes} bytes")
}

pub(crate) fn default_preview_chars_non_zero() -> NonZeroUsize {
    NonZeroUsize::new(default_preview_chars()).expect("preview chars constant should be non-zero")
}
