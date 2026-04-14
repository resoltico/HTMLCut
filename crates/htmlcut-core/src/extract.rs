use std::time::Instant;

use regex::RegexBuilder;
use scraper::{ElementRef, Selector};
use serde::Serialize;
use serde_json::{Value, json};

use crate::catalog::OperationId;
use crate::contracts::{
    CORE_RESULT_SCHEMA_NAME, CORE_RESULT_SCHEMA_VERSION, CORE_SOURCE_INSPECTION_SCHEMA_NAME,
    CORE_SOURCE_INSPECTION_SCHEMA_VERSION, CORE_SPEC_VERSION, DEFAULT_REGEX_FLAGS,
    DelimiterPairMatchMetadata, Diagnostic, ExtractionMatch, ExtractionMatchMetadata,
    ExtractionRequest, ExtractionResult, ExtractionStats, ExtractionStrategy, InspectionOptions,
    ParseDocumentResult, ParsedDocument, PatternMode, RuntimeOptions, SelectionSpec,
    SelectorMatchMetadata, SliceSpec, SourceInspectionResult, SourceMetadata, SourceRequest,
    ValueSpec,
};
use crate::diagnostics::{
    error_diagnostic, has_errors, unresolved_effective_base_diagnostic, warning_diagnostic,
};
use crate::document::{
    apply_whitespace_mode, build_node_path, build_preview, document_base_href, element_attributes,
    extract_document_title, first_fragment_attributes, parse_document_node, render_html_as_text,
    resolve_document_base_url, rewrite_html_urls, serialize_children, serialize_element,
};
use crate::inspect::build_document_inspection;
use crate::result::Range;
use crate::source::{LoadedSource, empty_source_metadata, load_source, source_metadata};

#[derive(Clone, Debug)]
pub(crate) struct SliceCandidate {
    pub(crate) inner_html: String,
    pub(crate) outer_html: String,
    pub(crate) selected_html: String,
    pub(crate) selected_range: Range,
    pub(crate) inner_range: Range,
    pub(crate) outer_range: Range,
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
}

#[derive(Clone, Debug)]
pub(crate) struct FinalizedExtraction {
    operation_id: OperationId,
    source: SourceMetadata,
    document_title: Option<String>,
    diagnostics: Vec<Diagnostic>,
    matches: Vec<ExtractionMatch>,
    candidate_count: usize,
}

/// Loads and parses a source so callers can inspect the document tree directly.
pub fn parse_document(source: &SourceRequest, runtime: &RuntimeOptions) -> ParseDocumentResult {
    match load_source(source, runtime) {
        Ok(loaded) => {
            let document = parse_document_node(&loaded.text);
            let effective_base_url =
                resolve_document_base_url(&document, loaded.input_base_url.as_deref());
            let metadata = source_metadata(&loaded, false, effective_base_url);
            ParseDocumentResult {
                operation_id: OperationId::DocumentParse,
                ok: true,
                source: metadata.clone(),
                diagnostics: Vec::new(),
                document: Some(ParsedDocument {
                    source: metadata,
                    document,
                }),
            }
        }
        Err(diagnostic) => ParseDocumentResult {
            operation_id: OperationId::DocumentParse,
            ok: false,
            source: empty_source_metadata(source),
            diagnostics: vec![diagnostic],
            document: None,
        },
    }
}

/// Produces a structured source summary that helps callers choose extraction strategies.
pub fn inspect_source(
    source: &SourceRequest,
    runtime: &RuntimeOptions,
    options: &InspectionOptions,
) -> SourceInspectionResult {
    match load_source(source, runtime) {
        Ok(loaded) => {
            let document = parse_document_node(&loaded.text);
            let effective_base_url =
                resolve_document_base_url(&document, loaded.input_base_url.as_deref());
            let document_inspection = build_document_inspection(
                &document,
                effective_base_url.as_deref(),
                options.sample_limit,
            );
            let diagnostics = if document_inspection.document_base_href.is_some()
                && effective_base_url.is_none()
            {
                vec![unresolved_effective_base_diagnostic(
                    document_inspection.document_base_href.as_deref(),
                    false,
                )]
            } else {
                Vec::new()
            };
            let metadata = source_metadata(
                &loaded,
                options.include_source_text,
                effective_base_url.clone(),
            );
            SourceInspectionResult {
                operation_id: OperationId::SourceInspect,
                schema_name: CORE_SOURCE_INSPECTION_SCHEMA_NAME.to_owned(),
                schema_version: CORE_SOURCE_INSPECTION_SCHEMA_VERSION,
                ok: true,
                source: metadata,
                document: Some(document_inspection),
                diagnostics,
            }
        }
        Err(diagnostic) => SourceInspectionResult {
            operation_id: OperationId::SourceInspect,
            schema_name: CORE_SOURCE_INSPECTION_SCHEMA_NAME.to_owned(),
            schema_version: CORE_SOURCE_INSPECTION_SCHEMA_VERSION,
            ok: false,
            source: empty_source_metadata(source),
            document: None,
            diagnostics: vec![diagnostic],
        },
    }
}

/// Executes the extraction request but keeps the full structured report for inspection.
pub fn preview_extraction(
    request: &ExtractionRequest,
    runtime: &RuntimeOptions,
) -> ExtractionResult {
    run_extraction(request, runtime, true)
}

/// Executes the extraction request and returns the final structured extraction result.
pub fn extract(request: &ExtractionRequest, runtime: &RuntimeOptions) -> ExtractionResult {
    run_extraction(request, runtime, false)
}

const fn extraction_operation_id(strategy: ExtractionStrategy, preview: bool) -> OperationId {
    match (preview, strategy) {
        (true, ExtractionStrategy::Selector) => OperationId::SelectPreview,
        (true, ExtractionStrategy::Slice) => OperationId::SlicePreview,
        (false, ExtractionStrategy::Selector) => OperationId::SelectExtract,
        (false, ExtractionStrategy::Slice) => OperationId::SliceExtract,
    }
}

pub(crate) fn run_extraction(
    request: &ExtractionRequest,
    runtime: &RuntimeOptions,
    preview: bool,
) -> ExtractionResult {
    let started_at = Instant::now();
    let mut diagnostics = validate_request(request);

    if has_errors(&diagnostics) {
        return finalize_result(
            request,
            FinalizedExtraction {
                operation_id: extraction_operation_id(request.extraction.strategy(), preview),
                source: empty_source_metadata(&request.source),
                document_title: None,
                diagnostics,
                matches: Vec::new(),
                candidate_count: 0,
            },
            started_at,
        );
    }

    let loaded = match load_source(&request.source, runtime) {
        Ok(source) => source,
        Err(diagnostic) => {
            diagnostics.push(diagnostic);
            return finalize_result(
                request,
                FinalizedExtraction {
                    operation_id: extraction_operation_id(request.extraction.strategy(), preview),
                    source: empty_source_metadata(&request.source),
                    document_title: None,
                    diagnostics,
                    matches: Vec::new(),
                    candidate_count: 0,
                },
                started_at,
            );
        }
    };

    let extraction = match request.extraction.strategy() {
        ExtractionStrategy::Selector => run_selector_extraction(request, &loaded),
        ExtractionStrategy::Slice => run_slice_extraction(request, &loaded),
    };
    let source_meta = source_metadata(
        &loaded,
        request.output.include_source_text,
        extraction.effective_base_url.clone(),
    );

    diagnostics.extend(extraction.diagnostics);
    finalize_result(
        request,
        FinalizedExtraction {
            operation_id: extraction_operation_id(request.extraction.strategy(), preview),
            source: source_meta,
            document_title: extraction.document_title,
            diagnostics,
            matches: extraction.matches,
            candidate_count: extraction.candidate_count,
        },
        started_at,
    )
}

pub(crate) fn finalize_result(
    request: &ExtractionRequest,
    finalized: FinalizedExtraction,
    started_at: Instant,
) -> ExtractionResult {
    ExtractionResult {
        operation_id: finalized.operation_id,
        schema_name: CORE_RESULT_SCHEMA_NAME.to_owned(),
        schema_version: CORE_RESULT_SCHEMA_VERSION,
        ok: !has_errors(&finalized.diagnostics),
        source: finalized.source,
        document_title: finalized.document_title,
        extraction: request.extraction.clone(),
        stats: ExtractionStats {
            duration_ms: started_at.elapsed().as_millis(),
            candidate_count: finalized.candidate_count,
            match_count: finalized.matches.len(),
        },
        matches: finalized.matches,
        diagnostics: finalized.diagnostics,
    }
}

pub(crate) fn validate_request(request: &ExtractionRequest) -> Vec<Diagnostic> {
    (request.spec_version != CORE_SPEC_VERSION)
        .then(|| {
            error_diagnostic(
                "UNSUPPORTED_SPEC_VERSION",
                format!(
                    "Unsupported spec version {}. Expected {}.",
                    request.spec_version, CORE_SPEC_VERSION
                ),
                Some(json!({
                    "expected": CORE_SPEC_VERSION,
                    "received": request.spec_version,
                })),
            )
        })
        .into_iter()
        .collect()
}

pub(crate) struct ExtractionRun {
    pub(crate) document_title: Option<String>,
    pub(crate) effective_base_url: Option<String>,
    pub(crate) candidate_count: usize,
    pub(crate) diagnostics: Vec<Diagnostic>,
    pub(crate) matches: Vec<ExtractionMatch>,
}

pub(crate) fn run_selector_extraction(
    request: &ExtractionRequest,
    source: &LoadedSource,
) -> ExtractionRun {
    let document = parse_document_node(&source.text);
    let effective_base_url = resolve_document_base_url(&document, source.input_base_url.as_deref());
    let mut diagnostics = if request.normalization.rewrite_urls && effective_base_url.is_none() {
        vec![unresolved_effective_base_diagnostic(
            document_base_href(&document).as_deref(),
            true,
        )]
    } else {
        Vec::new()
    };
    let selector = request
        .extraction
        .selector_query()
        .expect("selector extraction should carry a selector");
    let parsed_selector = match Selector::parse(selector.as_str()) {
        Ok(selector) => selector,
        Err(_) => {
            return ExtractionRun {
                document_title: None,
                effective_base_url,
                candidate_count: 0,
                diagnostics: {
                    diagnostics.push(error_diagnostic(
                        "INVALID_SELECTOR",
                        format!("Invalid selector: {selector}"),
                        Some(json!({ "selector": selector.as_str() })),
                    ));
                    diagnostics
                },
                matches: Vec::new(),
            };
        }
    };
    let document_title = extract_document_title(&document);

    let candidates: Vec<ElementRef<'_>> = document.select(&parsed_selector).collect();
    let candidate_count = candidates.len();
    let (selected, selection_diagnostics) =
        select_candidates(&candidates, request.extraction.selection());
    diagnostics.extend(selection_diagnostics);
    let mut matches = Vec::new();
    let match_count = selected.len();

    for (position, selected_candidate) in selected.iter().enumerate() {
        let built_match = build_selector_match(
            request,
            effective_base_url.as_deref(),
            &selected_candidate.candidate,
            position + 1,
            match_count,
            selected_candidate.candidate_index,
            candidate_count,
        );

        match built_match {
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

pub(crate) fn build_selector_match(
    request: &ExtractionRequest,
    effective_base_url: Option<&str>,
    node: &ElementRef<'_>,
    match_index: usize,
    match_count: usize,
    candidate_index: usize,
    candidate_count: usize,
) -> Result<ExtractionMatch, Diagnostic> {
    let value_spec = request.extraction.value();
    let path = build_node_path(node);
    let tag_name = node.value().name().to_owned();
    let attributes =
        element_attributes(node, effective_base_url, request.normalization.rewrite_urls);
    let inner_html = serialize_children(node);
    let outer_html = serialize_element(node);
    let rewritten_inner_html = rewrite_html_urls(&inner_html, effective_base_url, false);
    let rewritten_outer_html = rewrite_html_urls(&outer_html, effective_base_url, false);
    let text = render_html_as_text(&rewritten_inner_html, request.normalization.whitespace);
    let value = match value_spec {
        ValueSpec::Text => Value::String(text.clone()),
        ValueSpec::InnerHtml => Value::String(rewritten_inner_html.clone()),
        ValueSpec::OuterHtml => Value::String(rewritten_outer_html.clone()),
        ValueSpec::Attribute { name } => {
            let Some(value) = attributes.get(name.as_str()) else {
                return Err(error_diagnostic(
                    "MISSING_ATTRIBUTE",
                    format!("Matched node is missing attribute \"{name}\"."),
                    Some(json!({
                        "attribute": name.as_str(),
                        "path": path,
                    })),
                ));
            };

            Value::String(apply_whitespace_mode(
                value,
                request.normalization.whitespace,
            ))
        }
        ValueSpec::Structured => serde_json::to_value(SelectorStructuredValue {
            match_index,
            match_count,
            candidate_index,
            candidate_count,
            tag_name: tag_name.clone(),
            path: path.clone(),
            text: text.clone(),
            html: rewritten_inner_html.clone(),
            outer_html: rewritten_outer_html.clone(),
            attributes: attributes.clone(),
        })
        .expect("selector structured value should serialize"),
    };

    let preview = build_preview(&value, request.output.preview_chars.get());
    let html = if request.output.include_html {
        Some(rewritten_outer_html)
    } else {
        None
    };
    let text_value = if request.output.include_text {
        Some(text.clone())
    } else {
        None
    };

    Ok(ExtractionMatch {
        index: match_index,
        path: Some(path.clone()),
        value_type: value_spec.value_type(),
        value,
        html,
        text: text_value,
        preview,
        metadata: ExtractionMatchMetadata::Selector(SelectorMatchMetadata {
            candidate_count,
            candidate_index,
            path,
            tag_name,
            attributes,
        }),
    })
}

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
        ValueSpec::Attribute { name } => {
            let attributes = first_fragment_attributes(
                &selected_html,
                effective_base_url,
                request.normalization.rewrite_urls,
            );
            let Some(value) = attributes.get(name.as_str()) else {
                let hint_include_start = !slice.include_start
                    && candidate.selected_range.start != candidate.outer_range.start;
                let message = if hint_include_start {
                    format!(
                        "Extracted fragment is missing attribute \"{name}\". If the attribute lives on the opening tag, use --include-start so the fragment keeps that tag."
                    )
                } else {
                    format!("Extracted fragment is missing attribute \"{name}\".")
                };
                return Err(error_diagnostic(
                    "MISSING_ATTRIBUTE",
                    message,
                    Some(json!({
                    "attribute": name.as_str(),
                        "selectedRange": candidate.selected_range,
                        "hint": hint_include_start.then_some("use --include-start"),
                    })),
                ));
            };

            Value::String(apply_whitespace_mode(
                value,
                request.normalization.whitespace,
            ))
        }
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
        }),
    })
}

pub(crate) fn extract_slice_candidates(
    source_text: &str,
    slice: &SliceSpec,
) -> Result<Vec<SliceCandidate>, Diagnostic> {
    let start_finder = build_finder(slice.from().as_str(), slice.mode(), slice.flags())?;
    let end_finder = build_finder(slice.to().as_str(), slice.mode(), slice.flags())?;
    let mut candidates = Vec::new();
    let mut cursor = 0usize;

    while cursor <= source_text.len() {
        let Some(start) = start_finder(source_text, cursor) else {
            break;
        };

        let Some(end) = end_finder(source_text, start.end) else {
            return Err(error_diagnostic(
                "NO_MATCH",
                format!(
                    "End pattern was not found after offset {}: {}",
                    start.start,
                    slice.to()
                ),
                Some(json!({
                    "from": slice.from().as_str(),
                    "to": slice.to().as_str(),
                    "offset": start.start,
                })),
            ));
        };

        let inner_range = Range {
            start: start.end,
            end: end.start,
        };
        let outer_range = Range {
            start: start.start,
            end: end.end,
        };
        let selected_range = Range {
            start: if slice.include_start {
                start.start
            } else {
                start.end
            },
            end: if slice.include_end {
                end.end
            } else {
                end.start
            },
        };
        let candidate = SliceCandidate {
            inner_html: source_text[inner_range.start..inner_range.end].to_owned(),
            outer_html: source_text[outer_range.start..outer_range.end].to_owned(),
            selected_html: source_text[selected_range.start..selected_range.end].to_owned(),
            selected_range,
            inner_range,
            outer_range,
        };

        let next_cursor = if candidate.outer_range.end > candidate.outer_range.start {
            candidate.outer_range.end
        } else {
            candidate.outer_range.start + 1
        };
        candidates.push(candidate);
        cursor = next_cursor;
    }

    if candidates.is_empty() {
        return Err(error_diagnostic(
            "NO_MATCH",
            format!("Start pattern was not found: {}", slice.from()),
            Some(json!({
                "from": slice.from().as_str(),
                "to": slice.to().as_str(),
            })),
        ));
    }

    Ok(candidates)
}

#[derive(Clone, Copy)]
pub(crate) struct FoundRange {
    pub(crate) start: usize,
    pub(crate) end: usize,
}

pub(crate) type Finder = Box<dyn Fn(&str, usize) -> Option<FoundRange>>;

pub(crate) fn build_finder(
    pattern: &str,
    mode: PatternMode,
    flags: Option<&str>,
) -> Result<Finder, Diagnostic> {
    if pattern.is_empty() {
        return Err(error_diagnostic(
            "INVALID_SLICE_PATTERN",
            "Patterns must not be empty.",
            None,
        ));
    }

    match mode {
        PatternMode::Literal => {
            let pattern = pattern.to_owned();
            Ok(Box::new(move |source, offset| {
                source[offset..].find(&pattern).map(|relative| FoundRange {
                    start: offset + relative,
                    end: offset + relative + pattern.len(),
                })
            }))
        }
        PatternMode::Regex => {
            let regex = build_regex(pattern, flags.unwrap_or(DEFAULT_REGEX_FLAGS))?;
            Ok(Box::new(move |source, offset| {
                regex.find_at(source, offset).map(|matched| FoundRange {
                    start: matched.start(),
                    end: matched.end(),
                })
            }))
        }
    }
}

pub(crate) fn build_regex(pattern: &str, flags: &str) -> Result<regex::Regex, Diagnostic> {
    let mut builder = RegexBuilder::new(pattern);

    for flag in flags.chars() {
        match flag {
            'g' | 'u' => {}
            'i' => {
                builder.case_insensitive(true);
            }
            'm' => {
                builder.multi_line(true);
            }
            's' => {
                builder.dot_matches_new_line(true);
            }
            'U' => {
                builder.swap_greed(true);
            }
            'x' => {
                builder.ignore_whitespace(true);
            }
            unsupported => {
                return Err(error_diagnostic(
                    "INVALID_SLICE_PATTERN",
                    format!("Unsupported regex flag: {unsupported}"),
                    Some(json!({ "flags": flags })),
                ));
            }
        }
    }

    builder.build().map_err(|error| {
        error_diagnostic(
            "INVALID_SLICE_PATTERN",
            format!("Invalid regular expression: {error}"),
            Some(json!({ "pattern": pattern, "flags": flags })),
        )
    })
}

pub(crate) fn select_candidates<T: Clone>(
    candidates: &[T],
    selection: &SelectionSpec,
) -> (Vec<SelectedCandidate<T>>, Vec<Diagnostic>) {
    if candidates.is_empty() {
        return (
            Vec::new(),
            vec![error_diagnostic(
                "NO_MATCH",
                "No matches were found for the extraction request.",
                None,
            )],
        );
    }

    match selection {
        SelectionSpec::All => (
            candidates
                .iter()
                .enumerate()
                .map(|(index, candidate)| SelectedCandidate {
                    candidate_index: index + 1,
                    candidate: candidate.clone(),
                })
                .collect(),
            Vec::new(),
        ),
        SelectionSpec::Single => {
            if candidates.len() > 1 {
                return (
                    Vec::new(),
                    vec![error_diagnostic(
                        "AMBIGUOUS_MATCH",
                        format!(
                            "Exact-one selection requires exactly one candidate, but {} were found.",
                            candidates.len()
                        ),
                        Some(json!({
                            "candidateCount": candidates.len(),
                        })),
                    )],
                );
            }

            (
                vec![SelectedCandidate {
                    candidate_index: 1,
                    candidate: candidates[0].clone(),
                }],
                Vec::new(),
            )
        }
        SelectionSpec::First => {
            let diagnostics = if candidates.len() > 1 {
                vec![warning_diagnostic(
                    "MULTIPLE_MATCHES",
                    format!(
                        "Matched {} candidates while using match type first.",
                        candidates.len()
                    ),
                    Some(json!({
                        "candidateCount": candidates.len(),
                        "selectedIndex": 1,
                    })),
                )]
            } else {
                Vec::new()
            };

            (
                vec![SelectedCandidate {
                    candidate_index: 1,
                    candidate: candidates[0].clone(),
                }],
                diagnostics,
            )
        }
        SelectionSpec::Nth { index } => {
            let requested_index = index.get();
            if requested_index > candidates.len() {
                return (
                    Vec::new(),
                    vec![error_diagnostic(
                        "MATCH_INDEX_OUT_OF_RANGE",
                        format!(
                            "Match index {} is out of range for {} candidates.",
                            requested_index,
                            candidates.len()
                        ),
                        Some(json!({
                            "requestedIndex": requested_index,
                            "candidateCount": candidates.len(),
                        })),
                    )],
                );
            }

            (
                vec![SelectedCandidate {
                    candidate_index: requested_index,
                    candidate: candidates[requested_index - 1].clone(),
                }],
                Vec::new(),
            )
        }
    }
}
