//! URL resolution and rewriting for parsed HTML documents and fragments.

mod base;
mod css;
mod rewrite;
#[cfg(test)]
mod tests;

#[cfg(test)]
use base::starts_with_ignore_ascii_case;
pub(crate) use base::{
    document_base_href, href_is_meaningful_destination, resolve_document_base_url, resolve_url,
};
#[cfg(test)]
use css::rewrite_css_urls;
#[cfg(test)]
pub(crate) use css::rewrite_css_urls_for_tests;
#[cfg(test)]
use css::{
    css_comment_end, find_css_string_end, is_css_identifier_char, rewrite_css_import_string_at,
    rewrite_css_url_function_at, skip_ascii_whitespace,
};
#[cfg(test)]
use rewrite::raw_element_is_meta_refresh;
#[cfg(test)]
pub(crate) use rewrite::{
    attribute_supports_url_rewrite, rewrite_srcset_for_tests,
    rewrite_urls_in_document_with_node_ids_for_tests,
};
pub(crate) use rewrite::{
    looks_like_full_document, rewrite_attribute_value, rewrite_html_urls, rewrite_urls_in_document,
};
