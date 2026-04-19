use std::collections::BTreeMap;

use schemars::JsonSchema;
use scraper::Html;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::catalog::OperationId;

use super::{ExtractionSpec, SourceKind, ValueType};

/// Severity level for emitted diagnostics.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DiagnosticLevel {
    /// The operation failed.
    Error,
    /// The operation succeeded but with a risk or fallback.
    Warning,
    /// Supplemental informational context.
    Info,
}

/// Machine-readable error, warning, or informational message.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct Diagnostic {
    /// Severity level for the diagnostic.
    pub level: DiagnosticLevel,
    /// Stable diagnostic code.
    pub code: String,
    /// Human-readable diagnostic message.
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional structured details for automation and debugging.
    pub details: Option<Value>,
}

/// Metadata describing the source that was loaded into HTMLCut.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct SourceMetadata {
    /// The concrete source kind that produced the loaded HTML.
    pub kind: SourceKind,
    /// The resolved file path, URL, or symbolic value for the loaded source.
    pub value: String,
    /// The base URL available before the document itself was parsed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_base_url: Option<String>,
    /// The base URL actually used after honoring any document `<base href>`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effective_base_url: Option<String>,
    /// The number of bytes read into memory for this source.
    pub bytes_read: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Structured trace of successful source-loading steps, currently used for URL fetches.
    pub load_steps: Vec<SourceLoadStep>,
    /// The original source text when the caller explicitly asks to include it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

/// One successful source-loading action.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum SourceLoadAction {
    /// Probe a remote source with `HEAD` before downloading the body.
    HeadPreflight,
    /// Load a remote source body with `GET`.
    Get,
}

/// Outcome class for one successful source-loading action.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum SourceLoadOutcome {
    /// The action completed normally.
    Succeeded,
    /// The action was intentionally skipped.
    Skipped,
    /// The action did not complete, but HTMLCut recovered and continued.
    Fallback,
    /// The action failed and the load could not continue.
    Failed,
}

/// One structured source-loading step emitted for a source-loading attempt.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct SourceLoadStep {
    /// Load action that HTMLCut attempted.
    pub action: SourceLoadAction,
    /// Outcome of that load action.
    pub outcome: SourceLoadOutcome,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// HTTP status code when the step received a response.
    pub status: Option<u16>,
    /// Human-readable explanation of what HTMLCut observed.
    pub message: String,
}

/// Basic performance and cardinality summary for one extraction run.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ExtractionStats {
    /// Wall-clock duration in milliseconds.
    pub duration_ms: u128,
    /// Number of candidates discovered before match filtering.
    pub candidate_count: usize,
    /// Number of matches returned after filtering.
    pub match_count: usize,
}

/// One extracted or previewed match.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ExtractionMatch {
    /// One-based position in the returned match list.
    pub index: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// DOM path when the match came from selector extraction.
    pub path: Option<String>,
    /// Value shape stored in [`Self::value`].
    pub value_type: ValueType,
    /// Final extracted value.
    pub value: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// HTML payload when available.
    pub html: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Text payload when available.
    pub text: Option<String>,
    /// Compact preview string for humans and agents.
    pub preview: String,
    /// Stable typed per-match metadata.
    pub metadata: ExtractionMatchMetadata,
}

/// Stable typed selector metadata for one extracted match.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct SelectorMatchMetadata {
    /// Total candidates discovered before selection.
    pub candidate_count: usize,
    /// One-based ordinal within all discovered candidates.
    pub candidate_index: usize,
    /// DOM path to the matched node.
    pub path: String,
    /// Matched element tag name.
    pub tag_name: String,
    /// Matched element attributes after optional URL rewriting.
    pub attributes: BTreeMap<String, String>,
}

/// Stable typed delimiter-pair metadata for one extracted match.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct DelimiterPairMatchMetadata {
    /// Total candidates discovered before selection.
    pub candidate_count: usize,
    /// One-based ordinal within all discovered candidates.
    pub candidate_index: usize,
    /// Half-open byte range of the selected payload.
    pub selected_range: Range,
    /// Half-open byte range between the matched boundaries.
    pub inner_range: Range,
    /// Half-open byte range including both matched boundaries.
    pub outer_range: Range,
    /// Whether the matched start boundary was included in the selected payload.
    pub include_start: bool,
    /// Whether the matched end boundary was included in the selected payload.
    pub include_end: bool,
    /// Exact start-boundary text that matched this candidate.
    pub matched_start: String,
    /// Exact end-boundary text that matched this candidate.
    pub matched_end: String,
}

/// Stable typed metadata emitted for one extracted match.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ExtractionMatchMetadata {
    /// Metadata for CSS-selector extraction.
    Selector(SelectorMatchMetadata),
    /// Metadata for delimiter-pair slice extraction.
    DelimiterPair(DelimiterPairMatchMetadata),
}

impl ExtractionMatchMetadata {
    /// Returns the total number of discovered candidates.
    pub const fn candidate_count(&self) -> usize {
        match self {
            Self::Selector(metadata) => metadata.candidate_count,
            Self::DelimiterPair(metadata) => metadata.candidate_count,
        }
    }

    /// Returns the one-based candidate ordinal.
    pub const fn candidate_index(&self) -> usize {
        match self {
            Self::Selector(metadata) => metadata.candidate_index,
            Self::DelimiterPair(metadata) => metadata.candidate_index,
        }
    }
}

/// Structured result produced by `extract` and `preview_extraction`.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ExtractionResult {
    /// The canonical operation that produced this result.
    pub operation_id: OperationId,
    /// The stable schema name for this result document.
    pub schema_name: String,
    /// The version of the extraction result schema.
    pub schema_version: u32,
    /// Whether the extraction finished without error diagnostics.
    pub ok: bool,
    /// Source metadata for the loaded input.
    pub source: SourceMetadata,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// The parsed document title when one was available.
    pub document_title: Option<String>,
    /// The normalized extraction request contract.
    pub extraction: ExtractionSpec,
    /// Match counts and timing for the extraction.
    pub stats: ExtractionStats,
    /// The extracted matches in output order.
    pub matches: Vec<ExtractionMatch>,
    /// Diagnostics emitted while validating, loading, or extracting.
    pub diagnostics: Vec<Diagnostic>,
}

/// Name/count pair used in document inspection summaries.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct InspectionCount {
    /// The tag or class name being counted.
    pub name: String,
    /// Number of occurrences for this name.
    pub count: usize,
}

/// One sampled heading from document inspection.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct HeadingInspection {
    /// Heading level from `h1` through `h6`.
    pub level: u8,
    /// Rendered heading text.
    pub text: String,
    /// DOM path to the heading element.
    pub path: String,
}

/// One sampled link from document inspection.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct LinkInspection {
    /// Rendered link text.
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Raw `href` attribute when present.
    pub href: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Resolved absolute URL when one could be computed.
    pub resolved_href: Option<String>,
    /// DOM path to the link element.
    pub path: String,
}

/// Document-level summary produced by `inspect source`.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct DocumentInspection {
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Parsed document title when present.
    pub title: Option<String>,
    /// Name of the root element used for inspection.
    pub root_tag: String,
    /// Number of parsed elements.
    pub element_count: usize,
    /// Character count of normalized body text.
    pub text_char_count: usize,
    /// Number of anchor elements.
    pub link_count: usize,
    /// Number of image elements.
    pub image_count: usize,
    /// Number of form elements.
    pub form_count: usize,
    /// Number of table elements.
    pub table_count: usize,
    /// Number of script elements.
    pub script_count: usize,
    /// Number of style elements.
    pub style_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Raw document `<base href>` when present.
    pub document_base_href: Option<String>,
    /// Most frequent element names.
    pub top_tags: Vec<InspectionCount>,
    /// Most frequent CSS classes.
    pub top_classes: Vec<InspectionCount>,
    /// Sampled headings up to the configured limit.
    pub headings: Vec<HeadingInspection>,
    /// Sampled links up to the configured limit.
    pub links: Vec<LinkInspection>,
}

/// Structured result produced by `inspect_source`.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct SourceInspectionResult {
    /// The canonical operation that produced this result.
    pub operation_id: OperationId,
    /// The stable schema name for this result document.
    pub schema_name: String,
    /// The version of the source-inspection result schema.
    pub schema_version: u32,
    /// Whether the inspection finished without error diagnostics.
    pub ok: bool,
    /// Source metadata for the loaded input.
    pub source: SourceMetadata,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Document-level analysis when parsing succeeded.
    pub document: Option<DocumentInspection>,
    /// Diagnostics emitted while validating, loading, or parsing.
    pub diagnostics: Vec<Diagnostic>,
}

/// Parsed document tree plus the source metadata that produced it.
#[derive(Clone, Debug)]
pub struct ParsedDocument {
    /// Metadata for the loaded source.
    pub source: SourceMetadata,
    /// Parsed HTML tree.
    pub document: Html,
}

/// Parse-only result produced by [`crate::parse_document`].
#[derive(Clone, Debug)]
pub struct ParseDocumentResult {
    /// The canonical operation that produced this result.
    pub operation_id: OperationId,
    /// Whether parsing finished without error diagnostics.
    pub ok: bool,
    /// Source metadata for the loaded input.
    pub source: SourceMetadata,
    /// Diagnostics emitted while validating, loading, or parsing.
    pub diagnostics: Vec<Diagnostic>,
    /// The parsed document tree when parsing succeeded.
    pub document: Option<ParsedDocument>,
}

/// Half-open source range using byte offsets.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct Range {
    /// Inclusive start byte offset.
    pub start: usize,
    /// Exclusive end byte offset.
    pub end: usize,
}
