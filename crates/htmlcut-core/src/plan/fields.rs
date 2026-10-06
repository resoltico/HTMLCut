// SPDX-License-Identifier: MPL-2.0
//! Closed readings and row-relative fields.

use super::present;
use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{fmt, str::FromStr};

/// One supported parsed-DOM representation.
#[derive(Clone, Debug, Default, PartialEq, Eq, JsonSchema)]
#[schemars(schema_with = "reading_schema")]
pub enum Reading {
    /// Static structural text, retaining hidden content and parsed pre text.
    #[default]
    Text,
    /// Literal parsed descendant-text concatenation, including inert content.
    Literal,
    /// Selected-content Markdown with protected code and structural roles.
    Markdown,
    /// Markdown whose supported URLs use explicit base metadata.
    ResolvedMarkdown,
    /// Parsed descendants' HTML serialization.
    InnerHtml,
    /// Parsed HTML including the selected root.
    OuterHtml,
    /// Required attribute; a present empty value is a value.
    Attribute(String),
    /// Required supported URL attribute resolved against explicit base metadata.
    Url(String),
}
impl fmt::Display for Reading {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Text => f.write_str("text"),
            Self::Literal => f.write_str("literal"),
            Self::Markdown => f.write_str("markdown"),
            Self::ResolvedMarkdown => f.write_str("resolved-markdown"),
            Self::InnerHtml => f.write_str("inner-html"),
            Self::OuterHtml => f.write_str("outer-html"),
            Self::Attribute(name) => write!(f, "attr:{name}"),
            Self::Url(name) => write!(f, "url:{name}"),
        }
    }
}
impl FromStr for Reading {
    type Err = crate::ExtractionError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let read = match value {
            "text" => Self::Text,
            "literal" => Self::Literal,
            "markdown" => Self::Markdown,
            "resolved-markdown" => Self::ResolvedMarkdown,
            "inner-html" => Self::InnerHtml,
            "outer-html" => Self::OuterHtml,
            _ => {
                if let Some(name) = value.strip_prefix("attr:") {
                    Self::Attribute(name.into())
                } else if let Some(name) = value.strip_prefix("url:") {
                    Self::Url(name.into())
                } else {
                    return Err(super::validation::invalid());
                }
            }
        };
        super::validation::reading(&read, &[])?;
        Ok(read)
    }
}
impl Serialize for Reading {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}
impl<'de> Deserialize<'de> for Reading {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}
fn reading_schema(_: &mut SchemaGenerator) -> Schema {
    schemars::json_schema!({"type":"string", "pattern":"^(text|literal|markdown|resolved-markdown|inner-html|outer-html|attr:.+|url:(href|src|action|poster|cite|formaction|data))$"})
}

/// Cardinality within one selected row forest; optionality never hides a missing attribute.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum FieldMatch {
    /// Exactly one node.
    #[default]
    One,
    /// Zero or one node, returning null only for absence.
    Optional,
    /// Every matched node within declared count assumptions.
    All,
    /// One positive one-based position after complete counting.
    Nth,
}

/// One field projected within the selected original-DOM row forest.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RecordField {
    /// Required selector; context predicates still see the original DOM.
    pub select: String,
    /// Exactly one by default; optional, all and nth are explicit alternatives.
    #[serde(default, rename = "match")]
    pub match_mode: FieldMatch,
    /// Minimum for all, default one.
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    #[schemars(with = "u32")]
    pub min: Option<u32>,
    /// Optional maximum assumption for all.
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    #[schemars(with = "u32")]
    pub max: Option<u32>,
    /// Positive one-based nth position.
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    #[schemars(with = "u32")]
    pub index: Option<u32>,
    /// Scalar reading, default structural text.
    #[serde(default)]
    pub read: Reading,
    /// Excluded subtrees inside selected field nodes.
    #[serde(default)]
    pub exclude: Vec<String>,
}
impl RecordField {
    /// Validates the shared field-name contract without echoing invalid supplied names.
    pub fn validate_name(name: &str) -> Result<(), crate::ExtractionError> {
        if super::validation::field_name(name) {
            Ok(())
        } else {
            Err(super::validation::invalid())
        }
    }

    /// Constructs a required structural-text field.
    pub fn css(select: impl Into<String>) -> Self {
        Self {
            select: select.into(),
            match_mode: FieldMatch::One,
            min: None,
            max: None,
            index: None,
            read: Reading::Text,
            exclude: Vec::new(),
        }
    }
    pub(crate) fn selection(&self) -> super::Selection {
        match self.match_mode {
            FieldMatch::One | FieldMatch::Optional => super::Selection::Single,
            FieldMatch::All => super::Selection::All {
                min: self.min.unwrap_or(1),
                max: self.max,
            },
            FieldMatch::Nth => super::Selection::Nth {
                index: self.index.expect("validated field nth index"),
            },
        }
    }
}
