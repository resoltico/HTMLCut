use std::time::Instant;

use serde_json::json;

use crate::catalog::OperationId;
use crate::contracts::{
    CORE_RESULT_SCHEMA_NAME, CORE_RESULT_SCHEMA_VERSION, CORE_SOURCE_INSPECTION_SCHEMA_NAME,
    CORE_SOURCE_INSPECTION_SCHEMA_VERSION, CORE_SPEC_VERSION, Diagnostic, ExtractionRequest,
    ExtractionResult, ExtractionStats, ExtractionStrategy, InspectionOptions, ParseDocumentResult,
    ParsedDocument, RuntimeOptions, SourceInspectionResult, SourceRequest,
};
use crate::diagnostics::{
    DiagnosticCode, error_diagnostic, has_errors, unresolved_effective_base_diagnostic,
};
use crate::document::{parse_document_node, resolve_document_base_url};
use crate::inspect::build_document_inspection;
use crate::source::{empty_source_metadata, load_source, source_metadata};

use super::{FinalizedExtraction, run_selector_extraction, run_slice_extraction};

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
        Err(failure) => {
            let (source, diagnostic) = failure.into_parts();
            ParseDocumentResult {
                operation_id: OperationId::DocumentParse,
                ok: false,
                source,
                diagnostics: vec![diagnostic],
                document: None,
            }
        }
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
        Err(failure) => {
            let (source, diagnostic) = failure.into_parts();
            SourceInspectionResult {
                operation_id: OperationId::SourceInspect,
                schema_name: CORE_SOURCE_INSPECTION_SCHEMA_NAME.to_owned(),
                schema_version: CORE_SOURCE_INSPECTION_SCHEMA_VERSION,
                ok: false,
                source,
                document: None,
                diagnostics: vec![diagnostic],
            }
        }
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
        Err(failure) => {
            let (source, diagnostic) = failure.into_parts();
            diagnostics.push(diagnostic);
            return finalize_result(
                request,
                FinalizedExtraction {
                    operation_id: extraction_operation_id(request.extraction.strategy(), preview),
                    source,
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
                DiagnosticCode::UnsupportedSpecVersion,
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
    pub(crate) matches: Vec<crate::contracts::ExtractionMatch>,
}
