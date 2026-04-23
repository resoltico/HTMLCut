use serde_json::{Value, json};

use crate::contracts::{
    DelimiterPairMatchMetadata, Diagnostic, ExtractionMatch, ExtractionMatchMetadata,
    ExtractionRequest, SliceSpec, ValueSpec,
};
use crate::diagnostics::{DiagnosticCode, error_diagnostic, unresolved_effective_base_diagnostic};
use crate::document::{
    apply_whitespace_mode, build_preview, document_base_href, extract_document_title,
    first_fragment_attributes, parse_document_node, render_html_as_text, resolve_document_base_url,
    rewrite_html_urls,
};
use crate::source::LoadedSource;

use super::super::{ExtractionRun, SliceCandidate, SliceStructuredValue};
use super::markup::slice_markup_diagnostics;
use super::patterns::extract_slice_candidates;
use super::selection::select_candidates;

pub(crate) fn run_slice_extraction(
    request: &ExtractionRequest,
    source: &LoadedSource,
) -> ExtractionRun {
    let document = parse_document_node(&source.text);
    let document_title = extract_document_title(&document);
    let effective_base_url = resolve_document_base_url(&document, source.input_base_url.as_deref());
    let mut diagnostics = if request.normalization.rewrite_urls && effective_base_url.is_none() {
        vec![unresolved_effective_base_diagnostic(
            document_base_href(&document).as_deref(),
            true,
        )]
    } else {
        Vec::new()
    };
    let slice = request
        .extraction
        .slice_spec()
        .expect("slice extraction should carry slice boundaries");

    let candidates = match extract_slice_candidates(&source.text, slice) {
        Ok(candidates) => candidates,
        Err(diagnostic) => {
            return ExtractionRun {
                document_title,
                effective_base_url,
                candidate_count: 0,
                diagnostics: {
                    diagnostics.push(diagnostic);
                    diagnostics
                },
                matches: Vec::new(),
            };
        }
    };

    let candidate_count = candidates.len();
    let (selected, selection_diagnostics) =
        select_candidates(&candidates, request.extraction.selection());
    diagnostics.extend(selection_diagnostics);
    diagnostics.extend(slice_markup_diagnostics(&source.text, &selected));
    let mut matches = Vec::new();
    let match_count = selected.len();

    for (position, selected_candidate) in selected.iter().enumerate() {
        match build_slice_match(
            request,
            effective_base_url.as_deref(),
            &selected_candidate.candidate,
            position + 1,
            match_count,
            selected_candidate.candidate_index,
            candidate_count,
        ) {
            Ok(extraction_match) => matches.push(extraction_match),
            Err(diagnostic) => diagnostics.push(diagnostic),
        }
    }

    ExtractionRun {
        document_title,
        effective_base_url,
        candidate_count,
        diagnostics,
        matches,
    }
}

pub(crate) fn build_slice_match(
    request: &ExtractionRequest,
    effective_base_url: Option<&str>,
    candidate: &SliceCandidate,
    match_index: usize,
    match_count: usize,
    candidate_index: usize,
    candidate_count: usize,
) -> Result<ExtractionMatch, Diagnostic> {
    let value_spec = request.extraction.value();
    let slice = request
        .extraction
        .slice_spec()
        .expect("slice extraction should carry slice boundaries");
    let selected_html = rewrite_html_urls(&candidate.selected_html, effective_base_url, false);
    let outer_html = rewrite_html_urls(&candidate.outer_html, effective_base_url, false);
    let inner_html = rewrite_html_urls(&candidate.inner_html, effective_base_url, false);
    let text = render_html_as_text(&selected_html, request.normalization.whitespace);
    let value = match value_spec {
        ValueSpec::Text => Value::String(text.clone()),
        ValueSpec::InnerHtml => Value::String(selected_html.clone()),
        ValueSpec::OuterHtml => Value::String(outer_html.clone()),
        ValueSpec::Attribute { name } => build_attribute_value(
            request,
            slice,
            candidate,
            &selected_html,
            effective_base_url,
            name.as_str(),
        )?,
        ValueSpec::Structured => serde_json::to_value(SliceStructuredValue {
            match_index,
            match_count,
            candidate_index,
            candidate_count,
            text: text.clone(),
            html: selected_html.clone(),
            inner_html: inner_html.clone(),
            outer_html: outer_html.clone(),
            selected_range: candidate.selected_range.clone(),
            inner_range: candidate.inner_range.clone(),
            outer_range: candidate.outer_range.clone(),
            include_start: slice.include_start,
            include_end: slice.include_end,
            matched_start: candidate.matched_start.clone(),
            matched_end: candidate.matched_end.clone(),
        })
        .expect("slice structured value should serialize"),
    };

    Ok(ExtractionMatch {
        index: match_index,
        path: None,
        value_type: value_spec.value_type(),
        preview: build_preview(&value, request.output.preview_chars.get()),
        value,
        html: request.output.include_html.then_some(outer_html.clone()),
        text: request.output.include_text.then_some(text),
        metadata: ExtractionMatchMetadata::DelimiterPair(DelimiterPairMatchMetadata {
            candidate_count,
            candidate_index,
            selected_range: candidate.selected_range.clone(),
            inner_range: candidate.inner_range.clone(),
            outer_range: candidate.outer_range.clone(),
            include_start: slice.include_start,
            include_end: slice.include_end,
            matched_start: candidate.matched_start.clone(),
            matched_end: candidate.matched_end.clone(),
        }),
    })
}

fn build_attribute_value(
    request: &ExtractionRequest,
    slice: &SliceSpec,
    candidate: &SliceCandidate,
    selected_html: &str,
    effective_base_url: Option<&str>,
    attribute_name: &str,
) -> Result<Value, Diagnostic> {
    let attributes = first_fragment_attributes(
        selected_html,
        effective_base_url,
        request.normalization.rewrite_urls,
    );
    let Some(value) = attributes.get(attribute_name) else {
        let hint_include_start =
            !slice.include_start && candidate.selected_range.start != candidate.outer_range.start;
        let message = if hint_include_start {
            format!(
                "Extracted fragment is missing attribute \"{attribute_name}\". If the attribute lives on the opening tag, use --include-start so the fragment keeps that tag."
            )
        } else {
            format!("Extracted fragment is missing attribute \"{attribute_name}\".")
        };
        return Err(error_diagnostic(
            DiagnosticCode::MissingAttribute,
            message,
            Some(json!({
                "attribute": attribute_name,
                "selectedRange": candidate.selected_range,
                "hint": hint_include_start.then_some("use --include-start"),
            })),
        ));
    };

    Ok(Value::String(apply_whitespace_mode(
        value,
        request.normalization.whitespace,
    )))
}
