//! Complete selection, original-DOM guards, and fresh shared operation work.

use std::collections::HashSet;

use ego_tree::NodeId;
use scraper::{ElementRef, Html, Selector};
use selectors::work_budget::SelectorWorkBudget;

mod records;

use crate::compilation::{CompiledDomProjection, CompiledStrategy, Matcher};
use crate::{
    CompiledPlan, DataKind, ErrorCode, ExecutionReceipt, ExtractionData, ExtractionError,
    ExtractionResult, GuardRead, GuardScope, Predicate, PreparedDocument, Projection,
    SCHEMA_VERSION, SEMANTICS_VERSION, Selection, SourceRange, ValueProjection,
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
        let plan = &compiled.plan;
        let budget = SelectorWorkBudget::new(plan.limits.max_work);
        let mut bytes = plan.limits.max_total_value_bytes as usize;
        let mut cells = plan.limits.max_cells;
        let mut fields = Vec::new();
        let (data, ranges, candidate_count) = match &compiled.strategy {
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
                spend_cells(&mut cells, positions.len() as u32)?;
                let mut values = Vec::with_capacity(positions.len());
                let mut selected_ranges = Vec::with_capacity(positions.len());
                for position in positions {
                    let range = ranges[position];
                    let value = &self.snapshot.html()[range.start..range.end];
                    if value.len() > (plan.limits.max_value_bytes as usize).min(bytes) {
                        return Err(ExtractionError::limit("projection"));
                    }
                    charge(&budget, value.len().div_ceil(64))?;
                    bytes -= value.len();
                    values.push(value.to_owned());
                    selected_ranges.push(range);
                }
                (ExtractionData::Values(values), Some(selected_ranges), count)
            }
            CompiledStrategy::Css {
                selector,
                projection,
            } => {
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
                let data = match projection {
                    CompiledDomProjection::Records => {
                        let output = records::RecordExecution {
                            document,
                            compiled,
                            base: self.snapshot.metadata().base_url.as_deref(),
                            budget: &budget,
                            bytes: &mut bytes,
                            cells: &mut cells,
                        }
                        .run(&selected)?;
                        fields = output.counts;
                        ExtractionData::Records(output.rows)
                    }
                    CompiledDomProjection::Value(projection) => {
                        spend_cells(&mut cells, selected.len() as u32)?;
                        let mut values = Vec::with_capacity(selected.len());
                        for root in selected {
                            let excluded =
                                exclusions(document, root, &compiled.exclusions, &budget)?;
                            let value = crate::projection::project(
                                root,
                                projection,
                                &excluded,
                                &plan.transforms,
                                self.snapshot.metadata().base_url.as_deref(),
                                (plan.limits.max_value_bytes as usize).min(bytes),
                                &budget,
                            )?;
                            bytes -= value.len();
                            values.push(value);
                        }
                        ExtractionData::Values(values)
                    }
                };
                (data, None, count)
            }
        };
        let (data_kind, selected_count) = match &data {
            ExtractionData::Values(values) => (DataKind::Values, values.len() as u32),
            ExtractionData::Records(rows) => (DataKind::Records, rows.len() as u32),
        };
        let data_sha256 =
            crate::identity::data_digest(&data, crate::limits::MAX_DATA_BYTES, &budget)?;
        let receipt = ExecutionReceipt {
            schema: "htmlcut.extraction.receipt".into(),
            version: SCHEMA_VERSION,
            semantics: SEMANTICS_VERSION,
            data_kind,
            source_sha256: self.snapshot.source_sha256().into(),
            plan_sha256: compiled.digest.clone(),
            extraction_sha256: identity,
            data_sha256,
            candidate_count,
            selected_count,
            fields,
            ranges,
        };
        // This also charges actual receipt serialization work under the same execution budget.
        let _ = crate::identity::data_digest(&receipt, crate::limits::MAX_RECEIPT_BYTES, &budget)?;
        Ok(ExtractionResult { data, receipt })
    }

    fn extraction_identity(&self, compiled: &CompiledPlan) -> Result<String, ExtractionError> {
        let preparation = crate::canonical_json(&self.limits)?;
        let metadata = crate::canonical_json(self.snapshot.metadata())?;
        let semantics = SEMANTICS_VERSION.to_be_bytes();
        let identity = crate::identity::framed(
            "htmlcut.extraction/3",
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

pub(crate) fn selected_positions(
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
        Selection::Single {} if count == 0 => Err(ExtractionError::new(
            ErrorCode::NoMatch,
            "selection",
            "The plan selected no candidates.",
        )),
        Selection::Single {} if count > 1 => {
            let mut error = ExtractionError::new(
                ErrorCode::AmbiguousSelection,
                "selection",
                "Single selection requires exactly one candidate.",
            );
            error.candidate_count = Some(count);
            Err(error)
        }
        Selection::Single {} => Ok(vec![0]),
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
    let mut matcher = selector
        .budgeted(document, scope, budget)
        .map_err(|error| selector_failure(error, "selection"))?;
    for node in root.descendants() {
        charge(budget, 1)?;
        let Some(element) = ElementRef::wrap(node) else {
            continue;
        };
        // Descendant traversal only visits attached document nodes; the fixed orphan
        // parser sentinel cannot enter this candidate set.
        if matcher
            .matches(&element)
            .map_err(|error| selector_failure(error, "selection"))?
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

pub(crate) fn spend_cells(remaining: &mut u32, count: u32) -> Result<(), ExtractionError> {
    *remaining = remaining
        .checked_sub(count)
        .ok_or_else(|| ExtractionError::limit("cells"))?;
    Ok(())
}

fn selector_failure(
    error: scraper::selector::SelectorMatchError,
    stage: &'static str,
) -> ExtractionError {
    match error {
        scraper::selector::SelectorMatchError::WorkLimitExceeded => ExtractionError::limit(stage),
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
        .map_err(|error| selector_failure(error, "exclusion"))?;
    for node in root.descendants() {
        charge(budget, 1)?;
        let Some(element) = ElementRef::wrap(node) else {
            continue;
        };
        for matcher in &mut matchers {
            if matcher
                .matches(&element)
                .map_err(|error| selector_failure(error, "exclusion"))?
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
                if scope.is_some() && matches!(compiled.plan.projection, Projection::Records { .. })
                {
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
    scope: Option<ElementRef<'_>>,
    limits: &crate::ExecutionLimits,
    budget: &SelectorWorkBudget,
) -> Result<(), ExtractionError> {
    let nodes = matches(
        document,
        scope,
        &grammar.selector,
        limits.max_candidates,
        budget,
    )?;
    if nodes.len() < guard.min as usize || nodes.len() > guard.max.unwrap() as usize {
        return Err(guard_failure());
    }
    for node in nodes {
        let projection = match &guard.read {
            GuardRead::DomText {} => ValueProjection::DomText {},
            GuardRead::Attribute { name } => ValueProjection::Attribute { name: name.clone() },
        };
        let value = crate::projection::project(
            node,
            &projection,
            &HashSet::new(),
            &[],
            None,
            limits.max_value_bytes as usize,
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

#[cfg(test)]
#[path = "tests/execution_accounting.rs"]
mod accounting_tests;
