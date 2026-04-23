use super::*;

#[test]
fn document_base_resolution_covers_absolute_relative_and_fallback_paths() {
    let absolute_document = parse_document_node(
        "<html><head><base href=\"https://cdn.example.com/shared/\"></head><body></body></html>",
    );
    assert_eq!(
        resolve_document_base_url(
            &absolute_document,
            Some("https://example.com/docs/start.html")
        )
        .as_deref(),
        Some("https://cdn.example.com/shared/")
    );

    let relative_document =
        parse_document_node("<html><head><base href=\"../shared/\"></head><body></body></html>");
    assert_eq!(
        resolve_document_base_url(
            &relative_document,
            Some("https://example.com/docs/start.html")
        )
        .as_deref(),
        Some("https://example.com/shared/")
    );

    assert_eq!(
        resolve_document_base_url(&relative_document, Some("not a url")).as_deref(),
        Some("not a url")
    );
}
#[test]
fn document_base_resolution_ignores_fragment_only_base_hrefs() {
    let document = parse_document_node(
        "<html><head><base href=\"#chapter-1\"></head><body><a href=\"guide.html\">Guide</a></body></html>",
    );

    assert_eq!(
        resolve_document_base_url(&document, Some("https://example.com/docs/start.html"))
            .as_deref(),
        Some("https://example.com/docs/start.html")
    );
}
#[test]
fn document_base_resolution_rejects_unsupported_absolute_schemes() {
    let document = parse_document_node(
        "<html><head><base href=\"mailto:owner@example.com\"></head><body></body></html>",
    );

    assert_eq!(
        resolve_document_base_url(&document, Some("https://example.com/docs/start.html"))
            .as_deref(),
        Some("https://example.com/docs/start.html")
    );
}
