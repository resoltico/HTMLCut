// SPDX-License-Identifier: MPL-2.0
//! Direct cell/header facts; spans or heterogeneity never become inferred fields.

use std::collections::HashSet;

use scraper::ElementRef;
use selectors::work_budget::SelectorWorkBudget;

use super::{Candidate, TableShape};
use crate::ExtractionError;

pub(super) fn summarize(
    group: &Candidate<'_>,
    members: &[ElementRef<'_>],
    budget: &SelectorWorkBudget,
) -> Result<Option<TableShape>, ExtractionError> {
    if group.key.tag != "tr" {
        return Ok(None);
    }
    let mut shape = TableShape {
        headers: Vec::new(),
        header_read: "text".into(),
        headers_complete: true,
        headers_unique: false,
        header_rows: 0,
        data_rows: 0,
        other_rows: 0,
        min_data_cells: None,
        max_data_cells: None,
        spans_present: false,
    };
    for row in members {
        crate::execution::charge(budget, 1)?;
        let mut headers = Vec::new();
        let mut data_count = 0_u32;
        for child in row.children() {
            crate::execution::charge(budget, 1)?;
            let Some(cell) = ElementRef::wrap(child) else {
                continue;
            };
            match cell.value().name() {
                "th" => headers.push(cell),
                "td" => data_count += 1,
                _ => continue,
            }
            if cell.attr("rowspan").is_some() || cell.attr("colspan").is_some() {
                shape.spans_present = true;
            }
        }
        if !headers.is_empty() && data_count == 0 {
            shape.header_rows += 1;
            if shape.header_rows == 1 && headers.len() <= 8 {
                for cell in headers {
                    if let Some(text) = header_text(cell, budget)? {
                        shape.headers.push(text);
                    } else {
                        shape.headers_complete = false;
                    }
                }
            } else {
                shape.headers_complete = false;
            }
        } else if data_count > 0 && headers.is_empty() {
            shape.data_rows += 1;
            shape.min_data_cells = Some(
                shape
                    .min_data_cells
                    .map_or(data_count, |v| v.min(data_count)),
            );
            shape.max_data_cells = Some(
                shape
                    .max_data_cells
                    .map_or(data_count, |v| v.max(data_count)),
            );
        } else {
            shape.other_rows += 1;
        }
    }
    if !shape.headers_complete {
        shape.headers.clear();
    } else if shape.header_rows == 1 {
        let mut seen = HashSet::new();
        shape.headers_unique = shape
            .headers
            .iter()
            .all(|header| !header.trim().is_empty() && seen.insert(header.as_str()));
    }
    Ok(Some(shape))
}

fn header_text(
    cell: ElementRef<'_>,
    budget: &SelectorWorkBudget,
) -> Result<Option<String>, ExtractionError> {
    let (text, complete) =
        crate::projection::text(cell, &HashSet::new(), true, 128, Some(128), budget)?;
    Ok(complete.then_some(text))
}
