// SPDX-License-Identifier: MPL-2.0
//! Scalar representations and cardinality for fields of an original-DOM row.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{Transform, minimum_one};

/// A representation of one DOM node, with no nested records or source slicing.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ValueProjection {
    /// Literal concatenation of parsed descendant text.
    DomText {},
    /// Conventional Markdown over original DOM content and structural context.
    Markdown {},
    /// Parsed serialization of descendants.
    InnerHtml {},
    /// Parsed serialization including the selected root.
    OuterHtml {},
    /// Required parsed attribute; a present empty string is a value.
    Attribute {
        /// Attribute name, explicitly requested.
        name: String,
    },
}

impl Default for ValueProjection {
    fn default() -> Self {
        Self::DomText {}
    }
}

/// Explicit field cardinality; optionality never suppresses projection failures.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum FieldSelection {
    /// Exactly one node, returning one string.
    Single {},
    /// Zero or one node, returning null or one string.
    Optional {},
    /// All nodes within bounds, returning an array of strings.
    All {
        /// Minimum node count; zero must be explicit.
        #[serde(default = "minimum_one")]
        min: u32,
        /// Maximum node count; defaults to the plan's selected-value limit.
        #[serde(default)]
        max: Option<u32>,
    },
    /// A positive one-based positional node, returning one string.
    Nth {
        /// Position; does not establish identity.
        index: u32,
    },
}

impl Default for FieldSelection {
    fn default() -> Self {
        Self::Single {}
    }
}

/// One named scalar field projected from the selected row or its descendants.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RecordField {
    /// Unique ASCII identifier of at most sixty-four bytes.
    pub name: String,
    /// CSS selector in original DOM context, with payload restricted to the row.
    pub selector: String,
    /// Cardinality; defaults to exactly one.
    #[serde(default)]
    pub selection: FieldSelection,
    /// Scalar representation; defaults to literal text.
    #[serde(default)]
    pub projection: ValueProjection,
    /// Excluded subtrees inside selected field nodes only.
    #[serde(default)]
    pub exclude: Vec<String>,
    /// Zero or one compatible value transform.
    #[serde(default)]
    pub transforms: Vec<Transform>,
}
