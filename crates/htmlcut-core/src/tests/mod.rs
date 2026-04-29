use super::*;
use crate::contracts::{
    default_fetch_timeout_ms, default_inspection_sample_limit, default_max_bytes,
    default_preview_chars, default_spec_version, default_true,
};
use crate::diagnostics::{error_diagnostic, has_errors, warning_diagnostic};
use crate::document::{
    ELLIPSIS, apply_whitespace_mode, attribute_supports_url_rewrite, build_node_path,
    build_preview, collapse_blank_lines_for_tests, collapse_inline_whitespace, element_attributes,
    element_name, first_body, first_body_child_element, first_fragment_attributes,
    looks_like_full_document, needs_space, parse_document_node, parse_wrapped_fragment,
    push_newline, render_element_as_text, render_html_as_text, render_node,
    resolve_document_base_url, resolve_url, rewrite_html_urls, rewrite_srcset_for_tests,
    rewrite_urls_in_document, select_first, serialize_children, serialize_document,
};
use crate::extract::{
    build_finder, build_regex, build_selector_match, build_slice_match, extract_slice_candidates,
    position_inside_markup_for_tests, run_selector_extraction, run_slice_extraction,
    select_candidates, validate_request,
};
use crate::result::ExtractionMatchMetadata;
#[cfg(feature = "http-client")]
use crate::source::read_url_source_from_href;
use crate::source::{
    LoadedSource, load_source, read_file_source_from_path, read_limited_to_string, source_metadata,
};
#[cfg(feature = "http-client")]
use crate::source::{
    build_http_agent, content_type_is_obviously_non_html_for_tests,
    finish_url_source_from_reader_for_tests, head_error_allows_get_fallback_for_tests,
    read_stdin_source_from_reader_for_tests,
};
#[cfg(not(feature = "http-client"))]
use crate::source::{
    finish_url_source_from_reader_for_tests, read_stdin_source_from_reader_for_tests,
};
use scraper::ElementRef;
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::io;
#[cfg(feature = "http-client")]
use std::io::Write;
use std::io::{Cursor, Error as IoError, Read};
#[cfg(feature = "http-client")]
use std::net::TcpListener;
use std::num::NonZeroUsize;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::str::FromStr;
#[cfg(feature = "http-client")]
use std::sync::{Arc, Mutex};
#[cfg(feature = "http-client")]
use std::thread;
#[cfg(feature = "http-client")]
use ureq::tls::RootCerts;
use url::Url;

use crate::source::empty_source_metadata;

fn selector_request(html: &str) -> ExtractionRequest {
    ExtractionRequest::new(
        SourceRequest::memory("inline", html),
        ExtractionSpec::selector(selector_query("article")),
    )
}

fn selector_query(selector: &str) -> SelectorQuery {
    SelectorQuery::new(selector).expect("selector")
}

fn slice_boundary(boundary: &str) -> SliceBoundary {
    SliceBoundary::new(boundary).expect("slice boundary")
}

fn slice_spec(from: &str, to: &str) -> SliceSpec {
    SliceSpec::new(slice_boundary(from), slice_boundary(to))
}

fn regex_slice_spec(from: &str, to: &str) -> SliceSpec {
    SliceSpec::regex(slice_boundary(from), slice_boundary(to), "")
}

fn slice_request(html: &str, from: &str, to: &str) -> ExtractionRequest {
    ExtractionRequest::new(
        SourceRequest::memory("inline", html),
        ExtractionSpec::slice(slice_spec(from, to)),
    )
}

fn nth_selection(index: usize) -> SelectionSpec {
    SelectionSpec::nth(NonZeroUsize::new(index).expect("match index"))
}

fn attribute_value(name: &str) -> ValueSpec {
    ValueSpec::Attribute {
        name: AttributeName::new(name).expect("attribute name"),
    }
}

fn memory_source(label: &str, text: impl Into<String>) -> SourceRequest {
    SourceRequest::memory(label, text)
}

fn memory_source_with_base(label: &str, text: impl Into<String>, base_url: &str) -> SourceRequest {
    SourceRequest::memory(label, text).with_base_url(Url::parse(base_url).expect("base url"))
}

fn file_source(path: impl AsRef<Path>) -> SourceRequest {
    SourceRequest::file(path.as_ref())
}

fn url_source(url: &str) -> SourceRequest {
    SourceRequest::url(Url::parse(url).expect("url"))
}

fn read_file_source(
    source: &SourceRequest,
    runtime: &RuntimeOptions,
) -> Result<LoadedSource, crate::source::SourceLoadFailure> {
    let SourceInput::File { path } = &source.input else {
        panic!("test helper expected a file source request");
    };
    read_file_source_from_path(source, path, runtime)
}

#[cfg(feature = "http-client")]
fn read_url_source(
    source: &SourceRequest,
    runtime: &RuntimeOptions,
) -> Result<LoadedSource, crate::source::SourceLoadFailure> {
    let SourceInput::Url { href } = &source.input else {
        panic!("test helper expected a URL source request");
    };
    read_url_source_from_href(source, href, runtime)
}

mod catalog;
mod document;
mod extraction;
mod inspection;
mod interop_v1;
mod source;
