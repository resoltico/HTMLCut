mod engine;
mod selector;
mod slice;

use crate::catalog::OperationId;
use crate::contracts::{Diagnostic, ExtractionMatch, SourceMetadata};
use crate::result::Range;

pub(crate) use engine::ExtractionRun;
#[cfg(test)]
pub(crate) use engine::validate_request;
pub use engine::{extract, inspect_source, parse_document, preview_extraction};
#[cfg(test)]
pub(crate) use selector::build_selector_match;
#[cfg(test)]
pub(crate) use selector::run_selector_extraction;
pub(crate) use selector::{run_validated_selector_extraction, validate_selector_query};
#[cfg(test)]
pub(crate) use slice::run_slice_extraction;
pub(crate) use slice::run_validated_slice_extraction;
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
