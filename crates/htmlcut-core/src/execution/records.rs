// SPDX-License-Identifier: MPL-2.0
//! Row-relative scalar fields with shared byte, cell and selector-work allowances.

use std::collections::BTreeMap;

use scraper::Html;
use selectors::work_budget::SelectorWorkBudget;

use crate::plan::Selection;
use crate::{CompiledPlan, ExtractionError, FieldCount, FieldMatch, FieldValue, RecordField};

pub(crate) struct RecordExecution<'a> {
    pub(crate) document: &'a Html,
    pub(crate) compiled: &'a CompiledPlan,
    pub(crate) base: Option<&'a str>,
    pub(crate) budget: &'a SelectorWorkBudget,
    pub(crate) bytes: &'a mut usize,
    pub(crate) cells: &'a mut u32,
}

pub(crate) struct RecordOutput {
    pub(crate) rows: Vec<BTreeMap<String, FieldValue>>,
    pub(crate) counts: BTreeMap<String, FieldCount>,
}

impl RecordExecution<'_> {
    pub(crate) fn run(
        &mut self,
        roots: &[super::SelectionScope<'_>],
    ) -> Result<RecordOutput, ExtractionError> {
        let fields = &self.compiled.fields;
        let mut counts = fields
            .iter()
            .map(|field| {
                (
                    field.name.clone(),
                    FieldCount {
                        candidate_count: 0,
                        projected_count: 0,
                        absent_count: 0,
                    },
                )
            })
            .collect::<BTreeMap<_, _>>();
        let mut rows = Vec::with_capacity(roots.len());
        for (row_index, root) in roots.iter().enumerate() {
            let mut row = BTreeMap::new();
            for (index, grammar) in fields.iter().enumerate() {
                let value = self
                    .field(
                        root,
                        index,
                        &grammar.field,
                        counts
                            .get_mut(&grammar.name)
                            .expect("compiled field aggregate"),
                    )
                    .map_err(|mut error| {
                        error.row_index = Some(row_index as u32 + 1);
                        error.field_name = Some(grammar.name.clone());
                        error
                    })?;
                row.insert(grammar.name.clone(), value);
            }
            rows.push(row);
        }
        Ok(RecordOutput { rows, counts })
    }

    fn field(
        &mut self,
        root: &super::SelectionScope<'_>,
        index: usize,
        field: &RecordField,
        count: &mut FieldCount,
    ) -> Result<FieldValue, ExtractionError> {
        super::charge(self.budget, 1)?;
        super::spend_cells(self.cells, 1, self.compiled.plan.limits.max_cells)?;
        let grammar = &self.compiled.fields[index];
        let limits = &self.compiled.plan.limits;
        let candidates = super::matches_scope(
            self.document,
            Some(root),
            &grammar.selector,
            limits.max_candidates,
            self.budget,
        )?;
        let total = candidates.len() as u32;
        count.candidate_count += u64::from(total); // Aggregate matched candidates are bounded by charged work.
        if field.match_mode == FieldMatch::Optional && total == 0 {
            count.absent_count += 1;
            return Ok(FieldValue::Absent);
        }
        let selection = if field.match_mode == FieldMatch::Optional {
            Selection::Single
        } else {
            field.selection()
        };
        let positions = super::selected_positions(&selection, total, limits.max_selected).map_err(
            |mut error| {
                // Absent optional fields returned above; their only selection error is ambiguity.
                if field.match_mode == FieldMatch::Optional {
                    error.expected_min = Some(0);
                    error.message = "Optional field matched multiple candidates.".into();
                }
                error
            },
        )?;
        let many = field.match_mode == FieldMatch::All;
        if many {
            super::spend_cells(self.cells, positions.len() as u32, limits.max_cells)?;
        }
        let mut values = Vec::with_capacity(positions.len());
        for position in positions {
            let node = candidates[position];
            let excluded =
                super::exclusions(self.document, node, &grammar.exclusions, self.budget)?;
            let value = crate::projection::project(
                node,
                &field.read,
                &excluded,
                self.base,
                (limits.max_value_bytes as usize).min(*self.bytes),
                self.budget,
            )
            .map_err(|error| super::projection_failure(error, limits, *self.bytes))?;
            *self.bytes -= value.len();
            values.push(value);
            count.projected_count += 1;
        }
        if many {
            Ok(FieldValue::Many(values))
        } else {
            Ok(FieldValue::Text(
                values
                    .pop()
                    .expect("single/nth selection yields one position"),
            ))
        }
    }
}
