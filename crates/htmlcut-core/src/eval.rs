// SPDX-License-Identifier: MPL-2.0
//! Scope-relative recursive contract evaluation, with complete-result admission.
use crate::{
    Code, Contract, Document, Error, Value,
    contract::{Body, Cardinality, Check, Node, Predicate},
    limits::Clock,
    select::Selector,
    value::OutputSize,
};
use ego_tree::{NodeId, NodeRef};
use scraper::{ElementRef, Node as DomNode};
use std::collections::{BTreeMap, HashSet};

pub(crate) struct Scope<'d> {
    pub(crate) anchor: ElementRef<'d>,
    pub(crate) extent: Vec<NodeRef<'d, DomNode>>,
}
impl<'d> Scope<'d> {
    pub(crate) fn one(anchor: ElementRef<'d>) -> Self {
        Self {
            anchor,
            extent: vec![*anchor],
        }
    }
}

impl Document {
    /// Runs a compiled contract with fresh time/output policy and returns the whole value or one refusal.
    pub fn extract(&self, contract: &Contract) -> Result<Value, Error> {
        let mut engine = Engine {
            document: self,
            clock: Clock::new(&self.limits),
            output: OutputSize::new(self.limits.max_output_bytes),
        };
        engine.clock.check()?;
        let value = engine.node(&contract.node, &Scope::one(self.html.root_element()), &[])?;
        engine.clock.check()?;
        Ok(value)
    }
}

pub(crate) fn candidates<'d>(
    select: Option<&Selector>,
    scope: &Scope<'d>,
    clock: &Clock,
) -> Result<Vec<ElementRef<'d>>, Error> {
    let Some(select) = select else {
        return Ok(vec![scope.anchor]);
    };
    let mut caches = selectors::matching::SelectorCaches::default();
    let mut found = Vec::new();
    for root in &scope.extent {
        for node in root.descendants() {
            clock.tick()?;
            if let Some(element) = ElementRef::wrap(node) {
                if select.matches(element, scope.anchor, &mut caches) {
                    found.push(element);
                }
            }
        }
    }
    clock.check()?;
    Ok(found)
}

fn range(min: usize, max: Option<usize>) -> String {
    match max {
        Some(max) if max == min => min.to_string(),
        Some(max) => format!("{min}..{max}"),
        None => format!("{min}.."),
    }
}
fn location(mut error: Error, path: &str, rows: &[usize]) -> Error {
    error.path = Some(path.into());
    if !rows.is_empty() {
        error.row = Some(rows.to_vec());
    }
    error
}
enum PredicateFailure {
    Count(usize, String),
    Value(String),
    MissingAttribute(usize, String),
}
struct Engine<'d> {
    document: &'d Document,
    clock: Clock,
    output: OutputSize,
}
impl<'d> Engine<'d> {
    fn holds(
        &self,
        predicate: &Predicate,
        scope: &Scope<'d>,
    ) -> Result<Option<PredicateFailure>, Error> {
        let elements = candidates(predicate.select.as_ref(), scope, &self.clock)?;
        let count = elements.len();
        let expected = range(predicate.min, Some(predicate.max));
        if count < predicate.min || count > predicate.max {
            return Ok(Some(PredicateFailure::Count(count, expected)));
        }
        let tests = predicate.check.equals.is_some() || predicate.check.pattern.is_some();
        for element in elements {
            self.clock.tick()?;
            // An explicitly requested attribute must exist even without a value comparison.
            if !tests && !matches!(predicate.read, crate::contract::Reading::Attr(_)) {
                continue;
            }
            let value = match crate::read::read(
                self.document,
                &Scope::one(element),
                &predicate.read,
                &HashSet::new(),
                &self.clock,
            ) {
                Ok(value) => value,
                Err(e) if e.error == Code::MissingAttribute => {
                    return Ok(Some(PredicateFailure::MissingAttribute(count, expected)));
                }
                Err(e) => return Err(e),
            };
            if predicate.check.equals.as_ref().is_some_and(|s| s != &value)
                || predicate
                    .check
                    .pattern
                    .as_ref()
                    .is_some_and(|p| !p.is_match(&value))
            {
                return Ok(Some(PredicateFailure::Value(value)));
            }
            self.clock.check()?;
        }
        Ok(None)
    }
    fn node(&mut self, node: &Node, scope: &Scope<'d>, rows: &[usize]) -> Result<Value, Error> {
        self.clock.tick()?;
        for predicate in &node.expect {
            if let Some(failure) = self.holds(predicate, scope)? {
                let mut e = location(
                    Error::new(Code::ExpectationFailed, "An expectation failed."),
                    &predicate.path,
                    rows,
                );
                e.select = predicate.select.as_ref().map(|s| s.text.clone());
                match failure {
                    PredicateFailure::Count(found, expected) => {
                        e.message = format!(
                            "{}: selector {:?} found {found}; expected {expected}.",
                            predicate.path, e.select
                        );
                        e.found = Some(found);
                        e.expected = Some(expected);
                    }
                    PredicateFailure::Value(value) => {
                        e = e.observed(&value);
                        e.message = format!(
                            "{}: selector {:?} read {:?}, which fails the expectation.",
                            predicate.path, e.select, e.value
                        );
                    }
                    PredicateFailure::MissingAttribute(found, expected) => {
                        e.message = format!(
                            "{}: selector {:?} found {found}, within {expected}, but its required attribute is missing.",
                            predicate.path, e.select
                        );
                        e.found = Some(found);
                        e.expected = Some(expected);
                    }
                }
                return Err(e);
            }
        }
        let mut found = Vec::new();
        for element in candidates(node.select.as_ref(), scope, &self.clock)? {
            self.clock.tick()?;
            let candidate_scope = Scope::one(element);
            let mut keep = true;
            for predicate in &node.predicates {
                if self.holds(predicate, &candidate_scope)?.is_some() {
                    keep = false;
                    break;
                }
            }
            if keep {
                found.push(element);
            }
        }
        let count = found.len();
        let (acceptable, expected) = match node.cardinality {
            Cardinality::One(Some(n)) => (count >= n, range(n, None)),
            Cardinality::One(None) => (count == 1, String::from("1")),
            Cardinality::Optional => (count <= 1, String::from("0..1")),
            Cardinality::All { min, max } => (
                count >= min && max.is_none_or(|n| count <= n),
                range(min, max),
            ),
        };
        if !acceptable {
            let selector = node.select.as_ref().map(|s| s.text.clone());
            let mut e = location(
                Error::new(
                    Code::CountMismatch,
                    format!(
                        "{}: selector {selector:?} found {count}; expected {expected}.",
                        node.path
                    ),
                ),
                &node.path,
                rows,
            );
            e.select = selector;
            e.found = Some(count);
            e.expected = Some(expected);
            return Err(e);
        }
        if let Cardinality::One(Some(n)) = node.cardinality {
            found = vec![found[n - 1]];
        }
        let all = matches!(node.cardinality, Cardinality::All { .. });
        if all {
            self.output.syntax(1)?;
        }
        let mut values = Vec::new();
        for (index, element) in found.into_iter().enumerate() {
            if all && index > 0 {
                self.output.syntax(1)?;
            }
            let mut here = rows.to_vec();
            if all {
                here.push(index + 1);
            }
            let extent = self.extent(element, node.until.as_ref())?;
            let value = match &node.body {
                Body::Record(fields) => {
                    self.output.syntax(1)?;
                    let mut record = BTreeMap::new();
                    for (index, (name, field)) in fields.iter().enumerate() {
                        if index > 0 {
                            self.output.syntax(1)?;
                        }
                        self.output.value(name, &self.clock)?;
                        self.output.syntax(1)?;
                        record.insert(name.clone(), self.node(field, &extent, &here)?);
                    }
                    self.output.syntax(1)?;
                    Value::Record(record)
                }
                Body::Value {
                    read,
                    exclude,
                    check,
                } => {
                    let mut excluded = HashSet::<NodeId>::new();
                    if matches!(
                        read,
                        crate::contract::Reading::Text
                            | crate::contract::Reading::TextContent
                            | crate::contract::Reading::OuterHtml
                    ) {
                        for selector in exclude {
                            for element in candidates(Some(selector), &extent, &self.clock)? {
                                excluded.insert(element.id());
                            }
                        }
                    }
                    let value =
                        crate::read::read(self.document, &extent, read, &excluded, &self.clock)
                            .map_err(|e| {
                                if matches!(e.error, Code::MissingAttribute | Code::UnresolvableUrl)
                                {
                                    location(e, &node.path, &here)
                                } else {
                                    e
                                }
                            })?;
                    let value = self.value(check, value, &node.path, &here)?;
                    self.output.value(&value, &self.clock)?;
                    Value::String(value)
                }
            };
            values.push(value);
        }
        if all {
            self.output.syntax(1)?;
            return Ok(Value::List(values));
        }
        match values.pop() {
            Some(value) => Ok(value),
            None => {
                self.output.syntax(4)?;
                Ok(Value::Null)
            }
        }
    }
    fn extent(
        &self,
        element: ElementRef<'d>,
        until: Option<&Selector>,
    ) -> Result<Scope<'d>, Error> {
        let mut scope = Scope::one(element);
        if let Some(until) = until {
            let mut next = element.next_sibling();
            let mut caches = selectors::matching::SelectorCaches::default();
            while let Some(node) = next {
                self.clock.tick()?;
                if ElementRef::wrap(node).is_some_and(|e| until.matches(e, element, &mut caches)) {
                    break;
                }
                scope.extent.push(node);
                next = node.next_sibling();
            }
        }
        self.clock.check()?;
        Ok(scope)
    }
    fn value(
        &self,
        check: &Check,
        value: String,
        path: &str,
        rows: &[usize],
    ) -> Result<String, Error> {
        let failure = || {
            location(
                Error::new(
                    Code::ValueMismatch,
                    format!(
                        "{path}: observed value {:?} does not satisfy its declared shape.",
                        value.chars().take(120).collect::<String>()
                    ),
                )
                .observed(&value),
                path,
                rows,
            )
        };
        if check.equals.as_ref().is_some_and(|s| s != &value) {
            return Err(failure());
        }
        if let Some(pattern) = &check.pattern {
            let captures = pattern.captures(&value).ok_or_else(failure)?;
            if pattern.capture_names().any(|name| name == Some("value")) {
                let narrowed = captures
                    .name("value")
                    .ok_or_else(failure)?
                    .as_str()
                    .to_owned();
                self.clock.check()?;
                return Ok(narrowed);
            }
        }
        self.clock.check()?;
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Limits, Source};
    use std::path::Path;

    #[test]
    fn evaluation_conformance_checks_complete_values_and_exact_refusal_facts() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance/contract");
        let mut count = 0;
        for entry in std::fs::read_dir(root).unwrap() {
            let path = entry.unwrap().path();
            let prefix: usize = path.file_name().unwrap().to_str().unwrap()[..2]
                .parse()
                .unwrap();
            if !matches!(prefix, 1..=8 | 20..=32 | 40..=44 | 50..=54 | 68) {
                continue;
            }
            let options: serde_json::Value = std::fs::read(path.join("options.json"))
                .ok()
                .map(|v| serde_json::from_slice(&v).unwrap())
                .unwrap_or_default();
            let bytes = std::fs::read(path.join("page.html")).unwrap();
            let mut source = Source::new(&bytes);
            source.base_url = options["base_url"].as_str();
            let document = Document::parse(source, &Limits::default()).unwrap();
            let contract =
                Contract::from_json(&std::fs::read_to_string(path.join("contract.json")).unwrap())
                    .unwrap();
            match document.extract(&contract) {
                Ok(value) => {
                    let expected: serde_json::Value =
                        serde_json::from_slice(&std::fs::read(path.join("expected.json")).unwrap())
                            .unwrap();
                    assert_eq!(serde_json::to_value(value).unwrap(), expected, "{path:?}");
                }
                Err(error) => {
                    let mut expected: serde_json::Value =
                        serde_json::from_slice(&std::fs::read(path.join("error.json")).unwrap())
                            .unwrap();
                    let mut actual = serde_json::to_value(error).unwrap();
                    expected.as_object_mut().unwrap().remove("message");
                    actual.as_object_mut().unwrap().remove("message");
                    assert_eq!(actual, expected, "{path:?}");
                }
            }
            count += 1;
        }
        assert_eq!(count, 32);
    }
    fn extract(html: &str, contract: &str) -> Result<Value, Error> {
        Document::parse(Source::new(html.as_bytes()), &Limits::default())
            .unwrap()
            .extract(&Contract::from_json(contract).unwrap())
    }
    #[test]
    fn nested_all_rows_report_indexes_and_fields_fail_in_byte_order() {
        let error = extract("<main><section><p>a</p></section><section><p>b</p><p>x</p></section></main>",
            r#"{"select":"section","match":"all","fields":{"items":{"select":"p","match":"all","equals":"b"}}}"#).unwrap_err();
        assert_eq!(error.row.as_deref(), Some([1, 1].as_slice()));
        let error = extract("<main><section><p>a</p></section><section><p>b</p><p>x</p></section></main>",
            r#"{"select":"section","match":"all","fields":{"items":{"select":"p","match":"all","pattern":"^[ab]$"}}}"#).unwrap_err();
        assert_eq!(error.row.as_deref(), Some([2, 2].as_slice()));
        let error = extract(
            "<p>x</p>",
            r#"{"fields":{"z":"missing-z","a":"missing-a"}}"#,
        )
        .unwrap_err();
        assert_eq!(error.path.as_deref(), Some("$.fields.a"));
    }
    #[test]
    fn missing_attribute_predicates_filter_without_becoming_read_errors() {
        assert_eq!(
            extract(
                "<p>a</p><p id=x>b</p>",
                r#"{"select":"p","match":"all","where":[{"read":"attr:id"}]}"#
            )
            .unwrap(),
            Value::List(vec![Value::String("b".into())])
        );
    }
    #[test]
    fn an_unmatched_value_group_refuses_and_an_empty_participating_group_is_valid() {
        let error = extract("<p>b</p>", r#"{"select":"p","pattern":"(?<value>a)|b"}"#).unwrap_err();
        assert_eq!(error.error, Code::ValueMismatch);
        assert_eq!(error.value.as_deref(), Some("b"));
        assert_eq!(
            extract("<p>b</p>", r#"{"select":"p","pattern":"(?<value>a*)b"}"#).unwrap(),
            Value::String(String::new())
        );
    }
    #[test]
    fn every_result_shape_counts_exact_encoded_bytes_before_returning() {
        let html = "<p>é &quot;x&quot;</p><p>b</p>";
        for text in [
            r#""p:nth-child(1)""#,
            r#"{"select":"p","match":"all"}"#,
            r#"{"fields":{"z":{"select":"p","match":"all"},"a":{"select":"none","match":"optional"}}}"#,
        ] {
            let contract = Contract::from_json(text).unwrap();
            let value = Document::parse(Source::new(html.as_bytes()), &Limits::default())
                .unwrap()
                .extract(&contract)
                .unwrap();
            let size = value.to_json(usize::MAX).unwrap().len();
            let mut limits = Limits::default();
            limits.max_output_bytes = size;
            assert!(
                Document::parse(Source::new(html.as_bytes()), &limits)
                    .unwrap()
                    .extract(&contract)
                    .is_ok()
            );
            limits.max_output_bytes -= 1;
            let error = Document::parse(Source::new(html.as_bytes()), &limits)
                .unwrap()
                .extract(&contract)
                .unwrap_err();
            assert_eq!(error.limit.as_deref(), Some("output_bytes"));
            assert_eq!(error.bound, Some((size - 1).into()));
        }
    }
    #[test]
    fn compiled_selectors_reuse_across_documents_without_scope_cache_leaks() {
        let contract = Contract::from_json(r#"{"select":"section","match":"all","fields":{"second":"p:nth-child(2)","self":{"select":":scope","read":"attr:id"}}}"#).unwrap();
        for title in ["a", "b", "a"] {
            let html = format!("<section id={title}><p>first</p><p>{title}</p></section>");
            let document =
                Document::parse(Source::new(html.as_bytes()), &Limits::default()).unwrap();
            let expected = Value::List(vec![Value::Record(BTreeMap::from([
                ("second".into(), Value::String(title.into())),
                ("self".into(), Value::String(title.into())),
            ]))]);
            assert_eq!(document.extract(&contract).unwrap(), expected);
            assert_eq!(document.extract(&contract).unwrap(), expected);
        }
    }
}
