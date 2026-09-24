//! Deterministic, cursor-bound discovery over one prepared HTMLCut document.

use std::collections::BTreeMap;
use std::num::NonZeroU32;

use scraper::ElementRef;
use selectors::work_budget::SelectorWorkBudget;

use crate::interop::v2::stable_json::digest_stable_json;
use crate::interop::v2::{HTMLCUT_DISCOVERY_SEMANTICS_VERSION, PreparedDocument};

use super::{
    CssSelectorText, EXPLORATION_ERROR_SCHEMA_NAME, EXPLORATION_ERROR_SCHEMA_VERSION,
    EXPLORATION_RESULT_SCHEMA_NAME, EXPLORATION_RESULT_SCHEMA_VERSION, ElementDescriptor,
    ElementName, ElementNamespace, ElementTargetHint, ExplorationAttribute, ExplorationCursor,
    ExplorationError, ExplorationErrorCode, ExplorationOmission, ExplorationOmissionReason,
    ExplorationOptions, ExplorationResult, ExplorationTruncationReason, SelectorProposal,
    SelectorStability, SelectorStabilityReason, TARGET_RESOLUTION_RESULT_SCHEMA_NAME,
    TARGET_RESOLUTION_RESULT_SCHEMA_VERSION, TargetResolutionOptions, TargetResolutionResult,
};

const MAX_PATH_BYTES: usize = 16_384;
const MAX_ELEMENT_NAME_BYTES: usize = 256;
const MAX_ATTRIBUTES: usize = 32;
const MAX_ATTRIBUTE_NAME_BYTES: usize = 256;
const MAX_ATTRIBUTE_VALUE_BYTES: usize = 512;
const MAX_SELECTOR_BYTES: usize = 2_048;
const DOM_PATH_PROFILE: &str = "htmlcut.dom_path.v1";
const MAX_TARGET_TEXT_BYTES: usize = 1_048_576;

mod attributes;
mod proposals;
mod target;
mod work;

#[cfg(test)]
mod test_support;

#[cfg(test)]
pub(crate) use test_support::exploration_cursor_for_tests;

#[cfg(test)]
use attributes::bounded_prefix;
use attributes::{exceeds_byte_limit, safe_attributes};
#[cfg(test)]
pub(crate) use proposals::selector_is_unique_for_tests;
use proposals::selector_proposals;
use target::is_lower_sha256;
use work::{WorkLimitExceeded, inspect_element};

#[cfg(test)]
pub(crate) use target::{is_lower_sha256_for_tests, normalized_dom_text_nodes_for_tests};
pub use target::{normalized_dom_text_digest, resolve_target_and_propose};

/// Explores one prepared static document in deterministic document order.
pub fn explore(
    document: &PreparedDocument,
    options: &ExplorationOptions,
) -> Result<ExplorationResult, Box<ExplorationError>> {
    let discovery_identity = discovery_identity(document);
    options.validate().map_err(|_| {
        Box::new(exploration_error(
            ExplorationErrorCode::InvalidLimits,
            discovery_identity.clone(),
            "Exploration limits are outside HTMLCut's supported range.",
        ))
    })?;
    let options_digest = options_digest(options);
    let start_ordinal = cursor_start(
        options.cursor.as_ref(),
        &discovery_identity,
        &options_digest,
        document.element_count(),
    )?;

    let budget = SelectorWorkBudget::new(options.max_work_units.get());
    let mut elements = Vec::new();
    let mut omissions = BTreeMap::<ExplorationOmissionReason, u32>::new();
    let mut next_ordinal = start_ordinal.get();
    let mut truncation_reason = None;
    while next_ordinal <= document.element_count() {
        let ordinal = next_ordinal;
        let element = document.element_at_ordinal(
            NonZeroU32::new(ordinal).expect("prepared element ordinals are one-based"),
        );
        if inspect_element(&budget).is_err() {
            next_ordinal = ordinal;
            truncation_reason = Some(ExplorationTruncationReason::WorkLimit);
            break;
        }
        next_ordinal = ordinal.saturating_add(1);

        match descriptor_for(
            document.document(),
            &element,
            options.preview_bytes.get(),
            options.max_proposals_per_element,
            &budget,
        ) {
            Ok(outcome) => {
                elements.push(outcome.descriptor);
                if outcome.proposal_work_truncated {
                    truncation_reason = Some(ExplorationTruncationReason::WorkLimit);
                    break;
                }
            }
            Err(DescriptorFailure::Omission(reason)) => {
                let count = omissions.entry(reason).or_default();
                *count = count.saturating_add(1);
            }
            Err(DescriptorFailure::WorkLimit) => {
                let count = omissions
                    .entry(ExplorationOmissionReason::WorkLimit)
                    .or_default();
                *count = count.saturating_add(1);
                truncation_reason = Some(ExplorationTruncationReason::WorkLimit);
                break;
            }
        }
        if elements.len() >= options.max_elements.get() as usize {
            truncation_reason = Some(ExplorationTruncationReason::ElementLimit);
            break;
        }
    }

    let has_more_elements = next_ordinal <= document.element_count();
    let next_cursor = if has_more_elements {
        NonZeroU32::new(next_ordinal).map(|ordinal| {
            ExplorationCursor::new(discovery_identity.clone(), options_digest, ordinal)
        })
    } else {
        None
    };
    if !has_more_elements && truncation_reason == Some(ExplorationTruncationReason::ElementLimit) {
        truncation_reason = None;
    }
    Ok(ExplorationResult {
        schema_name: EXPLORATION_RESULT_SCHEMA_NAME.to_owned(),
        schema_version: EXPLORATION_RESULT_SCHEMA_VERSION,
        discovery_identity_sha256: discovery_identity,
        consumed_work_units: options
            .max_work_units
            .get()
            .saturating_sub(budget.remaining()),
        elements,
        omitted_element_counts: omissions
            .into_iter()
            .map(|(reason, count)| ExplorationOmission { reason, count })
            .collect(),
        truncation_reason,
        next_cursor,
    })
}

fn cursor_start(
    cursor: Option<&ExplorationCursor>,
    discovery_identity: &str,
    options_digest: &str,
    element_count: u32,
) -> Result<NonZeroU32, Box<ExplorationError>> {
    let Some(cursor) = cursor else {
        return Ok(NonZeroU32::new(1).expect("one is non-zero"));
    };
    if !is_lower_sha256(cursor.discovery_identity_sha256())
        || !is_lower_sha256(cursor.options_digest_sha256())
        || cursor.next_document_ordinal().get() > element_count
    {
        return Err(Box::new(exploration_error(
            ExplorationErrorCode::InvalidCursor,
            discovery_identity.to_owned(),
            "Exploration cursor is malformed or outside this document.",
        )));
    }
    if cursor.discovery_identity_sha256() != discovery_identity {
        return Err(Box::new(exploration_error(
            ExplorationErrorCode::CursorSnapshotMismatch,
            discovery_identity.to_owned(),
            "Exploration cursor belongs to a different prepared document snapshot.",
        )));
    }
    if cursor.options_digest_sha256() != options_digest {
        return Err(Box::new(exploration_error(
            ExplorationErrorCode::CursorOptionsMismatch,
            discovery_identity.to_owned(),
            "Exploration cursor belongs to different exploration options.",
        )));
    }
    Ok(cursor.next_document_ordinal())
}

struct DescriptorOutcome {
    descriptor: ElementDescriptor,
    proposal_work_truncated: bool,
}

#[derive(Debug)]
enum DescriptorFailure {
    Omission(ExplorationOmissionReason),
    WorkLimit,
}

fn descriptor_for(
    document: &scraper::Html,
    element: &ElementRef<'_>,
    preview_bytes: u32,
    max_proposals: NonZeroU32,
    budget: &SelectorWorkBudget,
) -> Result<DescriptorOutcome, DescriptorFailure> {
    let element_name = element_name(element).map_err(DescriptorFailure::Omission)?;
    let path = namespaced_path(element, budget).map_err(|error| match error {
        PathBuildFailure::Omission(reason) => DescriptorFailure::Omission(reason),
        PathBuildFailure::WorkLimit => DescriptorFailure::WorkLimit,
    })?;
    let (text_preview, text_preview_truncated) =
        bounded_text_preview(document, element, preview_bytes as usize, budget)
            .map_err(|_| DescriptorFailure::WorkLimit)?;
    // A prepared document contains at most one million elements, so a direct fixed-width count
    // is exact and cannot fabricate a saturated public value.
    let child_element_count =
        child_element_count(element, budget).map_err(|_| DescriptorFailure::WorkLimit)?;
    let (attributes, attributes_truncated) = safe_attributes(element);
    let (selector_proposals, proposal_work_truncated) =
        selector_proposals(document, element, max_proposals, budget);
    let no_bounded_unique_selector = selector_proposals.is_empty() && !proposal_work_truncated;
    Ok(DescriptorOutcome {
        descriptor: ElementDescriptor {
            path,
            element_name,
            text_preview,
            text_preview_truncated,
            attributes,
            attributes_truncated,
            selector_proposals,
            child_element_count,
            proposal_work_truncated,
            no_bounded_unique_selector,
        },
        proposal_work_truncated,
    })
}

pub(super) fn element_name(
    element: &ElementRef<'_>,
) -> Result<ElementName, ExplorationOmissionReason> {
    let local_name = element.value().name.local.to_string();
    if exceeds_byte_limit(local_name.len(), MAX_ELEMENT_NAME_BYTES) {
        return Err(ExplorationOmissionReason::ElementNameTooLong);
    }
    let namespace = element_namespace(element.value().name.ns.as_ref())?;
    Ok(ElementName {
        namespace,
        local_name,
    })
}

fn element_namespace(namespace: &str) -> Result<ElementNamespace, ExplorationOmissionReason> {
    Ok(match namespace {
        "http://www.w3.org/1999/xhtml" => ElementNamespace::Html,
        "http://www.w3.org/2000/svg" => ElementNamespace::Svg,
        "http://www.w3.org/1998/Math/MathML" => ElementNamespace::Mathml,
        _ => return Err(ExplorationOmissionReason::UnsupportedNamespace),
    })
}

#[cfg(test)]
pub(crate) fn element_namespace_for_tests(
    namespace: &str,
) -> Result<ElementNamespace, ExplorationOmissionReason> {
    element_namespace(namespace)
}

#[derive(Debug)]
pub(super) enum PathBuildFailure {
    /// The path cannot be represented without weakening identity evidence.
    Omission(ExplorationOmissionReason),
    /// The shared request budget was exhausted while inspecting path elements.
    WorkLimit,
}

pub(super) fn namespaced_path(
    element: &ElementRef<'_>,
    budget: &SelectorWorkBudget,
) -> Result<String, PathBuildFailure> {
    let mut segments = Vec::new();
    let mut current = Some(*element);
    while let Some(node) = current {
        inspect_element(budget).map_err(|_| PathBuildFailure::WorkLimit)?;
        let name = element_name(&node).map_err(PathBuildFailure::Omission)?;
        let namespace = match name.namespace {
            ElementNamespace::Html => "html",
            ElementNamespace::Svg => "svg",
            ElementNamespace::Mathml => "mathml",
        };
        let ordinal = node
            .parent()
            .and_then(ElementRef::wrap)
            .map(|parent| same_name_sibling_ordinal(&parent, &node, budget))
            .transpose()?
            .unwrap_or(1);
        segments.push(format!("{namespace}:{}[{ordinal}]", name.local_name));
        current = node.parent().and_then(ElementRef::wrap);
    }
    segments.reverse();
    let path = segments.join("/");
    if exceeds_byte_limit(path.len(), MAX_PATH_BYTES) {
        return Err(PathBuildFailure::Omission(
            ExplorationOmissionReason::PathTooLong,
        ));
    }
    Ok(path)
}

/// Builds the positional CSS fallback while charging every ancestor and sibling inspection.
pub(super) fn structural_selector_path(
    element: &ElementRef<'_>,
    budget: &SelectorWorkBudget,
) -> Result<String, WorkLimitExceeded> {
    let mut segments = Vec::new();
    let mut current = Some(*element);
    while let Some(node) = current {
        inspect_element(budget)?;
        let name = node.value().name().to_owned();
        let ordinal = node
            .parent()
            .and_then(ElementRef::wrap)
            .map(|parent| same_name_sibling_ordinal(&parent, &node, budget))
            .transpose()
            .map_err(|_| WorkLimitExceeded)?
            .unwrap_or(1);
        segments.push(format!("{name}:nth-of-type({ordinal})"));
        current = node.parent().and_then(ElementRef::wrap);
    }
    segments.reverse();
    Ok(segments.join(" > "))
}

fn same_name_sibling_ordinal(
    parent: &ElementRef<'_>,
    node: &ElementRef<'_>,
    budget: &SelectorWorkBudget,
) -> Result<u32, PathBuildFailure> {
    inspect_element(budget).map_err(|_| PathBuildFailure::WorkLimit)?;
    let (_, ordinal) = parent.child_elements().try_fold(
        (0_u32, None),
        |(count, found), sibling| -> Result<_, PathBuildFailure> {
            if found.is_some() {
                return Ok((count, found));
            }
            inspect_element(budget).map_err(|_| PathBuildFailure::WorkLimit)?;
            let count = count.saturating_add(u32::from(sibling.value().name == node.value().name));
            Ok((count, (sibling.id() == node.id()).then_some(count)))
        },
    )?;
    Ok(
        ordinal
            .expect("a node reached through its parent must appear among that parent's children"),
    )
}

fn child_element_count(
    element: &ElementRef<'_>,
    budget: &SelectorWorkBudget,
) -> Result<u32, WorkLimitExceeded> {
    element.child_elements().try_fold(0_u32, |count, _child| {
        inspect_element(budget)?;
        Ok(count.saturating_add(1))
    })
}

fn bounded_text_preview(
    document: &scraper::Html,
    element: &ElementRef<'_>,
    maximum: usize,
    budget: &SelectorWorkBudget,
) -> Result<(String, bool), WorkLimitExceeded> {
    let mut preview = String::new();
    let node = document
        .tree
        .get(element.id())
        .expect("prepared descriptor element belongs to its prepared document");
    for descendant in node.descendants() {
        if ElementRef::wrap(descendant).is_some() {
            inspect_element(budget)?;
        }
        let scraper::Node::Text(text) = descendant.value() else {
            continue;
        };
        for character in text.chars() {
            let width = character.len_utf8();
            if preview.len().saturating_add(width) > maximum {
                return Ok((preview, true));
            }
            preview.push(character);
        }
    }
    Ok((preview, false))
}

pub(super) fn discovery_identity(document: &PreparedDocument) -> String {
    digest_stable_json(&(
        document.input_digest_sha256(),
        HTMLCUT_DISCOVERY_SEMANTICS_VERSION,
    ))
    .expect("closed prepared-document identity inputs must serialize to stable JSON")
}

fn options_digest(options: &ExplorationOptions) -> String {
    digest_stable_json(&(
        options.max_elements,
        options.max_work_units,
        options.max_proposals_per_element,
        options.preview_bytes,
    ))
    .expect("closed exploration options must serialize to stable JSON")
}

pub(super) fn exploration_error(
    error_code: ExplorationErrorCode,
    discovery_identity_sha256: String,
    message: impl Into<String>,
) -> ExplorationError {
    ExplorationError {
        schema_name: EXPLORATION_ERROR_SCHEMA_NAME.to_owned(),
        schema_version: EXPLORATION_ERROR_SCHEMA_VERSION,
        error_code,
        discovery_identity_sha256,
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scraper::{Html, Selector};

    #[test]
    fn bounded_prefix_rounds_an_interior_utf8_boundary_down_without_iteration() {
        assert_eq!(bounded_prefix("abé", 3), "ab");
        assert_eq!(bounded_prefix("abé", 2), "ab");
        assert_eq!(bounded_prefix("éclair", 1), "");
    }

    #[test]
    fn byte_bounds_include_the_exact_limit_and_reject_only_larger_values() {
        for maximum in [
            MAX_ELEMENT_NAME_BYTES,
            MAX_PATH_BYTES,
            MAX_ATTRIBUTE_VALUE_BYTES,
        ] {
            assert!(!exceeds_byte_limit(maximum, maximum));
            assert!(exceeds_byte_limit(maximum.saturating_add(1), maximum));
        }
        assert!(!exceeds_byte_limit(MAX_ATTRIBUTES, MAX_ATTRIBUTES));
        assert!(exceeds_byte_limit(MAX_ATTRIBUTES + 1, MAX_ATTRIBUTES));
    }

    #[test]
    fn attribute_evidence_at_the_exact_public_limits_is_complete_not_truncated() {
        let value = "v".repeat(MAX_ATTRIBUTE_VALUE_BYTES);
        let attributes = (0..MAX_ATTRIBUTES)
            .map(|index| format!("data-key-{index}=\"{value}\""))
            .collect::<Vec<_>>()
            .join(" ");
        let document = Html::parse_document(&format!("<main {attributes}>body</main>"));
        let selector = Selector::parse("main").expect("HTMLCut-owned selector");
        let element = document.select(&selector).next().expect("main element");

        let (evidence, truncated) = safe_attributes(&element);

        assert_eq!(evidence.len(), MAX_ATTRIBUTES);
        assert!(!truncated);
        assert!(
            evidence
                .iter()
                .all(|attribute| attribute.value.len() == MAX_ATTRIBUTE_VALUE_BYTES)
        );
        assert!(evidence.iter().all(|attribute| !attribute.value_truncated));

        let exact_name = format!("data-{}", "n".repeat(MAX_ATTRIBUTE_NAME_BYTES - 5));
        let named_document =
            Html::parse_document(&format!("<main {exact_name}=\"value\">body</main>"));
        let named_element = named_document
            .select(&selector)
            .next()
            .expect("main element with exact-length attribute name");
        let (named_evidence, named_truncated) = safe_attributes(&named_element);
        assert!(!named_truncated);
        assert_eq!(named_evidence[0].name, exact_name);
    }

    #[test]
    fn descriptor_distinguishes_no_bounded_selector_from_work_truncation() {
        let depth = 300;
        let markup = format!("{}leaf{}", "<div>".repeat(depth), "</div>".repeat(depth));
        let document = Html::parse_document(&markup);
        let element = document
            .tree
            .root()
            .descendants()
            .filter_map(scraper::ElementRef::wrap)
            .last()
            .expect("deep leaf element");
        let budget = SelectorWorkBudget::new(5_000_000);

        let outcome = descriptor_for(
            &document,
            &element,
            32,
            NonZeroU32::new(16).expect("non-zero proposal count"),
            &budget,
        )
        .expect("descriptor without bounded selector proposal");

        assert!(outcome.descriptor.selector_proposals.is_empty());
        assert!(!outcome.proposal_work_truncated);
        assert!(outcome.descriptor.no_bounded_unique_selector);
    }

    #[test]
    fn descriptor_never_reports_missing_bounded_selectors_when_a_unique_proposal_exists() {
        let document = Html::parse_document("<main id=\"content\">body</main>");
        let selector = Selector::parse("main").expect("HTMLCut-owned selector");
        let element = document.select(&selector).next().expect("main element");
        let budget = SelectorWorkBudget::new(100_000);

        let outcome = descriptor_for(
            &document,
            &element,
            32,
            NonZeroU32::new(16).expect("non-zero proposal count"),
            &budget,
        )
        .expect("descriptor with bounded selector proposal");

        assert!(!outcome.descriptor.selector_proposals.is_empty());
        assert!(!outcome.proposal_work_truncated);
        assert!(!outcome.descriptor.no_bounded_unique_selector);
    }

    #[test]
    fn descriptor_reports_the_exact_direct_child_element_count() {
        let document = Html::parse_document(
            "<main id=\"content\"><span>first</span>text<strong>second</strong></main>",
        );
        let selector = Selector::parse("main").expect("HTMLCut-owned selector");
        let element = document.select(&selector).next().expect("main element");
        let budget = SelectorWorkBudget::new(100_000);

        let outcome = descriptor_for(
            &document,
            &element,
            32,
            NonZeroU32::new(16).expect("non-zero proposal count"),
            &budget,
        )
        .expect("descriptor with direct child elements");

        assert_eq!(outcome.descriptor.child_element_count, 2);
    }

    #[test]
    fn structural_selector_path_preserves_every_ancestor_and_same_name_ordinal() {
        let document = Html::parse_document(
            "<main><section><span>first</span><span id=\"target\">second</span></section></main>",
        );
        let selector = Selector::parse("#target").expect("HTMLCut-owned selector");
        let target = document.select(&selector).next().expect("target element");

        assert_eq!(
            structural_selector_path(&target, &SelectorWorkBudget::new(100_000))
                .expect("structural path"),
            "html:nth-of-type(1) > body:nth-of-type(1) > main:nth-of-type(1) > section:nth-of-type(1) > span:nth-of-type(2)"
        );
    }

    #[test]
    fn namespaced_paths_preserve_svg_element_identity() {
        let document = Html::parse_document("<svg><circle></circle></svg>");
        let selector = Selector::parse("circle").expect("HTMLCut-owned selector");
        let circle = document.select(&selector).next().expect("circle element");

        let path = namespaced_path(&circle, &SelectorWorkBudget::new(100_000)).expect("SVG path");

        assert!(path.ends_with("svg:svg[1]/svg:circle[1]"));
    }

    #[test]
    fn unsupported_namespace_is_refused_before_descriptor_construction() {
        assert_eq!(
            element_namespace_for_tests("urn:htmlcut:unsupported"),
            Err(ExplorationOmissionReason::UnsupportedNamespace)
        );
    }
}
