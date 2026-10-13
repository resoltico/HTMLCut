// SPDX-License-Identifier: MPL-2.0
//! Closed recursive contracts, validated and compiled before document evaluation.
use crate::limits::{
    MAX_ATTRIBUTE_NAME_CHARS, MAX_CONTRACT_BYTES, MAX_CONTRACT_NODES, MAX_DECLARED_COUNT,
    MAX_ENTRIES, MAX_FIELD_NAME_CHARS, MAX_PATTERN_BYTES, MAX_RECORD_DEPTH, MAX_RECORD_FIELDS,
};
use crate::{Error, ErrorCode, select::Selector};
use regex::{Regex, RegexBuilder};
use serde_json::{Map, Value as Json};
use std::collections::BTreeMap;

/// A compiled extraction contract, reusable across documents.
pub struct Contract {
    pub(crate) node: Node,
}
pub(crate) struct Node {
    pub(crate) path: String,
    pub(crate) select: Option<Selector>,
    pub(crate) predicates: Vec<Predicate>,
    pub(crate) expect: Vec<Predicate>,
    pub(crate) cardinality: Cardinality,
    pub(crate) until: Option<Selector>,
    pub(crate) body: Body,
}
pub(crate) enum Cardinality {
    One(Option<usize>),
    Optional,
    All { min: usize, max: Option<usize> },
}
pub(crate) enum Body {
    Record(BTreeMap<String, Node>),
    Value {
        read: Reading,
        exclude: Vec<Selector>,
        check: Check,
    },
}
pub(crate) struct Predicate {
    pub(crate) path: String,
    pub(crate) select: Option<Selector>,
    pub(crate) read: Reading,
    pub(crate) check: Check,
    pub(crate) min: usize,
    pub(crate) max: usize,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Reading {
    Text,
    TextContent,
    OuterHtml,
    Attr(String),
    Url(String),
}
pub(crate) struct Check {
    pub(crate) equals: Option<String>,
    pub(crate) pattern: Option<Regex>,
}

impl Contract {
    /// Reads one bounded JSON contract, rejecting duplicates and compiling its grammar.
    pub fn from_json(text: &str) -> Result<Self, Error> {
        if text.len() > MAX_CONTRACT_BYTES {
            return Err(invalid(
                None,
                &format!("The contract exceeds the {MAX_CONTRACT_BYTES}-byte grammar bound."),
            ));
        }
        let value = crate::json::read(text.as_bytes(), MAX_CONTRACT_BYTES).map_err(|e| {
            invalid(
                None,
                &format!("The contract is not valid closed JSON: {e}."),
            )
        })?;
        let patterns = pattern_count(&value);
        let mut compiler = Compiler {
            nodes: 0,
            regex_bytes: crate::limits::MAX_REGEX_BYTES / (2 * patterns.max(1)),
        };
        Ok(Self {
            node: compiler.node(&value, "$", 0)?,
        })
    }
    /// JSON Schema for the closed contract surface; byte and compiled bounds remain engine checks.
    pub fn schema() -> Json {
        schema()
    }
    /// Whether the root produces records, which cannot be emitted as raw strings.
    pub fn produces_records(&self) -> bool {
        matches!(self.node.body, Body::Record(_))
    }
}

fn invalid(path: Option<&str>, message: &str) -> Error {
    let e = Error::new(ErrorCode::InvalidContract, message);
    match path {
        Some(path) => e.at(path),
        None => e,
    }
}
fn pattern_count(value: &Json) -> usize {
    let Some(map) = value.as_object() else {
        return 0;
    };
    let predicates = ["where", "expect"]
        .into_iter()
        .filter_map(|key| map.get(key).and_then(Json::as_array))
        .flatten()
        .filter(|v| v.as_object().is_some_and(|m| m.contains_key("pattern")))
        .count();
    let children = map
        .get("fields")
        .and_then(Json::as_object)
        .map(|fields| fields.values().map(pattern_count).sum::<usize>())
        .unwrap_or(0);
    usize::from(map.contains_key("pattern")) + predicates + children
}

fn object(value: &Json, path: &str, allowed: &[&str]) -> Result<Map<String, Json>, Error> {
    let map = if let Some(text) = value.as_str() {
        Map::from_iter([(String::from("select"), Json::String(text.into()))])
    } else {
        value.as_object().cloned().ok_or_else(|| {
            invalid(
                Some(path),
                "A contract node or predicate must be a selector string or an object.",
            )
        })?
    };
    for (key, value) in &map {
        if !allowed.contains(&key.as_str()) {
            return Err(invalid(
                Some(&format!("{path}.{key}")),
                &format!("Unknown contract member {key:?}."),
            ));
        }
        if value.is_null() {
            return Err(invalid(
                Some(&format!("{path}.{key}")),
                "JSON null cannot replace an omitted member.",
            ));
        }
    }
    Ok(map)
}
fn string(map: &Map<String, Json>, name: &str, path: &str) -> Result<Option<String>, Error> {
    map.get(name)
        .map(|v| {
            v.as_str().map(String::from).ok_or_else(|| {
                invalid(
                    Some(&format!("{path}.{name}")),
                    &format!("The {name} member must be a string."),
                )
            })
        })
        .transpose()
}
fn integer(map: &Map<String, Json>, name: &str, path: &str) -> Result<Option<usize>, Error> {
    map.get(name)
        .map(|v| {
            v.as_u64()
                .filter(|n| *n <= MAX_DECLARED_COUNT as u64)
                .map(|n| n as usize)
                .ok_or_else(|| {
                    invalid(
                        Some(&format!("{path}.{name}")),
                        &format!(
                            "The {name} member must be an integer from 0 to {MAX_DECLARED_COUNT}."
                        ),
                    )
                })
        })
        .transpose()
}
fn array<'a>(map: &'a Map<String, Json>, name: &str, path: &str) -> Result<&'a [Json], Error> {
    match map.get(name) {
        None => Ok(&[]),
        Some(Json::Array(v)) if v.len() <= MAX_ENTRIES => Ok(v),
        _ => Err(invalid(
            Some(&format!("{path}.{name}")),
            &format!("The {name} member must be an array with at most {MAX_ENTRIES} entries."),
        )),
    }
}
fn selector(map: &Map<String, Json>, name: &str, path: &str) -> Result<Option<Selector>, Error> {
    string(map, name, path)?
        .map(|s| Selector::compile(&s, &format!("{path}.{name}")))
        .transpose()
}
fn reading(map: &Map<String, Json>, path: &str, predicate: bool) -> Result<Reading, Error> {
    let text = string(map, "read", path)?.unwrap_or_else(|| "text".into());
    let valid_name = |name: &str| {
        !name.is_empty()
            && name.chars().count() <= MAX_ATTRIBUTE_NAME_CHARS
            && !name
                .chars()
                .any(|c| c.is_whitespace() || matches!(c, '"' | '\'' | '<' | '>' | '/' | '='))
    };
    match text.as_str() {
        "text" => Ok(Reading::Text),
        "text-content" => Ok(Reading::TextContent),
        "outer-html" if !predicate => Ok(Reading::OuterHtml),
        s if s.strip_prefix("attr:").is_some_and(valid_name) => Ok(Reading::Attr(s[5..].into())),
        s if !predicate && s.strip_prefix("url:").is_some_and(valid_name) => {
            Ok(Reading::Url(s[4..].into()))
        }
        _ => Err(invalid(
            Some(&format!("{path}.read")),
            &format!(
                "Unknown or invalid reading {text:?}. {}",
                if predicate {
                    "A predicate reads text, text-content or attr:NAME."
                } else {
                    "Readings are text, text-content, outer-html, attr:NAME and url:NAME."
                }
            ),
        )),
    }
}

struct Compiler {
    nodes: usize,
    regex_bytes: usize,
}
impl Compiler {
    fn check(&self, map: &Map<String, Json>, path: &str) -> Result<Check, Error> {
        let equals = string(map, "equals", path)?;
        let text = string(map, "pattern", path)?;
        if equals.is_some() && text.is_some() {
            return Err(invalid(
                Some(path),
                "equals and pattern are mutually exclusive.",
            ));
        }
        let pattern = text.map(|text| {
            let location = format!("{path}.pattern");
            if text.len() > MAX_PATTERN_BYTES { return Err(invalid(Some(&location), &format!("The pattern exceeds the {MAX_PATTERN_BYTES}-byte grammar bound."))); }
            RegexBuilder::new(&text).size_limit(self.regex_bytes).dfa_size_limit(self.regex_bytes).nest_limit(64)
                .build().map_err(|e| Error::new(ErrorCode::InvalidPattern,
                    format!("Invalid regular expression or exceeded compiled-size bound {}: {e}.", self.regex_bytes)).at(&location))
        }).transpose()?;
        Ok(Check { equals, pattern })
    }
    fn predicates(
        &mut self,
        map: &Map<String, Json>,
        name: &str,
        path: &str,
    ) -> Result<Vec<Predicate>, Error> {
        array(map, name, path)?
            .iter()
            .enumerate()
            .map(|(i, v)| self.predicate(v, &format!("{path}.{name}[{i}]")))
            .collect()
    }
    fn predicate(&mut self, value: &Json, path: &str) -> Result<Predicate, Error> {
        let map = object(
            value,
            path,
            &["select", "read", "equals", "pattern", "min", "max"],
        )?;
        let min = integer(&map, "min", path)?.unwrap_or(1);
        let max = integer(&map, "max", path)?.unwrap_or(1);
        if min > max {
            return Err(invalid(
                Some(path),
                "The predicate minimum exceeds its maximum.",
            ));
        }
        Ok(Predicate {
            path: path.into(),
            select: selector(&map, "select", path)?,
            read: reading(&map, path, true)?,
            check: self.check(&map, path)?,
            min,
            max,
        })
    }
    fn node(&mut self, value: &Json, path: &str, records: usize) -> Result<Node, Error> {
        self.nodes += 1;
        if self.nodes > MAX_CONTRACT_NODES {
            return Err(invalid(
                Some(path),
                &format!("The contract exceeds the {MAX_CONTRACT_NODES}-node grammar bound."),
            ));
        }
        let map = object(
            value,
            path,
            &[
                "select", "where", "match", "min", "max", "nth", "until", "fields", "read",
                "exclude", "equals", "pattern", "expect",
            ],
        )?;
        let select = selector(&map, "select", path)?;
        let until = selector(&map, "until", path)?;
        let nth = integer(&map, "nth", path)?;
        if (map.contains_key("where") || until.is_some() || nth.is_some()) && select.is_none() {
            return Err(invalid(Some(path), "where, until and nth require select."));
        }
        if nth == Some(0) {
            return Err(invalid(
                Some(&format!("{path}.nth")),
                "nth must be at least 1.",
            ));
        }
        let mode = string(&map, "match", path)?.unwrap_or_else(|| "one".into());
        if mode != "all" && (map.contains_key("min") || map.contains_key("max")) {
            return Err(invalid(Some(path), "min and max require match all."));
        }
        if nth.is_some() && mode != "one" {
            return Err(invalid(Some(path), "nth requires match one."));
        }
        let cardinality = match mode.as_str() {
            "one" => Cardinality::One(nth),
            "optional" => Cardinality::Optional,
            "all" => {
                let min = integer(&map, "min", path)?.unwrap_or(1);
                let max = integer(&map, "max", path)?;
                if max.is_some_and(|n| n < min) {
                    return Err(invalid(Some(path), "The minimum exceeds the maximum."));
                }
                Cardinality::All { min, max }
            }
            _ => {
                return Err(invalid(
                    Some(&format!("{path}.match")),
                    "match must be one, optional or all.",
                ));
            }
        };
        let expect = self.predicates(&map, "expect", path)?;
        let predicates = self.predicates(&map, "where", path)?;
        let body = if let Some(fields) = map.get("fields") {
            for name in ["read", "exclude", "equals", "pattern"] {
                if map.contains_key(name) {
                    return Err(invalid(
                        Some(&format!("{path}.{name}")),
                        &format!("{name} cannot be combined with fields."),
                    ));
                }
            }
            if records >= MAX_RECORD_DEPTH {
                return Err(invalid(
                    Some(path),
                    &format!("Records exceed the nesting bound of {MAX_RECORD_DEPTH}."),
                ));
            }
            let fields = fields.as_object().ok_or_else(|| {
                invalid(Some(&format!("{path}.fields")), "fields must be an object.")
            })?;
            if fields.is_empty() {
                return Err(invalid(
                    Some(&format!("{path}.fields")),
                    "A record must have at least one field.",
                ));
            }
            if fields.len() > MAX_RECORD_FIELDS {
                return Err(invalid(
                    Some(&format!("{path}.fields")),
                    &format!("A record exceeds the {MAX_RECORD_FIELDS}-field grammar bound."),
                ));
            }
            let mut compiled = BTreeMap::new();
            for (name, field) in fields {
                if name.is_empty()
                    || name.len() > MAX_FIELD_NAME_CHARS
                    || !name
                        .bytes()
                        .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
                {
                    return Err(invalid(
                        Some(&format!("{path}.fields")),
                        &format!(
                            "Field name {name:?} must be 1–{MAX_FIELD_NAME_CHARS} ASCII letters, digits, underscores or hyphens."
                        ),
                    ));
                }
                compiled.insert(
                    name.clone(),
                    self.node(field, &format!("{path}.fields.{name}"), records + 1)?,
                );
            }
            Body::Record(compiled)
        } else {
            let read = reading(&map, path, false)?;
            if map.contains_key("exclude") && matches!(read, Reading::Attr(_) | Reading::Url(_)) {
                return Err(invalid(
                    Some(&format!("{path}.exclude")),
                    "exclude cannot be combined with an attribute or URL reading.",
                ));
            }
            let exclude = array(&map, "exclude", path)?
                .iter()
                .enumerate()
                .map(|(i, v)| {
                    let location = format!("{path}.exclude[{i}]");
                    let text = v.as_str().ok_or_else(|| {
                        invalid(Some(&location), "An exclusion must be a selector string.")
                    })?;
                    Selector::compile(text, &location)
                })
                .collect::<Result<Vec<_>, _>>()?;
            Body::Value {
                read,
                exclude,
                check: self.check(&map, path)?,
            }
        };
        Ok(Node {
            path: path.into(),
            select,
            predicates,
            expect,
            cardinality,
            until,
            body,
        })
    }
}

fn schema() -> Json {
    let selector = serde_json::json!({"type":"string","maxLength":MAX_PATTERN_BYTES});
    let count = serde_json::json!({"type":"integer","minimum":0,"maximum":MAX_DECLARED_COUNT});
    let pattern = serde_json::json!({"type":"string","maxLength":MAX_PATTERN_BYTES});
    let attr = format!("attr:[^\\s\"'<>/=]{{1,{MAX_ATTRIBUTE_NAME_CHARS}}}");
    let predicate = serde_json::json!({"anyOf":[selector.clone(), {
        "type":"object","additionalProperties":false,
        "properties":{"select":selector,"read":{"type":"string","pattern":format!("^(text|text-content|{attr})$")},
            "equals":{"type":"string"},"pattern":pattern,"min":count,"max":count},
        "not":{"required":["equals","pattern"]}
    }]});
    let predicates = serde_json::json!({"type":"array","maxItems":MAX_ENTRIES,"items":{"$ref":"#/$defs/predicate"}});
    serde_json::json!({"$schema":"https://json-schema.org/draft/2020-12/schema",
    "title":"HTMLCut extraction contract","$ref":"#/$defs/node",
    "$defs":{"predicate":predicate,"node":{"anyOf":[selector.clone(), {
        "type":"object","additionalProperties":false,
        "properties":{"select":selector,"where":predicates,"expect":predicates,
            "until":selector,"match":{"enum":["one","optional","all"]},"min":count,"max":count,
            "nth":{"type":"integer","minimum":1,"maximum":MAX_DECLARED_COUNT},
            "fields":{"type":"object","minProperties":1,"maxProperties":MAX_RECORD_FIELDS,"propertyNames":{"pattern":format!("^[A-Za-z0-9_-]{{1,{MAX_FIELD_NAME_CHARS}}}$")},"additionalProperties":{"$ref":"#/$defs/node"}},
            "read":{"type":"string","pattern":format!("^(text|text-content|outer-html|{attr}|url:[^\\s\"'<>/=]{{1,{MAX_ATTRIBUTE_NAME_CHARS}}})$")},
            "exclude":{"type":"array","maxItems":MAX_ENTRIES,"items":selector},"equals":{"type":"string"},"pattern":pattern},
        "dependentRequired":{"where":["select"],"until":["select"],"nth":["select"]},
        "not":{"required":["equals","pattern"]},
        "allOf":[
            {"if":{"required":["fields"]},"then":{"not":{"anyOf":[{"required":["read"]},{"required":["exclude"]},{"required":["equals"]},{"required":["pattern"]}]}}},
            {"if":{"anyOf":[{"required":["min"]},{"required":["max"]}]},"then":{"required":["match"],"properties":{"match":{"const":"all"}}}},
            {"if":{"required":["nth"]},"then":{"properties":{"match":{"const":"one"}}}}
        ]
    }]}}})
}

#[cfg(test)]
mod tests {
    use super::*;
    fn compile(value: Json) -> Result<Contract, Error> {
        Contract::from_json(&serde_json::to_string(&value).unwrap())
    }
    fn refused(value: Json, message: &str) {
        let error = compile(value).err().expect("grammar must refuse");
        assert!(error.message.contains(message), "{}", error.message);
        assert_eq!(error.error.exit_status(), 2);
    }
    #[test]
    fn contract_text_bound_is_checked_before_json_reading() {
        let mut text = String::from("{}");
        text.push_str(&" ".repeat(256 * 1024 - 2));
        assert!(Contract::from_json(&text).is_ok());
        text.push(' ');
        assert!(
            Contract::from_json(&text)
                .err()
                .unwrap()
                .message
                .contains("262144")
        );
    }
    #[test]
    fn node_bound_has_an_admitted_and_a_refused_control() {
        let mut fields = Map::new();
        for row in 0..8 {
            let children = (0..if row == 7 { 62 } else { 63 })
                .map(|n| (format!("f{n:02}"), serde_json::json!({})))
                .collect::<Map<_, _>>();
            fields.insert(format!("r{row}"), serde_json::json!({"fields":children}));
        }
        let mut value = serde_json::json!({"fields":fields});
        assert!(compile(value.clone()).is_ok());
        value["fields"]["r7"]["fields"]["f62"] = serde_json::json!({});
        refused(value, "512");
    }
    #[test]
    fn record_depth_and_field_bounds_are_checked() {
        let mut value = serde_json::json!({});
        for _ in 0..8 {
            value = serde_json::json!({"fields":{"x":value}});
        }
        assert!(compile(value.clone()).is_ok());
        refused(serde_json::json!({"fields":{"x":value}}), "8");
        let fields = (0..64)
            .map(|n| (format!("f{n}"), serde_json::json!("p")))
            .collect::<Map<_, _>>();
        let mut value = serde_json::json!({"fields":fields});
        assert!(compile(value.clone()).is_ok());
        value["fields"]["extra"] = serde_json::json!("p");
        refused(value, "64");
    }
    #[test]
    fn each_predicate_and_exclusion_array_has_the_declared_bound() {
        for name in ["where", "expect", "exclude"] {
            let mut value = serde_json::json!({"select":"p"});
            value[name] = Json::Array(vec![serde_json::json!("p"); 32]);
            assert!(compile(value.clone()).is_ok());
            value[name]
                .as_array_mut()
                .unwrap()
                .push(serde_json::json!("p"));
            refused(value, "32");
        }
    }
    #[test]
    fn selector_bytes_syntax_and_matching_depth_have_negative_controls() {
        assert!(compile(serde_json::json!({"select": "a".repeat(8192)})).is_ok());
        refused(serde_json::json!({"select": "a".repeat(8193)}), "8192");
        refused(
            serde_json::json!({"select":format!("{}p{}", ":is(".repeat(65), ")".repeat(65))}),
            "64",
        );
        assert!(compile(serde_json::json!({"select": vec!["p";64].join(" ")})).is_ok());
        refused(serde_json::json!({"select": vec!["p";65].join(" ")}), "64");
    }
    #[test]
    fn pattern_bytes_compiled_size_and_named_groups_are_checked() {
        assert!(compile(serde_json::json!({"pattern":"a".repeat(8192)})).is_ok());
        refused(serde_json::json!({"pattern":"a".repeat(8193)}), "8192");
        refused(serde_json::json!({"pattern":"a{999999}"}), "compiled-size");
        let contract = compile(serde_json::json!({"pattern":"^£(?<value>[0-9.]+)$"})).unwrap();
        let Body::Value { check, .. } = contract.node.body else {
            panic!("not a value");
        };
        assert_eq!(
            &check.pattern.unwrap().captures("£51.77").unwrap()["value"],
            "51.77"
        );
    }
    #[test]
    fn numeric_and_attribute_name_bounds_are_checked() {
        for name in ["min", "max", "nth"] {
            let mut value =
                serde_json::json!({"select":"p", "match": if name == "nth" {"one"} else {"all"}});
            value[name] = 1_000_000.into();
            assert!(compile(value.clone()).is_ok());
            value[name] = 1_000_001.into();
            refused(value, "1000000");
        }
        assert!(compile(serde_json::json!({"read":format!("attr:{}", "é".repeat(256))})).is_ok());
        refused(
            serde_json::json!({"read":format!("attr:{}", "é".repeat(257))}),
            "reading",
        );
    }
    #[test]
    fn ineffective_members_are_refused_and_schema_requires_record_fields() {
        let validator = jsonschema::validator_for(&Contract::schema()).unwrap();
        let empty = serde_json::json!({"fields":{}});
        assert!(!validator.is_valid(&empty));
        assert_eq!(
            compile(empty).err().unwrap().path.as_deref(),
            Some("$.fields")
        );
        for read in ["attr:title", "url:href"] {
            let error = compile(serde_json::json!({"read":read,"exclude":[]}))
                .err()
                .unwrap();
            assert_eq!(error.path.as_deref(), Some("$.exclude"));
        }
        assert!(validator.is_valid(&serde_json::json!({"fields":{"title":"h1"}})));
    }
    #[test]
    fn refused_readings_describe_their_position() {
        for read in ["url:href", "outer-html"] {
            let error = compile(serde_json::json!({"select":"a","where":[{"read":read}]}))
                .err()
                .unwrap();
            assert_eq!(error.path.as_deref(), Some("$.where[0].read"));
            assert!(
                error
                    .message
                    .ends_with("A predicate reads text, text-content or attr:NAME.")
            );
            assert!(!error.message.contains("Readings are"));
        }
        let error = compile(serde_json::json!({"read":"bogus"})).err().unwrap();
        assert!(
            error
                .message
                .ends_with("Readings are text, text-content, outer-html, attr:NAME and url:NAME.")
        );
    }
    #[test]
    fn closed_members_and_member_combinations_refuse_with_locations() {
        for value in [
            serde_json::json!({"read":null}),
            serde_json::json!({"select":2}),
            serde_json::json!({"where":[]}),
            serde_json::json!({"select":"p","nth":0}),
            serde_json::json!({"equals":"x","pattern":"x"}),
            serde_json::json!({"match":"optional","nth":2,"select":"p"}),
            serde_json::json!({"fields":{},"exclude":[]}),
            serde_json::json!({"expect":[{"read":"url:href"}]}),
            serde_json::json!({"min":2,"max":1,"match":"all"}),
        ] {
            let error = compile(value).err().expect("invalid grammar admitted");
            assert_eq!(error.error, ErrorCode::InvalidContract);
            assert!(error.path.is_some());
        }
    }
}
