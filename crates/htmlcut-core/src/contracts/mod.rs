use std::num::NonZeroUsize;

mod constants;
mod request;
mod results;

#[cfg(test)]
pub(crate) use constants::default_preview_chars;
pub use constants::{
    BUILTIN_HTTP_CLIENT_AVAILABLE, CORE_RESULT_SCHEMA_NAME, CORE_RESULT_SCHEMA_VERSION,
    CORE_SOURCE_INSPECTION_SCHEMA_NAME, CORE_SOURCE_INSPECTION_SCHEMA_VERSION, CORE_SPEC_VERSION,
    DEFAULT_FETCH_CONNECT_TIMEOUT_MS, DEFAULT_FETCH_PREFLIGHT_MODE, DEFAULT_FETCH_TIMEOUT_MS,
    DEFAULT_INSPECTION_SAMPLE_LIMIT, DEFAULT_MAX_BYTES, DEFAULT_PREVIEW_CHARS,
};
pub(crate) use constants::{
    default_fetch_connect_timeout_ms, default_fetch_timeout_ms, default_inspection_sample_limit,
    default_max_bytes, default_true,
};
pub use request::{
    AttributeName, BoundaryRetention, ContractValueError, DisplayedHttpUrl, ExtractionDefinition,
    ExtractionRequest, ExtractionSpec, ExtractionStrategy, FetchConnectTimeoutMs,
    FetchPreflightMode, FetchTimeoutMs, HttpUrl, InspectionOptions, MaxBytes, OutputOptions,
    PatternMode, PersistedHttpUrl, RenderingOptions, RuntimeOptions, SelectionSpec, SelectorQuery,
    SliceBoundary, SlicePatternSpec, SliceSpec, SourceInput, SourceKind, SourceRequest,
    TlsTrustPolicy, ValueSpec, ValueType, WhitespaceMode,
};
pub use results::{
    ContentCandidateInspection, DelimiterPairMatchMetadata, Diagnostic, DiagnosticLevel,
    DocumentInspection, ExtractionMatch, ExtractionMatchMetadata, ExtractionResult,
    ExtractionStats, HeadingInspection, InspectionCount, LinkInspection, ParseDocumentResult,
    ParsedDocument, Range, SelectorMatchMetadata, SourceInspectionResult, SourceLoadAction,
    SourceLoadOutcome, SourceLoadStep, SourceMetadata,
};

pub(crate) fn default_preview_chars_non_zero() -> NonZeroUsize {
    NonZeroUsize::new(DEFAULT_PREVIEW_CHARS).unwrap_or(NonZeroUsize::MIN)
}
