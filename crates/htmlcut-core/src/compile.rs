// SPDX-License-Identifier: MPL-2.0
//! Semantic admission and compilation of typed contracts.
use crate::bounds::*;
use crate::{
    Error, Match, NodeObject, NodeSpec, PredicateObject, PredicateSpec, Reading, Stop,
    select::Selector,
};
use regex::{Regex, RegexBuilder};
use serde_json::Value as Json;
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
pub(crate) struct Check {
    pub(crate) equals: Option<String>,
    pub(crate) pattern: Option<Regex>,
}
impl Contract {
    /// Maximum contract JSON byte length.
    pub const MAX_BYTES: usize = MAX_CONTRACT_BYTES;
    /// Read bounded closed JSON, deserialize its shape, then compile its semantics.
    pub fn from_json(text: &str, stop: &Stop) -> Result<Self, Error> {
        stop.check()?;
        if text.len() > Self::MAX_BYTES {
            return Err(invalid(
                None,
                &format!(
                    "The contract exceeds the {}-byte grammar bound.",
                    Self::MAX_BYTES
                ),
            ));
        }
        let value = crate::json::read(text.as_bytes()).map_err(|e| {
            invalid(
                None,
                &format!("The contract is not valid closed JSON: {e}."),
            )
        })?;
        let spec: NodeSpec = serde_path_to_error::deserialize(value).map_err(|e| {
            let path = e.path().to_string();
            let path = if path == "." {
                "$".into()
            } else if path.starts_with('[') {
                format!("${path}")
            } else {
                format!("$.{path}")
            };
            invalid(Some(&path), &e.inner().to_string())
        })?;
        Self::compile(&spec, stop)
    }
    /// Compile a programmatic specification without a JSON round trip.
    pub fn compile(spec: &NodeSpec, stop: &Stop) -> Result<Self, Error> {
        stop.check()?;
        // Bound traversal before serialization or recursive compilation, including programmatic input.
        let mut pending = vec![(spec, 0, String::from("$"))];
        let mut nodes = 0;
        let mut patterns = 0;
        while let Some((spec, depth, path)) = pending.pop() {
            stop.check()?;
            nodes += 1;
            let oversized = match spec {
                NodeSpec::Selector(s) => s.len() > Self::MAX_BYTES,
                NodeSpec::Node(o) => [&o.select, &o.until, &o.equals, &o.pattern]
                    .into_iter()
                    .flatten()
                    .any(|s| s.len() > Self::MAX_BYTES),
            };
            if oversized {
                return Err(invalid(
                    None,
                    "The specification exceeds the contract byte bound.",
                ));
            }

            if nodes > MAX_CONTRACT_NODES {
                return Err(invalid(
                    Some(&path),
                    "The contract exceeds the 512-node grammar bound.",
                ));
            }
            if let NodeSpec::Node(o) = spec {
                patterns += usize::from(o.pattern.is_some());
                for (name, predicates) in [("where", &o.r#where), ("expect", &o.expect)] {
                    if let Some(predicates) = predicates {
                        if predicates.len() > MAX_ENTRIES {
                            return Err(invalid(
                                Some(&format!("{path}.{name}")),
                                "The member must be an array with at most 32 entries.",
                            ));
                        }
                        patterns += predicates
                            .iter()
                            .filter(
                                |p| matches!(p,PredicateSpec::Predicate(o) if o.pattern.is_some()),
                            )
                            .count();
                    }
                }
                if let Some(fields) = &o.fields {
                    if depth >= MAX_RECORD_DEPTH {
                        return Err(invalid(
                            Some(&path),
                            "Records exceed the nesting bound of 8.",
                        ));
                    }
                    if fields.is_empty() || fields.len() > MAX_RECORD_FIELDS {
                        return Err(invalid(
                            Some(&format!("{path}.fields")),
                            "A record must have 1–64 fields.",
                        ));
                    }
                    pending.extend(
                        fields
                            .iter()
                            .rev()
                            .map(|(name, spec)| (spec, depth + 1, format!("{path}.fields.{name}"))),
                    );
                }
            }
        }
        let mut size = crate::value::OutputSize::new(Self::MAX_BYTES);
        size.value(spec, &crate::stop::Clock::for_stop(stop))
            .map_err(|error| match error {
                Error::LimitExceeded {
                    limit: crate::Limit::OutputBytes,
                    ..
                } => invalid(None, "The specification exceeds the contract byte bound."),
                e => e,
            })?;
        let mut compiler = Compiler {
            stop,
            regex_bytes: MAX_REGEX_BYTES / (2 * patterns.max(1)),
        };
        let node = compiler.node(spec, "$")?;
        stop.check()?;
        Ok(Self { node })
    }
    /// Generated authoring schema; compilation remains authoritative for semantic bounds.
    pub fn schema() -> Json {
        crate::spec::schema()
    }
    /// Refuse output representations that cannot preserve the root shape.
    pub fn supports(&self, format: crate::ExportFormat) -> Result<(), Error> {
        if format == crate::ExportFormat::Raw && matches!(self.node.body, Body::Record(_)) {
            return Err(Error::UnrepresentableOutput {
                path: "$".into(),
                row: None,
                reason: "Raw export requires a value root, not a record root.".into(),
            });
        }
        Ok(())
    }
}
fn invalid(path: Option<&str>, reason: &str) -> Error {
    Error::InvalidContract {
        path: path.map(Into::into),
        reason: reason.into(),
    }
}
fn count(value: Option<u32>, path: &str, name: &str) -> Result<Option<usize>, Error> {
    value
        .map(|n| {
            if n as usize > MAX_DECLARED_COUNT {
                Err(invalid(
                    Some(&format!("{path}.{name}")),
                    "Counts must be integers from 0 to 1000000.",
                ))
            } else {
                Ok(n as usize)
            }
        })
        .transpose()
}
fn selector(value: Option<&String>, path: &str, name: &str) -> Result<Option<Selector>, Error> {
    value
        .map(|s| Selector::compile(s, &format!("{path}.{name}")))
        .transpose()
}
struct Compiler<'a> {
    stop: &'a Stop,
    regex_bytes: usize,
}
impl Compiler<'_> {
    fn check(
        &self,
        equals: &Option<String>,
        pattern: &Option<String>,
        path: &str,
    ) -> Result<Check, Error> {
        self.stop.check()?;
        if equals.is_some() && pattern.is_some() {
            return Err(invalid(
                Some(path),
                "equals and pattern are mutually exclusive.",
            ));
        }
        let pattern = pattern
            .as_ref()
            .map(|text| {
                let location = format!("{path}.pattern");
                if text.len() > MAX_PATTERN_BYTES {
                    return Err(invalid(
                        Some(&location),
                        "The pattern exceeds the 8192-byte grammar bound.",
                    ));
                }
                RegexBuilder::new(text)
                    .size_limit(self.regex_bytes)
                    .dfa_size_limit(self.regex_bytes)
                    .nest_limit(MAX_PATTERN_DEPTH)
                    .build()
                    .map_err(|e| Error::InvalidPattern {
                        path: location,
                        reason: format!(
                            "Invalid regular expression or exceeded compiled-size bound {}: {e}.",
                            self.regex_bytes
                        ),
                    })
            })
            .transpose()?;
        Ok(Check {
            equals: equals.clone(),
            pattern,
        })
    }
    fn predicates(
        &self,
        spec: &Option<Vec<PredicateSpec>>,
        path: &str,
        name: &str,
    ) -> Result<Vec<Predicate>, Error> {
        spec.as_deref()
            .unwrap_or_default()
            .iter()
            .enumerate()
            .map(|(i, p)| self.predicate(p, &format!("{path}.{name}[{i}]")))
            .collect()
    }
    fn predicate(&self, spec: &PredicateSpec, path: &str) -> Result<Predicate, Error> {
        let short;
        let o = match spec {
            PredicateSpec::Selector(s) => {
                short = PredicateObject {
                    select: Some(s.clone()),
                    ..Default::default()
                };
                &short
            }
            PredicateSpec::Predicate(o) => o,
        };
        let min = count(o.min, path, "min")?.unwrap_or(1);
        let max = count(o.max, path, "max")?.unwrap_or(1);
        if min > max {
            return Err(invalid(
                Some(path),
                "The predicate minimum exceeds its maximum.",
            ));
        }
        let read = o.read.clone().unwrap_or_default();
        read.to_string()
            .parse::<Reading>()
            .map_err(|reason| invalid(Some(&format!("{path}.read")), &reason))?;
        if matches!(read, Reading::OuterHtml | Reading::Url(_)) {
            return Err(invalid(
                Some(&format!("{path}.read")),
                "A predicate reads text, text-content or attr:NAME.",
            ));
        }
        Ok(Predicate {
            path: path.into(),
            select: selector(o.select.as_ref(), path, "select")?,
            read,
            check: self.check(&o.equals, &o.pattern, path)?,
            min,
            max,
        })
    }
    fn node(&mut self, spec: &NodeSpec, path: &str) -> Result<Node, Error> {
        self.stop.check()?;
        let short;
        let o = match spec {
            NodeSpec::Selector(s) => {
                short = NodeObject {
                    select: Some(s.clone()),
                    ..Default::default()
                };
                &short
            }
            NodeSpec::Node(o) => o,
        };
        let select = selector(o.select.as_ref(), path, "select")?;
        let until = selector(o.until.as_ref(), path, "until")?;
        let nth = count(o.nth, path, "nth")?;
        let min = count(o.min, path, "min")?;
        let max = count(o.max, path, "max")?;
        if (o.r#where.is_some() || until.is_some() || nth.is_some()) && select.is_none() {
            return Err(invalid(Some(path), "where, until and nth require select."));
        }
        if nth == Some(0) {
            return Err(invalid(
                Some(&format!("{path}.nth")),
                "nth must be at least 1.",
            ));
        }
        let mode = o.r#match.unwrap_or_default();
        if mode != Match::All && (min.is_some() || max.is_some()) {
            return Err(invalid(Some(path), "min and max require match all."));
        }
        if nth.is_some() && mode != Match::One {
            return Err(invalid(Some(path), "nth requires match one."));
        }
        let cardinality = match mode {
            Match::One => Cardinality::One(nth),
            Match::Optional => Cardinality::Optional,
            Match::All => {
                let min = min.unwrap_or(1);
                if max.is_some_and(|max| max < min) {
                    return Err(invalid(Some(path), "The minimum exceeds the maximum."));
                }
                Cardinality::All { min, max }
            }
        };
        let expect = self.predicates(&o.expect, path, "expect")?;
        let predicates = self.predicates(&o.r#where, path, "where")?;
        let body = if let Some(fields) = &o.fields {
            for (name, present) in [
                ("read", o.read.is_some()),
                ("exclude", o.exclude.is_some()),
                ("equals", o.equals.is_some()),
                ("pattern", o.pattern.is_some()),
            ] {
                if present {
                    return Err(invalid(
                        Some(&format!("{path}.{name}")),
                        &format!("{name} cannot be combined with fields."),
                    ));
                }
            }
            let mut compiled = BTreeMap::new();
            for (name, spec) in fields {
                if name.is_empty()
                    || name.len() > MAX_FIELD_NAME_CHARS
                    || !name
                        .bytes()
                        .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
                {
                    return Err(invalid(
                        Some(&format!("{path}.fields")),
                        "Field names must be 1–64 ASCII letters, digits, underscores or hyphens.",
                    ));
                }
                compiled.insert(
                    name.clone(),
                    self.node(spec, &format!("{path}.fields.{name}"))?,
                );
            }
            Body::Record(compiled)
        } else {
            let read = o.read.clone().unwrap_or_default();
            if o.exclude.is_some() && matches!(read, Reading::Attr(_) | Reading::Url(_)) {
                return Err(invalid(
                    Some(&format!("{path}.exclude")),
                    "exclude cannot be combined with an attribute or URL reading.",
                ));
            }
            let exclusions = o.exclude.as_deref().unwrap_or_default();
            if exclusions.len() > MAX_ENTRIES {
                return Err(invalid(
                    Some(&format!("{path}.exclude")),
                    "The member must be an array with at most 32 entries.",
                ));
            }
            let exclude = exclusions
                .iter()
                .enumerate()
                .map(|(i, s)| Selector::compile(s, &format!("{path}.exclude[{i}]")))
                .collect::<Result<_, _>>()?;
            // Programmatic Reading values obey the same attribute-name rules as JSON.
            read.to_string()
                .parse::<Reading>()
                .map_err(|reason| invalid(Some(&format!("{path}.read")), &reason))?;
            Body::Value {
                read,
                exclude,
                check: self.check(&o.equals, &o.pattern, path)?,
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
        let context = crate::stop::Clock::testing(move || {
            observed.fetch_add(1, Ordering::Relaxed);
            Ok(())
        });
        let query = crate::test_support::QueryFixture {
            select: "\\".repeat(super::MAX_CONTRACT_BYTES + 1),
            read: "text".into(),
            cardinality: crate::Match::One,
        };
        let error = crate::test_support::compile_query(&query, &context)
            .err()
            .unwrap();
        assert_eq!(error.code(), crate::ErrorCode::InvalidContract);
        // Allow ordinary admission checks. Escaping this input before refusing
        // would require hundreds of the context's 1024-iteration checkpoints.
        assert!(
            checks.load(Ordering::Relaxed) < 16,
            "oversized quick syntax reached encoding work"
        );
    }
    use super::*;
    use crate::ErrorCode;
    #[allow(unused_imports)]
    use crate::{
        stop::Clock,
        test_support::{DocumentFixture, TestOptions, parse_fixture},
    };
    use serde_json::Map;
    fn compile(value: Json) -> Result<Contract, Error> {
        Contract::from_json(
            &serde_json::to_string(&value).unwrap(),
            &crate::stop::Clock::new(None).unwrap(),
        )
    }
    fn refused(value: Json, message: &str) {
        let error = compile(value).err().expect("grammar must refuse");
        assert!(error.to_string().contains(message), "{}", error.to_string());
        assert!(matches!(
            error.code(),
            ErrorCode::InvalidContract | ErrorCode::InvalidSelector | ErrorCode::InvalidPattern
        ));
    }
    #[test]
    fn contract_text_bound_is_checked_before_json_reading() {
        let mut text = String::from("{}");
        text.push_str(&" ".repeat(256 * 1024 - 2));
        assert!(Contract::from_json(&text, &crate::stop::Clock::new(None).unwrap()).is_ok());
        text.push(' ');
        assert!(
            Contract::from_json(&text, &crate::stop::Clock::new(None).unwrap())
                .err()
                .unwrap()
                .to_string()
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
    fn unsigned_limits_preserve_raw_numeric_token_spelling_at_every_position() {
        let context = crate::stop::Clock::new(None).unwrap();
        for (prefix, suffix, path) in [
            (r#"{"select":"p","match":"all","min":"#, "}", "$.min"),
            (r#"{"select":"p","match":"all","max":"#, "}", "$.max"),
            (r#"{"select":"p","nth":"#, "}", "$.nth"),
            (r#"{"select":"p","where":[{"min":"#, "}]}", "$.where[0].min"),
            (r#"{"select":"p","where":[{"max":"#, "}]}", "$.where[0].max"),
            (r#"{"expect":[{"min":"#, "}]}", "$.expect[0].min"),
            (r#"{"expect":[{"max":"#, "}]}", "$.expect[0].max"),
        ] {
            for token in [
                "1",
                "1.0",
                "1e0",
                "-0",
                "1.0000000000000000001",
                "1e-999",
                "-1",
                "true",
                "null",
                r#""1""#,
            ] {
                let text = format!("{prefix}{token}{suffix}");
                let actual = Contract::from_json(&text, &context);
                if token == "1" {
                    assert!(actual.is_ok(), "{text}");
                } else {
                    let error = actual.err().unwrap_or_else(|| panic!("admitted {text}"));
                    assert_eq!(error.code(), ErrorCode::InvalidContract, "{text}");
                    assert_eq!(
                        serde_json::to_value(&error).unwrap()["path"].as_str(),
                        Some(path),
                        "{text}"
                    );
                }
            }
        }
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
        for (member, admitted_type) in [
            ("select", "string"),
            ("read", "string"),
            ("match", "string"),
            ("fields", "object"),
            ("exclude", "array"),
            ("expect", "array"),
            ("where", "array"),
            ("until", "string"),
            ("nth", "number"),
            ("min", "number"),
            ("max", "number"),
            ("equals", "string"),
            ("pattern", "string"),
        ] {
            for (kind, value) in [
                ("null", serde_json::Value::Null),
                ("boolean", serde_json::json!(false)),
                ("number", serde_json::json!(1)),
                ("string", serde_json::json!("p")),
                ("array", serde_json::json!([])),
                ("object", serde_json::json!({})),
            ] {
                if kind == admitted_type {
                    continue;
                }
                let mut contract = serde_json::json!({"select":"p"});
                if ["min", "max"].contains(&member) {
                    contract["match"] = serde_json::json!("all");
                }
                contract[member] = value.clone();
                assert!(!validator.is_valid(&contract), "{contract}");
                assert!(compile(contract).is_err());
                if ["select", "read", "min", "max", "equals", "pattern"].contains(&member) {
                    let predicate = serde_json::json!({member:value});
                    let contract = serde_json::json!({"expect":[predicate]});
                    assert!(!validator.is_valid(&contract), "{contract}");
                    assert!(compile(contract).is_err());
                }
            }
        }
        for contract in [
            serde_json::json!({"read":"attr:id","exclude":[]}),
            serde_json::json!({"read":"url:href","exclude":[]}),
            serde_json::json!({"read":"text\n"}),
            serde_json::json!({"fields":{"title\n":{}}}),
            serde_json::json!({"expect":[{"read":"text\n"}]}),
            serde_json::json!({"fields":{"x":{}},"read":"text"}),
            serde_json::json!({"fields":{"x":{}},"exclude":[]}),
            serde_json::json!({"fields":{"x":{}},"equals":"x"}),
            serde_json::json!({"fields":{"x":{}},"pattern":"x"}),
            serde_json::json!({"where":[]}),
            serde_json::json!({"until":"p"}),
            serde_json::json!({"nth":1}),
            serde_json::json!({"select":"p","nth":0}),
            serde_json::json!({"select":"p","nth":1,"match":"optional"}),
            serde_json::json!({"select":"p","nth":1,"match":"all"}),
            serde_json::json!({"min":0}),
            serde_json::json!({"max":1,"match":"optional"}),
            serde_json::json!({"equals":"x","pattern":"x"}),
            serde_json::json!({"expect":[{"read":"outer-html"}]}),
            serde_json::json!({"expect":[{"read":"url:href"}]}),
            serde_json::json!({"expect":[{"nth":1}]}),
            serde_json::json!({"expect":[{"until":"p"}]}),
            serde_json::json!({"expect":[{"fields":{"x":{}}}]}),
            serde_json::json!({"expect":[{"where":[]}]}),
            serde_json::json!({"expect":[{"match":"all"}]}),
            serde_json::json!({"expect":[{"exclude":[]}]}),
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
            serde_json::to_value(compile(empty).err().unwrap()).unwrap()["path"].as_str(),
            Some("$.fields")
        );
        for read in ["attr:title", "url:href"] {
            let error = compile(serde_json::json!({"read":read,"exclude":[]}))
                .err()
                .unwrap();
            assert_eq!(
                serde_json::to_value(&error).unwrap()["path"].as_str(),
                Some("$.exclude")
            );
        }
        assert!(validator.is_valid(&serde_json::json!({"fields":{"title":"h1"}})));
    }
    #[test]
    fn refused_readings_describe_their_position() {
        for read in ["url:href", "outer-html"] {
            let error = compile(serde_json::json!({"select":"a","where":[{"read":read}]}))
                .err()
                .unwrap();
            assert_eq!(
                serde_json::to_value(&error).unwrap()["path"].as_str(),
                Some("$.where[0].read")
            );
            assert!(
                error
                    .to_string()
                    .ends_with("A predicate reads text, text-content or attr:NAME.")
            );
            assert!(!error.to_string().contains("Readings are"));
        }
        let error = compile(serde_json::json!({"read":"bogus"})).err().unwrap();
        assert!(
            error
                .to_string()
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
            assert_eq!(error.code(), ErrorCode::InvalidContract);
            assert!(serde_json::to_value(&error).unwrap().get("path").is_some());
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
        assert_eq!(error.code(), ErrorCode::InvalidPattern);
        assert!(
            serde_json::to_value(&error).unwrap()["path"]
                .as_str()
                .unwrap()
                .ends_with(".pattern")
        );
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
        assert_eq!(error.code(), ErrorCode::InvalidPattern);
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
                assert_eq!(error.code(), ErrorCode::InvalidContract);
                assert_eq!(
                    serde_json::to_value(&error).unwrap()["path"].as_str(),
                    Some("$.read")
                );
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
            assert_eq!(error.code(), ErrorCode::InvalidContract);
            assert_eq!(
                serde_json::to_value(&error).unwrap()["path"].as_str(),
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
            assert_eq!(error.code(), ErrorCode::InvalidContract);
            assert_eq!(
                serde_json::to_value(&error).unwrap()["path"].as_str(),
                Some("$")
            );
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
