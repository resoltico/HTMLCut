use crate::interop::v2::{
    PreparationErrorCode, PreparationLimits, PrepareSourceError, prepare_loaded_source_for_tests,
    prepare_source, source_inspection_from_prepared_for_tests,
};
use crate::source::LoadedSource;
use crate::{
    Diagnostic, DiagnosticCode, DiagnosticLevel, InspectionOptions, RuntimeOptions, SourceKind,
    SourceLoadStep, SourceMetadata, SourceRequest,
};

#[test]
fn prepare_source_loads_once_and_returns_only_opaque_prepared_document() {
    let source = SourceRequest::memory("source-bridge", "<main><p>Hello</p></main>");
    let prepared = prepare_source(
        &source,
        &RuntimeOptions::default(),
        PreparationLimits::default(),
    )
    .expect("prepared source");

    assert_eq!(prepared.source.kind, SourceKind::Memory);
    assert_eq!(prepared.source.value, "source-bridge");
    assert_eq!(prepared.document().input_digest_sha256().len(), 64);
    assert_eq!(
        prepared.source_metadata(true).text.as_deref(),
        Some("<main><p>Hello</p></main>")
    );
}

#[test]
fn successful_source_inspection_without_any_base_information_has_no_diagnostic() {
    let source = SourceRequest::memory("inspection-success", "<main>One</main>");
    let prepared = prepare_source(
        &source,
        &RuntimeOptions::default(),
        PreparationLimits::default(),
    )
    .expect("prepared source");

    let report =
        source_inspection_from_prepared_for_tests(Ok(prepared), &InspectionOptions::default());

    assert!(report.ok);
    assert!(report.document.is_some());
    assert!(report.diagnostics.is_empty());
}

#[test]
fn prepared_source_reports_loading_and_preparation_failures_without_fabricating_a_plan() {
    let invalid_label = LoadedSource {
        kind: SourceKind::Memory,
        value: String::new(),
        text: "<main>One</main>".to_owned(),
        bytes_read: 16,
        input_base_url: None,
        load_steps: Vec::<SourceLoadStep>::new(),
    };
    let result = prepare_loaded_source_for_tests(invalid_label, PreparationLimits::default());
    let Err(error) = result else {
        panic!("invalid loaded label must not produce a prepared source");
    };
    match error {
        PrepareSourceError::Preparation { error, .. } => {
            assert_eq!(
                error.error_code,
                PreparationErrorCode::InternalInvariantViolation
            );
            assert!(error.input_digest_sha256.is_empty());
        }
        other => panic!("expected preparation error, got {other:?}"),
    }

    let invalid_base_url = LoadedSource {
        kind: SourceKind::Memory,
        value: "source".to_owned(),
        text: "<main>One</main>".to_owned(),
        bytes_read: 16,
        input_base_url: Some("not a URL".to_owned()),
        load_steps: Vec::new(),
    };
    assert!(matches!(
        prepare_loaded_source_for_tests(invalid_base_url, PreparationLimits::default(),),
        Err(PrepareSourceError::Preparation { .. })
    ));

    let preparation_limits_failure = LoadedSource {
        kind: SourceKind::Memory,
        value: "valid-source".to_owned(),
        text: "<main>One</main>".to_owned(),
        bytes_read: 16,
        input_base_url: None,
        load_steps: Vec::new(),
    };
    let invalid_limits = PreparationLimits {
        max_input_bytes: std::num::NonZeroU32::new(50 * 1024 * 1024 + 1).expect("non-zero"),
        max_elements: std::num::NonZeroU32::new(1).expect("non-zero"),
        max_dom_depth: std::num::NonZeroU32::new(1).expect("non-zero"),
    };
    assert!(matches!(
        prepare_loaded_source_for_tests(preparation_limits_failure, invalid_limits),
        Err(PrepareSourceError::Preparation { .. })
    ));

    let missing = SourceRequest::file("/definitely/not/a/htmlcut-source.html");
    assert!(matches!(
        prepare_source(
            &missing,
            &RuntimeOptions::default(),
            PreparationLimits::default(),
        ),
        Err(PrepareSourceError::SourceLoad { .. })
    ));

    let source_metadata = SourceMetadata {
        kind: SourceKind::Memory,
        value: "inspection-source".to_owned(),
        input_base_url: None,
        effective_base_url: None,
        bytes_read: 0,
        load_steps: Vec::new(),
        text: None,
    };
    let source_load_report = source_inspection_from_prepared_for_tests(
        Err(PrepareSourceError::SourceLoad {
            source: Box::new(source_metadata.clone()),
            diagnostic: Box::new(Diagnostic {
                level: DiagnosticLevel::Error,
                code: DiagnosticCode::SourceLoadFailed,
                message: "Could not load source.".to_owned(),
                details: None,
            }),
        }),
        &InspectionOptions::default(),
    );
    assert!(!source_load_report.ok);
    assert_eq!(
        source_load_report.diagnostics[0].code,
        DiagnosticCode::SourceLoadFailed
    );

    let preparation_report = source_inspection_from_prepared_for_tests(
        Err(PrepareSourceError::Preparation {
            source: Box::new(source_metadata),
            error: Box::new(crate::interop::v2::PreparationError {
                schema_name: "htmlcut.preparation_error".to_owned(),
                schema_version: 1,
                error_code: PreparationErrorCode::DocumentTooComplex,
                input_digest_sha256: "0".repeat(64),
                message: "Parsed HTML exceeds the configured element limit.".to_owned(),
            }),
        }),
        &InspectionOptions::default(),
    );
    assert!(!preparation_report.ok);
    assert_eq!(
        preparation_report.diagnostics[0].code,
        DiagnosticCode::SourcePreparationFailed
    );
}
