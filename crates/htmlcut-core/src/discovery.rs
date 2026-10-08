// SPDX-License-Identifier: MPL-2.0
//! Selector-scoped counts and deliberately abbreviated samples of one immutable snapshot.

use schemars::JsonSchema;
use scraper::ElementRef;
use selectors::work_budget::SelectorWorkBudget;
use serde::{Deserialize, Serialize};

use crate::{ErrorCode, ExtractionError, PreparedDocument};

mod survey;
pub use survey::{SurveyElement, SurveyGroup, SurveyResult, SurveySample, TableShape};

/// Bounded selector identifiers and structural text for one sampled element.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InspectionSample {
    /// Parsed tag, bounded to 128 UTF-8 bytes.
    pub tag: String,
    /// Exact id attribute when present and within 128 UTF-8 bytes.
    pub id: Option<String>,
    /// At most eight distinct exact class tokens, sorted lexically, of at most 64 UTF-8 bytes each.
    pub classes: Vec<String>,
    /// Whether all id and class tokens are represented.
    pub identifiers_complete: bool,
    /// At most eight exact supported attribute names, sorted lexically.
    pub attributes: Vec<String>,
    /// Whether every attribute name is represented.
    pub attributes_complete: bool,
    /// At most 160 Unicode scalar values of static structural text preview.
    pub text: String,
    /// Whether the structural text preview is complete.
    pub text_complete: bool,
}

/// Complete selector count with bounded identifier samples.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InspectionResult {
    /// Complete match count, never a lower bound or estimate.
    pub count: u32,
    /// Samples in original document order.
    pub samples: Vec<InspectionSample>,
    /// Whether every matched element has a sample.
    pub samples_complete: bool,
}

impl PreparedDocument {
    /// Counts an explicit selector and samples only bounded id/class values and structural text.
    pub fn inspect(&self, css: &str, samples: u32) -> Result<InspectionResult, ExtractionError> {
        let (nodes, budget) = self.inspection_matches(css, samples)?;
        let count = nodes.len() as u32;
        let selected = nodes
            .iter()
            .take(samples as usize)
            .map(|node| identifier_sample(*node, &budget))
            .collect::<Result<Vec<_>, _>>()?;
        let result = InspectionResult {
            count,
            samples_complete: count <= samples,
            samples: selected,
        };
        Ok(result)
    }

    fn inspection_matches<'a>(
        &'a self,
        css: &str,
        samples: u32,
    ) -> Result<(Vec<ElementRef<'a>>, SelectorWorkBudget), ExtractionError> {
        if !(1..=10).contains(&samples) {
            return Err(ExtractionError::new(
                ErrorCode::InvalidOptions,
                "inspection",
                "Sample count must be between one and ten.",
            ));
        }
        crate::plan::validation::pattern(css)?;
        let selector = crate::compilation::compile_selector(css)?;
        let limits = crate::ExecutionLimits::default();
        let budget = SelectorWorkBudget::new(limits.max_work);
        let document = self.document()?;
        let nodes =
            crate::execution::matches(document, None, &selector, limits.max_candidates, &budget)?;
        Ok((nodes, budget))
    }
}

fn identifier_sample(
    root: ElementRef<'_>,
    budget: &SelectorWorkBudget,
) -> Result<InspectionSample, ExtractionError> {
    let tag = root.value().name();
    if tag.len() > 128 {
        return Err(ExtractionError::resource("inspection", "tag_bytes", 128));
    }
    let mut complete = true;
    let id = if let Some(value) = root.attr("id") {
        crate::execution::charge(budget, value.len().div_ceil(64) + 1)?;
        if value.len() <= 128 {
            Some(value.to_owned())
        } else {
            complete = false;
            None
        }
    } else {
        None
    };
    let mut classes = Vec::new();
    if let Some(value) = root.attr("class") {
        crate::execution::charge(budget, value.len().div_ceil(64) + 1)?;
        for token in value.split_ascii_whitespace() {
            crate::execution::charge(budget, 1)?;
            if token.len() > 64 {
                complete = false;
            } else if !classes.iter().any(|class: &String| class == token) {
                classes.push(token.to_owned());
                classes.sort();
                if classes.len() > 8 {
                    classes.truncate(8);
                    complete = false;
                }
            }
        }
    }
    let mut attributes = Vec::new();
    let mut attribute_count = 0;
    for (name, _) in root.value().attrs() {
        crate::execution::charge(budget, name.len().div_ceil(64) + 1)?;
        attribute_count += 1;
        if name.len() <= 256 {
            attributes.push(name.to_owned());
            attributes.sort();
            attributes.truncate(8);
        }
    }
    let attributes_complete = attribute_count == attributes.len();
    let (text, text_complete) = structural_preview(root, budget, 160)?;
    Ok(InspectionSample {
        tag: tag.into(),
        id,
        classes,
        identifiers_complete: complete,
        attributes,
        attributes_complete,
        text,
        text_complete,
    })
}

fn structural_preview(
    root: ElementRef<'_>,
    budget: &SelectorWorkBudget,
    maximum: usize,
) -> Result<(String, bool), ExtractionError> {
    crate::projection::text(
        root,
        &std::collections::HashSet::new(),
        true,
        maximum * 4,
        Some(maximum),
        budget,
    )
}

#[cfg(test)]
#[path = "tests/inspection_accounting.rs"]
mod accounting_tests;
