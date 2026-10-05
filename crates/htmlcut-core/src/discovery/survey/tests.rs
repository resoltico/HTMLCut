// SPDX-License-Identifier: MPL-2.0
//! Exact work-accounting controls for the bounded survey helpers.

use scraper::Html;
use selectors::work_budget::SelectorWorkBudget;

use super::{class_signature, element_descriptor, samples};
use crate::ErrorCode;

#[test]
fn sample_budget_failure_cannot_publish_a_partial_preview() {
    let document = Html::parse_document("<p>one</p><p>two</p>");
    let members = document
        .tree
        .nodes()
        .filter_map(scraper::ElementRef::wrap)
        .filter(|element| element.value().name() == "p")
        .collect::<Vec<_>>();
    assert_eq!(
        samples(&members, &SelectorWorkBudget::new(1))
            .unwrap_err()
            .code,
        ErrorCode::ResourceLimit
    );
}

#[test]
fn identifier_scans_charge_the_value_and_processing_work() {
    let class = "c".repeat(64);
    let id = "i".repeat(64);
    let document = Html::parse_document(&format!("<div id='{id}' class='{class}'></div>"));
    let element = document
        .tree
        .nodes()
        .filter_map(scraper::ElementRef::wrap)
        .find(|element| element.value().name() == "div")
        .unwrap();
    assert_eq!(
        class_signature(element, &SelectorWorkBudget::new(2))
            .unwrap_err()
            .code,
        ErrorCode::ResourceLimit
    );
    assert_eq!(
        class_signature(element, &SelectorWorkBudget::new(3)).unwrap(),
        Some(vec![class])
    );

    let document = Html::parse_document(&format!("<div id='{id}'></div>"));
    let element = document
        .tree
        .nodes()
        .filter_map(scraper::ElementRef::wrap)
        .find(|element| element.value().name() == "div")
        .unwrap();
    assert_eq!(
        element_descriptor(element, &SelectorWorkBudget::new(1))
            .unwrap_err()
            .code,
        ErrorCode::ResourceLimit
    );
    assert_eq!(
        element_descriptor(element, &SelectorWorkBudget::new(2))
            .unwrap()
            .id,
        Some(id)
    );
}
