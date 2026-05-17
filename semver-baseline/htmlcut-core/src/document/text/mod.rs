mod policy;
mod render;
mod signals;
mod vocabulary;

pub(crate) use render::ELLIPSIS;
pub(crate) use render::{
    apply_whitespace_mode, extract_heading_text, render_element_as_text,
    render_element_children_as_text, render_html_as_text, render_selected_document_body_as_text,
};
#[cfg(test)]
pub(crate) use render::{
    collapse_blank_lines_for_tests, collapse_inline_whitespace, needs_space, push_newline,
    render_document_body_as_text, render_node,
};
pub(crate) use signals::{
    element_has_utility_chrome_ancestor, element_looks_like_utility_chrome,
    structural_signal_tokens, token_match_count,
};
