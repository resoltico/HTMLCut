// SPDX-License-Identifier: MPL-2.0
//! The closed, source-independent snapshot query contract.

use crate::{ErrorCode, ExecutionLimits, ExtractionError, SCHEMA_VERSION};
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize, de::IntoDeserializer};
use std::collections::BTreeMap;

mod fields;
pub(crate) mod validation;
pub use fields::{FieldMatch, Reading, RecordField};

/// Cardinality of a selected set of original DOM nodes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Match {
    /// Exactly one node.
    #[default]
    One,
    /// Every node within declared bounds.
    All,
    /// One positive, one-based position; candidates are still counted completely.
    Nth,
}

/// Original DOM scope of an expectation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum GuardScope {
    /// The entire original document, even when root selection is empty.
    #[default]
    Document,
    /// Each selected original subtree or record forest.
    Selected,
}

fn one() -> u32 {
    1
}

// Omission requests defaults. A supplied null is never an omitted member.
pub(super) fn present<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}

/// One conjunctive original-DOM expectation, evaluated before exclusions.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Guard {
    /// Required CSS selector.
    pub select: String,
    /// Document by default, or each selected subtree/forest.
    #[serde(default)]
    pub scope: GuardScope,
    /// Minimum matches, default one.
    #[serde(default = "one")]
    pub min: u32,
    /// Maximum matches, default one.
    #[serde(default = "one")]
    pub max: u32,
    /// Predicate reading; absent for count-only expectations.
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    #[schemars(with = "Reading")]
    pub read: Option<Reading>,
    /// Whole-value equality, mutually exclusive with pattern.
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    #[schemars(with = "String")]
    pub equals: Option<String>,
    /// Bounded Rust regex search; inline flags and anchors express matching policy.
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    #[schemars(with = "String")]
    pub pattern: Option<String>,
}

/// A compact, closed query over an immutable HTML snapshot.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExtractionPlan {
    /// Required current wire version.
    #[schemars(extend("const" = SCHEMA_VERSION))]
    pub version: u32,
    /// Required nonempty CSS selector.
    pub select: String,
    /// Root cardinality, default one; optional is a field-only mode.
    #[serde(default, rename = "match")]
    pub match_mode: Match,
    /// All minimum, default one; rejected for other modes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "u32")]
    pub min: Option<u32>,
    /// Optional declared all maximum, retained as an assumption.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "u32")]
    pub max: Option<u32>,
    /// Required positive position for nth; rejected otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "u32")]
    pub index: Option<u32>,
    /// Scalar reading, default text; rejected for records.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Reading")]
    pub read: Option<Reading>,
    /// Descendant exclusions for scalar reading; rejected for records.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Vec<String>")]
    pub exclude: Option<Vec<String>>,
    /// One to sixty-four fields, processed in ASCII lexical name order.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "BTreeMap<String, RecordField>")]
    pub fields: Option<BTreeMap<String, RecordField>>,
    /// Conjunctive count or value expectations.
    #[serde(default)]
    pub expect: Vec<Guard>,
    /// Additional element siblings in a record forest; default zero.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "u32")]
    pub following_siblings: Option<u32>,
    /// Fresh per-execution limits; partial JSON fills the maintained defaults.
    #[serde(default)]
    pub limits: ExecutionLimits,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PlanFields {
    version: u32,
    select: String,
    #[serde(default, rename = "match")]
    match_mode: Match,
    #[serde(default, deserialize_with = "present")]
    min: Option<u32>,
    #[serde(default, deserialize_with = "present")]
    max: Option<u32>,
    #[serde(default, deserialize_with = "present")]
    index: Option<u32>,
    #[serde(default, deserialize_with = "present")]
    read: Option<Reading>,
    #[serde(default, deserialize_with = "present")]
    exclude: Option<Vec<String>>,
    #[serde(default, deserialize_with = "present")]
    fields: Option<BTreeMap<String, RecordField>>,
    #[serde(default)]
    expect: Vec<Guard>,
    #[serde(default, deserialize_with = "present")]
    following_siblings: Option<u32>,
    #[serde(default)]
    limits: ExecutionLimits,
}
impl PlanFields {
    fn into_plan(self) -> ExtractionPlan {
        ExtractionPlan {
            version: self.version,
            select: self.select,
            match_mode: self.match_mode,
            min: self.min,
            max: self.max,
            index: self.index,
            read: self.read,
            exclude: self.exclude,
            fields: self.fields,
            expect: self.expect,
            following_siblings: self.following_siblings,
            limits: self.limits,
        }
    }
}
impl<'de> Deserialize<'de> for ExtractionPlan {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let closed = crate::json::ClosedValue::deserialize(deserializer)?;
        Self::from_value(closed.0).map_err(serde::de::Error::custom)
    }
}
impl ExtractionPlan {
    /// Constructs a default exactly-one structural-text query.
    pub fn css(select: impl Into<String>) -> Result<Self, ExtractionError> {
        let plan = Self {
            version: SCHEMA_VERSION,
            select: select.into(),
            match_mode: Match::One,
            min: None,
            max: None,
            index: None,
            read: None,
            exclude: None,
            fields: None,
            expect: Vec::new(),
            following_siblings: None,
            limits: ExecutionLimits::default(),
        };
        plan.validate()?;
        Ok(plan)
    }
    /// Parses bounded JSON, rejecting all duplicate keys before map conversion.
    pub fn from_json(bytes: &[u8]) -> Result<Self, ExtractionError> {
        Self::from_value(crate::parse_closed_json(bytes, crate::MAX_PLAN_BYTES)?)
    }
    fn from_value(mut value: serde_json::Value) -> Result<Self, ExtractionError> {
        let failure = |code, path: &str, problem, message| {
            let mut error = ExtractionError::new(code, "plan", message).with_cause(
                crate::FailureCause::Configuration {
                    role: crate::ConfigurationRole::Plan,
                    problem,
                },
            );
            error.plan_path = Some(path.into());
            error
        };
        let object = value.as_object().ok_or_else(|| {
            failure(
                ErrorCode::InvalidPlan,
                "$",
                crate::ConfigurationProblem::InvalidValue,
                "The query must be a JSON object.",
            )
        })?;
        let version = object.get("version").ok_or_else(|| {
            failure(
                ErrorCode::InvalidSchema,
                "$.version",
                crate::ConfigurationProblem::MissingRequired,
                "The query requires a version member.",
            )
        })?;
        if !version.is_i64() && !version.is_u64() {
            return Err(failure(
                ErrorCode::InvalidSchema,
                "$.version",
                crate::ConfigurationProblem::InvalidValue,
                "The query version must be an integer.",
            ));
        }
        if version.as_u64() != Some(SCHEMA_VERSION.into()) {
            return Err(failure(
                ErrorCode::InvalidSchema,
                "$.version",
                crate::ConfigurationProblem::UnsupportedVersion,
                "Unsupported query version.",
            ));
        }
        let select = object.get("select").ok_or_else(|| {
            failure(
                ErrorCode::InvalidPlan,
                "$.select",
                crate::ConfigurationProblem::MissingRequired,
                "The query requires a select member.",
            )
        })?;
        if !select.is_string() {
            return Err(failure(
                ErrorCode::InvalidPlan,
                "$.select",
                crate::ConfigurationProblem::InvalidValue,
                "The query select member must be a string.",
            ));
        }
        // Sort incoming objects before field deserialization too, including under
        // downstream preserve_order unification; malformed fields fail lexically.
        value.sort_all_objects();
        let fields: PlanFields = serde_path_to_error::deserialize(value.into_deserializer())
            .map_err(|error| {
                let mut failure = ExtractionError::new(
                    ErrorCode::InvalidPlan,
                    "plan",
                    "The query contains invalid members.",
                );
                failure.plan_path = Some(safe_plan_path(error.path()));
                failure.field_name = safe_field_name(error.path());
                failure
            })?;
        let plan = fields.into_plan();
        plan.validate()?;
        Ok(plan)
    }
    pub(crate) fn normalized(&self) -> Result<Self, ExtractionError> {
        self.validate()?;
        let mut plan = self.clone();
        if plan.match_mode == Match::All {
            plan.min = Some(plan.min.unwrap_or(1));
        }
        if let Some(fields) = &mut plan.fields {
            plan.following_siblings = Some(plan.following_siblings.unwrap_or(0));
            for field in fields.values_mut() {
                if field.match_mode == FieldMatch::All {
                    field.min = Some(field.min.unwrap_or(1));
                }
            }
        } else {
            plan.read = Some(plan.read.unwrap_or_default());
            plan.exclude = Some(plan.exclude.unwrap_or_default());
        }
        for guard in &mut plan.expect {
            if guard.equals.is_some() || guard.pattern.is_some() {
                guard.read = Some(guard.read.clone().unwrap_or_default());
            }
        }
        Ok(plan)
    }
    pub(crate) fn selection(&self) -> Selection {
        selection(self.match_mode, self.min, self.max, self.index)
    }
}

pub(crate) enum Selection {
    Single,
    All { min: u32, max: Option<u32> },
    Nth { index: u32 },
}
pub(super) fn selection(
    mode: Match,
    min: Option<u32>,
    max: Option<u32>,
    index: Option<u32>,
) -> Selection {
    match mode {
        Match::One => Selection::Single,
        Match::All => Selection::All {
            min: min.unwrap_or(1),
            max,
        },
        Match::Nth => Selection::Nth {
            index: index.expect("validated nth index"),
        },
    }
}
fn safe_plan_path(path: &serde_path_to_error::Path) -> String {
    use serde_path_to_error::Segment;
    let mut safe = String::from("$");
    let mut field_key = false;
    for segment in path {
        match segment {
            Segment::Seq { index } => safe.push_str(&format!("[{index}]")),
            Segment::Map { key } if field_key && validation::field_name(key) => {
                safe.push('.');
                safe.push_str(key);
                field_key = false;
            }
            Segment::Map { key }
                if matches!(
                    key.as_str(),
                    "version"
                        | "select"
                        | "match"
                        | "min"
                        | "max"
                        | "index"
                        | "read"
                        | "exclude"
                        | "fields"
                        | "expect"
                        | "following_siblings"
                        | "limits"
                        | "scope"
                        | "equals"
                        | "pattern"
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
                field_key = key == "fields";
            }
            _ => break,
        }
    }
    safe
}

fn safe_field_name(path: &serde_path_to_error::Path) -> Option<String> {
    let mut segments = path.iter();
    if matches!(segments.next(),Some(serde_path_to_error::Segment::Map {key}) if key == "fields")
        && let Some(serde_path_to_error::Segment::Map { key }) = segments.next()
        && validation::field_name(key)
    {
        return Some(key.clone());
    }
    None
}
