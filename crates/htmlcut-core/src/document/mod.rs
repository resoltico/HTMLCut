mod parse;
mod render;
mod summary;
mod urls;

pub(crate) use parse::parse_wrapped_fragment;
pub(crate) use parse::{
    build_node_path, element_attributes, first_body, first_body_child_element, parse_document_node,
    select_first, serialize_children, serialize_document, serialize_element,
};
#[cfg(test)]
pub(crate) use parse::{element_name, first_fragment_attributes};
#[cfg(test)]
pub(crate) use render::{
    ELLIPSIS, collapse_blank_lines_for_tests, collapse_inline_whitespace, needs_space,
    push_newline, render_node,
};
pub(crate) use render::{
    apply_whitespace_mode, render_document_body_as_text, render_element_as_text,
    render_html_as_text,
};
pub(crate) use summary::{build_preview, extract_document_title, heading_level, summarize_counts};
#[cfg(test)]
pub(crate) use urls::{
    attribute_supports_url_rewrite, looks_like_full_document, rewrite_srcset_for_tests,
    rewrite_urls_in_document_with_node_ids_for_tests,
};
pub(crate) use urls::{
    document_base_href, resolve_document_base_url, resolve_url, rewrite_html_urls,
    rewrite_urls_in_document,
};
