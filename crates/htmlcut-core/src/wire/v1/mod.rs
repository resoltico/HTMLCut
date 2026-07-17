use std::collections::BTreeMap;
use std::num::NonZeroUsize;
use std::path::PathBuf;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::result::{
    ContentCandidateInspection, DelimiterPairMatchMetadata, DocumentInspection, ExtractionMatch,
    ExtractionMatchMetadata, ExtractionStats, HeadingInspection, InspectionCount, LinkInspection,
    Range, SelectorMatchMetadata,
};
use crate::{
    AttributeName, BoundaryRetention, ContractValueError, Diagnostic, DiagnosticCode,
    DiagnosticLevel, ExtractionDefinition, ExtractionRequest, ExtractionResult, ExtractionSpec,
    FetchConnectTimeoutMs, FetchPreflightMode, FetchTimeoutMs, InspectionOptions, MaxBytes,
    OperationId, OutputOptions, PersistedHttpUrl, RenderingOptions, RuntimeOptions, SelectionSpec,
    SelectorQuery, SliceBoundary, SlicePatternSpec, SliceSpec, SourceInput, SourceInspectionResult,
    SourceKind, SourceLoadAction, SourceLoadOutcome, SourceLoadStep, SourceMetadata, SourceRequest,
    TlsTrustPolicy, ValueSpec, ValueType, WhitespaceMode,
};

/// Versioned JSON document for the `htmlcut.source_request` schema.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SourceRequestDocument {
    input: SourceInputDocument,
    #[serde(skip_serializing_if = "Option::is_none")]
    base_url: Option<PersistedHttpUrl>,
}

/// Versioned JSON document for the `htmlcut.runtime_options` schema.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RuntimeOptionsDocument {
    #[serde(default = "default_max_bytes_limit")]
    #[serde(skip_serializing_if = "is_default_runtime_max_bytes")]
    max_bytes: MaxBytes,
    #[serde(default = "default_fetch_timeout_limit")]
    #[serde(skip_serializing_if = "is_default_runtime_fetch_timeout")]
    fetch_timeout_ms: FetchTimeoutMs,
    #[serde(default = "default_fetch_connect_timeout_limit")]
    #[serde(skip_serializing_if = "is_default_runtime_fetch_connect_timeout")]
    fetch_connect_timeout_ms: FetchConnectTimeoutMs,
    #[serde(default)]
    #[serde(skip_serializing_if = "is_default_fetch_preflight")]
    fetch_preflight: FetchPreflightMode,
    #[serde(default)]
    #[serde(skip_serializing_if = "is_default_tls_trust_policy")]
    tls_trust: TlsTrustPolicyDocument,
}

/// Versioned JSON document for the `htmlcut.inspection_options` schema.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct InspectionOptionsDocument {
    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    include_source_text: bool,
    #[serde(default = "default_inspection_sample_limit_document")]
    #[serde(skip_serializing_if = "is_default_inspection_sample_limit_document")]
    sample_limit: usize,
}

/// Versioned JSON document for the `htmlcut.extraction_request` schema.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExtractionRequestDocument {
    spec_version: u32,
    source: SourceRequestDocument,
    extraction: ExtractionSpecDocument,
    #[serde(default)]
    #[serde(skip_serializing_if = "is_default_output_options_document")]
    output: OutputOptionsDocument,
}

/// Versioned JSON document for the `htmlcut.extraction_definition` schema.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExtractionDefinitionDocument {
    schema_name: String,
    schema_version: u32,
    request: ExtractionRequestDocument,
    #[serde(default)]
    #[serde(skip_serializing_if = "is_default_runtime_options_document")]
    runtime: RuntimeOptionsDocument,
}

/// Versioned JSON document for the `htmlcut.extraction_result` schema.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ExtractionResultDocument {
    operation_id: OperationId,
    schema_name: String,
    schema_version: u32,
    ok: bool,
    source: SourceMetadataDocument,
    #[serde(skip_serializing_if = "Option::is_none")]
    document_title: Option<String>,
    extraction: ExtractionSpecDocument,
    stats: ExtractionStatsDocument,
    matches: Vec<ExtractionMatchDocument>,
    diagnostics: Vec<DiagnosticDocument>,
}

/// Versioned JSON document for the `htmlcut.source_inspection_result` schema.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct SourceInspectionResultDocument {
    operation_id: OperationId,
    schema_name: String,
    schema_version: u32,
    ok: bool,
    source: SourceMetadataDocument,
    #[serde(skip_serializing_if = "Option::is_none")]
    document: Option<DocumentInspectionDocument>,
    diagnostics: Vec<DiagnosticDocument>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
#[serde(tag = "type", rename_all = "lowercase")]
enum SourceInputDocument {
    Url { href: PersistedHttpUrl },
    File { path: PathBuf },
    Stdin,
    Memory { label: String, text: String },
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
#[serde(tag = "mode", rename_all = "lowercase")]
enum SlicePatternSpecDocument {
    Literal {
        from: SliceBoundary,
        to: SliceBoundary,
    },
    Regex {
        from: SliceBoundary,
        to: SliceBoundary,
        #[serde(default)]
        flags: String,
    },
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
#[serde(tag = "type", rename_all = "lowercase")]
enum SelectionSpecDocument {
    Single,
    #[default]
    First,
    Nth {
        index: NonZeroUsize,
    },
    All,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
#[serde(tag = "type", rename_all = "kebab-case")]
enum ValueSpecDocument {
    #[default]
    Text,
    SelectedHtml,
    InnerHtml,
    OuterHtml,
    Attribute {
        name: AttributeName,
    },
    Structured,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum BoundaryRetentionDocument {
    #[default]
    ExcludeBoth,
    IncludeStart,
    IncludeEnd,
    IncludeBoth,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
#[serde(tag = "kind", rename_all = "kebab-case")]
enum ExtractionSpecDocument {
    Selector {
        selector: SelectorQuery,
        #[serde(default)]
        #[serde(skip_serializing_if = "is_default_selection_spec_document")]
        selection: SelectionSpecDocument,
        #[serde(default)]
        #[serde(skip_serializing_if = "is_default_value_spec_document")]
        value: ValueSpecDocument,
    },
    Slice {
        pattern: SlicePatternSpecDocument,
        #[serde(default)]
        #[serde(skip_serializing_if = "is_default_boundary_retention_document")]
        boundary_retention: BoundaryRetentionDocument,
        #[serde(default)]
        #[serde(skip_serializing_if = "is_default_selection_spec_document")]
        selection: SelectionSpecDocument,
        #[serde(default)]
        #[serde(skip_serializing_if = "is_default_value_spec_document")]
        value: ValueSpecDocument,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct RenderingOptionsDocument {
    #[serde(default)]
    #[serde(skip_serializing_if = "is_default_whitespace_mode")]
    whitespace: WhitespaceMode,
    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    rewrite_urls: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct OutputOptionsDocument {
    #[serde(default)]
    #[serde(skip_serializing_if = "is_default_rendering_options_document")]
    rendering: RenderingOptionsDocument,
    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    include_source_text: bool,
    #[serde(default = "default_true_document")]
    #[serde(skip_serializing_if = "is_true")]
    include_html: bool,
    #[serde(default = "default_true_document")]
    #[serde(skip_serializing_if = "is_true")]
    include_text: bool,
    #[serde(default = "default_preview_chars_non_zero_document")]
    #[serde(skip_serializing_if = "is_default_preview_chars_non_zero_document")]
    preview_chars: NonZeroUsize,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
#[serde(tag = "kind", rename_all = "kebab-case")]
enum TlsTrustPolicyDocument {
    #[default]
    WebPki,
    Platform,
    CustomCaBundle {
        path: PathBuf,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
struct DiagnosticDocument {
    level: DiagnosticLevel,
    code: DiagnosticCode,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
struct SourceMetadataDocument {
    kind: SourceKind,
    value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    input_base_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    effective_base_url: Option<String>,
    bytes_read: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    load_steps: Vec<SourceLoadStepDocument>,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
struct SourceLoadStepDocument {
    action: SourceLoadAction,
    outcome: SourceLoadOutcome,
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<u16>,
    message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
struct ExtractionStatsDocument {
    duration_ms: u128,
    candidate_count: usize,
    match_count: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
struct RangeDocument {
    start: usize,
    end: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
struct SelectorMatchMetadataDocument {
    candidate_count: usize,
    candidate_index: usize,
    path: String,
    tag_name: String,
    attributes: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
struct DelimiterPairMatchMetadataDocument {
    candidate_count: usize,
    candidate_index: usize,
    selected_range: RangeDocument,
    inner_range: RangeDocument,
    outer_range: RangeDocument,
    include_start: bool,
    include_end: bool,
    matched_start: String,
    matched_end: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "kebab-case")]
enum ExtractionMatchMetadataDocument {
    Selector(SelectorMatchMetadataDocument),
    DelimiterPair(DelimiterPairMatchMetadataDocument),
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
struct ExtractionMatchDocument {
    index: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    value_type: ValueType,
    value: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    html: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    preview: String,
    metadata: ExtractionMatchMetadataDocument,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
struct InspectionCountDocument {
    name: String,
    count: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
struct HeadingInspectionDocument {
    level: u8,
    text: String,
    path: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
struct LinkInspectionDocument {
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    href: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    resolved_href: Option<String>,
    path: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
struct ContentCandidateInspectionDocument {
    selector: String,
    path: String,
    tag_name: String,
    text_char_count: usize,
    heading_count: usize,
    link_count: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
struct DocumentInspectionDocument {
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    root_tag: String,
    element_count: usize,
    text_char_count: usize,
    link_count: usize,
    image_count: usize,
    form_count: usize,
    table_count: usize,
    script_count: usize,
    style_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    document_base_href: Option<String>,
    top_tags: Vec<InspectionCountDocument>,
    top_classes: Vec<InspectionCountDocument>,
    extraction_candidates: Vec<ContentCandidateInspectionDocument>,
    reading_candidates: Vec<ContentCandidateInspectionDocument>,
    headings: Vec<HeadingInspectionDocument>,
    links: Vec<LinkInspectionDocument>,
}

impl Default for RuntimeOptionsDocument {
    fn default() -> Self {
        Self {
            max_bytes: default_max_bytes_limit(),
            fetch_timeout_ms: default_fetch_timeout_limit(),
            fetch_connect_timeout_ms: default_fetch_connect_timeout_limit(),
            fetch_preflight: FetchPreflightMode::default(),
            tls_trust: TlsTrustPolicyDocument::default(),
        }
    }
}

impl Default for InspectionOptionsDocument {
    fn default() -> Self {
        Self {
            include_source_text: false,
            sample_limit: default_inspection_sample_limit_document(),
        }
    }
}

impl Default for RenderingOptionsDocument {
    fn default() -> Self {
        Self {
            whitespace: WhitespaceMode::Rendered,
            rewrite_urls: false,
        }
    }
}

impl Default for OutputOptionsDocument {
    fn default() -> Self {
        Self {
            rendering: RenderingOptionsDocument::default(),
            include_source_text: false,
            include_html: true,
            include_text: true,
            preview_chars: default_preview_chars_non_zero_document(),
        }
    }
}

mod defaults;
mod requests;
mod results;

use defaults::*;
