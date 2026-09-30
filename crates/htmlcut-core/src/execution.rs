//! Complete selection, original-DOM guards, and fresh shared operation work.

use std::collections::HashSet;

use ego_tree::NodeId;
use scraper::{ElementRef, Html, Selector};
use selectors::work_budget::SelectorWorkBudget;

use crate::compilation::{CompiledStrategy, Matcher};
use crate::{
    CompiledPlan, ErrorCode, ExtractionError, ExtractionResult, GuardRead, GuardScope, Predicate,
    PreparedDocument, Projection, SCHEMA_VERSION, SEMANTICS_VERSION, Selection, SourceRange,
};

pub(crate) fn charge(budget: &SelectorWorkBudget, units: usize) -> Result<(), ExtractionError> {
    if units > budget.remaining() as usize {
        return Err(ExtractionError::limit("execution"));
    }
    // The preceding capacity check and single-threaded budget ownership prove that
    // each consume succeeds; a second fallible branch here would be unreachable.
    for _ in 0..units {
        let _ = budget.consume();
    }
    Ok(())
}

impl PreparedDocument {
    /// Executes one compiled plan, reusing at most one lazily prepared DOM.
    pub fn execute(&self, compiled: &CompiledPlan) -> Result<ExtractionResult, ExtractionError> {
        let identity = self.extraction_identity(compiled)?;
        self.execute_identified(compiled, identity.clone())
            .map_err(|mut error| {
                error.source_sha256 = Some(self.snapshot.source_sha256().into());
                error.plan_sha256 = Some(compiled.digest.clone());
                error.extraction_sha256 = Some(identity);
                error
            })
    }

    fn execute_identified(
        &self,
        compiled: &CompiledPlan,
        identity: String,
    ) -> Result<ExtractionResult, ExtractionError> {
        let plan = &compiled.plan;
        let budget = SelectorWorkBudget::new(plan.limits.max_work);
        let (values, ranges, candidate_count) = match &compiled.strategy {
            CompiledStrategy::Slice {
                start,
                end,
                include_start,
                include_end,
            } => {
                let ranges = slice_ranges(
                    self.snapshot.html(),
                    start,
                    end,
                    *include_start,
                    *include_end,
                    plan.limits.max_candidates,
                    &budget,
                )?;
                let count = ranges.len() as u32;
                let positions =
                    selected_positions(&plan.selection, count, plan.limits.max_selected)?;
                let mut values = Vec::new();
                let mut selected_ranges = Vec::new();
                let mut remaining = plan.limits.max_total_value_bytes as usize;
                for position in positions {
                    let range = ranges[position];
                    let value = &self.snapshot.html()[range.start..range.end];
                    if value.len() > (plan.limits.max_value_bytes as usize).min(remaining) {
                        return Err(ExtractionError::limit("projection"));
                    }
                    remaining -= value.len();
                    values.push(value.to_owned());
                    selected_ranges.push(range);
                }
                (values, Some(selected_ranges), count)
            }
            CompiledStrategy::Css(selector) => {
                let document = self.document()?;
                let candidates = matches(
                    document,
                    None,
                    selector,
                    plan.limits.max_candidates,
                    &budget,
                )?;
                let count = candidates.len() as u32;
                let positions =
                    selected_positions(&plan.selection, count, plan.limits.max_selected)?;
                let selected = positions
                    .into_iter()
                    .map(|index| candidates[index])
                    .collect::<Vec<_>>();
                check_guards(document, &selected, compiled, &budget)?;
                let mut values = Vec::new();
                let mut remaining = plan.limits.max_total_value_bytes as usize;
                for root in selected {
                    let exclusions = exclusions(root, &compiled.exclusions, &budget)?;
                    let limit = (plan.limits.max_value_bytes as usize).min(remaining);
                    let value = crate::projection::project(
                        root,
                        &plan.projection,
                        &exclusions,
                        &plan.transforms,
                        self.snapshot.metadata().base_url.as_deref(),
                        limit,
                        &budget,
                    )?;
                    remaining -= value.len();
                    values.push(value);
                }
                (values, None, count)
            }
        };
        Ok(ExtractionResult {
            schema: "htmlcut.extraction.result".into(),
            version: SCHEMA_VERSION,
            semantics: SEMANTICS_VERSION,
            source_sha256: self.snapshot.source_sha256().into(),
            plan_sha256: compiled.digest.clone(),
            extraction_sha256: identity,
            candidate_count,
            selected_count: values.len() as u32,
            values,
            ranges,
        })
    }

    fn extraction_identity(&self, compiled: &CompiledPlan) -> Result<String, ExtractionError> {
        let preparation = crate::canonical_json(&self.limits)?;
        let metadata = crate::canonical_json(self.snapshot.metadata())?;
        let semantics = SEMANTICS_VERSION.to_be_bytes();
        let identity = crate::identity::framed(
            "htmlcut.extraction/1",
            &[
                self.snapshot.source_sha256().as_bytes(),
                compiled.digest.as_bytes(),
                metadata.as_bytes(),
                preparation.as_bytes(),
                &semantics,
            ],
        );
        Ok(identity)
    }
}

fn selected_positions(
    selection: &Selection,
    count: u32,
    maximum: u32,
) -> Result<Vec<usize>, ExtractionError> {
    if matches!(selection, Selection::All { .. }) && count > maximum {
        let mut error = ExtractionError::limit("selection");
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
        error
    };
    match selection {
        Selection::Single if count == 0 => Err(ExtractionError::new(
            ErrorCode::NoMatch,
            "selection",
            "The plan selected no candidates.",
        )),
        Selection::Single if count > 1 => {
            let mut error = ExtractionError::new(
                ErrorCode::AmbiguousSelection,
                "selection",
                "Single selection requires exactly one candidate.",
            );
            error.candidate_count = Some(count);
            Err(error)
        }
        Selection::Single => Ok(vec![0]),
        Selection::Nth { index } if *index > count => Err(cardinality()),
        Selection::Nth { index } => Ok(vec![(*index - 1) as usize]),
        Selection::All { min, max }
            if count < *min || count > max.unwrap_or(maximum).min(maximum) =>
        {
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
    let mut result = Vec::new();
    for node in root.descendants() {
        charge(budget, 1)?;
        let Some(element) = ElementRef::wrap(node) else {
            continue;
        };
        // Descendant traversal only visits attached document nodes; the fixed orphan
        // parser sentinel cannot enter this candidate set.
        if selector
            .matches_with_scope_and_budget(&element, scope, budget)
            .map_err(|_| ExtractionError::limit("selection"))?
        {
            if result.len() >= maximum as usize {
                let mut error = ExtractionError::limit("selection");
                error.observed_at_least = Some(maximum + 1);
                return Err(error);
            }
            result.push(element);
        }
    }
    Ok(result)
}

fn exclusions(
    root: ElementRef<'_>,
    selectors: &[Selector],
    budget: &SelectorWorkBudget,
) -> Result<HashSet<NodeId>, ExtractionError> {
    let mut excluded = HashSet::new();
    if selectors.is_empty() {
        return Ok(excluded);
    }
    for node in root.descendants() {
        charge(budget, 1)?;
        let Some(element) = ElementRef::wrap(node) else {
            continue;
        };
        for selector in selectors {
            if selector
                .matches_with_scope_and_budget(&element, Some(root), budget)
                .map_err(|_| ExtractionError::limit("exclusion"))?
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
    selected: &[ElementRef<'_>],
    compiled: &CompiledPlan,
    budget: &SelectorWorkBudget,
) -> Result<(), ExtractionError> {
    for (guard, grammar) in compiled.plan.guards.iter().zip(&compiled.guards) {
        let scopes = match guard.scope {
            GuardScope::Document => vec![None],
            GuardScope::Selected => selected.iter().copied().map(Some).collect(),
        };
        for scope in scopes {
            let nodes = matches(
                document,
                scope,
                &grammar.selector,
                compiled.plan.limits.max_candidates,
                budget,
            )?;
            if nodes.len() < guard.min as usize || nodes.len() > guard.max.unwrap() as usize {
                return Err(guard_failure());
            }
            for node in nodes {
                let projection = match &guard.read {
                    GuardRead::DomText => Projection::DomText,
                    GuardRead::Attribute { name } => Projection::Attribute { name: name.clone() },
                };
                let value = crate::projection::project(
                    node,
                    &projection,
                    &HashSet::new(),
                    &[],
                    None,
                    compiled.plan.limits.max_value_bytes as usize,
                    budget,
                )?;
                let satisfied = match &guard.predicate {
                    None => true,
                    Some(Predicate::Exact { value: expected }) => value == *expected,
                    Some(Predicate::Regex { .. }) => {
                        charge(budget, value.len().div_ceil(64) + 1)?;
                        grammar.predicate.as_ref().unwrap().is_match(&value)
                    }
                };
                if !satisfied {
                    return Err(guard_failure());
                }
            }
        }
    }
    Ok(())
}

fn guard_failure() -> ExtractionError {
    ExtractionError::new(
        ErrorCode::GuardFailed,
        "guard",
        "An original-DOM guard expectation failed.",
    )
}

fn find(
    matcher: &Matcher,
    source: &str,
    cursor: usize,
    budget: &SelectorWorkBudget,
) -> Result<Option<(usize, usize)>, ExtractionError> {
    // Conservative pre-search charging also bounds regex search over an unmatched tail.
    charge(budget, (source.len() - cursor).div_ceil(64) + 1)?;
    let found = matcher.find(source, cursor);
    if found.is_some_and(|(start, end)| start == end) {
        return Err(ExtractionError::new(
            ErrorCode::EmptyBoundaryMatch,
            "slice",
            "Source boundaries must consume at least one byte.",
        ));
    }
    Ok(found)
}

fn slice_ranges(
    source: &str,
    start: &Matcher,
    end: &Matcher,
    include_start: bool,
    include_end: bool,
    maximum: u32,
    budget: &SelectorWorkBudget,
) -> Result<Vec<SourceRange>, ExtractionError> {
    let mut ranges = Vec::new();
    let mut cursor = 0;
    while let Some((open_start, open_end)) = find(start, source, cursor, budget)? {
        let Some((close_start, close_end)) = find(end, source, open_end, budget)? else {
            return Err(ExtractionError::new(
                ErrorCode::MissingBoundary,
                "slice",
                "An opening boundary has no following closing boundary.",
            ));
        };
        if ranges.len() >= maximum as usize {
            return Err(ExtractionError::limit("slice"));
        }
        ranges.push(SourceRange {
            start: if include_start { open_start } else { open_end },
            end: if include_end { close_end } else { close_start },
        });
        cursor = close_end;
    }
    Ok(ranges)
}
