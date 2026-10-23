// SPDX-License-Identifier: MPL-2.0
//! The contract's typed JSON surface and its derived authoring schema.
use crate::bounds::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fmt, str::FromStr};
/// Match cardinality.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Match {
    /// Exactly one match, or the requested nth match.
    #[default]
    One,
    /// Zero or one match.
    Optional,
    /// All matches within the declared range.
    All,
}
impl fmt::Display for Match {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::One => "one",
            Self::Optional => "optional",
            Self::All => "all",
        })
    }
}
impl FromStr for Match {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "one" => Ok(Self::One),
            "optional" => Ok(Self::Optional),
            "all" => Ok(Self::All),
            _ => Err("match must be one, optional or all.".into()),
        }
    }
}
/// Reading performed on a selected scope.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Reading {
    /// Structural normalized text.
    #[default]
    Text,
    /// Descendant text without layout normalization.
    TextContent,
    /// Original-tree HTML serialization.
    OuterHtml,
    /// Required attribute value.
    Attr(String),
    /// Required attribute resolved as a URL.
    Url(String),
}
impl fmt::Display for Reading {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Text => f.write_str("text"),
            Self::TextContent => f.write_str("text-content"),
            Self::OuterHtml => f.write_str("outer-html"),
            Self::Attr(s) => write!(f, "attr:{s}"),
            Self::Url(s) => write!(f, "url:{s}"),
        }
    }
}
impl FromStr for Reading {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        let valid = |name: &str| {
            !name.is_empty()
                && name.chars().count() <= MAX_ATTRIBUTE_NAME_CHARS
                && !name
                    .chars()
                    .any(|c| c.is_whitespace() || matches!(c, '"' | '\'' | '<' | '>' | '/' | '='))
        };
        match s {
            "text" => Ok(Self::Text),
            "text-content" => Ok(Self::TextContent),
            "outer-html" => Ok(Self::OuterHtml),
            s if s.strip_prefix("attr:").is_some_and(valid) => Ok(Self::Attr(s[5..].into())),
            s if s.strip_prefix("url:").is_some_and(valid) => Ok(Self::Url(s[4..].into())),
            _ => Err(format!(
                "Unknown or invalid reading {s:?}. Readings are text, text-content, outer-html, attr:NAME and url:NAME."
            )),
        }
    }
}
impl Serialize for Reading {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}
impl<'de> Deserialize<'de> for Reading {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        String::deserialize(d)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}
impl JsonSchema for Reading {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "Reading".into()
    }
    fn json_schema(g: &mut schemars::SchemaGenerator) -> schemars::Schema {
        let mut schema = String::json_schema(g);
        schema.insert("pattern".into(), reading_pattern(false).into());
        schema
    }
}
fn reading_pattern(predicate: bool) -> String {
    let attr = format!("attr:[^\\s\"'<>/=]{{1,{MAX_ATTRIBUTE_NAME_CHARS}}}");
    if predicate {
        format!("^(text|text-content|{attr})(?![\\s\\S])")
    } else {
        format!(
            "^(text|text-content|outer-html|{attr}|url:[^\\s\"'<>/=]{{1,{MAX_ATTRIBUTE_NAME_CHARS}}})(?![\\s\\S])"
        )
    }
}
fn present<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(
    d: D,
) -> Result<Option<T>, D::Error> {
    T::deserialize(d).map(Some)
}
/// A full contract node.
#[derive(Clone, Debug, Default, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NodeObject {
    /// CSS selector list; omitted means the inherited anchor.
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    #[schemars(with="String" ,length(max=MAX_PATTERN_BYTES))]
    pub select: Option<String>,
    /// Predicates that filter candidates; requires select.
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    #[schemars(with="Vec<PredicateSpec>" ,length(max=MAX_ENTRIES))]
    pub r#where: Option<Vec<PredicateSpec>>,
    /// Accepted cardinality; defaults to one.
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    #[schemars(with = "Match")]
    pub r#match: Option<Match>,
    /// Inclusive minimum count for all matches.
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    #[schemars(with="u32" ,range(min=0,max=MAX_DECLARED_COUNT))]
    pub min: Option<u32>,
    /// Inclusive maximum count for all matches.
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    #[schemars(with="u32" ,range(min=0,max=MAX_DECLARED_COUNT))]
    pub max: Option<u32>,
    /// One-based match index; requires select and match one.
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    #[schemars(with="u32" ,range(min=1,max=MAX_DECLARED_COUNT))]
    pub nth: Option<u32>,
    /// Sibling boundary selector.
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    #[schemars(with="String" ,length(max=MAX_PATTERN_BYTES))]
    pub until: Option<String>,
    /// Record fields in name order.
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    #[schemars(with="BTreeMap<String,NodeSpec>" ,length(min=1,max=MAX_RECORD_FIELDS))]
    pub fields: Option<BTreeMap<String, NodeSpec>>,
    /// Value reading; defaults to text.
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    #[schemars(with = "Reading")]
    pub read: Option<Reading>,
    /// Structural reading exclusions.
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    #[schemars(with="Vec<String>" ,length(max=MAX_ENTRIES))]
    pub exclude: Option<Vec<String>>,
    /// Exact value check, exclusive with pattern.
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    #[schemars(with = "String")]
    pub equals: Option<String>,
    /// Regular expression check and optional named value capture.
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    #[schemars(with="String" ,length(max=MAX_PATTERN_BYTES))]
    pub pattern: Option<String>,
    /// Assertions in the inherited scope.
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    #[schemars(with="Vec<PredicateSpec>" ,length(max=MAX_ENTRIES))]
    pub expect: Option<Vec<PredicateSpec>>,
}
/// A predicate with count and optional reading checks.
#[derive(Clone, Debug, Default, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PredicateObject {
    /// CSS selector list; omitted means the inherited anchor.
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    #[schemars(with="String" ,length(max=MAX_PATTERN_BYTES))]
    pub select: Option<String>,
    /// Inclusive minimum count for all matches.
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    #[schemars(with="u32" ,range(min=0,max=MAX_DECLARED_COUNT))]
    pub min: Option<u32>,
    /// Inclusive maximum count for all matches.
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    #[schemars(with="u32" ,range(min=0,max=MAX_DECLARED_COUNT))]
    pub max: Option<u32>,
    /// Value reading; defaults to text.
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    #[schemars(with = "Reading")]
    pub read: Option<Reading>,
    /// Exact value check, exclusive with pattern.
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    #[schemars(with = "String")]
    pub equals: Option<String>,
    /// Regular expression check and optional named value capture.
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    #[schemars(with="String" ,length(max=MAX_PATTERN_BYTES))]
    pub pattern: Option<String>,
}
/// A selector shorthand or a full object.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(untagged)]
pub enum NodeSpec {
    /// Selector shorthand, equivalent to an object with only select.
    Selector(#[schemars(length(max=MAX_PATTERN_BYTES))] String),
    /// Full object form.
    Node(Box<NodeObject>),
}
impl<'de> Deserialize<'de> for NodeSpec {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = NodeSpec;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a selector string or a node object")
            }
            fn visit_str<E: serde::de::Error>(self, s: &str) -> Result<Self::Value, E> {
                Ok(NodeSpec::Selector(s.into()))
            }
            fn visit_map<M: serde::de::MapAccess<'de>>(
                self,
                m: M,
            ) -> Result<Self::Value, M::Error> {
                NodeObject::deserialize(serde::de::value::MapAccessDeserializer::new(m))
                    .map(|o| NodeSpec::Node(Box::new(o)))
            }
        }
        d.deserialize_any(Visitor)
    }
}
/// A selector shorthand or a full object.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(untagged)]
pub enum PredicateSpec {
    /// Selector shorthand, equivalent to an object with only select.
    Selector(#[schemars(length(max=MAX_PATTERN_BYTES))] String),
    /// Full object form.
    Predicate(Box<PredicateObject>),
}
impl<'de> Deserialize<'de> for PredicateSpec {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = PredicateSpec;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a selector string or a predicate object")
            }
            fn visit_str<E: serde::de::Error>(self, s: &str) -> Result<Self::Value, E> {
                Ok(PredicateSpec::Selector(s.into()))
            }
            fn visit_map<M: serde::de::MapAccess<'de>>(
                self,
                m: M,
            ) -> Result<Self::Value, M::Error> {
                PredicateObject::deserialize(serde::de::value::MapAccessDeserializer::new(m))
                    .map(|o| PredicateSpec::Predicate(Box::new(o)))
            }
        }
        d.deserialize_any(Visitor)
    }
}
/// Contract member and reading reference used by long CLI help.
pub const CONTRACT_SUMMARY: &str = "Node members: select where match min max nth until fields read exclude equals pattern expect\nPredicate members: select read equals pattern min max\nReadings: text text-content outer-html attr:NAME url:NAME";
pub(crate) fn schema() -> serde_json::Value {
    let mut schema =
        serde_json::to_value(schemars::schema_for!(NodeSpec)).expect("schema is serializable");
    let node = &mut schema["$defs"]["NodeObject"];
    node["dependentRequired"] =
        serde_json::json!({"where":["select"],"until":["select"],"nth":["select"]});
    node["not"] = serde_json::json!({"required":["equals","pattern"]});
    node["allOf"] = serde_json::json!([
        {"if":{"required":["read"],"properties":{"read":{"pattern":"^(attr:|url:)"}}},"then":{"not":{"required":["exclude"]}}},
        {"if":{"required":["fields"]},"then":{"not":{"anyOf":[{"required":["read"]},{"required":["exclude"]},{"required":["equals"]},{"required":["pattern"]}]}}},
        {"if":{"anyOf":[{"required":["min"]},{"required":["max"]}]},"then":{"required":["match"],"properties":{"match":{"const":"all"}}}},
        {"if":{"required":["nth"]},"then":{"properties":{"match":{"const":"one"}}}}
    ]);
    node["properties"]["fields"]["minProperties"] = 1.into();
    node["properties"]["fields"]["maxProperties"] = MAX_RECORD_FIELDS.into();
    node["properties"]["fields"]["propertyNames"] = serde_json::json!({"pattern":format!("^[A-Za-z0-9_-]{{1,{MAX_FIELD_NAME_CHARS}}}(?![\\s\\S])")});
    node["properties"]["exclude"]["items"]["maxLength"] = MAX_PATTERN_BYTES.into();
    let predicate = &mut schema["$defs"]["PredicateObject"];
    predicate["not"] = serde_json::json!({"required":["equals","pattern"]});
    predicate["properties"]["read"]["pattern"] = reading_pattern(true).into();
    schema
}

#[cfg(test)]
mod typed_admission_tests {
    use super::*;
    #[test]
    fn programmatic_readings_obey_the_same_attribute_bounds() {
        let stop = crate::Stop::none();
        for predicate in [false, true] {
            let mut object = NodeObject::default();
            if predicate {
                object.expect = Some(vec![PredicateSpec::Predicate(Box::new(PredicateObject {
                    read: Some(Reading::Attr("bad name".into())),
                    ..Default::default()
                }))]);
            } else {
                object.read = Some(Reading::Attr("bad name".into()));
            }
            let error = crate::Contract::compile(&NodeSpec::Node(Box::new(object)), &stop)
                .err()
                .unwrap();
            assert!(matches!(error, crate::Error::InvalidContract { .. }));
        }
    }
}
