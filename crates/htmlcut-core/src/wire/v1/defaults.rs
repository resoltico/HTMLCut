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

pub(super) fn is_default_runtime_max_bytes(value: &MaxBytes) -> bool {
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

pub(super) fn is_default_inspection_sample_limit_document(value: &usize) -> bool {
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

pub(super) fn default_preview_chars_non_zero_document() -> NonZeroUsize {
    OutputOptions::default().preview_chars
}

pub(super) fn is_default_preview_chars_non_zero_document(value: &NonZeroUsize) -> bool {
    *value == default_preview_chars_non_zero_document()
}

pub(super) fn default_true_document() -> bool {
    true
}

pub(super) fn default_inspection_sample_limit_document() -> usize {
    InspectionOptions::default().sample_limit
}

pub(super) fn default_max_bytes_limit() -> MaxBytes {
    RuntimeOptions::default().max_bytes
}

pub(super) fn default_fetch_timeout_limit() -> FetchTimeoutMs {
    RuntimeOptions::default().fetch_timeout_ms
}

pub(super) fn default_fetch_connect_timeout_limit() -> FetchConnectTimeoutMs {
    RuntimeOptions::default().fetch_connect_timeout_ms
}
