/// Current version of the embeddable extraction request contract.
pub const CORE_SPEC_VERSION: u32 = 4;
/// Frozen schema name for [`crate::ExtractionResult`].
pub const CORE_RESULT_SCHEMA_NAME: &str = "htmlcut.extraction_result";
/// Current schema version for [`crate::ExtractionResult`].
pub const CORE_RESULT_SCHEMA_VERSION: u32 = 5;
/// Frozen schema name for [`crate::SourceInspectionResult`].
pub const CORE_SOURCE_INSPECTION_SCHEMA_NAME: &str = "htmlcut.source_inspection_result";
/// Current schema version for [`crate::SourceInspectionResult`].
pub const CORE_SOURCE_INSPECTION_SCHEMA_VERSION: u32 = 3;
/// Default preview length captured in structured reports.
pub const DEFAULT_PREVIEW_CHARS: usize = 160;
/// Default maximum source size accepted by loaders.
pub const DEFAULT_MAX_BYTES: usize = 50 * 1024 * 1024;
/// Default timeout for HTTP source loading.
pub const DEFAULT_FETCH_TIMEOUT_MS: u64 = 15_000;
/// Default timeout for establishing one HTTP connection.
pub const DEFAULT_FETCH_CONNECT_TIMEOUT_MS: u64 = 5_000;
/// Default fetch preflight policy for URL source loading.
pub const DEFAULT_FETCH_PREFLIGHT_MODE: &str = "head-first";
/// Default limit for sampled inspection headings, links, tags, and classes.
pub const DEFAULT_INSPECTION_SAMPLE_LIMIT: usize = 8;

pub(crate) const fn default_spec_version() -> u32 {
    CORE_SPEC_VERSION
}

#[cfg(test)]
pub(crate) const fn default_preview_chars() -> usize {
    DEFAULT_PREVIEW_CHARS
}

pub(crate) const fn default_inspection_sample_limit() -> usize {
    DEFAULT_INSPECTION_SAMPLE_LIMIT
}

pub(crate) const fn default_true() -> bool {
    true
}

pub(crate) const fn default_max_bytes() -> usize {
    DEFAULT_MAX_BYTES
}

pub(crate) const fn default_fetch_timeout_ms() -> u64 {
    DEFAULT_FETCH_TIMEOUT_MS
}

pub(crate) const fn default_fetch_connect_timeout_ms() -> u64 {
    DEFAULT_FETCH_CONNECT_TIMEOUT_MS
}
