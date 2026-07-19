//! Successful-extraction evidence and selected-match value objects.

use std::collections::BTreeMap;
use std::num::NonZeroUsize;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::super::plan::{Output, SelectionMode, StrategyKind};
use crate::DisplayedHttpUrl;

/// Half-open source range using byte offsets.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct ByteRange {
    /// Inclusive start byte offset.
    pub start: usize,
    /// Exclusive end byte offset.
    pub end: usize,
}

impl From<&crate::result::Range> for ByteRange {
    fn from(value: &crate::result::Range) -> Self {
        Self {
            start: value.start,
            end: value.end,
        }
    }
}

/// Source summary carried in one successful extraction result.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ResultSource {
    /// Base URL supplied before document parsing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_base_url: Option<DisplayedHttpUrl>,
    /// Effective base URL after document `<base href>` resolution.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effective_base_url: Option<DisplayedHttpUrl>,
    /// Parsed document title when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_title: Option<String>,
}

/// Execution summary fields shared by one successful extraction result.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct ResultExecution {
    /// Digest of the exact validated plan document.
    pub plan_digest_sha256: String,
    /// Executed strategy kind.
    pub strategy_kind: StrategyKind,
    /// Executed selection mode.
    pub selection_mode: SelectionMode,
    /// Executed output contract.
    pub output: Output,
    /// Total candidate count before selection.
    pub candidate_count: usize,
}

impl ResultExecution {
    /// Builds one extraction execution summary.
    pub fn new(
        plan_digest_sha256: impl Into<String>,
        strategy_kind: StrategyKind,
        selection_mode: SelectionMode,
        output: Output,
        candidate_count: usize,
    ) -> Self {
        Self {
            plan_digest_sha256: plan_digest_sha256.into(),
            strategy_kind,
            selection_mode,
            output,
            candidate_count,
        }
    }
}

/// Typed metadata for one selected match.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SelectedMatchMetadata {
    /// Selector-backed selected match metadata.
    CssSelector {
        /// Total selector candidate count before selection.
        candidate_count: usize,
        /// Selected 1-based candidate ordinal.
        candidate_index: NonZeroUsize,
        /// DOM path to the selected node.
        path: String,
        /// Matched tag name.
        tag_name: String,
        /// Matched element attributes after optional URL rewriting.
        attributes: BTreeMap<String, String>,
    },
    /// Delimiter-pair selected match metadata.
    DelimiterPair {
        /// Total slice candidate count before selection.
        candidate_count: usize,
        /// Selected 1-based candidate ordinal.
        candidate_index: NonZeroUsize,
        /// Selected byte range after applying the boundary-retention policy.
        selected_range: ByteRange,
        /// Inner byte range between the two matched boundaries.
        inner_range: ByteRange,
        /// Outer byte range including both matched boundaries.
        outer_range: ByteRange,
        /// Whether the selected payload includes the matched start boundary.
        include_start: bool,
        /// Whether the selected payload includes the matched end boundary.
        include_end: bool,
        /// Exact matched start boundary text.
        matched_start: String,
        /// Exact matched end boundary text.
        matched_end: String,
    },
}

impl SelectedMatchMetadata {
    /// Returns the strategy kind described by this metadata.
    pub const fn kind(&self) -> StrategyKind {
        match self {
            Self::CssSelector { .. } => StrategyKind::CssSelector,
            Self::DelimiterPair { .. } => StrategyKind::DelimiterPair,
        }
    }

    pub(super) fn candidate_count(&self) -> usize {
        match self {
            Self::CssSelector {
                candidate_count, ..
            }
            | Self::DelimiterPair {
                candidate_count, ..
            } => *candidate_count,
        }
    }

    pub(super) fn candidate_index(&self) -> NonZeroUsize {
        match self {
            Self::CssSelector {
                candidate_index, ..
            }
            | Self::DelimiterPair {
                candidate_index, ..
            } => *candidate_index,
        }
    }
}

/// One selected match returned on successful extraction.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct SelectedMatch {
    /// Selected 1-based candidate ordinal within all discovered candidates.
    pub candidate_index: NonZeroUsize,
    /// Exact output payload returned for the selected candidate.
    pub output_value: Value,
    /// Text HTMLCut would return for text output.
    pub text_output: String,
    /// Text rendered from the detached canonicalized CSS-selected clone when configured.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comparison_text_output: Option<String>,
    /// DOM descendant text without rendered structural decoration, when the strategy has a DOM.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plain_text_output: Option<String>,
    /// Plain text from the detached canonicalized CSS-selected clone when configured.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comparison_plain_text_output: Option<String>,
    /// Exact selected HTML fragment when the strategy supports it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_html_output: Option<String>,
    /// True inner HTML for the selected candidate.
    pub inner_html_output: String,
    /// Outer HTML for the selected candidate.
    pub outer_html_output: String,
    /// Stable typed metadata for the selected match.
    pub metadata: SelectedMatchMetadata,
}
