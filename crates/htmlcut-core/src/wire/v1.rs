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
    AttributeName, BoundaryRetention, Diagnostic, DiagnosticCode, DiagnosticLevel,
    ExtractionDefinition, ExtractionRequest, ExtractionResult, ExtractionSpec,
    FetchConnectTimeoutMs, FetchPreflightMode, FetchTimeoutMs, HttpUrl, InspectionOptions,
    MaxBytes, OperationId, OutputOptions, RenderingOptions, RuntimeOptions, SelectionSpec,
    SelectorQuery, SliceBoundary, SlicePatternSpec, SliceSpec, SourceInput, SourceInspectionResult,
    SourceKind, SourceLoadAction, SourceLoadOutcome, SourceLoadStep, SourceMetadata, SourceRequest,
    TlsTrustPolicy, ValueSpec, ValueType, WhitespaceMode,
};

/// Versioned JSON document for the `htmlcut.source_request` schema.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct SourceRequestDocument {
    input: SourceInputDocument,
    #[serde(skip_serializing_if = "Option::is_none")]
    base_url: Option<HttpUrl>,
}

/// Versioned JSON document for the `htmlcut.runtime_options` schema.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct RuntimeOptionsDocument {
    #[serde(default = "default_max_bytes_limit")]
    #[serde(skip_serializing_if = "is_default_runtime_max_bytes")]
    max_bytes: MaxBytes,
    #[serde(default = "default_fetch_timeout_limit")]
    #[serde(skip_serializing_if = "is_default_runtime_fetch_timeout")]
    fetch_timeout: FetchTimeoutMs,
    #[serde(default = "default_fetch_connect_timeout_limit")]
    #[serde(skip_serializing_if = "is_default_runtime_fetch_connect_timeout")]
    fetch_connect_timeout: FetchConnectTimeoutMs,
    #[serde(default)]
    #[serde(skip_serializing_if = "is_default_fetch_preflight")]
    fetch_preflight: FetchPreflightMode,
    #[serde(default)]
    #[serde(skip_serializing_if = "is_default_tls_trust_policy")]
    tls_trust: TlsTrustPolicyDocument,
}

/// Versioned JSON document for the `htmlcut.inspection_options` schema.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
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
pub struct ExtractionRequestDocument {
    #[serde(default = "default_spec_version_document")]
    #[serde(skip_serializing_if = "is_default_spec_version_document")]
    spec_version: u32,
    source: SourceRequestDocument,
    extraction: ExtractionSpecDocument,
    #[serde(default)]
    #[serde(skip_serializing_if = "is_default_output_options_document")]
    output: OutputOptionsDocument,
}

/// Versioned JSON document for the `htmlcut.extraction_definition` schema.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct ExtractionDefinitionDocument {
    #[serde(default = "default_extraction_definition_schema_name_document")]
    #[serde(skip_serializing_if = "is_default_extraction_definition_schema_name_document")]
    schema_name: String,
    #[serde(default = "default_extraction_definition_schema_version_document")]
    #[serde(skip_serializing_if = "is_default_extraction_definition_schema_version_document")]
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
#[serde(tag = "type", rename_all = "lowercase")]
enum SourceInputDocument {
    #[cfg(feature = "http-client")]
    Url {
        href: HttpUrl,
    },
    File {
        path: PathBuf,
    },
    Stdin,
    Memory {
        label: String,
        text: String,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
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
struct SliceSpecDocument {
    #[serde(flatten)]
    pattern: SlicePatternSpecDocument,
    #[serde(default)]
    #[serde(skip_serializing_if = "is_default_boundary_retention_document")]
    boundary_retention: BoundaryRetentionDocument,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
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
        #[serde(flatten)]
        slice: SliceSpecDocument,
        #[serde(default)]
        #[serde(skip_serializing_if = "is_default_selection_spec_document")]
        selection: SelectionSpecDocument,
        #[serde(default)]
        #[serde(skip_serializing_if = "is_default_value_spec_document")]
        value: ValueSpecDocument,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
struct RenderingOptionsDocument {
    #[serde(default)]
    #[serde(skip_serializing_if = "is_default_whitespace_mode")]
    whitespace: WhitespaceMode,
    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    rewrite_urls: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
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
            fetch_timeout: default_fetch_timeout_limit(),
            fetch_connect_timeout: default_fetch_connect_timeout_limit(),
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

impl From<SourceRequest> for SourceRequestDocument {
    fn from(value: SourceRequest) -> Self {
        Self {
            input: value.input.into(),
            base_url: value.base_url,
        }
    }
}

impl From<SourceRequestDocument> for SourceRequest {
    fn from(value: SourceRequestDocument) -> Self {
        Self {
            input: value.input.into(),
            base_url: value.base_url,
        }
    }
}

impl From<RuntimeOptions> for RuntimeOptionsDocument {
    fn from(value: RuntimeOptions) -> Self {
        Self {
            max_bytes: value.max_bytes,
            fetch_timeout: value.fetch_timeout,
            fetch_connect_timeout: value.fetch_connect_timeout,
            fetch_preflight: value.fetch_preflight,
            tls_trust: value.tls_trust.into(),
        }
    }
}

impl From<RuntimeOptionsDocument> for RuntimeOptions {
    fn from(value: RuntimeOptionsDocument) -> Self {
        Self {
            max_bytes: value.max_bytes,
            fetch_timeout: value.fetch_timeout,
            fetch_connect_timeout: value.fetch_connect_timeout,
            fetch_preflight: value.fetch_preflight,
            tls_trust: value.tls_trust.into(),
        }
    }
}

impl From<InspectionOptions> for InspectionOptionsDocument {
    fn from(value: InspectionOptions) -> Self {
        Self {
            include_source_text: value.include_source_text,
            sample_limit: value.sample_limit,
        }
    }
}

impl From<InspectionOptionsDocument> for InspectionOptions {
    fn from(value: InspectionOptionsDocument) -> Self {
        Self {
            include_source_text: value.include_source_text,
            sample_limit: value.sample_limit,
        }
    }
}

impl From<ExtractionRequest> for ExtractionRequestDocument {
    fn from(value: ExtractionRequest) -> Self {
        Self {
            spec_version: value.spec_version,
            source: value.source.into(),
            extraction: value.extraction.into(),
            output: value.output.into(),
        }
    }
}

impl From<ExtractionRequestDocument> for ExtractionRequest {
    fn from(value: ExtractionRequestDocument) -> Self {
        Self {
            spec_version: value.spec_version,
            source: value.source.into(),
            extraction: value.extraction.into(),
            output: value.output.into(),
        }
    }
}

impl From<ExtractionDefinition> for ExtractionDefinitionDocument {
    fn from(value: ExtractionDefinition) -> Self {
        Self {
            schema_name: value.schema_name,
            schema_version: value.schema_version,
            request: value.request.into(),
            runtime: value.runtime.into(),
        }
    }
}

impl From<ExtractionDefinitionDocument> for ExtractionDefinition {
    fn from(value: ExtractionDefinitionDocument) -> Self {
        Self {
            schema_name: value.schema_name,
            schema_version: value.schema_version,
            request: value.request.into(),
            runtime: value.runtime.into(),
        }
    }
}

impl From<ExtractionResult> for ExtractionResultDocument {
    fn from(value: ExtractionResult) -> Self {
        Self {
            operation_id: value.operation_id,
            schema_name: value.schema_name,
            schema_version: value.schema_version,
            ok: value.ok,
            source: value.source.into(),
            document_title: value.document_title,
            extraction: value.extraction.into(),
            stats: value.stats.into(),
            matches: value.matches.into_iter().map(Into::into).collect(),
            diagnostics: value.diagnostics.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<ExtractionResultDocument> for ExtractionResult {
    fn from(value: ExtractionResultDocument) -> Self {
        Self {
            operation_id: value.operation_id,
            schema_name: value.schema_name,
            schema_version: value.schema_version,
            ok: value.ok,
            source: value.source.into(),
            document_title: value.document_title,
            extraction: value.extraction.into(),
            stats: value.stats.into(),
            matches: value.matches.into_iter().map(Into::into).collect(),
            diagnostics: value.diagnostics.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<SourceInspectionResult> for SourceInspectionResultDocument {
    fn from(value: SourceInspectionResult) -> Self {
        Self {
            operation_id: value.operation_id,
            schema_name: value.schema_name,
            schema_version: value.schema_version,
            ok: value.ok,
            source: value.source.into(),
            document: value.document.map(Into::into),
            diagnostics: value.diagnostics.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<SourceInspectionResultDocument> for SourceInspectionResult {
    fn from(value: SourceInspectionResultDocument) -> Self {
        Self {
            operation_id: value.operation_id,
            schema_name: value.schema_name,
            schema_version: value.schema_version,
            ok: value.ok,
            source: value.source.into(),
            document: value.document.map(Into::into),
            diagnostics: value.diagnostics.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<SourceInput> for SourceInputDocument {
    fn from(value: SourceInput) -> Self {
        match value {
            #[cfg(feature = "http-client")]
            SourceInput::Url { href } => Self::Url { href },
            SourceInput::File { path } => Self::File { path },
            SourceInput::Stdin => Self::Stdin,
            SourceInput::Memory { label, text } => Self::Memory { label, text },
        }
    }
}

impl From<SourceInputDocument> for SourceInput {
    fn from(value: SourceInputDocument) -> Self {
        match value {
            #[cfg(feature = "http-client")]
            SourceInputDocument::Url { href } => Self::Url { href },
            SourceInputDocument::File { path } => Self::File { path },
            SourceInputDocument::Stdin => Self::Stdin,
            SourceInputDocument::Memory { label, text } => Self::Memory { label, text },
        }
    }
}

impl From<SlicePatternSpec> for SlicePatternSpecDocument {
    fn from(value: SlicePatternSpec) -> Self {
        match value {
            SlicePatternSpec::Literal { from, to } => Self::Literal { from, to },
            SlicePatternSpec::Regex { from, to, flags } => Self::Regex { from, to, flags },
        }
    }
}

impl From<SlicePatternSpecDocument> for SlicePatternSpec {
    fn from(value: SlicePatternSpecDocument) -> Self {
        match value {
            SlicePatternSpecDocument::Literal { from, to } => Self::Literal { from, to },
            SlicePatternSpecDocument::Regex { from, to, flags } => Self::Regex { from, to, flags },
        }
    }
}

impl From<SelectionSpec> for SelectionSpecDocument {
    fn from(value: SelectionSpec) -> Self {
        match value {
            SelectionSpec::Single => Self::Single,
            SelectionSpec::First => Self::First,
            SelectionSpec::Nth { index } => Self::Nth { index },
            SelectionSpec::All => Self::All,
        }
    }
}

impl From<SelectionSpecDocument> for SelectionSpec {
    fn from(value: SelectionSpecDocument) -> Self {
        match value {
            SelectionSpecDocument::Single => Self::Single,
            SelectionSpecDocument::First => Self::First,
            SelectionSpecDocument::Nth { index } => Self::Nth { index },
            SelectionSpecDocument::All => Self::All,
        }
    }
}

impl From<ValueSpec> for ValueSpecDocument {
    fn from(value: ValueSpec) -> Self {
        match value {
            ValueSpec::Text => Self::Text,
            ValueSpec::SelectedHtml => Self::SelectedHtml,
            ValueSpec::InnerHtml => Self::InnerHtml,
            ValueSpec::OuterHtml => Self::OuterHtml,
            ValueSpec::Attribute { name } => Self::Attribute { name },
            ValueSpec::Structured => Self::Structured,
        }
    }
}

impl From<ValueSpecDocument> for ValueSpec {
    fn from(value: ValueSpecDocument) -> Self {
        match value {
            ValueSpecDocument::Text => Self::Text,
            ValueSpecDocument::SelectedHtml => Self::SelectedHtml,
            ValueSpecDocument::InnerHtml => Self::InnerHtml,
            ValueSpecDocument::OuterHtml => Self::OuterHtml,
            ValueSpecDocument::Attribute { name } => Self::Attribute { name },
            ValueSpecDocument::Structured => Self::Structured,
        }
    }
}

impl From<BoundaryRetention> for BoundaryRetentionDocument {
    fn from(value: BoundaryRetention) -> Self {
        match value {
            BoundaryRetention::ExcludeBoth => Self::ExcludeBoth,
            BoundaryRetention::IncludeStart => Self::IncludeStart,
            BoundaryRetention::IncludeEnd => Self::IncludeEnd,
            BoundaryRetention::IncludeBoth => Self::IncludeBoth,
        }
    }
}

impl From<BoundaryRetentionDocument> for BoundaryRetention {
    fn from(value: BoundaryRetentionDocument) -> Self {
        match value {
            BoundaryRetentionDocument::ExcludeBoth => Self::ExcludeBoth,
            BoundaryRetentionDocument::IncludeStart => Self::IncludeStart,
            BoundaryRetentionDocument::IncludeEnd => Self::IncludeEnd,
            BoundaryRetentionDocument::IncludeBoth => Self::IncludeBoth,
        }
    }
}

impl From<SliceSpec> for SliceSpecDocument {
    fn from(value: SliceSpec) -> Self {
        Self {
            pattern: value.pattern.into(),
            boundary_retention: value.boundary_retention.into(),
        }
    }
}

impl From<SliceSpecDocument> for SliceSpec {
    fn from(value: SliceSpecDocument) -> Self {
        Self {
            pattern: value.pattern.into(),
            boundary_retention: value.boundary_retention.into(),
        }
    }
}

impl From<ExtractionSpec> for ExtractionSpecDocument {
    fn from(value: ExtractionSpec) -> Self {
        match value {
            ExtractionSpec::Selector {
                selector,
                selection,
                value,
            } => Self::Selector {
                selector,
                selection: selection.into(),
                value: value.into(),
            },
            ExtractionSpec::Slice {
                slice,
                selection,
                value,
            } => Self::Slice {
                slice: slice.into(),
                selection: selection.into(),
                value: value.into(),
            },
        }
    }
}

impl From<ExtractionSpecDocument> for ExtractionSpec {
    fn from(value: ExtractionSpecDocument) -> Self {
        match value {
            ExtractionSpecDocument::Selector {
                selector,
                selection,
                value,
            } => Self::Selector {
                selector,
                selection: selection.into(),
                value: value.into(),
            },
            ExtractionSpecDocument::Slice {
                slice,
                selection,
                value,
            } => Self::Slice {
                slice: slice.into(),
                selection: selection.into(),
                value: value.into(),
            },
        }
    }
}

impl From<RenderingOptions> for RenderingOptionsDocument {
    fn from(value: RenderingOptions) -> Self {
        Self {
            whitespace: value.whitespace,
            rewrite_urls: value.rewrite_urls,
        }
    }
}

impl From<RenderingOptionsDocument> for RenderingOptions {
    fn from(value: RenderingOptionsDocument) -> Self {
        Self {
            whitespace: value.whitespace,
            rewrite_urls: value.rewrite_urls,
        }
    }
}

impl From<OutputOptions> for OutputOptionsDocument {
    fn from(value: OutputOptions) -> Self {
        Self {
            rendering: value.rendering.into(),
            include_source_text: value.include_source_text,
            include_html: value.include_html,
            include_text: value.include_text,
            preview_chars: value.preview_chars,
        }
    }
}

impl From<OutputOptionsDocument> for OutputOptions {
    fn from(value: OutputOptionsDocument) -> Self {
        Self {
            rendering: value.rendering.into(),
            include_source_text: value.include_source_text,
            include_html: value.include_html,
            include_text: value.include_text,
            preview_chars: value.preview_chars,
        }
    }
}

impl From<TlsTrustPolicy> for TlsTrustPolicyDocument {
    fn from(value: TlsTrustPolicy) -> Self {
        match value {
            TlsTrustPolicy::WebPki => Self::WebPki,
            TlsTrustPolicy::Platform => Self::Platform,
            TlsTrustPolicy::CustomCaBundle { path } => Self::CustomCaBundle { path },
        }
    }
}

impl From<TlsTrustPolicyDocument> for TlsTrustPolicy {
    fn from(value: TlsTrustPolicyDocument) -> Self {
        match value {
            TlsTrustPolicyDocument::WebPki => Self::WebPki,
            TlsTrustPolicyDocument::Platform => Self::Platform,
            TlsTrustPolicyDocument::CustomCaBundle { path } => Self::CustomCaBundle { path },
        }
    }
}

impl From<Diagnostic> for DiagnosticDocument {
    fn from(value: Diagnostic) -> Self {
        Self {
            level: value.level,
            code: value.code,
            message: value.message,
            details: value.details,
        }
    }
}

impl From<DiagnosticDocument> for Diagnostic {
    fn from(value: DiagnosticDocument) -> Self {
        Self {
            level: value.level,
            code: value.code,
            message: value.message,
            details: value.details,
        }
    }
}

impl From<SourceMetadata> for SourceMetadataDocument {
    fn from(value: SourceMetadata) -> Self {
        Self {
            kind: value.kind,
            value: value.value,
            input_base_url: value.input_base_url,
            effective_base_url: value.effective_base_url,
            bytes_read: value.bytes_read,
            load_steps: value.load_steps.into_iter().map(Into::into).collect(),
            text: value.text,
        }
    }
}

impl From<SourceMetadataDocument> for SourceMetadata {
    fn from(value: SourceMetadataDocument) -> Self {
        Self {
            kind: value.kind,
            value: value.value,
            input_base_url: value.input_base_url,
            effective_base_url: value.effective_base_url,
            bytes_read: value.bytes_read,
            load_steps: value.load_steps.into_iter().map(Into::into).collect(),
            text: value.text,
        }
    }
}

impl From<SourceLoadStep> for SourceLoadStepDocument {
    fn from(value: SourceLoadStep) -> Self {
        Self {
            action: value.action,
            outcome: value.outcome,
            status: value.status,
            message: value.message,
        }
    }
}

impl From<SourceLoadStepDocument> for SourceLoadStep {
    fn from(value: SourceLoadStepDocument) -> Self {
        Self {
            action: value.action,
            outcome: value.outcome,
            status: value.status,
            message: value.message,
        }
    }
}

impl From<ExtractionStats> for ExtractionStatsDocument {
    fn from(value: ExtractionStats) -> Self {
        Self {
            duration_ms: value.duration_ms,
            candidate_count: value.candidate_count,
            match_count: value.match_count,
        }
    }
}

impl From<ExtractionStatsDocument> for ExtractionStats {
    fn from(value: ExtractionStatsDocument) -> Self {
        Self {
            duration_ms: value.duration_ms,
            candidate_count: value.candidate_count,
            match_count: value.match_count,
        }
    }
}

impl From<Range> for RangeDocument {
    fn from(value: Range) -> Self {
        Self {
            start: value.start,
            end: value.end,
        }
    }
}

impl From<RangeDocument> for Range {
    fn from(value: RangeDocument) -> Self {
        Self {
            start: value.start,
            end: value.end,
        }
    }
}

impl From<SelectorMatchMetadata> for SelectorMatchMetadataDocument {
    fn from(value: SelectorMatchMetadata) -> Self {
        Self {
            candidate_count: value.candidate_count,
            candidate_index: value.candidate_index,
            path: value.path,
            tag_name: value.tag_name,
            attributes: value.attributes,
        }
    }
}

impl From<SelectorMatchMetadataDocument> for SelectorMatchMetadata {
    fn from(value: SelectorMatchMetadataDocument) -> Self {
        Self {
            candidate_count: value.candidate_count,
            candidate_index: value.candidate_index,
            path: value.path,
            tag_name: value.tag_name,
            attributes: value.attributes,
        }
    }
}

impl From<DelimiterPairMatchMetadata> for DelimiterPairMatchMetadataDocument {
    fn from(value: DelimiterPairMatchMetadata) -> Self {
        Self {
            candidate_count: value.candidate_count,
            candidate_index: value.candidate_index,
            selected_range: value.selected_range.into(),
            inner_range: value.inner_range.into(),
            outer_range: value.outer_range.into(),
            include_start: value.include_start,
            include_end: value.include_end,
            matched_start: value.matched_start,
            matched_end: value.matched_end,
        }
    }
}

impl From<DelimiterPairMatchMetadataDocument> for DelimiterPairMatchMetadata {
    fn from(value: DelimiterPairMatchMetadataDocument) -> Self {
        Self {
            candidate_count: value.candidate_count,
            candidate_index: value.candidate_index,
            selected_range: value.selected_range.into(),
            inner_range: value.inner_range.into(),
            outer_range: value.outer_range.into(),
            include_start: value.include_start,
            include_end: value.include_end,
            matched_start: value.matched_start,
            matched_end: value.matched_end,
        }
    }
}

impl From<ExtractionMatchMetadata> for ExtractionMatchMetadataDocument {
    fn from(value: ExtractionMatchMetadata) -> Self {
        match value {
            ExtractionMatchMetadata::Selector(metadata) => Self::Selector(metadata.into()),
            ExtractionMatchMetadata::DelimiterPair(metadata) => {
                Self::DelimiterPair(metadata.into())
            }
        }
    }
}

impl From<ExtractionMatchMetadataDocument> for ExtractionMatchMetadata {
    fn from(value: ExtractionMatchMetadataDocument) -> Self {
        match value {
            ExtractionMatchMetadataDocument::Selector(metadata) => Self::Selector(metadata.into()),
            ExtractionMatchMetadataDocument::DelimiterPair(metadata) => {
                Self::DelimiterPair(metadata.into())
            }
        }
    }
}

impl From<ExtractionMatch> for ExtractionMatchDocument {
    fn from(value: ExtractionMatch) -> Self {
        Self {
            index: value.index,
            path: value.path,
            value_type: value.value_type,
            value: value.value,
            html: value.html,
            text: value.text,
            preview: value.preview,
            metadata: value.metadata.into(),
        }
    }
}

impl From<ExtractionMatchDocument> for ExtractionMatch {
    fn from(value: ExtractionMatchDocument) -> Self {
        Self {
            index: value.index,
            path: value.path,
            value_type: value.value_type,
            value: value.value,
            html: value.html,
            text: value.text,
            preview: value.preview,
            metadata: value.metadata.into(),
        }
    }
}

impl From<InspectionCount> for InspectionCountDocument {
    fn from(value: InspectionCount) -> Self {
        Self {
            name: value.name,
            count: value.count,
        }
    }
}

impl From<InspectionCountDocument> for InspectionCount {
    fn from(value: InspectionCountDocument) -> Self {
        Self {
            name: value.name,
            count: value.count,
        }
    }
}

impl From<HeadingInspection> for HeadingInspectionDocument {
    fn from(value: HeadingInspection) -> Self {
        Self {
            level: value.level,
            text: value.text,
            path: value.path,
        }
    }
}

impl From<HeadingInspectionDocument> for HeadingInspection {
    fn from(value: HeadingInspectionDocument) -> Self {
        Self {
            level: value.level,
            text: value.text,
            path: value.path,
        }
    }
}

impl From<LinkInspection> for LinkInspectionDocument {
    fn from(value: LinkInspection) -> Self {
        Self {
            text: value.text,
            href: value.href,
            resolved_href: value.resolved_href,
            path: value.path,
        }
    }
}

impl From<LinkInspectionDocument> for LinkInspection {
    fn from(value: LinkInspectionDocument) -> Self {
        Self {
            text: value.text,
            href: value.href,
            resolved_href: value.resolved_href,
            path: value.path,
        }
    }
}

impl From<ContentCandidateInspection> for ContentCandidateInspectionDocument {
    fn from(value: ContentCandidateInspection) -> Self {
        Self {
            selector: value.selector,
            path: value.path,
            tag_name: value.tag_name,
            text_char_count: value.text_char_count,
            heading_count: value.heading_count,
            link_count: value.link_count,
        }
    }
}

impl From<ContentCandidateInspectionDocument> for ContentCandidateInspection {
    fn from(value: ContentCandidateInspectionDocument) -> Self {
        Self {
            selector: value.selector,
            path: value.path,
            tag_name: value.tag_name,
            text_char_count: value.text_char_count,
            heading_count: value.heading_count,
            link_count: value.link_count,
        }
    }
}

impl From<DocumentInspection> for DocumentInspectionDocument {
    fn from(value: DocumentInspection) -> Self {
        Self {
            title: value.title,
            root_tag: value.root_tag,
            element_count: value.element_count,
            text_char_count: value.text_char_count,
            link_count: value.link_count,
            image_count: value.image_count,
            form_count: value.form_count,
            table_count: value.table_count,
            script_count: value.script_count,
            style_count: value.style_count,
            document_base_href: value.document_base_href,
            top_tags: value.top_tags.into_iter().map(Into::into).collect(),
            top_classes: value.top_classes.into_iter().map(Into::into).collect(),
            extraction_candidates: value
                .extraction_candidates
                .into_iter()
                .map(Into::into)
                .collect(),
            reading_candidates: value
                .reading_candidates
                .into_iter()
                .map(Into::into)
                .collect(),
            headings: value.headings.into_iter().map(Into::into).collect(),
            links: value.links.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<DocumentInspectionDocument> for DocumentInspection {
    fn from(value: DocumentInspectionDocument) -> Self {
        Self {
            title: value.title,
            root_tag: value.root_tag,
            element_count: value.element_count,
            text_char_count: value.text_char_count,
            link_count: value.link_count,
            image_count: value.image_count,
            form_count: value.form_count,
            table_count: value.table_count,
            script_count: value.script_count,
            style_count: value.style_count,
            document_base_href: value.document_base_href,
            top_tags: value.top_tags.into_iter().map(Into::into).collect(),
            top_classes: value.top_classes.into_iter().map(Into::into).collect(),
            extraction_candidates: value
                .extraction_candidates
                .into_iter()
                .map(Into::into)
                .collect(),
            reading_candidates: value
                .reading_candidates
                .into_iter()
                .map(Into::into)
                .collect(),
            headings: value.headings.into_iter().map(Into::into).collect(),
            links: value.links.into_iter().map(Into::into).collect(),
        }
    }
}

fn default_spec_version_document() -> u32 {
    crate::CORE_SPEC_VERSION
}

fn is_false(value: &bool) -> bool {
    !value
}

fn is_true(value: &bool) -> bool {
    *value
}

fn is_default_fetch_preflight(value: &FetchPreflightMode) -> bool {
    *value == FetchPreflightMode::default()
}

fn is_default_whitespace_mode(value: &WhitespaceMode) -> bool {
    *value == WhitespaceMode::Rendered
}

fn is_default_runtime_max_bytes(value: &MaxBytes) -> bool {
    *value == default_max_bytes_limit()
}

fn is_default_runtime_fetch_timeout(value: &FetchTimeoutMs) -> bool {
    *value == default_fetch_timeout_limit()
}

fn is_default_runtime_fetch_connect_timeout(value: &FetchConnectTimeoutMs) -> bool {
    *value == default_fetch_connect_timeout_limit()
}

fn is_default_tls_trust_policy(value: &TlsTrustPolicyDocument) -> bool {
    *value == TlsTrustPolicyDocument::default()
}

fn is_default_runtime_options_document(value: &RuntimeOptionsDocument) -> bool {
    *value == RuntimeOptionsDocument::default()
}

fn is_default_inspection_sample_limit_document(value: &usize) -> bool {
    *value == default_inspection_sample_limit_document()
}

fn is_default_selection_spec_document(value: &SelectionSpecDocument) -> bool {
    *value == SelectionSpecDocument::default()
}

fn is_default_value_spec_document(value: &ValueSpecDocument) -> bool {
    *value == ValueSpecDocument::default()
}

fn is_default_boundary_retention_document(value: &BoundaryRetentionDocument) -> bool {
    *value == BoundaryRetentionDocument::default()
}

fn is_default_rendering_options_document(value: &RenderingOptionsDocument) -> bool {
    *value == RenderingOptionsDocument::default()
}

fn is_default_output_options_document(value: &OutputOptionsDocument) -> bool {
    *value == OutputOptionsDocument::default()
}

fn is_default_spec_version_document(value: &u32) -> bool {
    *value == default_spec_version_document()
}

fn default_extraction_definition_schema_name_document() -> String {
    crate::EXTRACTION_DEFINITION_SCHEMA_NAME.to_owned()
}

fn is_default_extraction_definition_schema_name_document(value: &String) -> bool {
    value == crate::EXTRACTION_DEFINITION_SCHEMA_NAME
}

const fn default_extraction_definition_schema_version_document() -> u32 {
    crate::EXTRACTION_DEFINITION_SCHEMA_VERSION
}

fn is_default_extraction_definition_schema_version_document(value: &u32) -> bool {
    *value == default_extraction_definition_schema_version_document()
}

fn default_preview_chars_non_zero_document() -> NonZeroUsize {
    OutputOptions::default().preview_chars
}

fn is_default_preview_chars_non_zero_document(value: &NonZeroUsize) -> bool {
    *value == default_preview_chars_non_zero_document()
}

fn default_true_document() -> bool {
    true
}

fn default_inspection_sample_limit_document() -> usize {
    InspectionOptions::default().sample_limit
}

fn default_max_bytes_limit() -> MaxBytes {
    RuntimeOptions::default().max_bytes
}

fn default_fetch_timeout_limit() -> FetchTimeoutMs {
    RuntimeOptions::default().fetch_timeout
}

fn default_fetch_connect_timeout_limit() -> FetchConnectTimeoutMs {
    RuntimeOptions::default().fetch_connect_timeout
}
