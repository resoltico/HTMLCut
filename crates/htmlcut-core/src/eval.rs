// SPDX-License-Identifier: MPL-2.0
//! Scope-relative recursive contract evaluation, with complete-result admission.
use crate::dom::{ElementRef, Node as DomNode};
use crate::{
    Contract, Document, Error, ErrorCode, Value,
    contract::{Body, Cardinality, Check, Node, Predicate},
    limits::ExecutionContext,
    select::Selector,
    value::OutputSize,
};
use ego_tree::{NodeId, NodeRef};
use std::collections::{BTreeMap, HashSet};

#[derive(Clone, Copy)]
pub(crate) struct Scope<'d> {
    pub(crate) anchor: ElementRef<'d>,
    end: Option<NodeRef<'d, DomNode>>,
}
impl<'d> Scope<'d> {
    pub(crate) fn one(anchor: ElementRef<'d>) -> Self {
        Self {
            anchor,
            end: anchor.next_sibling(),
        }
    }
    pub(crate) fn roots(&self) -> impl Iterator<Item = NodeRef<'d, DomNode>> {
        let end = self.end.map(|n| n.id());
        std::iter::successors(Some(*self.anchor), move |n| {
            n.next_sibling().filter(|n| Some(n.id()) != end)
        })
    }
    pub(crate) fn single(&self) -> bool {
        self.anchor.next_sibling().map(|n| n.id()) == self.end.map(|n| n.id())
    }
    fn contains(&self, node: NodeRef<'d, DomNode>, document: &Document) -> bool {
        let (start, _) = document.positions[&self.anchor.id()];
        let end = self
            .end
            .map(|n| document.positions[&n.id()].0)
            .unwrap_or_else(|| {
                self.anchor
                    .parent()
                    .map(|p| document.positions[&p.id()].1)
                    .unwrap_or(document.positions[&self.anchor.id()].1)
            });
        let position = document.positions[&node.id()].0;
        position >= start && position < end
    }
}

impl Document {
    /// Runs a compiled contract under the supplied execution policy/context, returning the whole value or one refusal.
    pub fn extract(
        &self,
        contract: &Contract,
        limits: &crate::ExecutionPolicy,
        context: &crate::ExecutionContext,
    ) -> Result<Value, Error> {
        let mut engine = Engine {
            document: self,
            reading_limit: limits.max_reading_bytes,
            clock: (context).clone(),
            output: OutputSize::new(limits.max_output_bytes),
            caches: Default::default(),
        };
        engine.clock.check()?;
        engine.output.syntax(1)?; // Complete canonical export includes its final LF.
        let value = engine.node(&contract.node, &Scope::one(self.html.root_element()), &[])?;
        engine.clock.check()?;
        Ok(value)
    }
}

pub(crate) fn candidates<'d>(
    select: Option<&Selector>,
    scope: &Scope<'d>,
    clock: &ExecutionContext,
    shared: &mut selectors::matching::SelectorCaches,
) -> Result<Vec<ElementRef<'d>>, Error> {
    let Some(select) = select else {
        return Ok(vec![scope.anchor]);
    };
    if select.only_scope() {
        clock.check()?;
        return Ok(vec![scope.anchor]);
    }
    shared.relative_selector = Default::default();
    shared.relative_selector_filter_map = Default::default();
    let caches = shared;
    let mut found = Vec::new();
    for root in scope.roots() {
        let mut skipped = 0;
        for edge in root.traverse() {
            clock.tick()?;
            match edge {
                ego_tree::iter::Edge::Open(node)
                    if skipped > 0 || matches!(node.value(), DomNode::Fragment) =>
                {
                    skipped += 1;
                }
                ego_tree::iter::Edge::Open(node) => {
                    if let Some(element) = ElementRef::wrap(node)
                        && select.matches(element, scope.anchor, caches)
                    {
                        found.push(element);
                    }
                }
                ego_tree::iter::Edge::Close(_) if skipped > 0 => skipped -= 1,
                ego_tree::iter::Edge::Close(_) => (),
            }
        }
    }
    clock.check()?;
    Ok(found)
}

fn needs_range(node: &Node) -> bool {
    let members = |select: Option<&Selector>| select.is_some_and(|s| !s.only_scope());
    if members(node.select.as_ref())
        || node
            .expect
            .iter()
            .any(|predicate| members(predicate.select.as_ref()))
    {
        return true;
    }
    match &node.body {
        Body::Value { read, .. } => !matches!(
            read,
            crate::contract::Reading::Attr(_) | crate::contract::Reading::Url(_)
        ),
        Body::Record(fields) => fields.values().any(needs_range),
    }
}

fn location(mut error: Error, path: &str, rows: &[usize]) -> Error {
    match &mut error {
        Error::MissingAttribute { path: at, row }
        | Error::UnresolvableUrl { path: at, row, .. } => {
            *at = path.into();
            *row = rows.to_vec();
        }
        _ => (),
    }
    error
}
type PredicateFailure = crate::ExpectationFailure;
struct Engine<'d> {
    document: &'d Document,
    reading_limit: usize,
    clock: ExecutionContext,
    output: OutputSize,
    caches: selectors::matching::SelectorCaches,
}
impl<'d> Engine<'d> {
    fn holds(
        &mut self,
        predicate: &Predicate,
        scope: &Scope<'d>,
    ) -> Result<Option<PredicateFailure>, Error> {
        let elements = candidates(
            predicate.select.as_ref(),
            scope,
            &self.clock,
            &mut self.caches,
        )?;
        let count = elements.len();
        let expected = crate::CountRange {
            min: predicate.min,
            max: Some(predicate.max),
        };
        if count < predicate.min || count > predicate.max {
            return Ok(Some(PredicateFailure::Count {
                found: count,
                expected,
            }));
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
                self.reading_limit,
                &Scope::one(element),
                &predicate.read,
                &HashSet::new(),
                &self.clock,
                None,
            ) {
                Ok(value) => value,
                Err(e) if e.code() == ErrorCode::MissingAttribute => {
                    return Ok(Some(PredicateFailure::MissingAttribute));
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
                return Ok(Some(PredicateFailure::Value {
                    value: crate::error::preview(&value),
                }));
            }
            self.clock.check()?;
        }
        Ok(None)
    }
    fn node(&mut self, node: &Node, scope: &Scope<'d>, rows: &[usize]) -> Result<Value, Error> {
        self.clock.tick()?;
        for predicate in &node.expect {
            if let Some(failure) = self.holds(predicate, scope)? {
                return Err(Error::ExpectationFailed {
                    path: predicate.path.clone(),
                    select: predicate.select.as_ref().map(|s| s.text.clone()),
                    row: rows.to_vec(),
                    failure,
                });
            }
        }
        let mut found = Vec::new();
        for element in candidates(node.select.as_ref(), scope, &self.clock, &mut self.caches)? {
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
        let expected = match node.cardinality {
            Cardinality::One(Some(n)) => crate::CountRange { min: n, max: None },
            Cardinality::One(None) => crate::CountRange {
                min: 1,
                max: Some(1),
            },
            Cardinality::Optional => crate::CountRange {
                min: 0,
                max: Some(1),
            },
            Cardinality::All { min, max } => crate::CountRange { min, max },
        };
        if count < expected.min || expected.max.is_some_and(|max| count > max) {
            return Err(Error::CountMismatch {
                path: node.path.clone(),
                select: node.select.as_ref().map(|s| s.text.clone()),
                row: rows.to_vec(),
                found: count,
                expected,
            });
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
            let structural = match &node.body {
                Body::Value { read, .. } => !matches!(
                    read,
                    crate::contract::Reading::Attr(_) | crate::contract::Reading::Url(_)
                ),
                Body::Record(fields) => fields.values().any(needs_range),
            };
            let extent = if node.select.is_none() {
                *scope
            } else if structural {
                self.extent(element, node.until.as_ref(), scope)?
            } else {
                Scope::one(element)
            };
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
                            for element in
                                candidates(Some(selector), &extent, &self.clock, &mut self.caches)?
                            {
                                excluded.insert(element.id());
                            }
                        }
                    }
                    let value = crate::read::read(
                        self.document,
                        self.reading_limit,
                        &extent,
                        read,
                        &excluded,
                        &self.clock,
                        check
                            .pattern
                            .is_none()
                            .then_some((self.output.remaining(), self.output.maximum())),
                    )
                    .map_err(|e| {
                        if matches!(
                            e.code(),
                            ErrorCode::MissingAttribute | ErrorCode::UnresolvableUrl
                        ) {
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
        &mut self,
        element: ElementRef<'d>,
        until: Option<&Selector>,
        inherited: &Scope<'d>,
    ) -> Result<Scope<'d>, Error> {
        let mut scope = Scope::one(element);
        if let Some(until) = until {
            let mut next = element.next_sibling();
            self.caches.relative_selector = Default::default();
            self.caches.relative_selector_filter_map = Default::default();
            let caches = &mut self.caches;
            while let Some(node) = next {
                self.clock.tick()?;
                if !inherited.contains(node, self.document) {
                    break;
                }
                if ElementRef::wrap(node).is_some_and(|e| until.matches(e, element, caches)) {
                    break;
                }
                scope.end = node.next_sibling();
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
        let failure = || Error::ValueMismatch {
            path: path.into(),
            row: rows.to_vec(),
            value: crate::error::preview(&value),
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
    fn extract(html: &str, contract: &str) -> Result<Value, Error> {
        Document::parse(
            Source::new(html.as_bytes()),
            &Limits::default().preparation(),
            &crate::ExecutionContext::new(Limits::default().time_limit).unwrap(),
        )
        .unwrap()
        .extract(
            &Contract::from_json(contract, &crate::ExecutionContext::new(None).unwrap()).unwrap(),
            &crate::Limits::default().execution(),
            &crate::ExecutionContext::new((crate::Limits::default()).time_limit).unwrap(),
        )
    }

    #[test]
    fn a_selector_list_starting_with_scope_keeps_its_other_members() {
        assert_eq!(
            extract(
                "<html id=h><body><p id=p>x</p></body></html>",
                r#"{"select":":scope, p","match":"all","read":"attr:id"}"#
            )
            .unwrap(),
            Value::List(vec![Value::String("h".into()), Value::String("p".into())])
        );
    }

    #[test]
    fn template_anchor_in_a_range_is_not_a_single_template_reading() {
        let html = "<main><template><p>inert</p></template><p>live</p><h2>stop</h2></main>";
        for read in ["text", "text-content"] {
            let contract =
                serde_json::json!({"select":"template","until":"h2","read":read}).to_string();
            assert_eq!(
                extract(html, &contract).unwrap(),
                Value::String("live".into()),
                "{read}"
            );
        }
    }

    #[test]
    fn each_ranged_field_preserves_its_independent_extent_requirement() {
        let html = "<main><h2 id=anchor>Title</h2><p id=body>Body</p><h2>Stop</h2></main>";
        for (field, expected) in [
            (serde_json::json!({"select":"p","read":"attr:id"}), "body"),
            (
                serde_json::json!({"read":"attr:id","expect":[{"select":"p","min":1,"max":1}]}),
                "anchor",
            ),
            (serde_json::json!({"read":"text-content"}), "TitleBody"),
        ] {
            let contract =
                serde_json::json!({"select":"#anchor","until":"h2","fields":{"field":field}})
                    .to_string();
            assert_eq!(
                extract(html, &contract).unwrap(),
                Value::Record(BTreeMap::from([(
                    "field".into(),
                    Value::String(expected.into())
                )]))
            );
        }
    }

    #[test]
    fn anchor_only_records_do_not_visit_irrelevant_sibling_tails() {
        for field in [
            serde_json::json!({"read":"attr:id"}),
            serde_json::json!({"select":":scope","read":"attr:id"}),
        ] {
            let mut work = Vec::new();
            for tail in [0, 2048] {
                let html = format!(
                    "<main><h2 id=anchor>Title</h2>{}<h2 class=stop>Stop</h2></main>",
                    "<p>irrelevant</p>".repeat(tail)
                );
                let document = Document::parse(
                    Source::new(html.as_bytes()),
                    &Limits::default().preparation(),
                    &crate::ExecutionContext::new(Limits::default().time_limit).unwrap(),
                )
                .unwrap();
                let anchor = document
                    .html
                    .select(&Selector::parse("#anchor").unwrap())
                    .next()
                    .unwrap();
                let scope = Scope { anchor, end: None };
                let contract = Contract::from_json(
                    &serde_json::json!({"select":":scope","until":".stop","fields":{"id":field}})
                        .to_string(),
                    &crate::ExecutionContext::new(None).unwrap(),
                )
                .unwrap();
                let limits = Limits {
                    time_limit: None,
                    ..Limits::default()
                };
                let mut engine = Engine {
                    document: &document,
                    reading_limit: limits.max_reading_bytes,
                    clock: ExecutionContext::new(limits.time_limit).unwrap(),
                    output: OutputSize::new(limits.max_output_bytes),
                    caches: Default::default(),
                };
                assert_eq!(
                    engine.node(&contract.node, &scope, &[]).unwrap(),
                    Value::Record(BTreeMap::from([(
                        "id".into(),
                        Value::String("anchor".into())
                    )]))
                );
                work.push(engine.clock.work_ticks());
                // Positive control: a genuinely requested extent does visit the tail.
                let before = engine.clock.work_ticks();
                let extent = engine
                    .extent(anchor, Some(&Selector::parse(".stop").unwrap()), &scope)
                    .unwrap();
                assert_eq!(extent.roots().count(), tail + 1);
                assert!(engine.clock.work_ticks() - before > tail);
            }
            assert_eq!(
                work[0], work[1],
                "irrelevant tail must not add work for {field}"
            );
        }
    }

    #[test]
    fn predicate_reading_preserves_a_cooperative_time_refusal() {
        let document = Document::parse(
            Source::new(b"<p id=x>x</p>"),
            &Limits::default().preparation(),
            &crate::ExecutionContext::new(Limits::default().time_limit).unwrap(),
        )
        .unwrap();
        let contract = Contract::from_json(
            r#"{"expect":[{"read":"attr:id"}]}"#,
            &crate::ExecutionContext::new(None).unwrap(),
        )
        .unwrap();
        let limits = Limits {
            time_limit: Some(std::time::Duration::ZERO),
            ..Limits::default()
        };
        let mut engine = Engine {
            document: &document,
            reading_limit: limits.max_reading_bytes,
            clock: ExecutionContext::new(limits.time_limit).unwrap(),
            output: OutputSize::new(limits.max_output_bytes),
            caches: Default::default(),
        };
        let scope = Scope::one(
            document
                .html
                .select(&crate::select::Selector::parse("p").unwrap())
                .next()
                .unwrap(),
        );
        // A direct predicate call isolates the reading boundary from the preceding extract/candidate checks.
        let error = match engine.holds(&contract.node.expect[0], &scope) {
            Err(error) => error,
            Ok(_) => {
                panic!("an expired reading must refuse instead of becoming a predicate mismatch")
            }
        };
        assert_eq!(error.code(), ErrorCode::LimitExceeded);
        assert_eq!(
            serde_json::to_value(&error).unwrap()["limit"].as_str(),
            Some("time")
        );
    }
    #[test]
    fn template_contents_stay_outside_every_selection_context() {
        let html = "<main><p id=live>live</p><template id=t><p>inert</p><template><p>nested</p></template><h2>boundary</h2></template><p>end</p><h2>stop</h2></main>";
        for selector in ["p", "main p"] {
            let contract = serde_json::json!({"select":selector,"match":"all"}).to_string();
            assert_eq!(
                extract(html, &contract).unwrap(),
                Value::List(vec![
                    Value::String("live".into()),
                    Value::String("end".into())
                ])
            );
        }
        assert_eq!(extract(html, r#"{"select":"main","where":[{"select":"p","min":2,"max":2}],"expect":[{"select":"p","min":2,"max":2}],"read":"text-content"}"#).unwrap(), Value::String("liveendstop".into()));
        assert_eq!(
            extract(html, r##"{"select":"#t","read":"text-content"}"##).unwrap(),
            Value::String("inertboundary".into())
        );
        assert_eq!(
            extract(
                html,
                r##"{"select":"#live","until":"h2","read":"text-content"}"##
            )
            .unwrap(),
            Value::String("liveend".into())
        );
        let plain = extract(html, r#"{"select":"main","read":"outer-html"}"#).unwrap();
        let excluded = extract(html, r#"{"select":"main","read":"outer-html","exclude":["template p","template template","template h2"]}"#).unwrap();
        assert_eq!(plain, excluded);
        assert_eq!(
            extract(
                html,
                r#"{"select":"main:has(template p)","match":"optional"}"#
            )
            .unwrap(),
            Value::Null
        );
    }
    #[test]
    fn nested_all_rows_report_indexes_and_fields_fail_in_byte_order() {
        let error = extract("<main><section><p>a</p></section><section><p>b</p><p>x</p></section></main>",
            r#"{"select":"section","match":"all","fields":{"items":{"select":"p","match":"all","equals":"b"}}}"#).unwrap_err();
        assert_eq!(
            serde_json::from_value::<Option<Vec<usize>>>(
                serde_json::to_value(&error).unwrap()["row"].clone()
            )
            .unwrap()
            .as_deref(),
            Some([1, 1].as_slice())
        );
        let error = extract("<main><section><p>a</p></section><section><p>b</p><p>x</p></section></main>",
            r#"{"select":"section","match":"all","fields":{"items":{"select":"p","match":"all","pattern":"^[ab]$"}}}"#).unwrap_err();
        assert_eq!(
            serde_json::from_value::<Option<Vec<usize>>>(
                serde_json::to_value(&error).unwrap()["row"].clone()
            )
            .unwrap()
            .as_deref(),
            Some([2, 2].as_slice())
        );
        let error = extract(
            "<p>x</p>",
            r#"{"fields":{"z":"missing-z","a":"missing-a"}}"#,
        )
        .unwrap_err();
        assert_eq!(
            serde_json::to_value(&error).unwrap()["path"].as_str(),
            Some("$.fields.a")
        );
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
        assert_eq!(error.code(), ErrorCode::ValueMismatch);
        assert_eq!(
            serde_json::to_value(&error).unwrap()["value"].as_str(),
            Some("b")
        );
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
            let contract =
                Contract::from_json(text, &crate::ExecutionContext::new(None).unwrap()).unwrap();
            let value = Document::parse(
                Source::new(html.as_bytes()),
                &Limits::default().preparation(),
                &crate::ExecutionContext::new(Limits::default().time_limit).unwrap(),
            )
            .unwrap()
            .extract(
                &contract,
                &crate::Limits::default().execution(),
                &crate::ExecutionContext::new((crate::Limits::default()).time_limit).unwrap(),
            )
            .unwrap();
            let size = value
                .export(
                    crate::ExportFormat::Json,
                    usize::MAX,
                    &crate::ExecutionContext::new(None).unwrap(),
                )
                .unwrap()
                .len();
            let mut limits = Limits {
                max_output_bytes: size,
                ..Limits::default()
            };
            assert!(
                Document::parse(
                    Source::new(html.as_bytes()),
                    &limits.preparation(),
                    &crate::ExecutionContext::new(limits.time_limit).unwrap()
                )
                .unwrap()
                .extract(
                    &contract,
                    &limits.execution(),
                    &crate::ExecutionContext::new((limits).time_limit).unwrap()
                )
                .is_ok()
            );
            limits.max_output_bytes -= 1;
            let error = Document::parse(
                Source::new(html.as_bytes()),
                &limits.preparation(),
                &crate::ExecutionContext::new(limits.time_limit).unwrap(),
            )
            .unwrap()
            .extract(
                &contract,
                &limits.execution(),
                &crate::ExecutionContext::new((limits).time_limit).unwrap(),
            )
            .unwrap_err();
            assert_eq!(
                serde_json::to_value(&error).unwrap()["limit"].as_str(),
                Some("output_bytes")
            );
            assert_eq!(
                serde_json::to_value(&error).unwrap().get("bound").cloned(),
                Some((size - 1).into())
            );
        }
    }
    #[test]
    fn relative_selector_answers_remain_local_to_each_row() {
        let contract = Contract::from_json(r#"{"select":"section","match":"all","min":0,"where":[{"select":":scope:has(> b)"}],"read":"attr:id"}"#, &crate::ExecutionContext::new(None).unwrap()).unwrap();
        let document = Document::parse(Source::new(b"<section id=a><b>x</b></section><section id=b><i>x</i></section><section id=c><b>x</b></section>"), &Limits::default().preparation(), &crate::ExecutionContext::new(Limits::default().time_limit).unwrap()).unwrap();
        for _ in 0..2 {
            assert_eq!(
                document
                    .extract(
                        &contract,
                        &crate::Limits::default().execution(),
                        &crate::ExecutionContext::new((crate::Limits::default()).time_limit)
                            .unwrap()
                    )
                    .unwrap(),
                Value::List(vec![Value::String("a".into()), Value::String("c".into())])
            );
        }
        assert!(
            Contract::from_json(
                r#"{"select":"p:nth-child(2 of :scope)"}"#,
                &crate::ExecutionContext::new(None).unwrap()
            )
            .is_err()
        );
    }
    #[test]
    fn compiled_selectors_reuse_across_documents_without_scope_cache_leaks() {
        let contract = Contract::from_json(r#"{"select":"section","match":"all","fields":{"second":"p:nth-child(2)","self":{"select":":scope","read":"attr:id"}}}"#, &crate::ExecutionContext::new(None).unwrap()).unwrap();
        for title in ["a", "b", "a"] {
            let html = format!("<section id={title}><p>first</p><p>{title}</p></section>");
            let document = Document::parse(
                Source::new(html.as_bytes()),
                &Limits::default().preparation(),
                &crate::ExecutionContext::new(Limits::default().time_limit).unwrap(),
            )
            .unwrap();
            let expected = Value::List(vec![Value::Record(BTreeMap::from([
                ("second".into(), Value::String(title.into())),
                ("self".into(), Value::String(title.into())),
            ]))]);
            assert_eq!(
                document
                    .extract(
                        &contract,
                        &crate::Limits::default().execution(),
                        &crate::ExecutionContext::new((crate::Limits::default()).time_limit)
                            .unwrap()
                    )
                    .unwrap(),
                expected
            );
            assert_eq!(
                document
                    .extract(
                        &contract,
                        &crate::Limits::default().execution(),
                        &crate::ExecutionContext::new((crate::Limits::default()).time_limit)
                            .unwrap()
                    )
                    .unwrap(),
                expected
            );
        }
    }
}
