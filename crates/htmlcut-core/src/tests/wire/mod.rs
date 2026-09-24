use super::*;
use crate::result::{
    ContentCandidateInspection, DelimiterPairMatchMetadata, DocumentInspection, ExtractionMatch,
    ExtractionStats, HeadingInspection, InspectionCount, LinkInspection, Range,
    SelectorMatchMetadata,
};
use crate::wire::v2::{
    ExtractionDefinitionDocument, ExtractionRequestDocument, ExtractionResultDocument,
    InspectionOptionsDocument, RuntimeOptionsDocument, SourceInspectionResultDocument,
    SourceRequestDocument, WireDocumentDecodeError, decode_document_json, preflight_document_json,
    wire_duration_ms_for_tests,
};
use std::collections::BTreeMap;

mod helpers;

use helpers::{
    extraction_request_document, tamper_wire_identity_field, tamper_wire_profile, v2_document,
};

#[test]
fn wire_v2_envelope_preflight_rejects_legacy_and_defers_body_decoding() {
    assert!(matches!(
        preflight_document_json("{", SOURCE_REQUEST_SCHEMA_NAME, CORE_REQUEST_SCHEMA_VERSION),
        Err(WireDocumentDecodeError::InvalidJson(_))
    ));
    assert!(matches!(
        preflight_document_json(
            r#"{"schema_name":"htmlcut.source_request","schema_version":7,"wire_profile":"htmlcut-json-schema-v1"}"#,
            SOURCE_REQUEST_SCHEMA_NAME,
            CORE_REQUEST_SCHEMA_VERSION,
        ),
        Err(WireDocumentDecodeError::IncompatibleEnvelope)
    ));
    for incompatible_identity in [
        r#"{"schema_name":"htmlcut.other","schema_version":8,"wire_profile":"htmlcut-json-schema-v2"}"#,
        r#"{"schema_name":"htmlcut.source_request","schema_version":7,"wire_profile":"htmlcut-json-schema-v2"}"#,
        r#"{"schema_name":"htmlcut.source_request","schema_version":8,"wire_profile":"htmlcut-json-schema-other"}"#,
    ] {
        assert!(matches!(
            preflight_document_json(
                incompatible_identity,
                SOURCE_REQUEST_SCHEMA_NAME,
                CORE_REQUEST_SCHEMA_VERSION,
            ),
            Err(WireDocumentDecodeError::IncompatibleEnvelope)
        ));
    }

    let current = v2_document(
        SOURCE_REQUEST_SCHEMA_NAME,
        CORE_REQUEST_SCHEMA_VERSION,
        json!({"input": {"type": "stdin"}}),
    );
    let current_json = serde_json::to_string(&current).expect("current JSON");
    preflight_document_json(
        &current_json,
        SOURCE_REQUEST_SCHEMA_NAME,
        CORE_REQUEST_SCHEMA_VERSION,
    )
    .expect("current envelope");
    let decoded: SourceRequestDocument = decode_document_json(
        &current_json,
        SOURCE_REQUEST_SCHEMA_NAME,
        CORE_REQUEST_SCHEMA_VERSION,
    )
    .expect("current source request");
    let _: SourceRequest = decoded.try_into().expect("domain source request");

    let invalid_body = v2_document(
        SOURCE_REQUEST_SCHEMA_NAME,
        CORE_REQUEST_SCHEMA_VERSION,
        json!({"input": {"type": "stdin"}, "unexpected": true}),
    );
    let invalid_body_json = serde_json::to_string(&invalid_body).expect("invalid body JSON");
    assert!(matches!(
        decode_document_json::<SourceRequestDocument>(
            &invalid_body_json,
            SOURCE_REQUEST_SCHEMA_NAME,
            CORE_REQUEST_SCHEMA_VERSION,
        ),
        Err(WireDocumentDecodeError::InvalidJson(_))
    ));
}

#[test]
fn wire_v2_domain_conversion_rejects_a_tampered_envelope_after_deserialization() {
    let document =
        SourceRequestDocument::try_from(SourceRequest::stdin()).expect("source request document");
    let mut value = serde_json::to_value(document).expect("document JSON");
    value["wire_profile"] = Value::String("htmlcut-json-schema-v1".to_owned());
    let tampered: SourceRequestDocument =
        serde_json::from_value(value).expect("shape remains deserializable");
    assert!(SourceRequest::try_from(tampered).is_err());
}

#[test]
fn standalone_wire_documents_refuse_a_tampered_outer_identity_before_domain_conversion() {
    assert!(
        RuntimeOptions::try_from(tamper_wire_profile(RuntimeOptionsDocument::default())).is_err()
    );
    assert!(
        InspectionOptions::try_from(tamper_wire_profile(InspectionOptionsDocument::default(),))
            .is_err()
    );
    assert!(
        RuntimeOptions::try_from(tamper_wire_identity_field(
            RuntimeOptionsDocument::default(),
            "schema_name",
            Value::String("htmlcut.other".to_owned()),
        ))
        .is_err()
    );
    assert!(
        InspectionOptions::try_from(tamper_wire_identity_field(
            InspectionOptionsDocument::default(),
            "schema_version",
            Value::from(CORE_REQUEST_SCHEMA_VERSION + 1),
        ))
        .is_err()
    );

    let request = ExtractionRequest::new(
        SourceRequest::stdin(),
        ExtractionSpec::selector(selector_query("article")),
    );
    assert!(
        ExtractionRequest::try_from(tamper_wire_profile(
            ExtractionRequestDocument::try_from(request.clone()).expect("request document"),
        ))
        .is_err()
    );
    assert!(
        ExtractionDefinition::try_from(tamper_wire_profile(
            ExtractionDefinitionDocument::try_from(ExtractionDefinition::new(request))
                .expect("definition document"),
        ))
        .is_err()
    );
}

#[test]
fn wire_duration_rejects_values_outside_the_fixed_width_document_range() {
    assert_eq!(wire_duration_ms_for_tests(42).expect("wire duration"), 42);
    assert!(matches!(
        wire_duration_ms_for_tests(u128::from(u64::MAX) + 1),
        Err(ContractValueError::WireNumericRange {
            field: "duration_ms"
        })
    ));
}

#[test]
fn wire_default_documents_match_domain_defaults() {
    let runtime: RuntimeOptions = serde_json::from_value::<RuntimeOptionsDocument>(v2_document(
        RUNTIME_OPTIONS_SCHEMA_NAME,
        CORE_REQUEST_SCHEMA_VERSION,
        json!({}),
    ))
    .expect("runtime doc")
    .try_into()
    .expect("current runtime document");
    assert_eq!(runtime, RuntimeOptions::default());

    let runtime_default: RuntimeOptions = RuntimeOptionsDocument::default()
        .try_into()
        .expect("current runtime document");
    assert_eq!(runtime_default, RuntimeOptions::default());

    let inspection: InspectionOptions =
        serde_json::from_value::<InspectionOptionsDocument>(v2_document(
            INSPECTION_OPTIONS_SCHEMA_NAME,
            CORE_REQUEST_SCHEMA_VERSION,
            json!({}),
        ))
        .expect("inspection doc")
        .try_into()
        .expect("current inspection document");
    assert_eq!(inspection, InspectionOptions::default());

    let inspection_default: InspectionOptions = InspectionOptionsDocument::default()
        .try_into()
        .expect("current inspection document");
    assert_eq!(inspection_default, InspectionOptions::default());

    let output_request: ExtractionRequest =
        serde_json::from_value::<ExtractionRequestDocument>(extraction_request_document(json!({
            "spec_version": CORE_SPEC_VERSION,
            "source": { "input": { "type": "stdin" } },
            "extraction": { "kind": "selector", "selector": "article" }
        })))
        .expect("request doc with default output")
        .try_into()
        .expect("current extraction request document");
    assert_eq!(output_request.output, OutputOptions::default());
}

#[test]
fn wire_default_documents_omit_only_default_json_fields() {
    assert_eq!(
        serde_json::to_value(RuntimeOptionsDocument::default()).expect("runtime JSON"),
        v2_document(
            RUNTIME_OPTIONS_SCHEMA_NAME,
            CORE_REQUEST_SCHEMA_VERSION,
            json!({}),
        )
    );
    assert_eq!(
        serde_json::to_value(InspectionOptionsDocument::default()).expect("inspection JSON"),
        v2_document(
            INSPECTION_OPTIONS_SCHEMA_NAME,
            CORE_REQUEST_SCHEMA_VERSION,
            json!({}),
        )
    );

    let selector_request = ExtractionRequest::new(
        SourceRequest::stdin(),
        ExtractionSpec::selector(selector_query("article")),
    );
    assert_eq!(
        serde_json::to_value(
            ExtractionRequestDocument::try_from(selector_request).expect("selector document"),
        )
        .expect("selector JSON"),
        extraction_request_document(json!({
            "spec_version": CORE_SPEC_VERSION,
            "source": { "input": { "type": "stdin" } },
            "extraction": { "kind": "selector", "selector": "article" }
        }))
    );

    let slice_request = ExtractionRequest::new(
        SourceRequest::stdin(),
        ExtractionSpec::slice(slice_spec("<article>", "</article>")),
    );
    assert_eq!(
        serde_json::to_value(
            ExtractionRequestDocument::try_from(slice_request).expect("slice document"),
        )
        .expect("slice JSON"),
        extraction_request_document(json!({
            "spec_version": CORE_SPEC_VERSION,
            "source": { "input": { "type": "stdin" } },
            "extraction": {
                "kind": "slice",
                "pattern": {
                    "mode": "literal",
                    "from": "<article>",
                    "to": "</article>"
                }
            }
        }))
    );

    let configured_output = OutputOptions {
        rendering: RenderingOptions {
            whitespace: WhitespaceMode::Normalize,
            rewrite_urls: true,
        },
        include_source_text: true,
        include_html: false,
        include_text: false,
        preview_chars: NonZeroUsize::new(12).expect("preview chars"),
    };
    let mut configured_request = ExtractionRequest::new(
        SourceRequest::stdin(),
        ExtractionSpec::selector(selector_query("article")),
    );
    configured_request.output = configured_output.clone();
    assert_eq!(
        serde_json::to_value(
            ExtractionRequestDocument::try_from(configured_request).expect("configured document"),
        )
        .expect("configured JSON"),
        extraction_request_document(json!({
            "spec_version": CORE_SPEC_VERSION,
            "source": { "input": { "type": "stdin" } },
            "extraction": { "kind": "selector", "selector": "article" },
            "output": {
                "rendering": { "whitespace": "normalize", "rewrite_urls": true },
                "include_source_text": true,
                "include_html": false,
                "include_text": false,
                "preview_chars": 12
            }
        }))
    );

    let configured_document =
        serde_json::from_value::<ExtractionRequestDocument>(extraction_request_document(json!({
            "spec_version": CORE_SPEC_VERSION,
            "source": { "input": { "type": "stdin" } },
            "extraction": { "kind": "selector", "selector": "article" },
            "output": {
                "rendering": { "whitespace": "normalize", "rewrite_urls": true },
                "include_source_text": true,
                "include_html": false,
                "include_text": false,
                "preview_chars": 12
            }
        })))
        .expect("configured output document");
    let configured_domain: ExtractionRequest = configured_document
        .try_into()
        .expect("current extraction request document");
    assert_eq!(configured_domain.output, configured_output);
}

#[test]
fn wire_non_default_documents_preserve_every_configured_field() {
    let runtime = RuntimeOptions {
        max_bytes: max_bytes_limit(4_096),
        fetch_timeout_ms: fetch_timeout_limit(2_750),
        fetch_connect_timeout_ms: FetchConnectTimeoutMs::new(750).expect("connect timeout"),
        fetch_preflight: FetchPreflightMode::GetOnly,
        tls_trust: TlsTrustPolicy::Platform,
    };
    assert_eq!(
        serde_json::to_value(
            RuntimeOptionsDocument::try_from(runtime.clone()).expect("runtime document"),
        )
        .expect("runtime JSON"),
        v2_document(
            RUNTIME_OPTIONS_SCHEMA_NAME,
            CORE_REQUEST_SCHEMA_VERSION,
            json!({
                "max_bytes": 4_096,
                "fetch_timeout_ms": 2_750,
                "fetch_connect_timeout_ms": 750,
                "fetch_preflight": "get-only",
                "tls_trust": { "kind": "platform" }
            })
        )
    );

    let inspection = InspectionOptions {
        include_source_text: true,
        sample_limit: 7,
    };
    assert_eq!(
        serde_json::to_value(
            InspectionOptionsDocument::try_from(inspection).expect("inspection document"),
        )
        .expect("inspection JSON"),
        v2_document(
            INSPECTION_OPTIONS_SCHEMA_NAME,
            CORE_REQUEST_SCHEMA_VERSION,
            json!({ "include_source_text": true, "sample_limit": 7 }),
        )
    );

    let output_json = |output| {
        let mut request = ExtractionRequest::new(
            SourceRequest::stdin(),
            ExtractionSpec::selector(selector_query("article")),
        );
        request.output = output;
        serde_json::to_value(ExtractionRequestDocument::try_from(request).expect("output document"))
            .expect("output JSON")["output"]
            .clone()
    };
    assert_eq!(
        output_json(OutputOptions {
            include_source_text: true,
            ..OutputOptions::default()
        }),
        json!({ "include_source_text": true })
    );
    assert_eq!(
        output_json(OutputOptions {
            rendering: RenderingOptions {
                rewrite_urls: true,
                ..RenderingOptions::default()
            },
            ..OutputOptions::default()
        }),
        json!({ "rendering": { "rewrite_urls": true } })
    );

    let output_defaults: ExtractionRequest =
        serde_json::from_value::<ExtractionRequestDocument>(extraction_request_document(json!({
            "spec_version": CORE_SPEC_VERSION,
            "source": { "input": { "type": "stdin" } },
            "extraction": { "kind": "selector", "selector": "article" },
            "output": {}
        })))
        .expect("explicit default output document")
        .try_into()
        .expect("current extraction request document");
    assert_eq!(output_defaults.output, OutputOptions::default());

    let selected = ExtractionRequest::new(
        SourceRequest::stdin(),
        ExtractionSpec::selector(selector_query("article")).with_selection(SelectionSpec::single()),
    );
    let selected_json = serde_json::to_value(
        ExtractionRequestDocument::try_from(selected).expect("selected document"),
    )
    .expect("selected JSON");
    assert_eq!(
        selected_json["extraction"]["selection"],
        json!({ "type": "single" })
    );

    let retained = ExtractionRequest::new(
        SourceRequest::stdin(),
        ExtractionSpec::slice(
            slice_spec("<article>", "</article>")
                .with_boundary_retention(BoundaryRetention::IncludeBoth),
        ),
    );
    let retained_json = serde_json::to_value(
        ExtractionRequestDocument::try_from(retained).expect("retained document"),
    )
    .expect("retained JSON");
    assert_eq!(
        retained_json["extraction"]["boundary_retention"],
        "include-both"
    );

    let definition_for = |runtime| ExtractionDefinition {
        schema_name: EXTRACTION_DEFINITION_SCHEMA_NAME.to_owned(),
        schema_version: EXTRACTION_DEFINITION_SCHEMA_VERSION,
        request: ExtractionRequest::new(
            SourceRequest::stdin(),
            ExtractionSpec::selector(selector_query("article")),
        ),
        runtime,
    };
    let configured_definition = serde_json::to_value(
        ExtractionDefinitionDocument::try_from(definition_for(runtime))
            .expect("configured definition"),
    )
    .expect("configured definition JSON");
    assert_eq!(configured_definition["runtime"]["max_bytes"], 4_096);

    let default_definition = serde_json::to_value(
        ExtractionDefinitionDocument::try_from(definition_for(RuntimeOptions::default()))
            .expect("default definition"),
    )
    .expect("default definition JSON");
    assert!(default_definition.get("runtime").is_none());
    assert!(
        default_definition["request"]["extraction"]
            .get("value")
            .is_none()
    );

    let non_default_value_definition = serde_json::to_value(
        ExtractionDefinitionDocument::try_from(ExtractionDefinition {
            schema_name: EXTRACTION_DEFINITION_SCHEMA_NAME.to_owned(),
            schema_version: EXTRACTION_DEFINITION_SCHEMA_VERSION,
            request: ExtractionRequest::new(
                SourceRequest::stdin(),
                ExtractionSpec::selector(selector_query("article"))
                    .with_value(ValueSpec::OuterHtml),
            ),
            runtime: RuntimeOptions::default(),
        })
        .expect("non-default value definition"),
    )
    .expect("non-default value definition JSON");
    assert_eq!(
        non_default_value_definition["request"]["extraction"]["value"],
        json!({ "type": "outer-html" })
    );
}

#[test]
fn wire_request_documents_round_trip_all_source_and_extraction_variants() {
    let sources = vec![
        SourceRequest::stdin(),
        SourceRequest::file("page.html"),
        SourceRequest::memory("inline", "<article>Hello</article>"),
        SourceRequest::url(http_url("https://example.com/input.html")),
    ];

    for source in sources {
        let roundtrip: SourceRequest = SourceRequestDocument::try_from(source.clone())
            .expect("source document")
            .try_into()
            .expect("current source request document");
        assert_eq!(roundtrip, source);
    }

    let requests = vec![
        ExtractionRequest::new(
            SourceRequest::stdin(),
            ExtractionSpec::selector(selector_query("article"))
                .with_selection(SelectionSpec::single())
                .with_value(ValueSpec::Text),
        ),
        ExtractionRequest::new(
            SourceRequest::file("page.html"),
            ExtractionSpec::selector(selector_query("article"))
                .with_selection(SelectionSpec::default())
                .with_value(ValueSpec::OuterHtml),
        ),
        ExtractionRequest::new(
            SourceRequest::memory("inline", "<article>Hello</article>"),
            ExtractionSpec::selector(selector_query("article"))
                .with_selection(nth_selection(2))
                .with_value(attribute_value("HREF")),
        ),
        ExtractionRequest::new(
            memory_source("inline", "<article>Hello</article>"),
            ExtractionSpec::selector(selector_query("article"))
                .with_selection(SelectionSpec::All)
                .with_value(ValueSpec::Structured),
        ),
        ExtractionRequest::new(
            memory_source("inline", "<article>Hello</article>"),
            ExtractionSpec::slice(
                slice_spec("<article>", "</article>")
                    .with_boundary_retention(BoundaryRetention::IncludeBoth),
            )
            .with_selection(SelectionSpec::single())
            .with_value(ValueSpec::SelectedHtml),
        ),
        ExtractionRequest::new(
            memory_source("inline", "<article>Hello</article>"),
            ExtractionSpec::slice(
                SliceSpec::regex(
                    slice_boundary("<article>"),
                    slice_boundary("</article>"),
                    "is",
                )
                .with_boundary_retention(BoundaryRetention::IncludeStart),
            )
            .with_selection(SelectionSpec::First)
            .with_value(ValueSpec::InnerHtml),
        ),
        ExtractionRequest::new(
            memory_source("inline", "<article>Hello</article>"),
            ExtractionSpec::slice(
                slice_spec("<article>", "</article>")
                    .with_boundary_retention(BoundaryRetention::IncludeEnd),
            )
            .with_selection(SelectionSpec::All)
            .with_value(ValueSpec::OuterHtml),
        ),
        ExtractionRequest::new(
            memory_source("inline", "<article data-id=\"7\">Hello</article>"),
            ExtractionSpec::slice(
                slice_spec("<article", "</article>")
                    .with_boundary_retention(BoundaryRetention::ExcludeBoth),
            )
            .with_selection(nth_selection(1))
            .with_value(attribute_value("data-id")),
        ),
    ];

    for request in requests {
        let roundtrip: ExtractionRequest = ExtractionRequestDocument::try_from(request.clone())
            .expect("request document")
            .try_into()
            .expect("current extraction request document");
        assert_eq!(roundtrip, request);
    }

    let definition = ExtractionDefinition {
        schema_name: EXTRACTION_DEFINITION_SCHEMA_NAME.to_owned(),
        schema_version: EXTRACTION_DEFINITION_SCHEMA_VERSION,
        request: ExtractionRequest::new(
            SourceRequest::memory("inline", "<article>Hello</article>")
                .with_base_url(http_url("https://example.com/base/")),
            ExtractionSpec::selector(selector_query("article"))
                .with_selection(SelectionSpec::single())
                .with_value(ValueSpec::Structured),
        ),
        runtime: RuntimeOptions {
            max_bytes: max_bytes_limit(2048),
            fetch_timeout_ms: fetch_timeout_limit(1500),
            fetch_connect_timeout_ms: FetchConnectTimeoutMs::new(250).expect("connect timeout"),
            fetch_preflight: FetchPreflightMode::GetOnly,
            tls_trust: TlsTrustPolicy::Platform,
        },
    };
    let roundtrip: ExtractionDefinition =
        ExtractionDefinitionDocument::try_from(definition.clone())
            .expect("definition document")
            .try_into()
            .expect("current extraction definition document");
    assert_eq!(roundtrip, definition);

    let runtime = RuntimeOptions {
        max_bytes: max_bytes_limit(4096),
        fetch_timeout_ms: fetch_timeout_limit(2750),
        fetch_connect_timeout_ms: FetchConnectTimeoutMs::new(750).expect("connect timeout"),
        fetch_preflight: FetchPreflightMode::HeadFirst,
        tls_trust: TlsTrustPolicy::CustomCaBundle {
            path: "certs/custom.pem".into(),
        },
    };
    let runtime_roundtrip: RuntimeOptions = RuntimeOptionsDocument::try_from(runtime.clone())
        .expect("runtime document")
        .try_into()
        .expect("current runtime document");
    assert_eq!(runtime_roundtrip, runtime);

    let inspection_options = InspectionOptions {
        include_source_text: true,
        sample_limit: 7,
    };
    let inspection_roundtrip: InspectionOptions =
        InspectionOptionsDocument::try_from(inspection_options.clone())
            .expect("inspection document")
            .try_into()
            .expect("current inspection document");
    assert_eq!(inspection_roundtrip, inspection_options);
}

mod results;
