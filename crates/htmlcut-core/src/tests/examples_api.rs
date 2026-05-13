use serde_json::{Value, json};

use crate::{
    AttributeName, EXTRACTION_DEFINITION_SCHEMA_VERSION, ExtractionDefinition, ExtractionRequest,
    ExtractionSpec, HttpUrl, OutputOptions, RenderingOptions, RuntimeOptions, SelectionSpec,
    SelectorQuery, SourceRequest, ValueSpec, extract, result::ExtractionMatchMetadata,
};

#[test]
fn request_and_result_namespaces_example_emits_a_namespace_summary() {
    let source = SourceRequest::memory(
        "inline",
        "<article><a href=\"../guide.html\">Guide</a></article>",
    )
    .with_base_url(HttpUrl::parse("https://example.com/docs/start.html").expect("base url"));
    let request = ExtractionRequest {
        output: OutputOptions {
            rendering: RenderingOptions {
                rewrite_urls: true,
                ..Default::default()
            },
            ..Default::default()
        },
        ..ExtractionRequest::new(
            source,
            ExtractionSpec::selector(SelectorQuery::new("article a").expect("selector"))
                .with_selection(SelectionSpec::single())
                .with_value(ValueSpec::Attribute {
                    name: AttributeName::new("href").expect("attribute name"),
                }),
        )
    };

    let result = extract(&request, &RuntimeOptions::default());
    assert!(result.ok);

    let (tag_name, path) = match &result.matches[0].metadata {
        ExtractionMatchMetadata::Selector(metadata) => {
            (metadata.tag_name.clone(), metadata.path.clone())
        }
        ExtractionMatchMetadata::DelimiterPair(_) => {
            panic!("selector extraction should yield selector metadata")
        }
    };

    let value: Value = json!({
        "request_namespace": "htmlcut_core::request",
        "result_namespace": "htmlcut_core::result",
        "value": result.matches[0].value,
        "tag_name": tag_name,
        "path": path,
    });

    assert_eq!(value["request_namespace"], "htmlcut_core::request");
    assert_eq!(value["result_namespace"], "htmlcut_core::result");
    assert_eq!(value["value"], "https://example.com/guide.html");
    assert_eq!(value["tag_name"], "a");
    assert!(
        value["path"]
            .as_str()
            .is_some_and(|path| path.contains("article:nth-of-type(1) > a:nth-of-type(1)"))
    );
}

#[test]
fn reusable_extraction_definition_example_emits_a_reusable_definition() {
    let source = SourceRequest::memory(
        "inline",
        "<article><a href=\"../guide.html\">Guide</a></article>",
    )
    .with_base_url(HttpUrl::parse("https://example.com/docs/start.html").expect("base url"));
    let mut request = ExtractionRequest::new(
        source,
        ExtractionSpec::selector(SelectorQuery::new("article a").expect("selector"))
            .with_selection(SelectionSpec::single())
            .with_value(ValueSpec::Attribute {
                name: AttributeName::new("href").expect("attribute name"),
            }),
    );
    request.output.rendering = RenderingOptions {
        rewrite_urls: true,
        ..Default::default()
    };

    let definition = ExtractionDefinition {
        runtime: RuntimeOptions::default(),
        ..ExtractionDefinition::new(request)
    };

    let encoded = serde_json::to_string_pretty(&definition).expect("encode definition");
    let value: Value = serde_json::from_str(&encoded).expect("parse definition");

    assert_eq!(value["schema_name"], "htmlcut.extraction_definition");
    assert_eq!(
        value["schema_version"],
        EXTRACTION_DEFINITION_SCHEMA_VERSION
    );
    assert_eq!(value["request"]["extraction"]["kind"], "selector");
    assert_eq!(value["request"]["extraction"]["selector"], "article a");
    assert_eq!(value["runtime"]["fetch_preflight"], "head-first");
}
