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

/// Neutral shorthand for a value-root contract.
#[derive(Clone, Debug)]
pub struct QuickQuery {
    /// CSS selection.
    pub select: String,
    /// Reading syntax, validated by the contract compiler.
    pub read: String,
    /// Match cardinality.
    pub cardinality: QueryCardinality,
}
/// Quick-query cardinality.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QueryCardinality {
    /// Exactly one match.
    One,
    /// Zero or one match.
    Optional,
    /// Every match.
    All,
}

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
    pub fn from_json(text: &str, context: &crate::ExecutionContext) -> Result<Self, Error> {
        context.check()?;
        if text.len() > MAX_CONTRACT_BYTES {
            return Err(invalid(
                None,
                &format!("The contract exceeds the {MAX_CONTRACT_BYTES}-byte grammar bound."),
            ));
        }
        let value = crate::json::read(text.as_bytes()).map_err(|e| {
            invalid(
                None,
                &format!("The contract is not valid closed JSON: {e}."),
            )
        })?;
        Self::compile(&value, context)
    }
    /// Maximum admitted contract-file bytes.
    pub const MAX_BYTES: usize = MAX_CONTRACT_BYTES;
    /// Compiles a typed quick query through the same semantic admission as JSON.
    pub fn quick(query: &QuickQuery, context: &crate::ExecutionContext) -> Result<Self, Error> {
        context.check()?;
        if query.select.len().saturating_add(query.read.len()) > MAX_CONTRACT_BYTES {
            return Err(invalid(
                None,
                "The quick query exceeds the contract byte bound.",
            ));
        }
        let value = serde_json::json!({"select": query.select, "read": query.read,
            "match": match query.cardinality { QueryCardinality::One => "one", QueryCardinality::Optional => "optional", QueryCardinality::All => "all" }});
        // Count escaped input as well as source strings before compiler admission.
        let mut size = crate::value::OutputSize::new(MAX_CONTRACT_BYTES);
        size.value(&value, &(context).clone()).map_err(|error| {
            if error.limit.as_deref() == Some("output_bytes") {
                invalid(None, "The quick query exceeds the contract byte bound.")
            } else {
                error
            }
        })?;
        Self::compile(&value, context)
    }
    fn compile(value: &Json, context: &crate::ExecutionContext) -> Result<Self, Error> {
        context.check()?;
        let patterns = pattern_count(value);
        let mut compiler = Compiler {
            nodes: 0,
            regex_bytes: crate::limits::MAX_REGEX_BYTES / (2 * patterns.max(1)),
            context,
        };
        let node = compiler.node(value, "$", 0)?;
        context.check()?;
        Ok(Self { node })
    }
    /// JSON Schema for the closed contract surface; byte and compiled bounds remain engine checks.
    pub fn schema() -> Json {
        schema()
    }
    /// Encodes the complete schema under the operation's output and stop policy.
    pub fn export_schema(
        maximum: usize,
        context: &crate::ExecutionContext,
    ) -> Result<Vec<u8>, Error> {
        context.check()?;
        crate::value::encode_json(&Self::schema(), maximum, context)
    }
    /// Admits export shape before acquiring any document bytes.
    pub fn admit_export(&self, format: crate::ExportFormat) -> Result<(), Error> {
        if format == crate::ExportFormat::Raw && self.produces_records() {
            return Err(Error::new(
                ErrorCode::UnrepresentableOutput,
                "Raw export requires a value root, not a record root.",
            ));
        }
        Ok(())
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
    let text = string(map, "read", path)?.unwrap_or_else(|| crate::DEFAULT_READING.into());
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

struct Compiler<'a> {
    context: &'a crate::ExecutionContext,
    nodes: usize,
    regex_bytes: usize,
}
impl Compiler<'_> {
    fn check(&self, map: &Map<String, Json>, path: &str) -> Result<Check, Error> {
        self.context.check()?;
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
        self.context.check()?;
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
    let count = serde_json::json!({"type":"integer","minimum":0,"maximum":MAX_DECLARED_COUNT,"description":"Compiler requires exact unsigned integer tokens: 1 is admitted; 1.0, 1e0 and -0 are refused. Schema validates mathematical values and cannot enforce token spelling."});
    let pattern = serde_json::json!({"type":"string","maxLength":MAX_PATTERN_BYTES});
    let attr = format!("attr:[^\\s\"'<>/=]{{1,{MAX_ATTRIBUTE_NAME_CHARS}}}");
    let predicate = serde_json::json!({"anyOf":[selector.clone(), {
        "type":"object","additionalProperties":false,
        "properties":{"select":selector,"read":{"type":"string","pattern":format!("^(text|text-content|{attr})(?![\\s\\S])")},
            "equals":{"type":"string"},"pattern":pattern,"min":count,"max":count},
        "not":{"required":["equals","pattern"]}
    }]});
    let predicates = serde_json::json!({"type":"array","maxItems":MAX_ENTRIES,"items":{"$ref":"#/$defs/predicate"}});
    serde_json::json!({"$schema":"https://json-schema.org/draft/2020-12/schema",
    "title":"HTMLCut extraction contract","description":"Authoring structure validation only. The compiler is authoritative for exact integer token spelling (1 versus 1.0/1e0/-0), duplicate keys, CSS/regex compilation, global byte/node/depth/compiled-cost limits and relationships between counts.","$ref":"#/$defs/node",
    "$defs":{"predicate":predicate,"node":{"anyOf":[selector.clone(), {
        "type":"object","additionalProperties":false,
        "properties":{"select":selector,"where":predicates,"expect":predicates,
            "until":selector,"match":{"enum":["one","optional","all"]},"min":count,"max":count,
            "nth":{"type":"integer","minimum":1,"maximum":MAX_DECLARED_COUNT},
            "fields":{"type":"object","minProperties":1,"maxProperties":MAX_RECORD_FIELDS,"propertyNames":{"pattern":format!("^[A-Za-z0-9_-]{{1,{MAX_FIELD_NAME_CHARS}}}(?![\\s\\S])")},"additionalProperties":{"$ref":"#/$defs/node"}},
            "read":{"type":"string","pattern":format!("^(text|text-content|outer-html|{attr}|url:[^\\s\"'<>/=]{{1,{MAX_ATTRIBUTE_NAME_CHARS}}})(?![\\s\\S])")},
            "exclude":{"type":"array","maxItems":MAX_ENTRIES,"items":selector},"equals":{"type":"string"},"pattern":pattern},
        "dependentRequired":{"where":["select"],"until":["select"],"nth":["select"]},
        "not":{"required":["equals","pattern"]},
        "allOf":[
            {"if":{"required":["read"],"properties":{"read":{"pattern":"^(attr:|url:)"}}},"then":{"not":{"required":["exclude"]}}},
            {"if":{"required":["fields"]},"then":{"not":{"anyOf":[{"required":["read"]},{"required":["exclude"]},{"required":["equals"]},{"required":["pattern"]}]}}},
            {"if":{"anyOf":[{"required":["min"]},{"required":["max"]}]},"then":{"required":["match"],"properties":{"match":{"const":"all"}}}},
            {"if":{"required":["nth"]},"then":{"properties":{"match":{"const":"one"}}}}
        ]
    }]}}})
}

#[cfg(test)]
mod tests {
    #[test]
    fn oversized_typed_query_refuses_before_encoding_work() {
        use std::sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        };
        let checks = Arc::new(AtomicUsize::new(0));
        let observed = checks.clone();
        let context = crate::ExecutionContext::with_checkpoint(move || {
            observed.fetch_add(1, Ordering::Relaxed);
            Ok(())
        });
        let query = crate::QuickQuery {
            select: "\\".repeat(super::MAX_CONTRACT_BYTES + 1),
            read: "text".into(),
            cardinality: crate::QueryCardinality::One,
        };
        let error = super::Contract::quick(&query, &context).err().unwrap();
        assert_eq!(error.error, crate::ErrorCode::InvalidContract);
        // Allow ordinary admission checks. Escaping this input before refusing
        // would require hundreds of the context's 1024-iteration checkpoints.
        assert!(
            checks.load(Ordering::Relaxed) < 16,
            "oversized quick syntax reached encoding work"
        );
    }
    use super::*;
    fn compile(value: Json) -> Result<Contract, Error> {
        Contract::from_json(
            &serde_json::to_string(&value).unwrap(),
            &crate::ExecutionContext::new(None).unwrap(),
        )
    }
    fn refused(value: Json, message: &str) {
        let error = compile(value).err().expect("grammar must refuse");
        assert!(error.message.contains(message), "{}", error.message);
        assert!(matches!(
            error.error,
            ErrorCode::InvalidContract | ErrorCode::InvalidSelector | ErrorCode::InvalidPattern
        ));
    }
    #[test]
    fn contract_text_bound_is_checked_before_json_reading() {
        let mut text = String::from("{}");
        text.push_str(&" ".repeat(256 * 1024 - 2));
        assert!(Contract::from_json(&text, &crate::ExecutionContext::new(None).unwrap()).is_ok());
        text.push(' ');
        assert!(
            Contract::from_json(&text, &crate::ExecutionContext::new(None).unwrap())
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
    fn schema_rejects_expressible_reading_and_field_shapes() {
        let validator = jsonschema::validator_for(&Contract::schema()).unwrap();
        for contract in [
            serde_json::json!({"read":"attr:id","exclude":[]}),
            serde_json::json!({"read":"url:href","exclude":[]}),
            serde_json::json!({"read":"text\n"}),
            serde_json::json!({"fields":{"title\n":{}}}),
            serde_json::json!({"expect":[{"read":"text\n"}]}),
        ] {
            assert!(!validator.is_valid(&contract), "{contract}");
            assert!(compile(contract).is_err());
        }
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

    #[test]
    fn compiled_regex_allowance_is_shared_by_value_and_predicate_patterns() {
        // Bounded repeats use little contract text but consume compiled regex memory.
        assert!(compile(serde_json::json!({"pattern":"a{100000}"})).is_ok());
        assert!(
            compile(serde_json::json!({
                "select":"p", "pattern":"a{50000}",
                "where":[{"pattern":"a{50000}"}]
            }))
            .is_ok()
        );
        let error = compile(serde_json::json!({
            "select":"p", "pattern":"a{50000}",
            "where":[{"pattern":"a{50000}"}],
            "expect":[{"pattern":"a{50000}"}]
        }))
        .err()
        .expect("three expensive patterns must exceed their shared allowance");
        assert_eq!(error.error, ErrorCode::InvalidPattern);
        assert!(error.path.as_deref().unwrap().ends_with(".pattern"));
    }

    #[test]
    fn compiled_regex_allowance_includes_patterns_in_nested_fields() {
        assert!(compile(serde_json::json!({
            "select":"p", "expect":[{"pattern":"a{40000}"}],
            "fields":{"first":{"pattern":"a{40000}"},"nested":{"fields":{"second":{"pattern":"a{40000}"}}}}
        })).is_ok());
        let error = compile(serde_json::json!({
            "select":"p", "where":[{"pattern":"a{40000}"}],
            "expect":[{"pattern":"a{40000}"}],
            "fields":{"first":{"pattern":"a{40000}"},"nested":{"fields":{"second":{"pattern":"a{40000}"}}}}
        })).err().expect("nested fields must share the same allowance as predicates");
        assert_eq!(error.error, ErrorCode::InvalidPattern);
    }

    #[test]
    fn attribute_reading_names_refuse_each_forbidden_delimiter() {
        for prefix in ["attr:", "url:"] {
            for name in [
                "", "a b", "a\tb", "a\nb", "a\u{a0}b", "a\"b", "a'b", "a<b", "a>b", "a/b", "a=b",
            ] {
                let error = compile(serde_json::json!({"read":format!("{prefix}{name}")}))
                    .err()
                    .expect("invalid attribute name admitted");
                assert_eq!(error.error, ErrorCode::InvalidContract);
                assert_eq!(error.path.as_deref(), Some("$.read"));
            }
            for name in ["data-price", "xlink:href", "viewBox", "é"] {
                assert!(compile(serde_json::json!({"read":format!("{prefix}{name}")})).is_ok());
            }
        }
    }

    #[test]
    fn predicate_ranges_admit_equal_bounds_and_refuse_inverted_bounds() {
        for predicate in [
            serde_json::json!({"min":0,"max":2}),
            serde_json::json!({"min":2,"max":2}),
        ] {
            assert!(compile(serde_json::json!({"expect":[predicate]})).is_ok());
        }
        for member in ["where", "expect"] {
            let mut value = serde_json::json!({"select":"p"});
            value[member] = serde_json::json!([{"min":3,"max":2}]);
            let error = compile(value)
                .err()
                .expect("inverted predicate range admitted");
            assert_eq!(error.error, ErrorCode::InvalidContract);
            assert_eq!(
                error.path.as_deref(),
                Some(format!("$.{member}[0]").as_str())
            );
        }
    }

    #[test]
    fn each_selection_member_requires_a_selector() {
        for mut value in [
            serde_json::json!({"where":[]}),
            serde_json::json!({"until":"h2"}),
            serde_json::json!({"nth":1}),
        ] {
            let error = compile(value.clone())
                .err()
                .expect("selection member without selection admitted");
            assert_eq!(error.error, ErrorCode::InvalidContract);
            assert_eq!(error.path.as_deref(), Some("$"));
            value["select"] = serde_json::json!("p");
            assert!(compile(value).is_ok());
        }
    }

    #[test]
    fn field_name_bound_admits_equality_and_refuses_longer_names() {
        for length in [1, 64, 65, 66] {
            let name = "a".repeat(length);
            let value = serde_json::json!({"fields":{name: {}}});
            assert_eq!(
                compile(value).is_ok(),
                length <= 64,
                "field name length {length}"
            );
        }
        assert!(compile(serde_json::json!({"fields":{"":{}}})).is_err());
    }
}
