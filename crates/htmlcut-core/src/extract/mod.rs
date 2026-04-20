mod engine;
mod selector;
mod slice;

use serde::Serialize;

use crate::catalog::OperationId;
use crate::contracts::{Diagnostic, ExtractionMatch, SourceMetadata};
use crate::result::Range;

pub(crate) use engine::ExtractionRun;
#[cfg(test)]
pub(crate) use engine::validate_request;
pub use engine::{extract, inspect_source, parse_document, preview_extraction};
#[cfg(test)]
pub(crate) use selector::build_selector_match;
pub(crate) use selector::run_selector_extraction;
pub(crate) use slice::run_slice_extraction;
#[cfg(test)]
pub(crate) use slice::{
    build_finder, build_regex, build_slice_match, extract_slice_candidates,
    position_inside_markup_for_tests, select_candidates,
};

#[derive(Clone, Debug)]
pub(crate) struct SliceCandidate {
    pub(crate) inner_html: String,
    pub(crate) outer_html: String,
    pub(crate) selected_html: String,
    pub(crate) selected_range: Range,
    pub(crate) inner_range: Range,
    pub(crate) outer_range: Range,
    pub(crate) matched_start: String,
    pub(crate) matched_end: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SelectedCandidate<T> {
    pub(crate) candidate_index: usize,
    pub(crate) candidate: T,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SelectorStructuredValue {
    match_index: usize,
    match_count: usize,
    candidate_index: usize,
    candidate_count: usize,
    tag_name: String,
    path: String,
    text: String,
    html: String,
    outer_html: String,
    attributes: std::collections::BTreeMap<String, String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SliceStructuredValue {
    match_index: usize,
    match_count: usize,
    candidate_index: usize,
    candidate_count: usize,
    text: String,
    html: String,
    inner_html: String,
    outer_html: String,
    selected_range: Range,
    inner_range: Range,
    outer_range: Range,
    include_start: bool,
    include_end: bool,
    matched_start: String,
    matched_end: String,
}

#[derive(Clone, Debug)]
pub(crate) struct FinalizedExtraction {
    pub(crate) operation_id: OperationId,
    pub(crate) source: SourceMetadata,
    pub(crate) document_title: Option<String>,
    pub(crate) diagnostics: Vec<Diagnostic>,
    pub(crate) matches: Vec<ExtractionMatch>,
    pub(crate) candidate_count: usize,
}

#[derive(Clone, Copy)]
pub(crate) struct FoundRange {
    pub(crate) start: usize,
    pub(crate) end: usize,
}

pub(crate) type Finder = Box<dyn Fn(&str, usize) -> Option<FoundRange>>;
