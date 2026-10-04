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
