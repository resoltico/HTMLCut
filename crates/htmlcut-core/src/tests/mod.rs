use super::*;
use crate::contracts::{
    default_fetch_timeout_ms, default_inspection_sample_limit, default_max_bytes,
    default_preview_chars, default_regex_flags, default_spec_version, default_true,
};
use crate::document::{
    ELLIPSIS, attribute_supports_url_rewrite, collapse_blank_lines_for_tests, element_name,
    rewrite_srcset_for_tests,
};
use crate::extract::position_inside_markup_for_tests;
use crate::result::ExtractionMatchMetadata;
use crate::source::{
    content_type_is_obviously_non_html_for_tests, finish_url_source_from_reader_for_tests,
    head_error_allows_get_fallback_for_tests, read_stdin_source_from_reader_for_tests,
};
use scraper::ElementRef;
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::io;
use std::io::{Cursor, Error as IoError, Read, Write};
use std::net::TcpListener;
use std::num::NonZeroUsize;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::thread;
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
    SliceSpec::regex(
        slice_boundary(from),
        slice_boundary(to),
        default_regex_flags(),
    )
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

mod catalog;
mod document;
mod extraction;
mod inspection;
mod interop_v1;
mod source;
