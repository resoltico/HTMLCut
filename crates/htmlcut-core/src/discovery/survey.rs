// SPDX-License-Identifier: MPL-2.0
//! Bounded repeated-sibling discovery from one immutable original DOM.

use std::collections::{HashMap, HashSet, hash_map::Entry};

use ego_tree::iter::Edge;
use schemars::JsonSchema;
use scraper::ElementRef;
use selectors::work_budget::SelectorWorkBudget;
use serde::{Deserialize, Serialize};

use super::structural_preview;
use crate::{ErrorCode, ExtractionError, PreparedDocument};

mod selector_hint;
mod table_shape;
#[cfg(test)]
mod tests;

const MAX_WORK: u32 = 10_000_000;
const MAX_SIGNATURES_PER_PARENT: usize = 1_024;
const MAX_RESULT_BYTES: usize = 16 * 1_024;
const SAMPLE_CHARACTERS: usize = 64;

/// Exact bounded parsed identifiers of one parent element.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SurveyElement {
    /// Parsed HTML tag.
    pub tag: String,
    /// Exact parsed id when present and within 128 UTF-8 bytes.
    pub id: Option<String>,
    /// Distinct class tokens in lexical order, at most eight of 64 UTF-8 bytes each.
    pub classes: Vec<String>,
    /// Whether all id and class tokens are represented.
    pub identifiers_complete: bool,
}

/// Bounded structural text from one repeated member.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SurveySample {
    /// At most 64 Unicode scalar values of structural text.
    pub text: String,
    /// Whether the sampled member's text was complete.
    pub text_complete: bool,
}

/// Mechanical row shape evidence, without inferred column meaning.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TableShape {
    /// Reading convention shared by extraction and header guards.
    pub header_read: String,
    /// Complete structural text of a sole all-header row, when representable.
    pub headers: Vec<String>,
    /// Whether the sole header row, if any, is fully represented.
    pub headers_complete: bool,
    /// Whether represented header labels are nonempty and distinct.
    pub headers_unique: bool,
    /// Rows composed only of header cells.
    pub header_rows: u32,
    /// Rows composed only of one or more data cells.
    pub data_rows: u32,
    /// Empty or mixed-header/data rows.
    pub other_rows: u32,
    /// Smallest direct data-cell count, absent when no data row exists.
    pub min_data_cells: Option<u32>,
    /// Largest direct data-cell count, absent when no data row exists.
    pub max_data_cells: Option<u32>,
    /// Whether any direct cell declared a row or column span.
    pub spans_present: bool,
}

/// One exact repeated-sibling group and bounded authoring evidence.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SurveyGroup {
    /// A selector proved to match precisely these nodes on this snapshot, if found.
    pub selector: Option<String>,
    /// Exact number of direct sibling members in this group.
    pub count: u32,
    /// Direct parent of every group member.
    pub parent: SurveyElement,
    /// Parsed HTML tag shared by group members.
    pub item_tag: String,
    /// Whether membership also requires the listed complete class set.
    pub class_constrained: bool,
    /// Shared class tokens when class_constrained is true; empty otherwise.
    pub item_classes: Vec<String>,
    /// Up to two original-order member previews.
    pub samples: Vec<SurveySample>,
    /// Row shape evidence when the group belongs to an HTML table.
    pub table: Option<TableShape>,
}

/// Snapshot-bound survey of repeated HTML sibling groups.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SurveyResult {
    /// SHA-256 of the accepted source bytes used for this survey.
    pub source_sha256: String,
    /// Complete count of groups meeting the declared survey criteria.
    pub group_count: u32,
    /// Largest groups first, breaking ties by original document order.
    pub groups: Vec<SurveyGroup>,
    /// Whether every discovered group is included.
    pub groups_complete: bool,
}

#[derive(Clone, Hash, PartialEq, Eq)]
struct GroupKey {
    tag: String,
    classes: Option<Vec<String>>,
}

struct Accumulator {
    count: u32,
    first_order: u64,
}

struct Candidate<'a> {
    parent: ElementRef<'a>,
    key: GroupKey,
    count: u32,
    first_order: u64,
}

impl PreparedDocument {
    /// Surveys repeated HTML siblings without asserting their business meaning.
    pub fn survey(
        &self,
        within: Option<&str>,
        limit: u32,
    ) -> Result<SurveyResult, ExtractionError> {
        if !(1..=16).contains(&limit) {
            return Err(ExtractionError::new(
                ErrorCode::InvalidOptions,
                "survey",
                "Group limit must be between one and sixteen.",
            ));
        }
        let selector = within
            .map(|css| {
                crate::plan::validation::pattern(css)?;
                crate::compilation::compile_selector(css)
            })
            .transpose()?;
        let budget = SelectorWorkBudget::new(MAX_WORK);
        let document = self.document()?;
        let root = if let Some(selector) = &selector {
            let matches = crate::execution::matches(
                document,
                None,
                selector,
                crate::ExecutionLimits::default().max_candidates,
                &budget,
            )?;
            match matches.as_slice() {
                [root] => **root,
                [] => {
                    return Err(ExtractionError::new(
                        ErrorCode::NoMatch,
                        "survey",
                        "The outline scope selected no element.",
                    ));
                }
                _ => {
                    return Err(ExtractionError::new(
                        ErrorCode::AmbiguousSelection,
                        "survey",
                        "The outline scope must select exactly one element.",
                    ));
                }
            }
        } else {
            document.tree.root()
        };
        let mut top = Vec::new();
        let mut group_count = 0_u32;
        let mut suppressed = 0_u32;
        for edge in root.traverse() {
            crate::execution::charge(&budget, 1)?;
            match edge {
                Edge::Open(node) => {
                    if suppressed > 0 {
                        suppressed += 1;
                    } else if let Some(element) = ElementRef::wrap(node) {
                        if excluded_element(element) {
                            suppressed = 1;
                        } else {
                            collect_groups(
                                element,
                                limit as usize,
                                &budget,
                                &mut top,
                                &mut group_count,
                            )?;
                        }
                    }
                }
                Edge::Close(_) if suppressed > 0 => suppressed -= 1,
                Edge::Close(_) => (),
            }
        }
        let mut groups = Vec::with_capacity(top.len());
        for candidate in &top {
            let members = members(candidate, &budget)?;
            groups.push(SurveyGroup {
                selector: selector_hint::verified(document, candidate, &members, &budget)?,
                count: candidate.count,
                parent: element_descriptor(candidate.parent, &budget)?,
                item_tag: candidate.key.tag.clone(),
                class_constrained: candidate.key.classes.is_some(),
                item_classes: candidate.key.classes.clone().unwrap_or_default(),
                samples: samples(&members, &budget)?,
                table: table_shape::summarize(candidate, &members, &budget)?,
            });
        }
        let result = SurveyResult {
            source_sha256: self.snapshot.source_sha256().into(),
            group_count,
            groups_complete: group_count as usize <= limit as usize,
            groups,
        };
        let _ = crate::identity::encoded(&result, MAX_RESULT_BYTES, &budget)?;
        Ok(result)
    }
}

fn samples(
    members: &[ElementRef<'_>],
    budget: &SelectorWorkBudget,
) -> Result<Vec<SurveySample>, ExtractionError> {
    let mut result = Vec::new();
    for member in members.iter().take(2) {
        let (text, text_complete) = structural_preview(*member, budget, SAMPLE_CHARACTERS)?;
        result.push(SurveySample {
            text,
            text_complete,
        });
    }
    Ok(result)
}

fn excluded_element(element: ElementRef<'_>) -> bool {
    let namespace: &str = element.value().name.ns.as_ref();
    namespace != "http://www.w3.org/1999/xhtml"
        || matches!(
            element.value().name(),
            "head" | "script" | "style" | "template" | "pre" | "code" | "noscript"
        )
}

fn collect_groups<'a>(
    parent: ElementRef<'a>,
    limit: usize,
    budget: &SelectorWorkBudget,
    top: &mut Vec<Candidate<'a>>,
    group_count: &mut u32,
) -> Result<(), ExtractionError> {
    let mut groups: HashMap<GroupKey, Accumulator> = HashMap::new();
    for (index, child) in parent.children().enumerate() {
        crate::execution::charge(budget, 1)?;
        let Some(element) = ElementRef::wrap(child) else {
            continue;
        };
        if excluded_element(element) {
            continue;
        }
        let tag = element.value().name();
        if tag.len() > 128 {
            return Err(ExtractionError::resource("survey", "tag_bytes", 128));
        }
        let order = index as u64;
        add_group(
            &mut groups,
            GroupKey {
                tag: tag.into(),
                classes: None,
            },
            order,
        )?;
        if let Some(classes) = class_signature(element, budget)?
            && !classes.is_empty()
        {
            add_group(
                &mut groups,
                GroupKey {
                    tag: tag.into(),
                    classes: Some(classes),
                },
                order,
            )?;
        }
    }
    let mut fully_classed = HashSet::new();
    for (key, accumulator) in &groups {
        if key.classes.is_some()
            && groups
                .get(&GroupKey {
                    tag: key.tag.clone(),
                    classes: None,
                })
                .is_some_and(|plain| plain.count == accumulator.count)
        {
            fully_classed.insert(key.tag.clone());
        }
    }
    let mut ordered = groups.into_iter().collect::<Vec<_>>();
    ordered.sort_by(|(left_key, left), (right_key, right)| {
        left.first_order
            .cmp(&right.first_order)
            .then(left_key.classes.is_none().cmp(&right_key.classes.is_none()))
            .then(left_key.tag.cmp(&right_key.tag))
            .then(left_key.classes.cmp(&right_key.classes))
    });
    for (key, accumulator) in ordered {
        if accumulator.count < 3
            || (key.classes.is_none() && fully_classed.contains(key.tag.as_str()))
        {
            continue;
        }
        *group_count += 1;
        let candidate = Candidate {
            parent,
            key,
            count: accumulator.count,
            first_order: *group_count as u64,
        };
        top.push(candidate);
        top.sort_by(|a, b| {
            b.count
                .cmp(&a.count)
                .then(a.first_order.cmp(&b.first_order))
        });
        if top.len() > limit {
            top.pop();
        }
    }
    Ok(())
}

fn add_group(
    groups: &mut HashMap<GroupKey, Accumulator>,
    key: GroupKey,
    order: u64,
) -> Result<(), ExtractionError> {
    let at_limit = groups.len() >= MAX_SIGNATURES_PER_PARENT;
    match groups.entry(key) {
        Entry::Occupied(mut occupied) => occupied.get_mut().count += 1,
        Entry::Vacant(vacant) => {
            if at_limit {
                return Err(ExtractionError::resource(
                    "survey",
                    "signatures_per_parent",
                    MAX_SIGNATURES_PER_PARENT as u64,
                ));
            }
            vacant.insert(Accumulator {
                count: 1,
                first_order: order,
            });
        }
    }
    Ok(())
}

fn class_signature(
    element: ElementRef<'_>,
    budget: &SelectorWorkBudget,
) -> Result<Option<Vec<String>>, ExtractionError> {
    let Some(value) = element.attr("class") else {
        return Ok(Some(Vec::new()));
    };
    crate::execution::charge(budget, value.len().div_ceil(64) + 1)?;
    let mut tokens = Vec::new();
    for token in value.split_ascii_whitespace() {
        crate::execution::charge(budget, 1)?;
        if token.len() > 64 {
            return Ok(None);
        }
        if !tokens.iter().any(|prior: &String| prior == token) {
            if tokens.len() >= 8 {
                return Ok(None);
            }
            tokens.push(token.into());
        }
    }
    tokens.sort();
    Ok(Some(tokens))
}

fn members<'a>(
    candidate: &Candidate<'a>,
    budget: &SelectorWorkBudget,
) -> Result<Vec<ElementRef<'a>>, ExtractionError> {
    let mut found = Vec::with_capacity(candidate.count as usize);
    for child in candidate.parent.children() {
        crate::execution::charge(budget, 1)?;
        let Some(element) = ElementRef::wrap(child) else {
            continue;
        };
        if element.value().name() == candidate.key.tag
            && (candidate.key.classes.is_none()
                || class_signature(element, budget)? == candidate.key.classes)
        {
            found.push(element);
        }
    }
    Ok(found)
}

fn element_descriptor(
    element: ElementRef<'_>,
    budget: &SelectorWorkBudget,
) -> Result<SurveyElement, ExtractionError> {
    let tag = element.value().name();
    let mut complete = true;
    let id = if let Some(value) = element.attr("id") {
        crate::execution::charge(budget, value.len().div_ceil(64) + 1)?;
        if value.len() <= 128 {
            Some(value.into())
        } else {
            complete = false;
            None
        }
    } else {
        None
    };
    let classes = match class_signature(element, budget)? {
        Some(classes) => classes,
        None => {
            complete = false;
            Vec::new()
        }
    };
    Ok(SurveyElement {
        tag: tag.into(),
        id,
        classes,
        identifiers_complete: complete,
    })
}
