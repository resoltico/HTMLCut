use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::num::NonZeroUsize;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::super::stable_json::digest_stable_json_omitting_field;
use super::plan::{Output, OutputKind, SelectionMode, StrategyKind};
use super::shared::{
    ContractError, ERROR_SCHEMA_NAME, ERROR_SCHEMA_VERSION, INTEROP_V1_PROFILE, RESULT_SCHEMA_NAME,
    RESULT_SCHEMA_VERSION, validate_schema_identity, validate_sha256_hex,
};
use crate::DisplayedHttpUrl;

macro_rules! interop_diagnostic_codes {
    (
        $(
            $(#[$meta:meta])*
            $variant:ident => $code:literal,
        )+
    ) => {
        /// Stable diagnostic-code identifiers published by `htmlcut-v1`.
        #[derive(
            Clone,
            Copy,
            Debug,
            Serialize,
            Deserialize,
            JsonSchema,
            PartialEq,
            Eq,
            PartialOrd,
            Ord,
            Hash,
        )]
        pub enum InteropDiagnosticCode {
            $(
                $(#[$meta])*
                #[serde(rename = $code)]
                $variant,
            )+
        }

        impl InteropDiagnosticCode {
            /// Returns the complete stable diagnostic-code inventory.
            pub const ALL: &'static [Self] = &[
                $(
                    Self::$variant,
                )+
            ];

            /// Returns the stable string form of this diagnostic code.
            pub const fn as_str(self) -> &'static str {
                match self {
                    $(
                        Self::$variant => $code,
                    )+
                }
            }
        }

        impl fmt::Display for InteropDiagnosticCode {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(self.as_str())
            }
        }

        impl PartialEq<&str> for InteropDiagnosticCode {
            fn eq(&self, other: &&str) -> bool {
                self.as_str() == *other
            }
        }

        impl PartialEq<InteropDiagnosticCode> for &str {
            fn eq(&self, other: &InteropDiagnosticCode) -> bool {
                *self == other.as_str()
            }
        }
    };
}

interop_diagnostic_codes! {
    /// The source could not be loaded or decoded.
    SourceLoadFailed => "SOURCE_LOAD_FAILED",
    /// The request spec version is unsupported.
    UnsupportedSpecVersion => "UNSUPPORTED_SPEC_VERSION",
    /// The CSS selector is invalid.
    InvalidSelector => "INVALID_SELECTOR",
    /// The slice pattern or regex flags are invalid.
    InvalidSlicePattern => "INVALID_SLICE_PATTERN",
    /// The requested value type is not valid for the chosen extraction strategy.
    UnsupportedValueType => "UNSUPPORTED_VALUE_TYPE",
    /// No candidates matched the request.
    NoMatch => "NO_MATCH",
    /// Exact-one selection found multiple candidates.
    AmbiguousMatch => "AMBIGUOUS_MATCH",
    /// The requested match index is outside the candidate set.
    MatchIndexOutOfRange => "MATCH_INDEX_OUT_OF_RANGE",
    /// The selected HTML is missing the requested attribute.
    MissingAttribute => "MISSING_ATTRIBUTE",
    /// More than one candidate matched while first-match mode was active.
    MultipleMatches => "MULTIPLE_MATCHES",
    /// URL rewriting depended on an unresolved effective base URL.
    EffectiveBaseUrlUnresolved => "EFFECTIVE_BASE_URL_UNRESOLVED",
    /// Slice selection appears to start or end inside HTML markup.
    SliceSplitsMarkup => "SLICE_SPLITS_MARKUP",
}

impl From<crate::DiagnosticCode> for InteropDiagnosticCode {
    fn from(value: crate::DiagnosticCode) -> Self {
        match value {
            crate::DiagnosticCode::SourceLoadFailed => Self::SourceLoadFailed,
            crate::DiagnosticCode::UnsupportedSpecVersion => Self::UnsupportedSpecVersion,
            crate::DiagnosticCode::InvalidSelector => Self::InvalidSelector,
            crate::DiagnosticCode::InvalidSlicePattern => Self::InvalidSlicePattern,
            crate::DiagnosticCode::UnsupportedValueType => Self::UnsupportedValueType,
            crate::DiagnosticCode::NoMatch => Self::NoMatch,
            crate::DiagnosticCode::AmbiguousMatch => Self::AmbiguousMatch,
            crate::DiagnosticCode::MatchIndexOutOfRange => Self::MatchIndexOutOfRange,
            crate::DiagnosticCode::MissingAttribute => Self::MissingAttribute,
            crate::DiagnosticCode::MultipleMatches => Self::MultipleMatches,
            crate::DiagnosticCode::EffectiveBaseUrlUnresolved => Self::EffectiveBaseUrlUnresolved,
            crate::DiagnosticCode::SliceSplitsMarkup => Self::SliceSplitsMarkup,
        }
    }
}

/// Severity level for published interop diagnostics.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum InteropDiagnosticLevel {
    /// The operation failed.
    Error,
    /// The operation succeeded but with a risk or fallback.
    Warning,
    /// Supplemental informational context.
    Info,
}

impl From<crate::DiagnosticLevel> for InteropDiagnosticLevel {
    fn from(value: crate::DiagnosticLevel) -> Self {
        match value {
            crate::DiagnosticLevel::Error => Self::Error,
            crate::DiagnosticLevel::Warning => Self::Warning,
            crate::DiagnosticLevel::Info => Self::Info,
        }
    }
}

/// Machine-readable diagnostic published by htmlcut-v1.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct InteropDiagnostic {
    /// Severity level for the diagnostic.
    pub level: InteropDiagnosticLevel,
    /// Stable interop diagnostic code.
    pub code: InteropDiagnosticCode,
    /// Human-readable diagnostic message.
    pub message: String,
    /// Optional structured details for automation and debugging.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

impl From<&crate::Diagnostic> for InteropDiagnostic {
    fn from(value: &crate::Diagnostic) -> Self {
        Self {
            level: value.level.into(),
            code: value.code.into(),
            message: value.message.clone(),
            details: value.details.clone(),
        }
    }
}

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

/// Successful extraction result owned by HTMLCut.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct InteropResult {
    /// Schema identity.
    pub schema_name: String,
    /// Schema version.
    pub schema_version: u32,
    /// Interoperability profile identifier.
    pub interop_profile: String,
    /// Digest of the exact validated plan document.
    pub plan_digest_sha256: String,
    /// Digest of this exact result document with this field omitted.
    pub result_digest_sha256: String,
    /// Executed strategy kind.
    pub strategy_kind: StrategyKind,
    /// Executed selection mode.
    pub selection_mode: SelectionMode,
    /// Executed output contract.
    pub output: Output,
    /// Total candidate count before selection.
    pub candidate_count: usize,
    /// Source summary.
    pub source: ResultSource,
    /// One or more selected matches.
    pub selected_matches: Vec<SelectedMatch>,
    /// Warning and informational diagnostics emitted during extraction.
    pub diagnostics: Vec<InteropDiagnostic>,
    /// Reserved extension object ignored by v1 consumers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<BTreeMap<String, Value>>,
}

impl InteropResult {
    /// Builds one successful extraction result with the v1 schema identity.
    pub fn new(
        execution: ResultExecution,
        source: ResultSource,
        selected_matches: Vec<SelectedMatch>,
        diagnostics: Vec<InteropDiagnostic>,
    ) -> Self {
        Self {
            schema_name: RESULT_SCHEMA_NAME.to_owned(),
            schema_version: RESULT_SCHEMA_VERSION,
            interop_profile: INTEROP_V1_PROFILE.to_owned(),
            plan_digest_sha256: execution.plan_digest_sha256,
            result_digest_sha256: String::new(),
            strategy_kind: execution.strategy_kind,
            selection_mode: execution.selection_mode,
            output: execution.output,
            candidate_count: execution.candidate_count,
            source,
            selected_matches,
            diagnostics,
            extensions: None,
        }
    }

    fn validate_body(&self) -> Result<(), ContractError> {
        validate_schema_identity(
            &self.schema_name,
            RESULT_SCHEMA_NAME,
            self.schema_version,
            RESULT_SCHEMA_VERSION,
            &self.interop_profile,
            INTEROP_V1_PROFILE,
        )?;
        validate_sha256_hex("plan_digest_sha256", &self.plan_digest_sha256)?;
        self.output.validate_for_strategy(self.strategy_kind)?;

        if self.candidate_count == 0 {
            return Err(ContractError::ZeroCandidateCount);
        }

        if self.selected_matches.is_empty() {
            return Err(ContractError::ZeroSelectedMatchCount);
        }

        let expected_selected_matches = match self.selection_mode {
            SelectionMode::Single | SelectionMode::First | SelectionMode::Nth => 1,
            SelectionMode::All => self.candidate_count,
        };
        if self.selected_matches.len() != expected_selected_matches {
            return Err(ContractError::SelectionModeCountMismatch {
                selection_mode: self.selection_mode,
                selected_match_count: self.selected_matches.len(),
                expected_selected_matches,
                candidate_count: self.candidate_count,
            });
        }

        let output_kind = self.output.kind();
        let mut seen_candidates = BTreeSet::new();
        for selected_match in &self.selected_matches {
            let selected = selected_match.candidate_index.get();
            if selected > self.candidate_count {
                return Err(ContractError::SelectedCandidateOutOfRange {
                    selected,
                    candidate_count: self.candidate_count,
                });
            }

            if !seen_candidates.insert(selected) {
                return Err(ContractError::DuplicateSelectedCandidate { selected });
            }

            if selected_match.metadata.kind() != self.strategy_kind {
                return Err(ContractError::MetadataKindMismatch {
                    strategy_kind: self.strategy_kind,
                    metadata_kind: selected_match.metadata.kind(),
                });
            }

            if selected_match.metadata.candidate_count() != self.candidate_count
                || selected_match.metadata.candidate_index() != selected_match.candidate_index
            {
                return Err(ContractError::SelectedCandidateOutOfRange {
                    selected,
                    candidate_count: self.candidate_count,
                });
            }

            match self.strategy_kind {
                StrategyKind::CssSelector => {
                    if selected_match.selected_html_output.is_some() {
                        return Err(ContractError::UnexpectedSelectedHtmlOutput);
                    }
                }
                StrategyKind::DelimiterPair => {
                    if selected_match.selected_html_output.is_none() {
                        return Err(ContractError::MissingSelectedHtmlOutput);
                    }
                }
            }

            match output_kind {
                OutputKind::Structured => {
                    if !selected_match.output_value.is_object() {
                        return Err(ContractError::NonObjectStructuredOutputValue);
                    }
                }
                _ => {
                    if !selected_match.output_value.is_string() {
                        return Err(ContractError::NonStringOutputValue { output_kind });
                    }
                }
            }
        }

        if self
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.level == InteropDiagnosticLevel::Error)
        {
            return Err(ContractError::ErrorDiagnosticsInSuccess);
        }

        Ok(())
    }

    /// Validates the schema identity, semantic invariants, and canonical digest for this result.
    pub fn validate(&self) -> Result<(), ContractError> {
        self.validate_body()?;
        validate_sha256_hex("result_digest_sha256", &self.result_digest_sha256)?;

        let expected = digest_stable_json_omitting_field(self, "result_digest_sha256")?;
        if self.result_digest_sha256 != expected {
            return Err(ContractError::DigestMismatch {
                field: "result_digest_sha256",
                expected,
                received: self.result_digest_sha256.clone(),
            });
        }

        Ok(())
    }

    /// Serializes this result with the stable JSON profile.
    pub fn stable_json(&self) -> Result<String, ContractError> {
        self.validate()?;
        super::super::stable_json::stable_json_v1(self)
    }

    /// Computes the SHA-256 digest of this result with `result_digest_sha256` omitted.
    pub fn digest_sha256(&self) -> Result<String, ContractError> {
        self.validate_body()?;
        digest_stable_json_omitting_field(self, "result_digest_sha256")
    }

    /// Computes and stores `result_digest_sha256` on this result.
    pub fn with_computed_digest(mut self) -> Result<Self, ContractError> {
        self.result_digest_sha256 = self.digest_sha256()?;
        Ok(self)
    }
}

/// Extraction error vocabulary owned by HTMLCut.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    /// The plan was invalid for the interop profile.
    PlanInvalid,
    /// No candidate matched the requested strategy and selection.
    NoMatch,
    /// Exact-one selection saw multiple candidates.
    AmbiguousMatch,
    /// The selected candidate did not carry the requested attribute.
    MissingAttribute,
    /// An internal failure occurred inside HTMLCut.
    InternalError,
}

/// Typed extraction error document owned by HTMLCut.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct InteropError {
    /// Schema identity.
    pub schema_name: String,
    /// Schema version.
    pub schema_version: u32,
    /// Interoperability profile identifier.
    pub interop_profile: String,
    /// Digest of the exact validated plan document.
    pub plan_digest_sha256: String,
    /// Digest of this exact error document with this field omitted.
    pub error_digest_sha256: String,
    /// Interop error code.
    pub error_code: ErrorCode,
    /// Human-readable error summary.
    pub message: String,
    /// Strategy kind when one was known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strategy_kind: Option<StrategyKind>,
    /// Machine-readable detail object.
    pub details: BTreeMap<String, Value>,
    /// Underlying HTMLCut diagnostics that produced this error.
    pub diagnostics: Vec<InteropDiagnostic>,
    /// Reserved extension object ignored by v1 consumers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<BTreeMap<String, Value>>,
}

impl InteropError {
    /// Builds one extraction error with the v1 schema identity.
    pub fn new(
        plan_digest_sha256: impl Into<String>,
        error_code: ErrorCode,
        message: impl Into<String>,
        strategy_kind: Option<StrategyKind>,
        details: BTreeMap<String, Value>,
        diagnostics: Vec<InteropDiagnostic>,
    ) -> Self {
        Self {
            schema_name: ERROR_SCHEMA_NAME.to_owned(),
            schema_version: ERROR_SCHEMA_VERSION,
            interop_profile: INTEROP_V1_PROFILE.to_owned(),
            plan_digest_sha256: plan_digest_sha256.into(),
            error_digest_sha256: String::new(),
            error_code,
            message: message.into(),
            strategy_kind,
            details,
            diagnostics,
            extensions: None,
        }
    }

    fn validate_body(&self) -> Result<(), ContractError> {
        validate_schema_identity(
            &self.schema_name,
            ERROR_SCHEMA_NAME,
            self.schema_version,
            ERROR_SCHEMA_VERSION,
            &self.interop_profile,
            INTEROP_V1_PROFILE,
        )?;
        validate_sha256_hex("plan_digest_sha256", &self.plan_digest_sha256)?;
        Ok(())
    }

    /// Validates the schema identity and canonical digest for this error document.
    pub fn validate(&self) -> Result<(), ContractError> {
        self.validate_body()?;
        validate_sha256_hex("error_digest_sha256", &self.error_digest_sha256)?;

        let expected = digest_stable_json_omitting_field(self, "error_digest_sha256")?;
        if self.error_digest_sha256 != expected {
            return Err(ContractError::DigestMismatch {
                field: "error_digest_sha256",
                expected,
                received: self.error_digest_sha256.clone(),
            });
        }

        Ok(())
    }

    /// Serializes this error with the stable JSON profile.
    pub fn stable_json(&self) -> Result<String, ContractError> {
        self.validate()?;
        super::super::stable_json::stable_json_v1(self)
    }

    /// Computes the SHA-256 digest of this error with `error_digest_sha256` omitted.
    pub fn digest_sha256(&self) -> Result<String, ContractError> {
        self.validate_body()?;
        digest_stable_json_omitting_field(self, "error_digest_sha256")
    }

    /// Computes and stores `error_digest_sha256` on this error document.
    pub fn with_computed_digest(mut self) -> Result<Self, ContractError> {
        self.error_digest_sha256 = self.digest_sha256()?;
        Ok(self)
    }
}
