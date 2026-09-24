use super::*;

pub(super) fn is_false(value: &bool) -> bool {
    !value
}

pub(super) fn is_true(value: &bool) -> bool {
    *value
}

pub(super) fn is_default_fetch_preflight(value: &FetchPreflightMode) -> bool {
    *value == FetchPreflightMode::default()
}

pub(super) fn is_default_whitespace_mode(value: &WhitespaceMode) -> bool {
    *value == WhitespaceMode::Rendered
}

pub(super) fn is_default_runtime_max_bytes(value: &u64) -> bool {
    *value == default_max_bytes_limit()
}

pub(super) fn is_default_runtime_fetch_timeout(value: &FetchTimeoutMs) -> bool {
    *value == default_fetch_timeout_limit()
}

pub(super) fn is_default_runtime_fetch_connect_timeout(value: &FetchConnectTimeoutMs) -> bool {
    *value == default_fetch_connect_timeout_limit()
}

pub(super) fn is_default_tls_trust_policy(value: &TlsTrustPolicyDocument) -> bool {
    *value == TlsTrustPolicyDocument::default()
}

pub(super) fn is_default_runtime_options_document(value: &RuntimeOptionsDocument) -> bool {
    *value == RuntimeOptionsDocument::default()
}

pub(super) fn is_default_inspection_sample_limit_document(value: &u32) -> bool {
    *value == default_inspection_sample_limit_document()
}

pub(super) fn is_default_selection_spec_document(value: &SelectionSpecDocument) -> bool {
    *value == SelectionSpecDocument::default()
}

pub(super) fn is_default_value_spec_document(value: &ValueSpecDocument) -> bool {
    *value == ValueSpecDocument::default()
}

pub(super) fn is_default_boundary_retention_document(value: &BoundaryRetentionDocument) -> bool {
    *value == BoundaryRetentionDocument::default()
}

pub(super) fn is_default_rendering_options_document(value: &RenderingOptionsDocument) -> bool {
    *value == RenderingOptionsDocument::default()
}

pub(super) fn is_default_output_options_document(value: &OutputOptionsDocument) -> bool {
    *value == OutputOptionsDocument::default()
}

pub(super) fn default_preview_chars_non_zero_document() -> NonZeroU32 {
    NonZeroU32::new(
        wire_u32(
            OutputOptions::default().preview_chars.get(),
            "preview_chars",
        )
        .expect("default preview length must fit v2 wire width"),
    )
    .expect("default preview length must be non-zero")
}

pub(super) fn is_default_preview_chars_non_zero_document(value: &NonZeroU32) -> bool {
    *value == default_preview_chars_non_zero_document()
}

pub(super) fn default_true_document() -> bool {
    true
}

pub(super) fn default_inspection_sample_limit_document() -> u32 {
    wire_u32(InspectionOptions::default().sample_limit, "sample_limit")
        .expect("default sample limit must fit v2 wire width")
}

pub(super) fn default_max_bytes_limit() -> u64 {
    wire_u64(RuntimeOptions::default().max_bytes.get(), "max_bytes")
        .expect("default maximum source size must fit v2 wire width")
}

pub(super) fn default_fetch_timeout_limit() -> FetchTimeoutMs {
    RuntimeOptions::default().fetch_timeout_ms
}

pub(super) fn default_fetch_connect_timeout_limit() -> FetchConnectTimeoutMs {
    RuntimeOptions::default().fetch_connect_timeout_ms
}
