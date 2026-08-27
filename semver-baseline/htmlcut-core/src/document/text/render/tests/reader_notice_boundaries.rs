use crate::document::{parse_document_node, select_first};

use super::super::super::policy::{collect_notice_text, element_looks_like_brief_reader_notice};

fn is_notice(text: &str, include_link: bool) -> bool {
    let link = if include_link {
        "<a href=\"/terms\">More</a>"
    } else {
        ""
    };
    let document = parse_document_node(&format!("<span>{text} {link}</span>"));
    let element = select_first(&document, "span").expect("notice fixture");
    element_looks_like_brief_reader_notice(&element)
}

fn rendered_notice_len(text: &str) -> usize {
    let document = parse_document_node(&format!("<span>{text} <a href=\"/terms\">More</a></span>"));
    let element = select_first(&document, "span").expect("notice fixture");
    collect_notice_text(*element, 421).chars().count()
}

fn pad_notice_to(text: &str, length: usize) -> String {
    format!("{text}{}", "a".repeat(length - rendered_notice_len(text)))
}

#[test]
fn reader_notice_policy_requires_a_link_and_recognizes_phrase_or_token_evidence() {
    let affiliate_notice =
        "When you purchase through links on our site, we may earn an affiliate commission. ";
    assert!(!is_notice(affiliate_notice, false));
    assert!(is_notice(affiliate_notice, true));
    assert!(is_notice("affiliate sponsored privacy reader notice", true));
    assert!(!is_notice("ordinary reader update", true));

    let short_token_notice =
        parse_document_node("<span>earn links terms abc <a href=\"/terms\">1</a></span>");
    let short_token_notice_element =
        select_first(&short_token_notice, "span").expect("short token notice");
    assert_eq!(
        collect_notice_text(*short_token_notice_element, 421)
            .chars()
            .filter(|character| character.is_alphabetic())
            .count(),
        17
    );
    assert!(!element_looks_like_brief_reader_notice(
        &short_token_notice_element
    ));

    let boundary_token_notice =
        parse_document_node("<span>earn links terms abcd <a href=\"/terms\">1</a></span>");
    let boundary_token_notice_element =
        select_first(&boundary_token_notice, "span").expect("boundary token notice");
    assert_eq!(
        collect_notice_text(*boundary_token_notice_element, 421)
            .chars()
            .filter(|character| character.is_alphabetic())
            .count(),
        18
    );
    assert!(element_looks_like_brief_reader_notice(
        &boundary_token_notice_element
    ));
}

#[test]
fn reader_notice_policy_enforces_its_documented_length_boundaries() {
    let prefix =
        "When you purchase through links on our site, we may earn an affiliate commission. ";
    let at_240 = pad_notice_to(prefix, 239);
    let at_241 = pad_notice_to(prefix, 240);
    assert_eq!(rendered_notice_len(&at_240), 240);
    assert_eq!(rendered_notice_len(&at_241), 241);
    assert!(is_notice(&at_240, true));
    assert!(!is_notice(&at_241, true));

    let strong_prefix = "this article was generated for a reader-facing summary. ";
    let at_420 = pad_notice_to(strong_prefix, 419);
    let at_421 = pad_notice_to(strong_prefix, 420);
    assert_eq!(rendered_notice_len(&at_420), 420);
    assert_eq!(rendered_notice_len(&at_421), 421);
    assert!(is_notice(&at_420, true));
    assert!(!is_notice(&at_421, true));
}
