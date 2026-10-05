// SPDX-License-Identifier: MPL-2.0
use super::*;

#[test]
fn sampling_an_empty_element_charges_the_attribute_name_and_original_node() {
    let html = scraper::Html::parse_document("<p id='x'></p>");
    let root = html
        .select(&scraper::Selector::parse("p").unwrap())
        .next()
        .unwrap();
    for (units, accepted) in [(2, false), (3, true)] {
        let budget = SelectorWorkBudget::new(units);
        let result = sample(root, &budget);
        if accepted {
            let value = result.unwrap();
            assert_eq!(value.attributes, ["id"]);
            assert_eq!(value.text, "");
            assert!(value.text_complete && value.attributes_complete);
            assert_eq!(budget.remaining(), 0);
        } else {
            assert_eq!(result.unwrap_err().code, ErrorCode::ResourceLimit);
        }
    }
}

#[test]
fn identifier_sampling_charges_attribute_values_tokens_and_edges() {
    let html = scraper::Html::parse_document("<p id=x class=y></p>");
    let root = html
        .select(&scraper::Selector::parse("p").unwrap())
        .next()
        .unwrap();
    for units in 1..7 {
        let budget = SelectorWorkBudget::new(units);
        assert_eq!(
            identifier_sample(root, &budget).unwrap_err().code,
            ErrorCode::ResourceLimit
        );
    }
    let budget = SelectorWorkBudget::new(7);
    let sample = identifier_sample(root, &budget).unwrap();
    assert_eq!(sample.id.as_deref(), Some("x"));
    assert_eq!(sample.classes, ["y"]);
    assert_eq!(budget.remaining(), 0);

    let empty = scraper::Html::parse_document("<p></p>");
    let root = empty
        .select(&scraper::Selector::parse("p").unwrap())
        .next()
        .unwrap();
    let budget = SelectorWorkBudget::new(2);
    let sample = identifier_sample(root, &budget).unwrap();
    assert_eq!(sample.id, None);
    assert!(sample.classes.is_empty() && sample.identifiers_complete);
}
