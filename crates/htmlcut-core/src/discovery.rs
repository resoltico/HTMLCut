//! Selector-scoped counts and deliberately abbreviated samples of one immutable snapshot.

use schemars::JsonSchema;
use scraper::{ElementRef, Node};
use selectors::work_budget::SelectorWorkBudget;
use serde::{Deserialize, Serialize};

use crate::{ErrorCode, ExtractionError, PreparedDocument};

/// One sample, separate from a complete extraction value.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InspectionSample {
    /// Parsed tag, bounded to 128 UTF-8 bytes.
    pub tag: String,
    /// At most eight exact supported names, sorted lexicographically.
    pub attributes: Vec<String>,
    /// Whether every attribute name is included.
    pub attributes_complete: bool,
    /// At most 160 normalized literal-text Unicode scalar values.
    pub text: String,
    /// Whether the normalized text is complete.
    pub text_complete: bool,
}

/// Complete selector count with explicitly bounded samples.
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
    /// Counts an explicit selector completely and returns up to ten bounded samples.
    pub fn inspect(&self, css: &str, samples: u32) -> Result<InspectionResult, ExtractionError> {
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
        let count = nodes.len() as u32;
        let selected = nodes
            .iter()
            .take(samples as usize)
            .map(|node| sample(*node, &budget))
            .collect::<Result<Vec<_>, _>>()?;
        let result = InspectionResult {
            count,
            samples_complete: count <= samples,
            samples: selected,
        };
        let _ = crate::identity::data_digest(&result, 16 * 1024, &budget)?;
        Ok(result)
    }
}

fn sample(
    root: ElementRef<'_>,
    budget: &SelectorWorkBudget,
) -> Result<InspectionSample, ExtractionError> {
    let tag = root.value().name();
    if tag.len() > 128 {
        return Err(ExtractionError::limit("inspection"));
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
    let (text, text_complete) = preview(root, budget)?;
    Ok(InspectionSample {
        tag: tag.into(),
        attributes,
        attributes_complete,
        text,
        text_complete,
    })
}

fn preview(
    root: ElementRef<'_>,
    budget: &SelectorWorkBudget,
) -> Result<(String, bool), ExtractionError> {
    let mut result = String::new();
    let mut count = 0;
    let mut space = false;
    for node in root.descendants() {
        crate::execution::charge(budget, 1)?;
        let Node::Text(text) = node.value() else {
            continue;
        };
        for c in text.text.chars() {
            crate::execution::charge(budget, 1)?;
            if c.is_ascii_whitespace() {
                space = !result.is_empty();
                continue;
            }
            let needed = 1 + usize::from(space);
            if needed > 160 - count {
                return Ok((result, false));
            }
            if space {
                result.push(' ');
                count += 1;
            }
            result.push(c);
            count += 1;
            space = false;
        }
    }
    Ok((result, true))
}
