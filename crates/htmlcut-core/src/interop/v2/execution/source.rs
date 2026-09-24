//! Source acquisition followed by opaque prepared-document construction.

use crate::{
    Diagnostic, DiagnosticCode, HttpUrl, InspectionOptions, OperationId, RuntimeOptions,
    SourceInspectionResult, SourceMetadata, SourceRequest,
    diagnostics::{error_diagnostic, unresolved_effective_base_diagnostic},
    inspect::build_document_inspection,
    source::{LoadedSource, load_source, source_metadata},
};

use super::super::{
    HtmlInput, PreparationError, PreparationErrorCode, PreparationLimits, PreparedDocument,
};
use super::preparation::prepare_document;

/// Source metadata and its opaque prepared document.
pub struct PreparedSource {
    /// Metadata of the source that produced the prepared snapshot.
    pub source: SourceMetadata,
    document: PreparedDocument,
}

impl PreparedSource {
    /// Returns the opaque prepared document for extraction, exploration, or target resolution.
    pub fn document(&self) -> &PreparedDocument {
        &self.document
    }

    /// Returns source metadata, including source text only when an explicit report requests it.
    pub fn source_metadata(&self, include_source_text: bool) -> SourceMetadata {
        let mut source = self.source.clone();
        source.text = include_source_text.then(|| self.document.input().html.clone());
        source
    }
}

/// Typed failure while loading or preparing one source for v2 operations.
#[derive(Debug)]
pub enum PrepareSourceError {
    /// Source acquisition failed before document preparation could begin.
    SourceLoad {
        /// Failure metadata suitable for user-visible reporting.
        source: Box<SourceMetadata>,
        /// Owned HTMLCut diagnostic.
        diagnostic: Box<Diagnostic>,
    },
    /// The loaded source could not satisfy prepared-document limits.
    Preparation {
        /// Metadata of the loaded source.
        source: Box<SourceMetadata>,
        /// Typed preparation failure.
        error: Box<PreparationError>,
    },
}

/// Loads one source and prepares its exact accepted HTML representation once.
pub fn prepare_source(
    source: &SourceRequest,
    runtime: &RuntimeOptions,
    limits: PreparationLimits,
) -> Result<PreparedSource, PrepareSourceError> {
    let loaded = load_source(source, runtime).map_err(|failure| {
        let (source, diagnostic) = failure.into_parts();
        PrepareSourceError::SourceLoad {
            source: Box::new(source),
            diagnostic: Box::new(diagnostic),
        }
    })?;
    prepare_loaded_source(loaded, limits)
}

fn prepare_loaded_source(
    loaded: LoadedSource,
    limits: PreparationLimits,
) -> Result<PreparedSource, PrepareSourceError> {
    let mut metadata = source_metadata(&loaded, false, None);
    let mut input = HtmlInput::new(loaded.value.clone(), loaded.text.clone()).map_err(|_| {
        PrepareSourceError::Preparation {
            source: Box::new(metadata.clone()),
            error: Box::new(PreparationError::new(
                PreparationErrorCode::InternalInvariantViolation,
                String::new(),
                "HTMLCut could not construct the loaded source preparation input.",
            )),
        }
    })?;
    if let Some(base_url) = loaded.input_base_url.as_deref() {
        let base_url = HttpUrl::parse(base_url).map_err(|_| PrepareSourceError::Preparation {
            source: Box::new(metadata.clone()),
            error: Box::new(PreparationError::new(
                PreparationErrorCode::InternalInvariantViolation,
                String::new(),
                "HTMLCut could not validate the loaded source base URL.",
            )),
        })?;
        input = input.with_input_base_url(base_url);
    }
    let document =
        prepare_document(input, limits).map_err(|error| PrepareSourceError::Preparation {
            source: Box::new(metadata.clone()),
            error,
        })?;
    metadata.effective_base_url = document.effective_base_url().map(str::to_owned);
    Ok(PreparedSource {
        source: metadata,
        document,
    })
}

#[cfg(test)]
pub(crate) fn prepare_loaded_source_for_tests(
    loaded: LoadedSource,
    limits: PreparationLimits,
) -> Result<PreparedSource, PrepareSourceError> {
    prepare_loaded_source(loaded, limits)
}

/// Loads and prepares one source once before building its legacy inspection report.
pub fn inspect_source(
    source: &SourceRequest,
    runtime: &RuntimeOptions,
    options: &InspectionOptions,
) -> SourceInspectionResult {
    source_inspection_from_prepared(
        prepare_source(source, runtime, PreparationLimits::default()),
        options,
    )
}

fn source_inspection_from_prepared(
    result: Result<PreparedSource, PrepareSourceError>,
    options: &InspectionOptions,
) -> SourceInspectionResult {
    match result {
        Ok(prepared) => {
            let document_inspection = build_document_inspection(
                prepared.document.document(),
                prepared.document.effective_base_url(),
                options.sample_limit,
            );
            let diagnostics = if document_inspection.document_base_href.is_some()
                && prepared.document.effective_base_url().is_none()
            {
                vec![unresolved_effective_base_diagnostic(
                    document_inspection.document_base_href.as_deref(),
                    false,
                )]
            } else {
                Vec::new()
            };
            SourceInspectionResult {
                operation_id: OperationId::SourceInspect,
                schema_name: crate::CORE_SOURCE_INSPECTION_SCHEMA_NAME.to_owned(),
                schema_version: crate::CORE_SOURCE_INSPECTION_SCHEMA_VERSION,
                ok: true,
                source: prepared.source_metadata(options.include_source_text),
                document: Some(document_inspection),
                diagnostics,
            }
        }
        Err(PrepareSourceError::SourceLoad { source, diagnostic }) => SourceInspectionResult {
            operation_id: OperationId::SourceInspect,
            schema_name: crate::CORE_SOURCE_INSPECTION_SCHEMA_NAME.to_owned(),
            schema_version: crate::CORE_SOURCE_INSPECTION_SCHEMA_VERSION,
            ok: false,
            source: *source,
            document: None,
            diagnostics: vec![*diagnostic],
        },
        Err(PrepareSourceError::Preparation { source, error }) => SourceInspectionResult {
            operation_id: OperationId::SourceInspect,
            schema_name: crate::CORE_SOURCE_INSPECTION_SCHEMA_NAME.to_owned(),
            schema_version: crate::CORE_SOURCE_INSPECTION_SCHEMA_VERSION,
            ok: false,
            source: *source,
            document: None,
            diagnostics: vec![error_diagnostic(
                DiagnosticCode::SourcePreparationFailed,
                error.message,
                Some(serde_json::json!({ "preparation_error_code": error.error_code })),
            )],
        },
    }
}

#[cfg(test)]
pub(crate) fn source_inspection_from_prepared_for_tests(
    result: Result<PreparedSource, PrepareSourceError>,
    options: &InspectionOptions,
) -> SourceInspectionResult {
    source_inspection_from_prepared(result, options)
}
