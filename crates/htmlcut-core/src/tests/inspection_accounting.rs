// SPDX-License-Identifier: MPL-2.0
use super::*;

#[test]
fn identifier_sampling_charges_attribute_values_tokens_and_edges() {
    let html = scraper::Html::parse_document("<p id=x class=y></p>");
    let root = html
        .select(&scraper::Selector::parse("p").unwrap())
        .next()
        .unwrap();
    for units in 1..14 {
        let budget = SelectorWorkBudget::new(units);
        assert_eq!(
            identifier_sample(root, &budget).unwrap_err().code,
            ErrorCode::ResourceLimit
        );
    }
    let budget = SelectorWorkBudget::new(14);
    let sample = identifier_sample(root, &budget).unwrap();
    assert_eq!(sample.id.as_deref(), Some("x"));
    assert_eq!(sample.classes, ["y"]);
    assert_eq!(budget.remaining(), 0);

    let empty = scraper::Html::parse_document("<p></p>");
    let root = empty
        .select(&scraper::Selector::parse("p").unwrap())
        .next()
        .unwrap();
    let budget = SelectorWorkBudget::new(5);
    let sample = identifier_sample(root, &budget).unwrap();
    assert_eq!(sample.id, None);
    assert!(sample.classes.is_empty() && sample.identifiers_complete);
}
