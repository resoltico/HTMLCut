// SPDX-License-Identifier: MPL-2.0
//! Finite core policy, independent of acquisition and publication.

use crate::{ErrorCode, ExtractionError};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Immutable policy for accepting and preparing a snapshot.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct PreparationLimits {
    /// Maximum accepted UTF-8 bytes.
    #[schemars(range(min = 1, max = 52428800))]
    pub max_source_bytes: u32,
    /// Maximum constructed element nodes, including templates.
    #[schemars(range(min = 1, max = 1000000))]
    pub max_elements: u32,
    /// Maximum constructed DOM nodes, including fixed parser bookkeeping.
    #[schemars(range(min = 1, max = 4000000))]
    pub max_nodes: u32,
    /// Maximum DOM depth, counting the document root at zero.
    #[schemars(range(min = 1, max = 4096))]
    pub max_depth: u32,
    /// Maximum parser construction and attachment accounting work.
    #[schemars(range(min = 1, max = 100000000))]
    pub max_parse_work: u32,
}

impl Default for PreparationLimits {
    fn default() -> Self {
        Self {
            max_source_bytes: MAX_SOURCE_BYTES as u32,
            max_elements: 250_000,
            max_nodes: 1_000_000,
            max_depth: 2_048,
            max_parse_work: 10_000_000,
        }
    }
}

impl PreparationLimits {
    /// Rejects zero budgets and values exceeding maintained hard maxima.
    pub fn validate(&self) -> Result<(), ExtractionError> {
        if self.max_source_bytes == 0 || self.max_source_bytes > MAX_SOURCE_BYTES as u32 {
            return Err(ExtractionError::new(
                ErrorCode::InvalidLimit,
                "validation",
                "max_source_bytes is outside its supported range.",
            ));
        }
        if self.max_elements == 0 || self.max_elements > 1_000_000 {
            return Err(ExtractionError::new(
                ErrorCode::InvalidLimit,
                "validation",
                "max_elements is outside its supported range.",
            ));
        }
        if self.max_nodes == 0 || self.max_nodes > 4_000_000 {
            return Err(ExtractionError::new(
                ErrorCode::InvalidLimit,
                "validation",
                "max_nodes is outside its supported range.",
            ));
        }
        if self.max_depth == 0 || self.max_depth > 4_096 {
            return Err(ExtractionError::new(
                ErrorCode::InvalidLimit,
                "validation",
                "max_depth is outside its supported range.",
            ));
        }
        if self.max_parse_work == 0 || self.max_parse_work > 100_000_000 {
            return Err(ExtractionError::new(
                ErrorCode::InvalidLimit,
                "validation",
                "max_parse_work is outside its supported range.",
            ));
        }
        Ok(())
    }
}

impl<'de> Deserialize<'de> for PreparationLimits {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(default, deny_unknown_fields)]
        struct Input {
            max_source_bytes: u32,
            max_elements: u32,
            max_nodes: u32,
            max_depth: u32,
            max_parse_work: u32,
        }
        impl Default for Input {
            fn default() -> Self {
                Self {
                    max_source_bytes: MAX_SOURCE_BYTES as u32,
                    max_elements: 250_000,
                    max_nodes: 1_000_000,
                    max_depth: 2_048,
                    max_parse_work: 10_000_000,
                }
            }
        }
        let input = Input::deserialize(deserializer)?;
        let limits = Self {
            max_source_bytes: input.max_source_bytes,
            max_elements: input.max_elements,
            max_nodes: input.max_nodes,
            max_depth: input.max_depth,
            max_parse_work: input.max_parse_work,
        };
        limits.validate().map_err(serde::de::Error::custom)?;
        Ok(limits)
    }
}

/// Hard candidate/count maximum shared by query admission and execution policy.
pub(crate) const MAX_CANDIDATES: u32 = 1_000_000;

/// Fresh, shared budgets for one compiled-plan execution.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct ExecutionLimits {
    /// Aggregate traversal, matching and source-search work units.
    #[schemars(range(min = 1, max = 10000000))]
    pub max_work: u32,
    /// Maximum enumerated selection candidates.
    #[schemars(range(min = 1, max = 1000000))]
    pub max_candidates: u32,
    /// Maximum selected values.
    #[schemars(range(min = 1, max = 100000))]
    pub max_selected: u32,
    /// Maximum field containers plus all-valued strings, or flat selected strings.
    #[schemars(range(min = 1, max = 1000000))]
    pub max_cells: u32,
    /// Maximum UTF-8 bytes in one projected value.
    #[schemars(range(min = 1, max = 8388608))]
    pub max_value_bytes: u32,
    /// Maximum aggregate projected-value bytes.
    #[schemars(range(min = 1, max = 67108864))]
    pub max_total_value_bytes: u32,
}

impl Default for ExecutionLimits {
    fn default() -> Self {
        Self {
            max_work: 1_000_000,
            max_candidates: 100_000,
            max_selected: 10_000,
            max_cells: 100_000,
            max_value_bytes: 8 * 1024 * 1024,
            max_total_value_bytes: 64 * 1024 * 1024,
        }
    }
}

impl ExecutionLimits {
    /// Rejects zero budgets and values exceeding maintained hard maxima.
    pub fn validate(&self) -> Result<(), ExtractionError> {
        if self.max_work == 0 || self.max_work > 10_000_000 {
            return Err(ExtractionError::new(
                ErrorCode::InvalidLimit,
                "validation",
                "max_work is outside its supported range.",
            ));
        }
        if self.max_candidates == 0 || self.max_candidates > MAX_CANDIDATES {
            return Err(ExtractionError::new(
                ErrorCode::InvalidLimit,
                "validation",
                "max_candidates is outside its supported range.",
            ));
        }
        if self.max_selected == 0 || self.max_selected > 100_000 {
            return Err(ExtractionError::new(
                ErrorCode::InvalidLimit,
                "validation",
                "max_selected is outside its supported range.",
            ));
        }
        if self.max_cells == 0 || self.max_cells > 1_000_000 {
            return Err(ExtractionError::new(
                ErrorCode::InvalidLimit,
                "validation",
                "max_cells is outside its supported range.",
            ));
        }
        if self.max_value_bytes == 0 || self.max_value_bytes > 8 * 1024 * 1024 {
            return Err(ExtractionError::new(
                ErrorCode::InvalidLimit,
                "validation",
                "max_value_bytes is outside its supported range.",
            ));
        }
        if self.max_total_value_bytes == 0 || self.max_total_value_bytes > 64 * 1024 * 1024 {
            return Err(ExtractionError::new(
                ErrorCode::InvalidLimit,
                "validation",
                "max_total_value_bytes is outside its supported range.",
            ));
        }
        Ok(())
    }
}

impl<'de> Deserialize<'de> for ExecutionLimits {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(default, deny_unknown_fields)]
        struct Input {
            max_work: u32,
            max_candidates: u32,
            max_selected: u32,
            max_cells: u32,
            max_value_bytes: u32,
            max_total_value_bytes: u32,
        }
        impl Default for Input {
            fn default() -> Self {
                Self {
                    max_work: 1_000_000,
                    max_candidates: 100_000,
                    max_selected: 10_000,
                    max_cells: 100_000,
                    max_value_bytes: 8 * 1024 * 1024,
                    max_total_value_bytes: 64 * 1024 * 1024,
                }
            }
        }
        let input = Input::deserialize(deserializer)?;
        let limits = Self {
            max_work: input.max_work,
            max_candidates: input.max_candidates,
            max_selected: input.max_selected,
            max_cells: input.max_cells,
            max_value_bytes: input.max_value_bytes,
            max_total_value_bytes: input.max_total_value_bytes,
        };
        limits.validate().map_err(serde::de::Error::custom)?;
        Ok(limits)
    }
}

/// Maximum encoded plan bytes; checked before JSON parsing.
pub const MAX_PLAN_BYTES: usize = 256 * 1024;
/// Maximum bytes in one selector or boundary/predicate pattern.
pub const MAX_PATTERN_BYTES: usize = 8 * 1024;
/// Maximum selector/regular-expression syntactic nesting.
pub const MAX_PATTERN_DEPTH: u32 = 64;
/// Maximum compiled regular-expression size.
pub const MAX_REGEX_BYTES: usize = 8 * 1024 * 1024;
/// Maximum guards and exclusions individually per plan.
pub const MAX_CHECKS: usize = 32;
/// Maximum nesting of incoming JSON containers.
pub const MAX_JSON_DEPTH: usize = 64;
/// Maximum raw URL input and accepted base-metadata bytes; URL parsing scratch is bounded separately.
pub const MAX_URL_INPUT_BYTES: usize = 8 * 1024;
/// Maximum URL-processing scratch/result bytes before publication-value limits are applied.
pub const MAX_URL_PROCESSING_BYTES: usize = 32 * 1024;

/// Maximum complete compact data JSON bytes, excluding its framing LF.
pub const MAX_DATA_BYTES: usize = 64 * 1024 * 1024;
/// Maximum named fields in a record projection.
/// Maximum number of named record fields in one extraction contract.
pub const MAX_RECORD_FIELDS: usize = 64;

/// Maximum accepted source bytes under the supported preparation policy.
pub const MAX_SOURCE_BYTES: usize = 50 * 1024 * 1024;

/// Maximum following element siblings in one explicitly declared row group.
pub(crate) const MAX_FOLLOWING_SIBLINGS: u32 = 63;
