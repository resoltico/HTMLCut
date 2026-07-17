//! Successful interop result document and its semantic validation.

use std::collections::{BTreeMap, BTreeSet};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::super::super::stable_json::digest_stable_json_omitting_field;
use super::super::plan::{Output, SelectionMode, StrategyKind};
use super::super::shared::{
    ContractError, INTEROP_V1_PROFILE, RESULT_SCHEMA_NAME, RESULT_SCHEMA_VERSION,
    validate_schema_identity, validate_sha256_hex,
};
use super::{
    InteropDiagnostic, InteropDiagnosticLevel, ResultExecution, ResultSource, SelectedMatch,
};

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

        for diagnostic in &self.diagnostics {
            diagnostic.validate_body()?;
        }

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
                    if selected_match.comparison_text_output.is_some() {
                        return Err(ContractError::UnexpectedComparisonTextOutput);
                    }
                }
            }

            if selected_match.comparison_text_output.is_some()
                && !matches!(self.output, Output::Text | Output::Structured)
            {
                return Err(ContractError::UnexpectedComparisonTextOutputForOutput { output_kind });
            }

            match &self.output {
                Output::Text => {
                    let Some(output_value) = selected_match.output_value.as_str() else {
                        return Err(ContractError::NonStringOutputValue { output_kind });
                    };
                    let expected_text = selected_match
                        .comparison_text_output
                        .as_deref()
                        .unwrap_or(&selected_match.text_output);
                    if output_value != expected_text {
                        return Err(ContractError::TextOutputValueMismatch);
                    }
                }
                Output::Structured => {
                    let Some(structured_output) = selected_match.output_value.as_object() else {
                        return Err(ContractError::NonObjectStructuredOutputValue);
                    };
                    if structured_output.contains_key("comparisonTextOutput") {
                        return Err(ContractError::StructuredOutputContainsComparisonText);
                    }
                }
                Output::SelectedHtml
                | Output::InnerHtml
                | Output::OuterHtml
                | Output::Attribute { .. } => {
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
        super::super::super::stable_json::stable_json_v1(self)
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
