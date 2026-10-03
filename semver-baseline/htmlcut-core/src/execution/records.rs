//! Row-relative scalar fields with shared byte, cell and selector-work allowances.

use std::collections::BTreeMap;

use scraper::{ElementRef, Html};
use selectors::work_budget::SelectorWorkBudget;

use crate::{
    CompiledPlan, ExtractionError, FieldCount, FieldSelection, FieldValue, RecordField, Selection,
};

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
    pub(crate) counts: Vec<FieldCount>,
}

impl RecordExecution<'_> {
    pub(crate) fn run(
        &mut self,
        roots: &[ElementRef<'_>],
    ) -> Result<RecordOutput, ExtractionError> {
        let fields = &self.compiled.fields;
        let mut counts = fields
            .iter()
            .enumerate()
            .map(|(index, _)| FieldCount {
                field_index: index as u32 + 1,
                candidate_count: 0,
                projected_count: 0,
                absent_count: 0,
            })
            .collect::<Vec<_>>();
        let mut rows = Vec::with_capacity(roots.len());
        for (row_index, root) in roots.iter().enumerate() {
            let mut row = BTreeMap::new();
            for (index, grammar) in fields.iter().enumerate() {
                let value = self
                    .field(*root, index, &grammar.field, &mut counts[index])
                    .map_err(|mut error| {
                        error.row_index = Some(row_index as u32 + 1);
                        error.field_index = Some(index as u32 + 1);
                        error
                    })?;
                row.insert(grammar.field.name.clone(), value);
            }
            rows.push(row);
        }
        Ok(RecordOutput { rows, counts })
    }

    fn field(
        &mut self,
        root: ElementRef<'_>,
        index: usize,
        field: &RecordField,
        count: &mut FieldCount,
    ) -> Result<FieldValue, ExtractionError> {
        super::charge(self.budget, 1)?;
        super::spend_cells(self.cells, 1)?;
        let grammar = &self.compiled.fields[index];
        let limits = &self.compiled.plan.limits;
        let candidates = super::matches(
            self.document,
            Some(root),
            &grammar.selector,
            limits.max_candidates,
            self.budget,
        )?;
        let total = candidates.len() as u32;
        count.candidate_count += u64::from(total); // Aggregate matched candidates are bounded by charged work.
        if matches!(field.selection, FieldSelection::Optional {}) && total == 0 {
            count.absent_count += 1;
            return Ok(FieldValue::Absent);
        }
        let selection = match field.selection {
            FieldSelection::Single {} | FieldSelection::Optional {} => Selection::Single {},
            FieldSelection::All { min, max } => Selection::All { min, max },
            FieldSelection::Nth { index } => Selection::Nth { index },
        };
        let positions = super::selected_positions(&selection, total, limits.max_selected)?;
        let many = matches!(field.selection, FieldSelection::All { .. });
        if many {
            super::spend_cells(self.cells, positions.len() as u32)?;
        }
        let mut values = Vec::with_capacity(positions.len());
        for position in positions {
            let node = candidates[position];
            let excluded =
                super::exclusions(self.document, node, &grammar.exclusions, self.budget)?;
            let value = crate::projection::project(
                node,
                &field.projection,
                &excluded,
                &field.transforms,
                self.base,
                (limits.max_value_bytes as usize).min(*self.bytes),
                self.budget,
            )?;
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
