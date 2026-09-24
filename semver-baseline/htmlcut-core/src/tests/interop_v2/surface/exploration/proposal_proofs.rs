use std::num::NonZeroU32;

use scraper::{ElementRef, Html, Selector};

use super::*;
use crate::interop::v2::{
    ExplorationOmissionReason, ExplorationOptions, ExplorationTruncationReason,
    element_namespace_for_tests, exploration_cursor_for_tests, explore,
    selector_is_unique_for_tests,
};

#[test]
fn selector_proof_rejects_duplicate_and_non_target_matches() {
    let document =
        Html::parse_document("<main><p class=\"same\">One</p><p class=\"same\">Two</p></main>");
    let target = document
        .tree
        .root()
        .descendants()
        .filter_map(ElementRef::wrap)
        .find(|element| element.value().name() == "p")
        .expect("first paragraph")
        .id();

    assert!(!selector_is_unique_for_tests(
        &document,
        &Selector::parse(".same").expect("selector"),
        target,
        100,
    ));
    assert!(!selector_is_unique_for_tests(
        &document,
        &Selector::parse("aside").expect("selector"),
        target,
        100,
    ));
    assert!(selector_is_unique_for_tests(
        &document,
        &Selector::parse("p:nth-of-type(1)").expect("selector"),
        target,
        100,
    ));
}

#[test]
fn exploration_skips_oversized_and_duplicate_proposals_before_returning_bounded_proofs() {
    let oversized_class = "x".repeat(2_100);
    let document = prepare_document(
        HtmlInput::new(
            "proposal-bounds",
            format!(
                "<main id=\"anchor\" class=\"{oversized_class}\" data-empty=\"\" data-generated=\"css-991827364551\"><p class=\"same\">One</p><p class=\"same\">Two</p></main>"
            ),
        )
        .expect("source"),
        PreparationLimits::default(),
    )
    .expect("prepared");
    let result = explore(
        &document,
        &ExplorationOptions {
            max_proposals_per_element: NonZeroU32::new(1).expect("non-zero"),
            ..ExplorationOptions::default()
        },
    )
    .expect("exploration");
    let main = result
        .elements
        .iter()
        .find(|element| element.element_name.local_name == "main")
        .expect("main descriptor");
    assert_eq!(main.selector_proposals.len(), 1);
    assert!(main.selector_proposals[0].selector.as_str() == "#anchor");
    assert!(
        main.selector_proposals
            .iter()
            .all(|proposal| !proposal.selector.as_str().contains("data-generated"))
    );
    for paragraph in result
        .elements
        .iter()
        .filter(|element| element.element_name.local_name == "p")
    {
        assert!(
            paragraph
                .selector_proposals
                .iter()
                .all(|proposal| proposal.selector.as_str() != ".same")
        );
    }
}

#[test]
fn exploration_truncates_a_multibyte_attribute_at_a_character_boundary() {
    let value = format!("{}é", "x".repeat(511));
    let document = prepare_document(
        HtmlInput::new(
            "multibyte-attribute-boundary",
            format!("<main data-value=\"{value}\">One</main>"),
        )
        .expect("source"),
        PreparationLimits::default(),
    )
    .expect("prepared");
    let result = explore(&document, &ExplorationOptions::default()).expect("exploration");
    let value = result
        .elements
        .iter()
        .find(|element| element.element_name.local_name == "main")
        .and_then(|element| {
            element
                .attributes
                .iter()
                .find(|attribute| attribute.name == "data-value")
        })
        .expect("bounded data attribute");
    assert!(value.value_truncated);
    assert_eq!(value.value.len(), 511);
    assert!(value.value.is_char_boundary(value.value.len()));
}

#[test]
fn exploration_reports_the_closed_unsupported_namespace_omission() {
    assert_eq!(
        element_namespace_for_tests("urn:unsupported-namespace"),
        Err(ExplorationOmissionReason::UnsupportedNamespace)
    );
}

#[test]
fn exploration_stops_before_the_next_element_when_an_omission_exhausts_page_work() {
    let local_name = "x".repeat(257);
    let document = prepare_document(
        HtmlInput::new(
            "exhausted-after-omission",
            format!("<main><{local_name}>One</{local_name}><p>Two</p></main>"),
        )
        .expect("source"),
        PreparationLimits::default(),
    )
    .expect("prepared");
    let mut options = ExplorationOptions {
        cursor: None,
        max_elements: NonZeroU32::new(16).expect("non-zero"),
        max_work_units: NonZeroU32::new(1).expect("non-zero"),
        max_proposals_per_element: NonZeroU32::new(1).expect("non-zero"),
        preview_bytes: NonZeroU32::new(32).expect("non-zero"),
    };
    for ordinal in 1..=8 {
        options.cursor = Some(exploration_cursor_for_tests(
            &document,
            &options,
            NonZeroU32::new(ordinal).expect("non-zero"),
        ));
        let result = explore(&document, &options).expect("exploration result");
        if !result.omitted_element_counts.iter().any(|omission| {
            omission.reason == ExplorationOmissionReason::ElementNameTooLong && omission.count == 1
        }) {
            continue;
        }
        assert_eq!(
            result.truncation_reason,
            Some(ExplorationTruncationReason::WorkLimit)
        );
        assert_eq!(
            result
                .next_cursor
                .as_ref()
                .expect("continuation cursor")
                .next_document_ordinal(),
            NonZeroU32::new(ordinal + 1).expect("non-zero")
        );
        return;
    }
    panic!("one cursor position must visit the overlong element");
}
