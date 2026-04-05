use std::collections::BTreeMap;

use scraper::{ElementRef, Html};

use crate::contracts::{DocumentInspection, HeadingInspection, LinkInspection, WhitespaceMode};
use crate::document::{
    build_node_path, document_base_href, extract_document_title, first_body, heading_level,
    render_html_as_text, resolve_url, select_first, serialize_children, serialize_document,
    summarize_counts,
};

pub(crate) fn build_document_inspection(
    document: &Html,
    effective_base_url: Option<&str>,
    sample_limit: usize,
) -> DocumentInspection {
    let root_tag = select_first(document, "html")
        .map(|html| html.value().name().to_owned())
        .or_else(|| first_body(document).map(|body| body.value().name().to_owned()))
        .unwrap_or_else(|| "html".to_owned());
    let body_html = first_body(document)
        .map(|body| serialize_children(&body))
        .unwrap_or_else(|| serialize_document(document));
    let body_text = render_html_as_text(&body_html, WhitespaceMode::Normalize);
    let mut tag_counts = BTreeMap::<String, usize>::new();
    let mut class_counts = BTreeMap::<String, usize>::new();
    let mut headings = Vec::new();
    let mut links = Vec::new();
    let mut link_count = 0usize;
    let mut image_count = 0usize;
    let mut form_count = 0usize;
    let mut table_count = 0usize;
    let mut script_count = 0usize;
    let mut style_count = 0usize;
    let mut element_count = 0usize;

    for node_ref in document.tree.nodes() {
        let Some(element) = ElementRef::wrap(node_ref) else {
            continue;
        };

        let tag_name = element.value().name().to_owned();
        *tag_counts.entry(tag_name.clone()).or_insert(0) += 1;
        element_count += 1;

        match tag_name.as_str() {
            "a" => {
                link_count += 1;
                if links.len() < sample_limit {
                    let href = element.value().attr("href").map(ToOwned::to_owned);
                    let resolved_href = href
                        .as_deref()
                        .map(|value| resolve_url(value, effective_base_url));
                    links.push(LinkInspection {
                        text: render_html_as_text(
                            &serialize_children(&element),
                            WhitespaceMode::Normalize,
                        ),
                        href,
                        resolved_href,
                        path: build_node_path(&element),
                    });
                }
            }
            "img" => image_count += 1,
            "form" => form_count += 1,
            "table" => table_count += 1,
            "script" => script_count += 1,
            "style" => style_count += 1,
            _ => {}
        }

        if let Some(classes) = element.value().attr("class") {
            for class_name in classes.split_whitespace() {
                *class_counts.entry(class_name.to_owned()).or_insert(0) += 1;
            }
        }

        if headings.len() < sample_limit
            && let Some(level) = heading_level(tag_name.as_str())
        {
            headings.push(HeadingInspection {
                level,
                text: render_html_as_text(&serialize_children(&element), WhitespaceMode::Normalize),
                path: build_node_path(&element),
            });
        }
    }

    DocumentInspection {
        title: extract_document_title(document),
        root_tag,
        element_count,
        text_char_count: body_text.chars().count(),
        link_count,
        image_count,
        form_count,
        table_count,
        script_count,
        style_count,
        document_base_href: document_base_href(document),
        top_tags: summarize_counts(tag_counts, sample_limit),
        top_classes: summarize_counts(class_counts, sample_limit),
        headings,
        links,
    }
}
