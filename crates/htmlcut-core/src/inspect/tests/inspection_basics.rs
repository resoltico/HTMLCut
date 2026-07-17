use super::*;

#[test]
fn empty_and_fragment_documents_report_structural_counts() {
    let empty_document = Html::new_document();
    assert_eq!(
        build_document_inspection(&empty_document, None, 1).root_tag,
        "html"
    );

    let fragment = Html::parse_fragment(
        "<section class=\"fragment-box\"><form></form><script></script><style></style><table></table><img src=\"hero.png\"><a href=\"/guide\">Guide</a></section>",
    );
    let fragment_inspection = build_document_inspection(&fragment, None, 1);
    assert_eq!(fragment_inspection.root_tag, "html");
    assert_eq!(fragment_inspection.form_count, 1);
    assert_eq!(fragment_inspection.script_count, 1);
    assert_eq!(fragment_inspection.style_count, 1);
    assert_eq!(fragment_inspection.table_count, 1);
    assert_eq!(fragment_inspection.image_count, 1);
    assert_eq!(fragment_inspection.link_count, 1);
}
