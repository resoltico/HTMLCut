use std::num::NonZeroUsize;

mod constants;
mod request;
mod results;

#[cfg(test)]
pub(crate) use constants::default_preview_chars;
pub use constants::{
    CORE_RESULT_SCHEMA_NAME, CORE_RESULT_SCHEMA_VERSION, CORE_SOURCE_INSPECTION_SCHEMA_NAME,
    CORE_SOURCE_INSPECTION_SCHEMA_VERSION, CORE_SPEC_VERSION, DEFAULT_FETCH_CONNECT_TIMEOUT_MS,
    DEFAULT_FETCH_PREFLIGHT_MODE, DEFAULT_FETCH_TIMEOUT_MS, DEFAULT_INSPECTION_SAMPLE_LIMIT,
    DEFAULT_MAX_BYTES, DEFAULT_PREVIEW_CHARS,
};
pub(crate) use constants::{
    default_fetch_connect_timeout_ms, default_fetch_timeout_ms, default_inspection_sample_limit,
    default_max_bytes, default_spec_version, default_true,
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

/// Formats a byte size using friendly IEC binary units.
pub fn format_byte_size(bytes: usize) -> String {
    const KIBIBYTE: u128 = 1024;
    const MEBIBYTE: u128 = KIBIBYTE * KIBIBYTE;
    const GIBIBYTE: u128 = MEBIBYTE * KIBIBYTE;
    const UNITS: [(&str, u128); 3] = [("GiB", GIBIBYTE), ("MiB", MEBIBYTE), ("KiB", KIBIBYTE)];

    if bytes == 1 {
        return "1 byte".to_owned();
    }

    let bytes = bytes as u128;
    for (label, unit_size) in UNITS {
        if bytes < unit_size {
            continue;
        }

        let tenths = ((bytes * 10) + (unit_size / 2)) / unit_size;
        let whole = tenths / 10;
        let fractional = tenths % 10;
        return if fractional == 0 {
            format!("{whole} {label}")
        } else {
            format!("{whole}.{fractional} {label}")
        };
    }

    format!("{bytes} bytes")
}

pub(crate) fn default_preview_chars_non_zero() -> NonZeroUsize {
    NonZeroUsize::new(DEFAULT_PREVIEW_CHARS).unwrap_or(NonZeroUsize::MIN)
}
