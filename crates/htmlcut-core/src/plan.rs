// SPDX-License-Identifier: MPL-2.0
//! The single closed, source-independent extraction language.

use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize, de::IntoDeserializer};

use crate::{ErrorCode, ExecutionLimits, ExtractionError, SCHEMA_VERSION};

mod fields;
pub(crate) mod validation;
pub use fields::{FieldSelection, RecordField, ValueProjection};

/// One source/DOM strategy, without implicit fragment reparsing.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Strategy {
    /// Select original DOM elements with CSS.
    Css {
        /// CSS selector grammar.
        selector: String,
    },
    /// Enumerate non-overlapping source boundary pairs.
    Slice {
        /// Opening boundary.
        start: Boundary,
        /// Closing boundary.
        end: Boundary,
        /// Include opening boundary bytes.
        #[serde(default)]
        include_start: bool,
        /// Include closing boundary bytes.
        #[serde(default)]
        include_end: bool,
    },
}

/// Literal or bounded regular-expression source boundary.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Boundary {
    /// Match exact UTF-8 bytes.
    Literal {
        /// Nonempty literal boundary.
        value: String,
    },
    /// Search with Rust regular-expression grammar.
    Regex {
        /// Nonempty pattern.
        pattern: String,
        /// Explicit flags from i, m, s, U and x.
        #[serde(default)]
        flags: String,
    },
}

/// Explicit cardinality or positional selection.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Selection {
    /// Exactly one candidate, the default.
    Single {},
    /// Select every candidate within declared bounds.
    All {
        /// Minimum candidate count, default one; zero explicitly permits empty results.
        #[serde(default = "minimum_one")]
        min: u32,
        /// Maximum candidate count, defaulting to the execution selected-value limit.
        #[serde(default)]
        max: Option<u32>,
    },
    /// One explicitly positional candidate; does not assert identity.
    Nth {
        /// Positive, one-based position.
        index: u32,
    },
}

impl Default for Selection {
    fn default() -> Self {
        Self::Single {}
    }
}

fn minimum_one() -> u32 {
    1
}

/// Exactly one requested representation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Projection {
    /// Named scalar fields projected relative to each selected original-DOM row.
    Records {
        /// Additional element-sibling subtrees in the declared row payload.
        #[serde(default)]
        #[schemars(range(max = crate::limits::MAX_FOLLOWING_SIBLINGS))]
        following_siblings: u32,
        /// Ordered fields; names must be unique and field count is bounded.
        fields: Vec<RecordField>,
    },
    /// Exact accepted source bytes; valid only for slicing.
    Source {},
    /// A scalar DOM representation shared with record fields.
    #[serde(untagged)]
    Value(ValueProjection),
}

impl Default for Projection {
    fn default() -> Self {
        Self::Value(ValueProjection::default())
    }
}

pub(crate) enum DomProjection<'a> {
    Value(&'a ValueProjection),
    Records(u32),
}

impl Projection {
    pub(crate) fn dom(&self) -> Result<DomProjection<'_>, ExtractionError> {
        match self {
            Self::Value(value) => Ok(DomProjection::Value(value)),
            Self::Records {
                following_siblings, ..
            } => Ok(DomProjection::Records(*following_siblings)),
            Self::Source {} => Err(ExtractionError::new(
                ErrorCode::InvalidPlan,
                "validation",
                "Source projection requires source slicing.",
            )),
        }
    }
}

/// Original-DOM scope of a declared guard.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GuardScope {
    /// Check the complete document.
    Document,
    /// Check each selected subtree independently.
    Selected,
}

/// Literal untransformed guard read.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum GuardRead {
    /// Concatenated original DOM text.
    DomText {},
    /// Required parsed attribute value.
    Attribute {
        /// Required attribute name.
        name: String,
    },
}

/// Predicate applied to every matched guard value.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Predicate {
    /// Exact whole-value comparison.
    Exact {
        /// Required exact literal value.
        value: String,
    },
    /// Search matching; anchors express a whole-value match.
    Regex {
        /// Bounded regular-expression grammar.
        pattern: String,
        /// Explicit flags from i, m, s, U and x.
        #[serde(default)]
        flags: String,
    },
}

/// One conjunctive, original-DOM expectation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Guard {
    /// Document or independently selected-root scope.
    pub scope: GuardScope,
    /// Original-DOM selector; :scope addresses a selected root.
    pub selector: String,
    /// Minimum guard matches, default one.
    #[serde(default = "minimum_one")]
    pub min: u32,
    /// Maximum guard matches, defaulting to the finite candidate limit.
    #[serde(default)]
    pub max: Option<u32>,
    /// Literal value to read, before exclusions and transformations.
    pub read: GuardRead,
    /// Required for dom_text; omission on attribute means presence only.
    #[serde(default)]
    pub predicate: Option<Predicate>,
}

/// Explicit value transforms; a projection permits at most one compatible operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Transform {
    /// Collapse source ASCII whitespace outside preformatted content.
    NormalizeWhitespace {},
    /// Resolve supported URL positions using explicit snapshot base metadata.
    ResolveUrls {},
}

/// One validated, fully defaulted extraction plan.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExtractionPlan {
    /// Closed plan role name.
    #[schemars(extend("const" = "htmlcut.extraction.plan"))]
    pub schema: String,
    /// Wire-family version.
    #[schemars(extend("const" = SCHEMA_VERSION))]
    pub version: u32,
    /// Source or DOM selection strategy.
    pub strategy: Strategy,
    /// Cardinality/position policy, default single.
    #[serde(default)]
    pub selection: Selection,
    /// Requested representation, default dom_text.
    #[serde(default)]
    pub projection: Projection,
    /// Explicit selectors removing nodes only inside selected subtrees.
    #[serde(default)]
    pub exclude: Vec<String>,
    /// Conjunctive original-DOM guards.
    #[serde(default)]
    pub guards: Vec<Guard>,
    /// Zero or one compatible value transform; record roots require an empty array.
    #[serde(default)]
    pub transforms: Vec<Transform>,
    /// Per-operation core limits, independent of adapter policy.
    #[serde(default)]
    pub limits: ExecutionLimits,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PlanFields {
    schema: String,
    version: u32,
    strategy: Strategy,
    #[serde(default)]
    selection: Selection,
    #[serde(default)]
    projection: Projection,
    #[serde(default)]
    exclude: Vec<String>,
    #[serde(default)]
    guards: Vec<Guard>,
    #[serde(default)]
    transforms: Vec<Transform>,
    #[serde(default)]
    limits: ExecutionLimits,
}

impl PlanFields {
    fn into_plan(self) -> ExtractionPlan {
        ExtractionPlan {
            schema: self.schema,
            version: self.version,
            strategy: self.strategy,
            selection: self.selection,
            projection: self.projection,
            exclude: self.exclude,
            guards: self.guards,
            transforms: self.transforms,
            limits: self.limits,
        }
    }
}

impl<'de> Deserialize<'de> for ExtractionPlan {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let closed = crate::json::ClosedValue::deserialize(deserializer)?;
        let fields: PlanFields = serde_json::from_value(closed.0)
            .map_err(|_| serde::de::Error::custom("Invalid extraction plan fields."))?;
        let plan = fields.into_plan();
        plan.validate().map_err(serde::de::Error::custom)?;
        Ok(plan)
    }
}

impl ExtractionPlan {
    /// Constructs a validated source-only single slice without compiling unrelated CSS grammar.
    pub fn slice(start: Boundary, end: Boundary) -> Result<Self, ExtractionError> {
        Self::for_strategy(
            Strategy::Slice {
                start,
                end,
                include_start: false,
                include_end: false,
            },
            Projection::Source {},
        )
    }
    /// Constructs a validated default single/dom_text plan over a CSS selector.
    pub fn css(selector: impl Into<String>) -> Result<Self, ExtractionError> {
        Self::for_strategy(
            Strategy::Css {
                selector: selector.into(),
            },
            Projection::default(),
        )
    }

    fn for_strategy(strategy: Strategy, projection: Projection) -> Result<Self, ExtractionError> {
        let plan = Self {
            schema: "htmlcut.extraction.plan".into(),
            version: SCHEMA_VERSION,
            strategy,
            selection: Selection::Single {},
            projection,
            exclude: Vec::new(),
            guards: Vec::new(),
            transforms: Vec::new(),
            limits: ExecutionLimits::default(),
        };
        plan.validate()?;
        Ok(plan)
    }

    /// Reads a size-bounded closed plan, rejecting recursive duplicate keys before map creation.
    pub fn from_json(bytes: &[u8]) -> Result<Self, ExtractionError> {
        if bytes.len() > crate::limits::MAX_PLAN_BYTES {
            return Err(ExtractionError::limit("plan"));
        }
        let value = crate::parse_closed_json(bytes, crate::MAX_PLAN_BYTES)?;
        if value.get("schema").and_then(serde_json::Value::as_str)
            != Some("htmlcut.extraction.plan")
            || value.get("version").and_then(serde_json::Value::as_u64)
                != Some(SCHEMA_VERSION as u64)
        {
            return Err(ExtractionError::new(
                ErrorCode::InvalidSchema,
                "plan",
                "Unsupported extraction plan schema or version.",
            )
            .with_cause(crate::FailureCause::Configuration {
                role: crate::ConfigurationRole::Plan,
                problem: crate::ConfigurationProblem::UnsupportedVersion,
            }));
        }
        let fields: PlanFields = serde_path_to_error::deserialize(value.into_deserializer())
            .map_err(|error| {
                let mut failure = ExtractionError::new(
                    ErrorCode::InvalidPlan,
                    "plan",
                    "The extraction plan contains invalid fields.",
                );
                failure.plan_path = Some(safe_plan_path(error.path()));
                failure
            })?;
        let plan = fields.into_plan();
        plan.validate().map_err(|error| {
            if error.code == ErrorCode::ResourceLimit {
                ExtractionError::limit("plan")
            } else {
                error
            }
        })?;
        Ok(plan)
    }

    pub(crate) fn normalized(&self) -> Result<Self, ExtractionError> {
        self.validate()?;
        let mut plan = self.clone();
        if let Selection::All { max, .. } = &mut plan.selection {
            *max = Some(max.unwrap_or(plan.limits.max_selected));
        }
        for guard in &mut plan.guards {
            guard.max = Some(guard.max.unwrap_or(plan.limits.max_candidates));
        }
        if let Projection::Records { fields, .. } = &mut plan.projection {
            for field in fields {
                if let FieldSelection::All { max, .. } = &mut field.selection {
                    *max = Some(max.unwrap_or(plan.limits.max_selected));
                }
            }
        }
        Ok(plan)
    }
}

fn safe_plan_path(path: &serde_path_to_error::Path) -> String {
    use serde_path_to_error::Segment;
    let mut safe = String::from("$");
    for segment in path {
        match segment {
            Segment::Seq { index } => safe.push_str(&format!("[{index}]")),
            Segment::Map { key }
                if matches!(
                    key.as_str(),
                    "schema"
                        | "version"
                        | "strategy"
                        | "kind"
                        | "selector"
                        | "start"
                        | "end"
                        | "include_start"
                        | "include_end"
                        | "value"
                        | "pattern"
                        | "flags"
                        | "selection"
                        | "index"
                        | "min"
                        | "max"
                        | "projection"
                        | "following_siblings"
                        | "fields"
                        | "name"
                        | "exclude"
                        | "guards"
                        | "transforms"
                        | "limits"
                        | "scope"
                        | "read"
                        | "predicate"
                        | "max_work"
                        | "max_candidates"
                        | "max_selected"
                        | "max_cells"
                        | "max_value_bytes"
                        | "max_total_value_bytes"
                ) =>
            {
                safe.push('.');
                safe.push_str(key);
            }
            _ => break,
        }
    }
    safe
}
