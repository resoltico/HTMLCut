// SPDX-License-Identifier: MPL-2.0
//! Complete selection, original-DOM guards, and fresh shared operation work.

use std::collections::HashSet;

use ego_tree::{NodeId, NodeRef};
use scraper::{ElementRef, Html, Node, Selector};
use selectors::work_budget::SelectorWorkBudget;

mod records;
mod scope;
use scope::SelectionScope;

use crate::plan::Selection;
use crate::{
    CompiledPlan, ErrorCode, ExtractionData, ExtractionError, ExtractionResult, GuardScope,
    PreparedDocument,
};

pub(crate) fn charge(budget: &SelectorWorkBudget, units: usize) -> Result<(), ExtractionError> {
    if units > budget.remaining() as usize {
        return Err(ExtractionError::resource(
            "execution",
            "max_work",
            budget.configured().into(),
        ));
    }
    // The preceding capacity check and single-threaded budget ownership prove that
    // each consume succeeds; a second fallible branch here would be unreachable.
    for _ in 0..units {
        let _ = budget.consume();
    }
    Ok(())
}

impl PreparedDocument {
    /// Executes one compiled query using a fresh budget and the lazily prepared original DOM.
    pub fn execute(&self, compiled: &CompiledPlan) -> Result<ExtractionResult, ExtractionError> {
        let plan = &compiled.plan;
        let budget = SelectorWorkBudget::new(plan.limits.max_work);
        let mut bytes = plan.limits.max_total_value_bytes as usize;
        let mut cells = plan.limits.max_cells;
        let document = self.document()?;
        let candidates = matches(
            document,
            None,
            &compiled.selector,
            plan.limits.max_candidates,
            &budget,
        )?;
        let count = candidates.len() as u32;
        let positions = selected_positions(&plan.selection(), count, plan.limits.max_selected)?;
        let selected = positions
            .into_iter()
            .map(|index| candidates[index])
            .collect::<Vec<_>>();
        let scopes = scope::selected_scopes(
            &selected,
            &candidates,
            plan.following_siblings.unwrap_or(0),
            &budget,
        )?;
        check_guards(document, &scopes, compiled, &budget)?;
        let (data, fields) = if plan.fields.is_some() {
            let output = records::RecordExecution {
                document,
                compiled,
                base: self.snapshot.metadata().base_url.as_deref(),
                budget: &budget,
                bytes: &mut bytes,
                cells: &mut cells,
            }
            .run(&scopes)?;
            (ExtractionData::Records(output.rows), output.counts)
        } else {
            spend_cells(&mut cells, selected.len() as u32, plan.limits.max_cells)?;
            let mut values = Vec::with_capacity(selected.len());
            for scope in scopes {
                let excluded = exclusions(document, scope.anchor, &compiled.exclusions, &budget)?;
                let value = crate::projection::project(
                    scope.anchor,
                    plan.read.as_ref().expect("normalized scalar reading"),
                    &excluded,
                    self.snapshot.metadata().base_url.as_deref(),
                    (plan.limits.max_value_bytes as usize).min(bytes),
                    &budget,
                )
                .map_err(|error| projection_failure(error, &plan.limits, bytes))?;
                bytes -= value.len();
                values.push(value);
            }
            (
                ExtractionData::Values(values),
                std::collections::BTreeMap::new(),
            )
        };
        let selected_count = selected.len() as u32;
        let payload = crate::identity::encoded(&data, crate::MAX_DATA_BYTES, &budget)?;
        Ok(ExtractionResult {
            data,
            payload,
            source: self.snapshot.clone(),
            query: compiled.normalized_bytes.clone(),
            preparation: self.limits.clone(),
            candidate_count: count,
            selected_count,
            fields,
            budget,
            evidence: std::cell::OnceCell::new(),
        })
    }
}

pub(crate) fn projection_failure(
    mut error: ExtractionError,
    limits: &crate::ExecutionLimits,
    remaining: usize,
) -> ExtractionError {
    if error.resource_counter.as_deref() == Some("value_bytes") {
        let (counter, bound) = if limits.max_value_bytes as usize <= remaining {
            ("max_value_bytes", limits.max_value_bytes)
        } else {
            ("max_total_value_bytes", limits.max_total_value_bytes)
        };
        let facts = ExtractionError::resource("projection", counter, bound.into());
        error.resource_counter = facts.resource_counter.clone();
        error.configured_bound = facts.configured_bound;
        error.message = facts.message;
    }
    error
}

pub(crate) fn selected_positions(
    selection: &Selection,
    count: u32,
    maximum: u32,
) -> Result<Vec<usize>, ExtractionError> {
    if matches!(selection, Selection::All { .. }) && count > maximum {
        let mut error = ExtractionError::resource("selection", "max_selected", maximum.into());
        error.candidate_count = Some(count);
        return Err(error);
    }
    let cardinality = || {
        let mut error = ExtractionError::new(
            ErrorCode::Cardinality,
            "selection",
            "Candidate count does not meet the declared selection bounds.",
        );
        error.candidate_count = Some(count);
        match selection {
            Selection::Single => {
                error.expected_min = Some(1);
                error.expected_max = Some(1);
            }
            Selection::All { min, max } => {
                error.expected_min = Some(*min);
                error.expected_max = *max;
            }
            Selection::Nth { index } => {
                error.required_index = Some(*index);
            }
        }
        error
    };
    match selection {
        Selection::Single if count == 0 => {
            let mut error = cardinality();
            error.code = ErrorCode::NoMatch;
            error.message = "Exactly-one selection matched zero candidates.".into();
            Err(error)
        }
        Selection::Single if count > 1 => {
            let mut error = cardinality();
            error.code = ErrorCode::AmbiguousSelection;
            error.message = "Exactly-one selection matched multiple candidates.".into();
            Err(error)
        }
        Selection::Single => Ok(vec![0]),
        Selection::Nth { index } if *index > count => Err(cardinality()),
        Selection::Nth { index } => Ok(vec![(*index - 1) as usize]),
        Selection::All { min, max } if count < *min || max.is_some_and(|max| count > max) => {
            Err(cardinality())
        }
        Selection::All { .. } => Ok((0..count as usize).collect()),
    }
}

pub(crate) fn matches<'a>(
    document: &'a Html,
    scope: Option<ElementRef<'a>>,
    selector: &Selector,
    maximum: u32,
    budget: &SelectorWorkBudget,
) -> Result<Vec<ElementRef<'a>>, ExtractionError> {
    let root = scope
        .map(|element| *element)
        .unwrap_or_else(|| document.tree.root());
    matches_payloads(
        document,
        scope,
        std::iter::once(root),
        selector,
        maximum,
        budget,
    )
}

pub(crate) fn matches_scope<'a>(
    document: &'a Html,
    scope: Option<&SelectionScope<'a>>,
    selector: &Selector,
    maximum: u32,
    budget: &SelectorWorkBudget,
) -> Result<Vec<ElementRef<'a>>, ExtractionError> {
    match scope {
        Some(scope) => matches_payloads(
            document,
            Some(scope.anchor),
            std::iter::once(*scope.anchor)
                .chain(scope.following_siblings.iter().map(|root| **root)),
            selector,
            maximum,
            budget,
        ),
        None => matches(document, None, selector, maximum, budget),
    }
}

fn matches_payloads<'a>(
    document: &'a Html,
    anchor: Option<ElementRef<'a>>,
    roots: impl Iterator<Item = NodeRef<'a, Node>>,
    selector: &Selector,
    maximum: u32,
    budget: &SelectorWorkBudget,
) -> Result<Vec<ElementRef<'a>>, ExtractionError> {
    let mut result = Vec::new();
    let mut matcher = selector
        .budgeted(document, anchor, budget)
        .map_err(|error| selector_failure(error, "selection", budget))?;
    for root in roots {
        for node in root.descendants() {
            charge(budget, 1)?;
            let Some(element) = ElementRef::wrap(node) else {
                continue;
            };
            if matcher
                .matches(&element)
                .map_err(|error| selector_failure(error, "selection", budget))?
            {
                if result.len() >= maximum as usize {
                    let mut error =
                        ExtractionError::resource("selection", "max_candidates", maximum.into());
                    error.observed_at_least = Some(maximum + 1);
                    return Err(error);
                }
                result.push(element);
            }
        }
    }
    Ok(result)
}

pub(crate) fn spend_cells(
    remaining: &mut u32,
    count: u32,
    configured: u32,
) -> Result<(), ExtractionError> {
    *remaining = remaining
        .checked_sub(count)
        .ok_or_else(|| ExtractionError::resource("cells", "max_cells", configured.into()))?;
    Ok(())
}

fn selector_failure(
    error: scraper::selector::SelectorMatchError,
    stage: &'static str,
    budget: &SelectorWorkBudget,
) -> ExtractionError {
    match error {
        scraper::selector::SelectorMatchError::WorkLimitExceeded => {
            ExtractionError::resource(stage, "max_work", budget.configured().into())
        }
        scraper::selector::SelectorMatchError::DocumentMismatch => ExtractionError::new(
            ErrorCode::InternalInvariant,
            stage,
            "Selector scope and candidates must belong to one document.",
        ),
    }
}

pub(crate) fn exclusions(
    document: &Html,
    root: ElementRef<'_>,
    selectors: &[Selector],
    budget: &SelectorWorkBudget,
) -> Result<HashSet<NodeId>, ExtractionError> {
    let mut excluded = HashSet::new();
    if selectors.is_empty() {
        return Ok(excluded);
    }
    let mut matchers: Vec<_> = selectors
        .iter()
        .map(|selector| selector.budgeted(document, Some(root), budget))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| selector_failure(error, "exclusion", budget))?;
    for node in root.descendants() {
        charge(budget, 1)?;
        let Some(element) = ElementRef::wrap(node) else {
            continue;
        };
        for matcher in &mut matchers {
            if matcher
                .matches(&element)
                .map_err(|error| selector_failure(error, "exclusion", budget))?
            {
                excluded.insert(node.id());
                break;
            }
        }
    }
    Ok(excluded)
}

fn check_guards(
    document: &Html,
    selected: &[SelectionScope<'_>],
    compiled: &CompiledPlan,
    budget: &SelectorWorkBudget,
) -> Result<(), ExtractionError> {
    for (guard_index, (guard, grammar)) in compiled
        .plan
        .expect
        .iter()
        .zip(&compiled.guards)
        .enumerate()
    {
        let scopes = match guard.scope {
            GuardScope::Document => vec![None],
            GuardScope::Selected => selected.iter().map(Some).collect(),
        };
        for (index, scope) in scopes.into_iter().enumerate() {
            check_guard_scope(
                document,
                guard,
                grammar,
                scope,
                &compiled.plan.limits,
                budget,
            )
            .map_err(|mut error| {
                error.plan_path = Some(format!("expect[{guard_index}]"));
                if scope.is_some() && compiled.plan.fields.is_some() {
                    error.row_index = Some(index as u32 + 1);
                }
                error
            })?;
        }
    }
    Ok(())
}

fn check_guard_scope(
    document: &Html,
    guard: &crate::Guard,
    grammar: &crate::compilation::CompiledGuard,
    scope: Option<&SelectionScope<'_>>,
    limits: &crate::ExecutionLimits,
    budget: &SelectorWorkBudget,
) -> Result<(), ExtractionError> {
    let nodes = matches_scope(
        document,
        scope,
        &grammar.selector,
        limits.max_candidates,
        budget,
    )?;
    if nodes.len() < guard.min as usize || nodes.len() > guard.max as usize {
        return Err(guard_failure(guard, nodes.len() as u32));
    }
    if guard.equals.is_none() && guard.pattern.is_none() {
        return Ok(());
    }
    let count = nodes.len() as u32;
    for node in nodes {
        let value = crate::projection::project(
            node,
            guard.read.as_ref().expect("normalized predicate reading"),
            &HashSet::new(),
            None,
            limits.max_value_bytes as usize,
            budget,
        )
        .map_err(|error| projection_failure(error, limits, limits.max_value_bytes as usize))?;
        let satisfied = if let Some(expected) = &guard.equals {
            value == *expected
        } else {
            charge(budget, value.len().div_ceil(64) + 1)?;
            grammar
                .predicate
                .as_ref()
                .expect("compiled regex predicate")
                .is_match(&value)
        };
        if !satisfied {
            return Err(guard_failure(guard, count));
        }
    }
    Ok(())
}

fn guard_failure(guard: &crate::Guard, count: u32) -> ExtractionError {
    let mut error = ExtractionError::new(
        ErrorCode::GuardFailed,
        "guard",
        "An original-DOM expectation failed.",
    );
    error.candidate_count = Some(count);
    error.expected_min = Some(guard.min);
    error.expected_max = Some(guard.max);
    error
}

#[cfg(test)]
#[path = "tests/execution_accounting.rs"]
mod accounting_tests;
