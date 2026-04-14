use super::*;
use crate::contracts::{
    default_fetch_timeout_ms, default_inspection_sample_limit, default_max_bytes,
    default_preview_chars, default_regex_flags, default_spec_version, default_true,
};
use crate::document::{
    ELLIPSIS, attribute_supports_url_rewrite, collapse_blank_lines_for_tests, element_name,
    rewrite_srcset_for_tests,
};
use crate::source::{
    content_type_is_obviously_non_html_for_tests, head_error_allows_get_fallback_for_tests,
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

#[test]
fn defaults_cover_public_default_contracts() {
    assert_eq!(WhitespaceMode::default(), WhitespaceMode::Preserve);
    assert_eq!(SelectionSpec::default(), SelectionSpec::First);
    assert_eq!(ValueSpec::default().value_type(), ValueType::Text);
    let slice = slice_spec("<p>", "</p>");
    assert_eq!(slice.mode(), PatternMode::Literal);
    assert!(!slice.include_start);
    assert!(!slice.include_end);
    assert_eq!(slice.flags(), None);
    assert_eq!(
        ExtractionSpec::selector(selector_query("article")).strategy(),
        ExtractionStrategy::Selector
    );
    assert!(!NormalizationOptions::default().rewrite_urls);
    assert_eq!(
        OutputOptions::default().preview_chars,
        NonZeroUsize::new(DEFAULT_PREVIEW_CHARS).expect("preview chars")
    );
    assert_eq!(SourceRequest::stdin().kind(), SourceKind::Stdin);
    assert_eq!(
        selector_request("<article />").spec_version,
        CORE_SPEC_VERSION
    );
    assert_eq!(RuntimeOptions::default().max_bytes, DEFAULT_MAX_BYTES);
    assert_eq!(default_spec_version(), CORE_SPEC_VERSION);
    assert_eq!(default_preview_chars(), DEFAULT_PREVIEW_CHARS);
    assert_eq!(
        InspectionOptions::default().sample_limit,
        DEFAULT_INSPECTION_SAMPLE_LIMIT
    );
    assert_eq!(
        default_inspection_sample_limit(),
        DEFAULT_INSPECTION_SAMPLE_LIMIT
    );
    assert_eq!(default_regex_flags(), DEFAULT_REGEX_FLAGS);
    assert!(default_true());
    assert_eq!(default_max_bytes(), DEFAULT_MAX_BYTES);
    assert_eq!(default_fetch_timeout_ms(), DEFAULT_FETCH_TIMEOUT_MS);
    assert_eq!(format_byte_size(1024), "1 KB");
    assert_eq!(format_byte_size(1024 * 1024), "1 MB");
    assert_eq!(format_byte_size(1024 * 1024 * 1024), "1 GB");
    assert_eq!(format_byte_size(1024 * 1024 + 1), "1048577 bytes");
    assert_eq!(format_byte_size(1024 * 1024 * 1024 + 1), "1073741825 bytes");
    assert_eq!(format_byte_size(1536), "1536 bytes");
}

#[test]
fn operation_catalog_is_unique_and_complete() {
    let ids = operation_catalog()
        .iter()
        .map(|descriptor| descriptor.id)
        .collect::<BTreeSet<_>>();
    let id_strings = operation_catalog()
        .iter()
        .map(|descriptor| descriptor.id.as_str())
        .collect::<BTreeSet<_>>();

    assert_eq!(operation_catalog().len(), 6);
    assert_eq!(ids.len(), operation_catalog().len());
    assert_eq!(
        id_strings,
        BTreeSet::from([
            "document.parse",
            "source.inspect",
            "select.preview",
            "slice.preview",
            "select.extract",
            "slice.extract",
        ])
    );
    let document_parse = operation_catalog()
        .iter()
        .find(|descriptor| descriptor.id == OperationId::DocumentParse)
        .expect("document.parse should stay in the catalog");
    let select_extract = operation_catalog()
        .iter()
        .find(|descriptor| descriptor.id == OperationId::SelectExtract)
        .expect("select.extract should stay in the catalog");
    let select_preview = operation_catalog()
        .iter()
        .find(|descriptor| descriptor.id == OperationId::SelectPreview)
        .expect("select.preview should stay in the catalog");

    assert_eq!(document_parse.cli_surface, None);
    assert_eq!(select_extract.cli_surface, Some("select"));
    assert!(select_extract.core_surface.contains("extract"));
    assert_eq!(select_preview.cli_surface, Some("inspect select"));
    assert!(select_preview.core_surface.contains("preview_extraction"));
    assert_eq!(OperationId::DocumentParse.to_string(), "document.parse");
    assert_eq!(
        "slice.extract"
            .parse::<OperationId>()
            .expect("operation id"),
        OperationId::SliceExtract
    );
    assert!(matches!(
        "nope".parse::<OperationId>(),
        Err(OperationIdParseError)
    ));
    assert_eq!(
        OperationIdParseError.to_string(),
        "unknown HTMLCut operation ID"
    );
    assert_eq!(
        operation_descriptor(OperationId::SourceInspect).cli_surface,
        Some("inspect source")
    );
}

#[test]
fn schema_catalog_is_unique_and_covers_core_and_interop_contracts() {
    let identities = schema_catalog()
        .iter()
        .map(|descriptor| {
            (
                descriptor.schema_ref.schema_name,
                descriptor.schema_ref.schema_version,
            )
        })
        .collect::<BTreeSet<_>>();

    assert_eq!(identities.len(), schema_catalog().len());
    assert!(identities.contains(&(SOURCE_REQUEST_SCHEMA_NAME, CORE_REQUEST_SCHEMA_VERSION)));
    assert!(identities.contains(&(RUNTIME_OPTIONS_SCHEMA_NAME, CORE_REQUEST_SCHEMA_VERSION)));
    assert!(identities.contains(&(EXTRACTION_REQUEST_SCHEMA_NAME, CORE_REQUEST_SCHEMA_VERSION)));
    assert!(identities.contains(&(CORE_RESULT_SCHEMA_NAME, CORE_RESULT_SCHEMA_VERSION)));
    assert!(identities.contains(&(
        CORE_SOURCE_INSPECTION_SCHEMA_NAME,
        CORE_SOURCE_INSPECTION_SCHEMA_VERSION,
    )));
    assert!(identities.contains(&(
        interop::v1::PLAN_SCHEMA_NAME,
        interop::v1::PLAN_SCHEMA_VERSION,
    )));
    assert!(identities.contains(&(
        interop::v1::RESULT_SCHEMA_NAME,
        interop::v1::RESULT_SCHEMA_VERSION,
    )));
    assert!(identities.contains(&(
        interop::v1::ERROR_SCHEMA_NAME,
        interop::v1::ERROR_SCHEMA_VERSION,
    )));

    let extraction_result_schema =
        schema_descriptor(CORE_RESULT_SCHEMA_NAME, CORE_RESULT_SCHEMA_VERSION)
            .expect("extraction result schema");
    assert_eq!(extraction_result_schema.owner_surface, "htmlcut-core");
    assert_eq!(extraction_result_schema.rust_shape, "ExtractionResult");

    let interop_result_schema = schema_descriptor(
        interop::v1::RESULT_SCHEMA_NAME,
        interop::v1::RESULT_SCHEMA_VERSION,
    )
    .expect("interop result schema");
    assert_eq!(
        interop_result_schema.owner_surface,
        "htmlcut_core::interop::v1"
    );
    assert_eq!(interop_result_schema.stability, SchemaStability::Frozen);
}

#[test]
fn schemas_cover_inner_html_and_structured_metadata_variants() {
    let extraction_request_schema =
        (schema_descriptor(EXTRACTION_REQUEST_SCHEMA_NAME, CORE_REQUEST_SCHEMA_VERSION)
            .expect("extraction request schema")
            .json_schema)();
    let value_spec_variants = extraction_request_schema["$defs"]["ValueSpec"]["oneOf"]
        .as_array()
        .expect("value spec variants");
    let serialized_value_modes = value_spec_variants
        .iter()
        .filter_map(|variant| variant.pointer("/properties/type/const"))
        .filter_map(Value::as_str)
        .collect::<BTreeSet<_>>();
    assert!(serialized_value_modes.contains("inner-html"));
    assert!(!serialized_value_modes.contains("html"));

    let extraction_result_schema =
        (schema_descriptor(CORE_RESULT_SCHEMA_NAME, CORE_RESULT_SCHEMA_VERSION)
            .expect("extraction result schema")
            .json_schema)();
    let metadata_variants = extraction_result_schema["$defs"]["ExtractionMatchMetadata"]["oneOf"]
        .as_array()
        .expect("metadata variants");
    let metadata_kinds = metadata_variants
        .iter()
        .filter_map(|variant| variant.pointer("/properties/kind/const"))
        .filter_map(Value::as_str)
        .collect::<BTreeSet<_>>();
    assert_eq!(
        metadata_kinds,
        BTreeSet::from(["delimiter-pair", "selector"])
    );

    let value_type_variants = extraction_result_schema["$defs"]["ValueType"]["oneOf"]
        .as_array()
        .expect("value type variants");
    let serialized_value_types = value_type_variants
        .iter()
        .filter_map(|variant| variant.get("const"))
        .filter_map(Value::as_str)
        .collect::<BTreeSet<_>>();
    assert!(serialized_value_types.contains("inner-html"));
    assert!(!serialized_value_types.contains("html"));
}

#[test]
fn parse_document_and_preview_cover_public_entrypoints() {
    let request = selector_request("<article>Hello</article>");
    let parsed = parse_document(&request.source, &RuntimeOptions::default());
    assert!(parsed.ok);
    assert_eq!(parsed.operation_id, OperationId::DocumentParse);
    assert!(parsed.document.is_some());

    let inspection = inspect_source(
        &request.source,
        &RuntimeOptions::default(),
        &InspectionOptions::default(),
    );
    assert!(inspection.ok);
    assert_eq!(inspection.operation_id, OperationId::SourceInspect);
    assert!(inspection.document.is_some());

    let preview = preview_extraction(&request, &RuntimeOptions::default());
    assert!(preview.ok);
    assert_eq!(preview.operation_id, OperationId::SelectPreview);

    let missing = file_source("/definitely/missing.html");
    let parsed_error = parse_document(&missing, &RuntimeOptions::default());
    assert!(!parsed_error.ok);
    assert_eq!(parsed_error.operation_id, OperationId::DocumentParse);
    assert_eq!(parsed_error.diagnostics[0].code, "SOURCE_LOAD_FAILED");

    let inspection_error = inspect_source(
        &missing,
        &RuntimeOptions::default(),
        &InspectionOptions::default(),
    );
    assert!(!inspection_error.ok);
    assert_eq!(inspection_error.operation_id, OperationId::SourceInspect);
    assert_eq!(inspection_error.diagnostics[0].code, "SOURCE_LOAD_FAILED");

    let mut invalid = selector_request("<article>Hello</article>");
    invalid.spec_version = 0;
    let invalid_result = extract(&invalid, &RuntimeOptions::default());
    assert!(!invalid_result.ok);
    assert_eq!(invalid_result.operation_id, OperationId::SelectExtract);
    assert_eq!(invalid_result.stats.match_count, 0);
    assert_eq!(invalid_result.source.bytes_read, 0);
    assert_eq!(
        invalid_result.diagnostics[0].code,
        "UNSUPPORTED_SPEC_VERSION"
    );
}

#[test]
fn unresolved_effective_base_is_reported_for_inspection_and_rewrite_requests() {
    let source = memory_source(
        "inline",
        "<html><head><base href=\"../content/\"></head><body><a href=\"guide.html\">Guide</a></body></html>",
    );

    let inspection = inspect_source(
        &source,
        &RuntimeOptions::default(),
        &InspectionOptions::default(),
    );
    assert!(inspection.ok);
    assert_eq!(
        inspection.diagnostics[0].code,
        "EFFECTIVE_BASE_URL_UNRESOLVED"
    );
    assert!(inspection.source.effective_base_url.is_none());

    let mut selector_request = ExtractionRequest::new(
        source.clone(),
        ExtractionSpec::selector(selector_query("a")).with_value(attribute_value("href")),
    );
    selector_request.normalization.rewrite_urls = true;
    let selector_result = extract(&selector_request, &RuntimeOptions::default());
    assert!(selector_result.ok);
    assert_eq!(
        selector_result.matches[0].value,
        Value::String("guide.html".to_owned())
    );
    assert_eq!(
        selector_result.diagnostics[0].code,
        "EFFECTIVE_BASE_URL_UNRESOLVED"
    );

    let mut slice_request = ExtractionRequest::new(
        source,
        ExtractionSpec::slice(slice_spec("<a ", "</a>").with_boundary_inclusion(true, true))
            .with_value(attribute_value("href")),
    );
    slice_request.normalization.rewrite_urls = true;
    let slice_result = extract(&slice_request, &RuntimeOptions::default());
    assert!(slice_result.ok);
    assert_eq!(
        slice_result.matches[0].value,
        Value::String("guide.html".to_owned())
    );
    assert_eq!(
        slice_result.diagnostics[0].code,
        "EFFECTIVE_BASE_URL_UNRESOLVED"
    );
}

#[test]
fn inspect_source_summarizes_document_structure() {
    let source = memory_source_with_base(
        "fixture.html",
        "<!DOCTYPE html><html><head><title>Fixture</title><base href=\"../content/\"></head><body><main><article class=\"story card\"><h1>Hello</h1><p>World</p><a href=\"../guide.html\">Guide</a><img src=\"hero.png\" alt=\"Hero\"><table><tr><td>A</td></tr></table></article><section class=\"card\"><h2>More</h2><a href=\"/docs\">Docs</a></section></main></body></html>",
        "https://example.test/docs/start.html",
    );
    let inspection = inspect_source(
        &source,
        &RuntimeOptions::default(),
        &InspectionOptions {
            include_source_text: true,
            sample_limit: 4,
        },
    );

    assert!(inspection.ok);
    assert_eq!(
        inspection.source.text.as_deref(),
        Some(
            "<!DOCTYPE html><html><head><title>Fixture</title><base href=\"../content/\"></head><body><main><article class=\"story card\"><h1>Hello</h1><p>World</p><a href=\"../guide.html\">Guide</a><img src=\"hero.png\" alt=\"Hero\"><table><tr><td>A</td></tr></table></article><section class=\"card\"><h2>More</h2><a href=\"/docs\">Docs</a></section></main></body></html>"
        )
    );
    assert_eq!(
        inspection.source.input_base_url.as_deref(),
        Some("https://example.test/docs/start.html")
    );
    assert_eq!(
        inspection.source.effective_base_url.as_deref(),
        Some("https://example.test/content/")
    );
    let document = inspection.document.expect("document inspection");
    assert_eq!(document.title.as_deref(), Some("Fixture"));
    assert_eq!(document.document_base_href.as_deref(), Some("../content/"));
    assert_eq!(document.root_tag, "html");
    assert!(document.element_count >= 10);
    assert_eq!(document.link_count, 2);
    assert_eq!(document.image_count, 1);
    assert_eq!(document.table_count, 1);
    assert_eq!(document.top_tags[0].name, "a");
    assert_eq!(document.top_tags[0].count, 2);
    assert_eq!(document.top_classes[0].name, "card");
    assert_eq!(document.top_classes[0].count, 2);
    assert_eq!(document.headings[0].level, 1);
    assert_eq!(document.headings[0].text, "Hello");
    assert_eq!(document.links[0].href.as_deref(), Some("../guide.html"));
    assert_eq!(
        document.links[0].resolved_href.as_deref(),
        Some("https://example.test/guide.html")
    );
    assert!(document.text_char_count > 0);
}

#[test]
fn inspect_source_honors_zero_sample_limit_without_collecting_previews() {
    let source = memory_source_with_base(
        "fixture.html",
        "<html><body><h1>Hello</h1><a href=\"/guide\">Guide</a><a>No href</a></body></html>",
        "https://example.test/start.html",
    );

    let inspection = inspect_source(
        &source,
        &RuntimeOptions::default(),
        &InspectionOptions {
            include_source_text: false,
            sample_limit: 0,
        },
    );

    assert!(inspection.ok);
    let document = inspection.document.expect("document inspection");
    assert_eq!(document.link_count, 2);
    assert!(document.headings.is_empty());
    assert!(document.links.is_empty());
    assert!(document.top_tags.is_empty());
    assert!(document.top_classes.is_empty());
}

#[test]
fn validate_request_covers_invalid_request_paths() {
    let mut request = selector_request("");
    request.spec_version = 99;
    let diagnostics = validate_request(&request);
    assert!(has_errors(&diagnostics));
    assert!(
        diagnostics
            .iter()
            .any(|item| item.code == "UNSUPPORTED_SPEC_VERSION")
    );
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn validate_request_accepts_consistent_requests() {
    let selector = selector_request("<article>Hello</article>");
    assert!(validate_request(&selector).is_empty());

    let mut slice = slice_request(
        "<section data-id=\"7\">Hello</section>",
        "<section",
        "</section>",
    );
    slice.extraction = ExtractionSpec::slice(SliceSpec {
        pattern: SlicePatternSpec::literal(
            slice_boundary("<section"),
            slice_boundary("</section>"),
        ),
        include_start: true,
        include_end: true,
    })
    .with_selection(nth_selection(1))
    .with_value(attribute_value("data-id"));
    slice.output.preview_chars = NonZeroUsize::new(32).expect("preview chars");

    assert!(validate_request(&slice).is_empty());
}

#[test]
fn selector_match_builder_covers_value_modes_and_output_toggles() {
    let mut request = selector_request("<article data-id=\"7\"><p>Hello</p></article>");
    let document = parse_document_node("<article data-id=\"7\"><p>Hello</p></article>");
    let node = select_first(&document, "article").expect("selector");
    let loaded = load_source(&request.source, &RuntimeOptions::default()).expect("loaded");
    let effective_base_url = resolve_document_base_url(&document, loaded.input_base_url.as_deref());

    request.extraction = request.extraction.clone().with_value(ValueSpec::InnerHtml);
    let html_match =
        build_selector_match(&request, effective_base_url.as_deref(), &node, 1, 1, 1, 1)
            .expect("html match");
    assert_eq!(html_match.value.as_str(), Some("<p>Hello</p>"));

    request.extraction = request.extraction.clone().with_value(ValueSpec::OuterHtml);
    let outer_match =
        build_selector_match(&request, effective_base_url.as_deref(), &node, 1, 1, 1, 1)
            .expect("outer match");
    assert!(
        outer_match
            .value
            .as_str()
            .is_some_and(|html| html.contains("article"))
    );

    request.extraction = request
        .extraction
        .clone()
        .with_value(attribute_value("data-id"));
    request.normalization.whitespace = WhitespaceMode::Normalize;
    let attribute_match =
        build_selector_match(&request, effective_base_url.as_deref(), &node, 1, 1, 1, 1)
            .expect("attribute match");
    assert_eq!(attribute_match.value.as_str(), Some("7"));

    request.extraction = request
        .extraction
        .clone()
        .with_value(attribute_value("href"));
    let missing_attribute =
        build_selector_match(&request, effective_base_url.as_deref(), &node, 1, 1, 1, 1)
            .expect_err("missing attr");
    assert_eq!(missing_attribute.code, "MISSING_ATTRIBUTE");

    request.extraction = request.extraction.clone().with_value(ValueSpec::Structured);
    request.output.include_html = false;
    request.output.include_text = false;
    let structured_match =
        build_selector_match(&request, effective_base_url.as_deref(), &node, 1, 1, 1, 2)
            .expect("structured");
    assert!(structured_match.html.is_none());
    assert!(structured_match.text.is_none());
    assert_eq!(structured_match.value["tagName"], "article");
    assert_eq!(structured_match.value["matchIndex"], 1);
    assert_eq!(structured_match.value["matchCount"], 1);
    assert_eq!(structured_match.value["candidateIndex"], 1);
    assert_eq!(structured_match.value["candidateCount"], 2);
    assert_eq!(structured_match.metadata.candidate_index(), 1);
    assert_eq!(structured_match.metadata.candidate_count(), 2);
}

#[test]
fn selector_match_builder_emits_optional_payloads_when_requested() {
    let request = selector_request("<article><p>Hello</p></article>");
    let document = parse_document_node("<article><p>Hello</p></article>");
    let node = select_first(&document, "article").expect("selector");
    let loaded = load_source(&request.source, &RuntimeOptions::default()).expect("loaded");
    let effective_base_url = resolve_document_base_url(&document, loaded.input_base_url.as_deref());

    let matched = build_selector_match(&request, effective_base_url.as_deref(), &node, 1, 1, 1, 1)
        .expect("match");

    assert!(
        matched
            .html
            .as_deref()
            .is_some_and(|html| html.contains("<article>"))
    );
    assert_eq!(matched.text.as_deref(), Some("Hello"));
}

#[test]
fn slice_match_builder_covers_value_modes() {
    let mut request = ExtractionRequest::new(
        memory_source_with_base(
            "inline",
            "<a href=\"/x\">Hello</a>",
            "https://example.com/base/",
        ),
        ExtractionSpec::slice(slice_spec("<a", "</a>").with_boundary_inclusion(true, true))
            .with_value(ValueSpec::InnerHtml),
    );
    request.normalization = NormalizationOptions {
        whitespace: WhitespaceMode::Normalize,
        rewrite_urls: true,
    };
    let loaded = load_source(&request.source, &RuntimeOptions::default()).expect("loaded");
    let effective_base_url = resolve_document_base_url(
        &parse_document_node(&loaded.text),
        loaded.input_base_url.as_deref(),
    );
    let candidate = extract_slice_candidates(
        &loaded.text,
        request.extraction.slice_spec().expect("slice spec"),
    )
    .expect("candidate")
    .remove(0);

    let html_match = build_slice_match(
        &request,
        effective_base_url.as_deref(),
        &candidate,
        1,
        1,
        1,
        1,
    )
    .expect("html");
    assert!(
        html_match
            .value
            .as_str()
            .is_some_and(|html| html.contains("https://example.com/x"))
    );

    let mut attribute_request = request.clone();
    attribute_request.extraction = attribute_request
        .extraction
        .clone()
        .with_value(attribute_value("href"));
    let attribute_match = build_slice_match(
        &attribute_request,
        effective_base_url.as_deref(),
        &candidate,
        1,
        1,
        1,
        1,
    )
    .expect("attribute");
    assert_eq!(
        attribute_match.value.as_str(),
        Some("https://example.com/x")
    );

    attribute_request.extraction = attribute_request
        .extraction
        .clone()
        .with_value(attribute_value("title"));
    let missing = build_slice_match(
        &attribute_request,
        effective_base_url.as_deref(),
        &candidate,
        1,
        1,
        1,
        1,
    )
    .expect_err("missing attr");
    assert_eq!(missing.code, "MISSING_ATTRIBUTE");
    assert!(
        missing
            .message
            .contains("Extracted fragment is missing attribute \"title\".")
    );

    let mut inner_capture_request = request.clone();
    inner_capture_request.extraction = ExtractionSpec::slice(SliceSpec {
        pattern: SlicePatternSpec::literal(slice_boundary("<a "), slice_boundary("</a>")),
        include_start: false,
        include_end: false,
    })
    .with_value(attribute_value("href"));
    let inner_candidate = extract_slice_candidates(
        &loaded.text,
        inner_capture_request
            .extraction
            .slice_spec()
            .expect("slice spec"),
    )
    .expect("candidate")
    .remove(0);
    let hinted_missing = build_slice_match(
        &inner_capture_request,
        effective_base_url.as_deref(),
        &inner_candidate,
        1,
        1,
        1,
        1,
    )
    .expect_err("inner capture should drop opening-tag attributes");
    assert_eq!(hinted_missing.code, "MISSING_ATTRIBUTE");
    assert!(hinted_missing.message.contains("use --include-start"));
    assert_eq!(
        hinted_missing
            .details
            .as_ref()
            .and_then(|details| details.get("hint"))
            .and_then(Value::as_str),
        Some("use --include-start")
    );

    let mut structured_request = request.clone();
    structured_request.extraction = structured_request
        .extraction
        .clone()
        .with_value(ValueSpec::Structured);
    let structured = build_slice_match(
        &structured_request,
        effective_base_url.as_deref(),
        &candidate,
        1,
        1,
        1,
        1,
    )
    .expect("structured");
    assert_eq!(structured.value["matchIndex"], 1);
    assert_eq!(structured.value["matchCount"], 1);
    assert_eq!(structured.value["candidateIndex"], 1);
    assert_eq!(structured.value["candidateCount"], 1);
    assert_eq!(
        structured.value["outerHtml"],
        "<a href=\"https://example.com/x\">Hello</a>"
    );
    assert_eq!(structured.value["includeStart"], true);
    assert_eq!(structured.value["includeEnd"], true);
}

#[test]
fn slice_match_builder_covers_text_and_outer_html_modes() {
    let mut request = ExtractionRequest::new(
        memory_source_with_base(
            "inline",
            "<div><a href=\"/x\"> Hello </a></div>",
            "https://example.com/base/",
        ),
        ExtractionSpec::slice(slice_spec("<a", "</a>").with_boundary_inclusion(true, true))
            .with_value(ValueSpec::Text),
    );
    request.normalization = NormalizationOptions {
        whitespace: WhitespaceMode::Normalize,
        rewrite_urls: true,
    };
    let loaded = load_source(&request.source, &RuntimeOptions::default()).expect("loaded");
    let effective_base_url = resolve_document_base_url(
        &parse_document_node(&loaded.text),
        loaded.input_base_url.as_deref(),
    );
    let candidate = extract_slice_candidates(
        &loaded.text,
        request.extraction.slice_spec().expect("slice spec"),
    )
    .expect("candidate")
    .remove(0);

    let text_match = build_slice_match(
        &request,
        effective_base_url.as_deref(),
        &candidate,
        1,
        1,
        1,
        1,
    )
    .expect("text");
    assert_eq!(text_match.value.as_str(), Some("Hello"));

    request.extraction = request.extraction.clone().with_value(ValueSpec::OuterHtml);
    let outer_html_match = build_slice_match(
        &request,
        effective_base_url.as_deref(),
        &candidate,
        1,
        1,
        1,
        1,
    )
    .expect("outer html");
    assert!(
        outer_html_match
            .value
            .as_str()
            .is_some_and(|html| html.contains("https://example.com/x"))
    );
}

#[test]
fn slice_candidate_extraction_and_regex_builder_cover_error_paths() {
    let slice = slice_spec("<p>", "</p>");
    let no_end = extract_slice_candidates("<p>Hello", &slice).expect_err("no end");
    assert_eq!(no_end.code, "NO_MATCH");

    let no_start = extract_slice_candidates("Hello", &slice).expect_err("no start");
    assert_eq!(no_start.code, "NO_MATCH");

    let empty_pattern = build_finder("", PatternMode::Literal, None)
        .err()
        .expect("empty pattern");
    assert_eq!(empty_pattern.code, "INVALID_SLICE_PATTERN");

    let regex = build_regex("a.*b", "imsUx").expect("regex");
    assert!(regex.is_match("A\nB"));

    let invalid_regex = build_regex("[", "u").expect_err("invalid regex");
    assert_eq!(invalid_regex.code, "INVALID_SLICE_PATTERN");

    let zero_width = extract_slice_candidates(
        "abc",
        &regex_slice_spec(r"\b", r"\b").with_boundary_inclusion(true, true),
    )
    .expect("zero width candidates");
    assert_eq!(zero_width.len(), 2);
    assert_eq!(zero_width[0].selected_range.start, 0);
    assert_eq!(zero_width[1].selected_range.start, 3);
}

#[test]
fn slice_finders_cover_literal_regex_and_empty_reader_edges() {
    let literal = build_finder("<p>", PatternMode::Literal, None).expect("literal finder");
    assert_eq!(literal("<p>Hello</p>", 0).expect("literal hit").start, 0);
    assert!(literal("<p>Hello</p>", 10).is_none());

    let regex = build_finder(r"h\w+", PatternMode::Regex, Some("iu")).expect("regex finder");
    assert_eq!(regex("Hello", 0).expect("regex hit").start, 0);
    assert!(regex("Hello", 5).is_none());

    let mut empty = Cursor::new(Vec::<u8>::new());
    assert_eq!(
        read_limited_to_string(&mut empty, 10, "Input").expect("empty input"),
        ""
    );
}

#[test]
fn extraction_runs_cover_selector_and_slice_candidate_selection_branches() {
    let mut selector_no_match_request = selector_request("<article>Hello</article>");
    selector_no_match_request.extraction = ExtractionSpec::selector(selector_query("aside"));
    let selector_no_match = extract(&selector_no_match_request, &RuntimeOptions::default());
    assert!(!selector_no_match.ok);
    assert_eq!(selector_no_match.diagnostics[0].code, "NO_MATCH");

    let selector_multiple = extract(
        &selector_request("<article>One</article><article>Two</article>"),
        &RuntimeOptions::default(),
    );
    assert!(selector_multiple.ok);
    assert!(
        selector_multiple
            .diagnostics
            .iter()
            .any(|item| item.code == "MULTIPLE_MATCHES")
    );

    let slice_no_match = extract(
        &slice_request("<div>Hello</div>", "<section>", "</section>"),
        &RuntimeOptions::default(),
    );
    assert!(!slice_no_match.ok);
    assert_eq!(slice_no_match.diagnostics[0].code, "NO_MATCH");

    let slice_multiple = extract(
        &slice_request(
            "<article>One</article><article>Two</article>",
            "<article>",
            "</article>",
        ),
        &RuntimeOptions::default(),
    );
    assert!(slice_multiple.ok);
    assert!(
        slice_multiple
            .diagnostics
            .iter()
            .any(|item| item.code == "MULTIPLE_MATCHES")
    );
}

#[test]
fn select_candidates_and_source_helpers_cover_remaining_branches() {
    let (selected, diagnostics) = select_candidates::<i32>(&[], &SelectionSpec::default());
    assert!(selected.is_empty());
    assert_eq!(diagnostics[0].code, "NO_MATCH");

    let (selected, diagnostics) = select_candidates(&[1, 2], &SelectionSpec::All);
    assert_eq!(
        selected
            .into_iter()
            .map(|item| item.candidate)
            .collect::<Vec<_>>(),
        vec![1, 2]
    );
    assert!(diagnostics.is_empty());

    let (selected, diagnostics) = select_candidates(
        &[1, 2],
        &SelectionSpec::nth(NonZeroUsize::new(2).expect("match index")),
    );
    assert_eq!(
        selected
            .into_iter()
            .map(|item| item.candidate)
            .collect::<Vec<_>>(),
        vec![2]
    );
    assert!(diagnostics.is_empty());

    let (selected, diagnostics) = select_candidates(
        &[1],
        &SelectionSpec::nth(NonZeroUsize::new(1).expect("match index")),
    );
    assert_eq!(
        selected
            .into_iter()
            .map(|item| item.candidate)
            .collect::<Vec<_>>(),
        vec![1]
    );
    assert!(diagnostics.is_empty());

    let source = memory_source("", "Hello");
    let loaded = load_source(&source, &RuntimeOptions::default()).expect("memory load");
    assert_eq!(loaded.value, "memory");
    let url_metadata = empty_source_metadata(&url_source("https://example.com/docs/page.html"));
    assert_eq!(url_metadata.value, "https://example.com/docs/page.html");
    assert_eq!(
        url_metadata.input_base_url.as_deref(),
        Some("https://example.com/docs/page.html")
    );
    assert_eq!(SourceRequest::stdin().kind(), SourceKind::Stdin);
    assert_eq!(url_source("https://example.com").kind(), SourceKind::Url);
    assert_eq!(file_source("page.html").kind(), SourceKind::File);
}

#[test]
fn select_candidates_covers_first_and_invalid_nth_cases() {
    let (selected, diagnostics) = select_candidates(&[1, 2], &SelectionSpec::default());
    assert_eq!(
        selected
            .into_iter()
            .map(|item| item.candidate)
            .collect::<Vec<_>>(),
        vec![1]
    );
    assert_eq!(diagnostics[0].code, "MULTIPLE_MATCHES");

    let (selected, diagnostics) = select_candidates(&[1], &SelectionSpec::First);
    assert_eq!(
        selected
            .into_iter()
            .map(|item| item.candidate)
            .collect::<Vec<_>>(),
        vec![1]
    );
    assert!(diagnostics.is_empty());

    let (selected, diagnostics) = select_candidates(&[1, 2], &SelectionSpec::single());
    assert!(selected.is_empty());
    assert_eq!(diagnostics[0].code, "AMBIGUOUS_MATCH");

    let (selected, diagnostics) = select_candidates(
        &[1, 2],
        &SelectionSpec::nth(NonZeroUsize::new(3).expect("match index")),
    );
    assert!(selected.is_empty());
    assert_eq!(diagnostics[0].code, "MATCH_INDEX_OUT_OF_RANGE");
}

#[test]
fn source_reading_helpers_cover_error_paths() {
    struct BrokenReader;
    impl Read for BrokenReader {
        fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
            Err(IoError::other("boom"))
        }
    }

    let mut broken = BrokenReader;
    let read_error = read_limited_to_string(&mut broken, 10, "Input").expect_err("read error");
    assert_eq!(read_error.code, "SOURCE_LOAD_FAILED");

    let mut oversized = Cursor::new(b"abcdef".to_vec());
    let size_error = read_limited_to_string(&mut oversized, 3, "Input").expect_err("size error");
    assert_eq!(size_error.code, "SOURCE_LOAD_FAILED");

    let mut invalid_utf8 = Cursor::new(vec![0xff, 0xfe]);
    let utf8_error =
        read_limited_to_string(&mut invalid_utf8, 10, "Input").expect_err("utf8 error");
    assert_eq!(utf8_error.code, "SOURCE_LOAD_FAILED");

    let tempdir = tempfile::tempdir().expect("tempdir");
    let large_path = tempdir.path().join("large.txt");
    std::fs::write(&large_path, "1234567890").expect("write");
    let file_error = read_file_source(
        &file_source(&large_path),
        &RuntimeOptions {
            max_bytes: 3,
            fetch_timeout_ms: 1000,
            ..RuntimeOptions::default()
        },
    )
    .expect_err("file too large");
    assert_eq!(file_error.code, "SOURCE_LOAD_FAILED");

    let missing_file = read_file_source(
        &file_source(tempdir.path().join("missing.txt")),
        &RuntimeOptions::default(),
    )
    .expect_err("missing file");
    assert_eq!(missing_file.code, "SOURCE_LOAD_FAILED");

    let invalid_utf8_path = tempdir.path().join("invalid-utf8.txt");
    std::fs::write(&invalid_utf8_path, [0xff, 0xfe]).expect("write invalid utf8");
    let invalid_utf8_file =
        read_file_source(&file_source(&invalid_utf8_path), &RuntimeOptions::default())
            .expect_err("invalid utf8 file");
    assert_eq!(invalid_utf8_file.code, "SOURCE_LOAD_FAILED");
    assert!(
        invalid_utf8_file
            .message
            .contains("File is not valid UTF-8:")
    );

    let directory_error =
        read_file_source(&file_source(tempdir.path()), &RuntimeOptions::default())
            .expect_err("directory input");
    assert_eq!(directory_error.code, "SOURCE_LOAD_FAILED");
    assert!(
        directory_error
            .message
            .contains("Input path is a directory, not a file:")
    );

    let closed_listener = TcpListener::bind("127.0.0.1:0").expect("bind closed listener");
    let closed_address = closed_listener.local_addr().expect("closed listener addr");
    drop(closed_listener);
    let fetch_error = read_url_source(
        &url_source(&format!("http://{closed_address}")),
        &RuntimeOptions {
            max_bytes: 1024,
            fetch_timeout_ms: 250,
            ..RuntimeOptions::default()
        },
    )
    .expect_err("fetch error");
    assert_eq!(fetch_error.code, "SOURCE_LOAD_FAILED");

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind size server");
    let address = listener.local_addr().expect("size server addr");
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let mut request_buffer = [0u8; 512];
        let _ = stream.read(&mut request_buffer).expect("read request");
        let response = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: 9999\r\n\r\n";
        stream
            .write_all(response.as_bytes())
            .expect("write response");
    });

    let oversized_response = read_url_source(
        &url_source(&format!("http://{address}")),
        &RuntimeOptions {
            max_bytes: 4,
            fetch_timeout_ms: 1000,
            ..RuntimeOptions::default()
        },
    )
    .expect_err("oversized response");
    server.join().expect("join server");
    assert_eq!(oversized_response.code, "SOURCE_LOAD_FAILED");
}

#[test]
fn source_loading_covers_memory_limits_and_extract_load_failures() {
    let oversized_memory = load_source(
        &memory_source("inline", "12345"),
        &RuntimeOptions {
            max_bytes: 3,
            fetch_timeout_ms: 1000,
            ..RuntimeOptions::default()
        },
    )
    .expect_err("oversized memory source");
    assert_eq!(oversized_memory.code, "SOURCE_LOAD_FAILED");

    assert_eq!(url_source("http://example.com").kind(), SourceKind::Url);

    let extract_result = extract(
        &ExtractionRequest::new(
            memory_source("inline", "12345"),
            selector_request("<article>Hello</article>").extraction,
        ),
        &RuntimeOptions {
            max_bytes: 3,
            fetch_timeout_ms: 1000,
            ..RuntimeOptions::default()
        },
    );
    assert!(!extract_result.ok);
    assert_eq!(extract_result.diagnostics[0].code, "SOURCE_LOAD_FAILED");
}

#[test]
fn file_and_url_loading_cover_successful_non_error_branches() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let file_path = tempdir.path().join("input.html");
    std::fs::write(&file_path, "<article>Hello</article>").expect("write html");
    let loaded = read_file_source(
        &file_source(&file_path)
            .with_base_url(Url::parse("https://example.com/base/").expect("base url")),
        &RuntimeOptions::default(),
    )
    .expect("file source");
    assert_eq!(
        loaded.input_base_url.as_deref(),
        Some("https://example.com/base/")
    );

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind success server");
    let address = listener.local_addr().expect("server addr");
    let server = thread::spawn(move || {
        for _ in 0..2 {
            let (mut stream, _) = listener.accept().expect("accept");
            let mut request_buffer = [0u8; 512];
            let read = stream.read(&mut request_buffer).expect("read request");
            let request = String::from_utf8_lossy(&request_buffer[..read]);
            let method = request
                .lines()
                .next()
                .expect("request line")
                .split_whitespace()
                .next()
                .expect("request method");
            let body = "<html><body>Hello</body></html>";
            let response = if method == "HEAD" {
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\n\r\n",
                    body.len()
                )
            } else {
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\n\r\n{}",
                    body.len(),
                    body
                )
            };
            stream
                .write_all(response.as_bytes())
                .expect("write response");
        }
    });
    let url = format!("http://{address}");
    let loaded_url =
        read_url_source(&url_source(&url), &RuntimeOptions::default()).expect("url source");
    server.join().expect("join server");
    let expected_url = format!("{url}/");
    assert_eq!(
        loaded_url.input_base_url.as_deref(),
        Some(expected_url.as_str())
    );

    let agent = build_http_agent(&RuntimeOptions::default());
    assert!(matches!(
        agent.config().tls_config().root_certs(),
        RootCerts::PlatformVerifier
    ));
}

#[test]
fn get_only_fetch_preflight_skips_head_requests() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind get-only server");
    let address = listener.local_addr().expect("get-only server addr");
    let methods = Arc::new(Mutex::new(Vec::new()));
    let methods_for_server = Arc::clone(&methods);
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let mut request_buffer = [0u8; 512];
        let read = stream.read(&mut request_buffer).expect("read request");
        let request = String::from_utf8_lossy(&request_buffer[..read]);
        let method = request
            .lines()
            .next()
            .expect("request line")
            .split_whitespace()
            .next()
            .expect("request method")
            .to_owned();
        methods_for_server
            .lock()
            .expect("lock methods")
            .push(method);

        let body = "<html><body>Hello</body></html>";
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        stream
            .write_all(response.as_bytes())
            .expect("write response");
    });

    let url = format!("http://{address}");
    let loaded = read_url_source(
        &url_source(&url),
        &RuntimeOptions {
            fetch_preflight: FetchPreflightMode::GetOnly,
            ..RuntimeOptions::default()
        },
    )
    .expect("get-only source");

    server.join().expect("join server");
    assert_eq!(methods.lock().expect("lock methods").as_slice(), ["GET"]);
    assert_eq!(loaded.text, "<html><body>Hello</body></html>");
}

#[test]
fn head_preflight_falls_back_to_get_when_head_is_unsupported() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind fallback server");
    let address = listener.local_addr().expect("fallback server addr");
    let methods = Arc::new(Mutex::new(Vec::new()));
    let methods_for_server = Arc::clone(&methods);
    let server = thread::spawn(move || {
        for _ in 0..2 {
            let (mut stream, _) = listener.accept().expect("accept");
            let mut request_buffer = [0u8; 512];
            let read = stream.read(&mut request_buffer).expect("read request");
            let request = String::from_utf8_lossy(&request_buffer[..read]);
            let method = request
                .lines()
                .next()
                .expect("request line")
                .split_whitespace()
                .next()
                .expect("request method")
                .to_owned();
            methods_for_server
                .lock()
                .expect("lock methods")
                .push(method.clone());

            let response = if method == "HEAD" {
                "HTTP/1.1 405 Method Not Allowed\r\nContent-Length: 0\r\n\r\n".to_owned()
            } else {
                let body = "<html><body>Fallback</body></html>";
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\n\r\n{}",
                    body.len(),
                    body
                )
            };
            stream
                .write_all(response.as_bytes())
                .expect("write response");
        }
    });

    let url = format!("http://{address}");
    let loaded =
        read_url_source(&url_source(&url), &RuntimeOptions::default()).expect("fallback source");

    server.join().expect("join server");
    assert_eq!(
        methods.lock().expect("lock methods").as_slice(),
        ["HEAD", "GET"]
    );
    assert_eq!(loaded.text, "<html><body>Fallback</body></html>");
}

#[test]
fn head_preflight_falls_back_to_get_when_head_transport_breaks() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind broken-head server");
    let address = listener.local_addr().expect("broken-head server addr");
    let methods = Arc::new(Mutex::new(Vec::new()));
    let methods_for_server = Arc::clone(&methods);
    let server = thread::spawn(move || {
        for _ in 0..2 {
            let (mut stream, _) = listener.accept().expect("accept");
            let mut request_buffer = [0u8; 512];
            let read = stream.read(&mut request_buffer).expect("read request");
            let request = String::from_utf8_lossy(&request_buffer[..read]);
            let method = request
                .lines()
                .next()
                .expect("request line")
                .split_whitespace()
                .next()
                .expect("request method")
                .to_owned();
            methods_for_server
                .lock()
                .expect("lock methods")
                .push(method.clone());

            if method == "HEAD" {
                continue;
            }

            let body = "<html><body>Recovered</body></html>";
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                body
            );
            stream
                .write_all(response.as_bytes())
                .expect("write response");
        }
    });

    let url = format!("http://{address}");
    let loaded = read_url_source(
        &url_source(&url),
        &RuntimeOptions {
            fetch_timeout_ms: 250,
            ..RuntimeOptions::default()
        },
    )
    .expect("fallback source after broken head transport");

    server.join().expect("join server");
    assert_eq!(
        methods.lock().expect("lock methods").as_slice(),
        ["HEAD", "GET"]
    );
    assert_eq!(loaded.text, "<html><body>Recovered</body></html>");
}

#[test]
fn head_preflight_fallback_classifier_accepts_only_head_intolerance_errors() {
    assert!(head_error_allows_get_fallback_for_tests(
        &ureq::Error::ConnectionFailed
    ));
    assert!(head_error_allows_get_fallback_for_tests(&ureq::Error::Io(
        io::Error::new(io::ErrorKind::UnexpectedEof, "peer disconnected"),
    )));
    assert!(!head_error_allows_get_fallback_for_tests(&ureq::Error::Io(
        io::Error::new(io::ErrorKind::TimedOut, "timed out"),
    )));
    assert!(!head_error_allows_get_fallback_for_tests(
        &ureq::Error::HostNotFound
    ));
}

#[test]
fn head_preflight_rejects_non_html_responses_before_get() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind non-html server");
    let address = listener.local_addr().expect("non-html server addr");
    let methods = Arc::new(Mutex::new(Vec::new()));
    let methods_for_server = Arc::clone(&methods);
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let mut request_buffer = [0u8; 512];
        let read = stream.read(&mut request_buffer).expect("read request");
        let request = String::from_utf8_lossy(&request_buffer[..read]);
        let method = request
            .lines()
            .next()
            .expect("request line")
            .split_whitespace()
            .next()
            .expect("request method")
            .to_owned();
        methods_for_server
            .lock()
            .expect("lock methods")
            .push(method);

        let response = "HTTP/1.1 200 OK\r\nContent-Type: image/png\r\nContent-Length: 0\r\n\r\n";
        stream
            .write_all(response.as_bytes())
            .expect("write response");
    });

    let url = format!("http://{address}");
    let error = read_url_source(&url_source(&url), &RuntimeOptions::default())
        .expect_err("non-html preflight error");

    server.join().expect("join server");
    assert_eq!(methods.lock().expect("lock methods").as_slice(), ["HEAD"]);
    assert_eq!(error.code, "SOURCE_LOAD_FAILED");
    assert!(
        error
            .message
            .contains("reported non-HTML content type image/png")
    );
    assert_eq!(
        error
            .details
            .as_ref()
            .and_then(|details| details.get("method"))
            .and_then(Value::as_str),
        Some("HEAD")
    );
}

#[test]
fn url_loading_get_error_and_status_failures_cover_remaining_branches() {
    let closed_listener = TcpListener::bind("127.0.0.1:0").expect("bind closed listener");
    let closed_address = closed_listener.local_addr().expect("closed listener addr");
    drop(closed_listener);

    let transport_error = read_url_source(
        &url_source(&format!("http://{closed_address}")),
        &RuntimeOptions {
            fetch_preflight: FetchPreflightMode::GetOnly,
            fetch_timeout_ms: 250,
            ..RuntimeOptions::default()
        },
    )
    .expect_err("get transport failure");
    assert_eq!(transport_error.code, "SOURCE_LOAD_FAILED");
    assert!(transport_error.message.contains("Could not fetch"));
    assert_eq!(
        transport_error
            .details
            .as_ref()
            .and_then(|details| details.get("method"))
            .and_then(Value::as_str),
        Some("GET")
    );

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind status server");
    let address = listener.local_addr().expect("status server addr");
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let mut request_buffer = [0u8; 512];
        let _ = stream.read(&mut request_buffer).expect("read request");
        let response =
            "HTTP/1.1 404 Not Found\r\nContent-Type: text/html\r\nContent-Length: 0\r\n\r\n";
        stream
            .write_all(response.as_bytes())
            .expect("write response");
    });

    let status_error = read_url_source(
        &url_source(&format!("http://{address}")),
        &RuntimeOptions {
            fetch_preflight: FetchPreflightMode::GetOnly,
            ..RuntimeOptions::default()
        },
    )
    .expect_err("unexpected get status");

    server.join().expect("join server");
    assert_eq!(status_error.code, "SOURCE_LOAD_FAILED");
    assert!(
        status_error
            .message
            .contains("returned unexpected status 404")
    );
    assert_eq!(
        status_error
            .details
            .as_ref()
            .and_then(|details| details.get("method"))
            .and_then(Value::as_str),
        Some("GET")
    );
    assert_eq!(
        status_error
            .details
            .as_ref()
            .and_then(|details| details.get("status"))
            .and_then(Value::as_u64),
        Some(404)
    );
}

#[test]
fn url_loading_accepts_headerless_and_xhtml_success_responses() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind headerless server");
    let address = listener.local_addr().expect("headerless server addr");
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let mut request_buffer = [0u8; 512];
        let _ = stream.read(&mut request_buffer).expect("read request");
        let body = "<html><body>Headerless</body></html>";
        let response = format!("HTTP/1.1 200 OK\r\n\r\n{body}");
        stream
            .write_all(response.as_bytes())
            .expect("write response");
    });

    let headerless = read_url_source(
        &url_source(&format!("http://{address}")),
        &RuntimeOptions {
            fetch_preflight: FetchPreflightMode::GetOnly,
            ..RuntimeOptions::default()
        },
    )
    .expect("headerless response");
    server.join().expect("join server");
    assert_eq!(headerless.text, "<html><body>Headerless</body></html>");

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind xhtml server");
    let address = listener.local_addr().expect("xhtml server addr");
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let mut request_buffer = [0u8; 512];
        let _ = stream.read(&mut request_buffer).expect("read request");
        let body = "<html xmlns=\"http://www.w3.org/1999/xhtml\"><body>XHTML</body></html>";
        let response =
            format!("HTTP/1.1 200 OK\r\nContent-Type: application/xhtml+xml\r\n\r\n{body}");
        stream
            .write_all(response.as_bytes())
            .expect("write response");
    });

    let xhtml = read_url_source(
        &url_source(&format!("http://{address}")),
        &RuntimeOptions {
            fetch_preflight: FetchPreflightMode::GetOnly,
            ..RuntimeOptions::default()
        },
    )
    .expect("xhtml response");
    server.join().expect("join server");
    assert!(xhtml.text.contains("XHTML"));
    assert!(!content_type_is_obviously_non_html_for_tests(""));
    assert!(!content_type_is_obviously_non_html_for_tests("text/html"));
    assert!(!content_type_is_obviously_non_html_for_tests(
        "application/xhtml+xml"
    ));
    assert!(content_type_is_obviously_non_html_for_tests("image/png"));
}

#[test]
fn rendering_and_url_helpers_cover_remaining_paths() {
    let node = parse_document_node("<article data-id=\"7\"><p>Hello</p></article>");
    assert!(serialize_document(&node).contains("Hello"));
    assert!(
        serialize_children(&select_first(&node, "article").expect("article"))
            .contains("<p>Hello</p>")
    );
    let article = select_first(&node, "article").expect("article");
    assert_eq!(
        build_node_path(&select_first(&node, "p").expect("p")),
        "html:nth-of-type(1) > body:nth-of-type(1) > article:nth-of-type(1) > p:nth-of-type(1)"
    );
    assert_eq!(element_name(node.tree.root()), None);
    assert_eq!(element_name(*article), Some("article".to_owned()));
    assert_eq!(
        element_attributes(&article, Some("https://example.com/base/"), false).get("data-id"),
        Some(&"7".to_owned())
    );
    let linked = parse_document_node(
        "<a class=\"card featured\" href=\"guide.html\" data-track=\"hero\">Guide</a>",
    );
    let anchor = select_first(&linked, "a").expect("anchor");
    let rewritten_attributes = element_attributes(&anchor, Some("https://example.com/base/"), true);
    assert_eq!(
        rewritten_attributes.get("href"),
        Some(&"https://example.com/base/guide.html".to_owned())
    );
    assert_eq!(
        rewritten_attributes.get("class"),
        Some(&"card featured".to_owned())
    );
    assert_eq!(
        rewritten_attributes.get("data-track"),
        Some(&"hero".to_owned())
    );
    assert!(attribute_supports_url_rewrite("href"));
    assert!(attribute_supports_url_rewrite("srcset"));
    assert!(attribute_supports_url_rewrite("ping"));
    assert!(!attribute_supports_url_rewrite("class"));
    assert!(first_fragment_attributes("plain text", None, false).is_empty());
    let non_refresh_meta = parse_document_node(
        "<meta http-equiv=\"content-security-policy\" content=\"0; url=next.html\">",
    );
    let meta = select_first(&non_refresh_meta, "meta").expect("meta");
    assert_eq!(
        element_attributes(&meta, Some("https://example.com/base/"), true).get("content"),
        Some(&"0; url=next.html".to_owned())
    );

    let mut detached_document = parse_document_node("<article><p>Hello</p></article>");
    let detached_id = {
        let detached = select_first(&detached_document, "p").expect("p");
        detached.id()
    };
    detached_document
        .tree
        .get_mut(detached_id)
        .expect("detached node")
        .detach();
    let detached = ElementRef::wrap(
        detached_document
            .tree
            .get(detached_id)
            .expect("detached ref"),
    )
    .expect("element ref");
    assert_eq!(build_node_path(&detached), "p:nth-of-type(1)");

    let rendered = render_html_as_text(
        "<article><p>Hello</p><ul><li>One</li></ul><hr><pre>  keep\n  spacing</pre></article>",
        WhitespaceMode::Preserve,
    );
    assert!(rendered.contains("Hello"));
    assert!(rendered.contains("- One"));
    assert!(rendered.contains("---"));
    assert!(rendered.contains("  keep"));
    let richer_rendered = render_html_as_text(
        "<blockquote><p>Quote</p></blockquote><dl><dt>Term</dt><dd>Definition</dd></dl><p>Use <code>cargo test</code></p>",
        WhitespaceMode::Preserve,
    );
    assert!(richer_rendered.contains("> Quote"));
    assert!(richer_rendered.contains("Term\n: Definition"));
    assert!(richer_rendered.contains("`cargo test`"));
    let collapsed_blockquote = render_html_as_text(
        "<blockquote><p>First</p><p></p><p></p><p>Second</p></blockquote>",
        WhitespaceMode::Preserve,
    );
    assert_eq!(collapsed_blockquote, "> First\n>\n> Second");
    let empty_blockquote =
        render_html_as_text("<blockquote>   </blockquote>", WhitespaceMode::Preserve);
    assert!(empty_blockquote.is_empty());

    assert_eq!(
        collapse_inline_whitespace("  Hello   world "),
        "Hello world"
    );
    assert!(needs_space("Hello", "world"));
    assert!(!needs_space("", "world"));
    assert!(!needs_space("Hello", ""));
    assert!(!needs_space("Hello", "."));
    assert!(!needs_space("Hello ", "world"));
    assert!(!needs_space("-", "world"));

    let mut output = String::from("Hello\n\n");
    push_newline(&mut output, 2);
    assert_eq!(output, "Hello\n\n");

    assert_eq!(
        apply_whitespace_mode(" Hello \n\n World ", WhitespaceMode::Normalize),
        "Hello\n\nWorld"
    );
    assert_eq!(
        apply_whitespace_mode("A\n\nB", WhitespaceMode::Normalize),
        "A\n\nB"
    );
    assert!(looks_like_full_document("<html><body></body></html>"));
    assert_eq!(
        rewrite_html_urls(
            "<a href=\"guide.html\">Guide</a>",
            Some("https://example.com/docs/"),
            false
        ),
        "<a href=\"https://example.com/docs/guide.html\">Guide</a>"
    );
    assert_eq!(resolve_url("#frag", Some("https://example.com")), "#frag");
    assert_eq!(
        resolve_url("https://openai.com", Some("https://example.com")),
        "https://openai.com"
    );
    assert_eq!(resolve_url("guide.html", None), "guide.html");
    assert_eq!(resolve_url("guide.html", Some("not a url")), "guide.html");
    assert_eq!(
        rewrite_html_urls("<p>Hello</p>", None, false),
        "<p>Hello</p>"
    );
    assert!(!looks_like_full_document("<body>Hello</body>"));
    assert!(first_body(&parse_wrapped_fragment("<p>Hello</p>")).is_some());
    assert!(first_body_child_element(&parse_wrapped_fragment("plain")).is_none());
    assert!(build_preview(&json!({"k": "v"}), 5).ends_with(ELLIPSIS));
    assert!(!has_errors(&[warning_diagnostic("WARN", "x", None)]));
    assert!(has_errors(&[error_diagnostic("ERR", "x", None)]));
    assert_eq!(
        warning_diagnostic("WARN", "x", None).level,
        DiagnosticLevel::Warning
    );

    let mut empty_output = String::new();
    let whitespace_fragment = parse_wrapped_fragment("   ");
    let whitespace_node = first_body(&whitespace_fragment)
        .expect("body")
        .first_child()
        .expect("text child");
    render_node(whitespace_node, &mut empty_output, false, false);
    assert!(empty_output.is_empty());

    let comment_fragment = parse_wrapped_fragment("<!-- keep nothing -->");
    let comment_node = first_body(&comment_fragment)
        .expect("body")
        .first_child()
        .expect("comment child");
    render_node(comment_node, &mut empty_output, false, false);
    assert!(empty_output.is_empty());

    let script_document = parse_wrapped_fragment("<script>alert(1)</script>");
    let script = select_first(&script_document, "script").expect("script");
    render_node(*script, &mut empty_output, false, false);
    assert!(empty_output.is_empty());

    let empty_code_document = parse_wrapped_fragment("<p>Use <code>   </code></p>");
    let empty_code = select_first(&empty_code_document, "code").expect("code");
    let mut empty_code_output = String::from("Use");
    render_node(*empty_code, &mut empty_code_output, false, false);
    assert_eq!(empty_code_output, "Use");

    let inline_code_document = parse_wrapped_fragment("<p>Use <code>cargo test</code></p>");
    let inline_code = select_first(&inline_code_document, "code").expect("code");
    let mut inline_code_output = String::from("Use");
    render_node(*inline_code, &mut inline_code_output, false, false);
    assert_eq!(inline_code_output, "Use `cargo test`");
    let mut inline_code_without_extra_space = String::from("Use ");
    render_node(
        *inline_code,
        &mut inline_code_without_extra_space,
        false,
        false,
    );
    assert_eq!(inline_code_without_extra_space, "Use `cargo test`");
    let pre_code_document = parse_wrapped_fragment("<pre><code>cargo test</code></pre>");
    let pre_code = select_first(&pre_code_document, "code").expect("code");
    let mut pre_code_output = String::new();
    render_node(*pre_code, &mut pre_code_output, true, false);
    assert_eq!(pre_code_output, "cargo test");

    let br_document = parse_wrapped_fragment("<br>");
    let br = select_first(&br_document, "br").expect("br");
    let mut line_output = String::from("Hello");
    render_node(*br, &mut line_output, false, false);
    assert_eq!(line_output, "Hello\n");

    let mut default_output = String::new();
    let default_document = parse_wrapped_fragment("<p>Hello</p>");
    let default_node = first_body(&default_document)
        .expect("body")
        .first_child()
        .expect("first child");
    render_node(default_node, &mut default_output, false, false);
    assert!(default_output.contains("Hello"));
    let list_item_document = parse_wrapped_fragment("<li><p>Hello</p></li>");
    let list_item = select_first(&list_item_document, "li").expect("list item");
    let mut list_item_output = String::new();
    render_node(*list_item, &mut list_item_output, false, false);
    assert!(list_item_output.contains("- Hello"));
    let pre_document = parse_wrapped_fragment("<pre>  keep   spacing</pre>");
    let pre = select_first(&pre_document, "pre").expect("pre");
    let mut pre_output = String::new();
    render_node(*pre, &mut pre_output, false, false);
    assert!(pre_output.contains("  keep   spacing"));
    let nested_pre_document = parse_wrapped_fragment("<pre><span>Hello</span></pre>");
    let span = select_first(&nested_pre_document, "span").expect("span");
    let mut nested_pre_output = String::new();
    render_node(*span, &mut nested_pre_output, true, false);
    assert_eq!(nested_pre_output, "Hello");
    let mut document_output = String::new();
    render_node(
        default_document.tree.root(),
        &mut document_output,
        false,
        false,
    );
    assert!(document_output.contains("Hello"));

    let rewritten_document = rewrite_html_urls(
        "<!DOCTYPE html><html><body><a href=\"guide.html\">Guide</a></body></html>",
        Some("https://example.com/docs/"),
        false,
    );
    assert!(rewritten_document.contains("https://example.com/docs/guide.html"));
    let forced_document = rewrite_html_urls(
        "<img src=\"asset.png\">",
        Some("https://example.com/docs/"),
        true,
    );
    assert!(forced_document.contains("https://example.com/docs/asset.png"));
    let rewritten_url_attributes = rewrite_html_urls(
        "<img srcset=\"small.png 1x, large.png 2x\"><form action=\"submit\"><button formaction=\"override\"></button></form><video poster=\"poster.png\"></video><a ping=\"/hit-one /hit-two\">Track</a><meta http-equiv=\"refresh\" content=\"0; url=next.html\">",
        Some("https://example.com/docs/"),
        false,
    );
    assert!(rewritten_url_attributes.contains(
        "srcset=\"https://example.com/docs/small.png 1x, https://example.com/docs/large.png 2x\""
    ));
    assert!(rewritten_url_attributes.contains("action=\"https://example.com/docs/submit\""));
    assert!(rewritten_url_attributes.contains("formaction=\"https://example.com/docs/override\""));
    assert!(rewritten_url_attributes.contains("poster=\"https://example.com/docs/poster.png\""));
    assert!(
        rewritten_url_attributes
            .contains("ping=\"https://example.com/hit-one https://example.com/hit-two\"")
    );
    assert!(
        rewritten_url_attributes.contains("content=\"0; url=https://example.com/docs/next.html\"")
    );
    let rewritten_single_srcset = rewrite_html_urls(
        "<img srcset=\"plain.png\">",
        Some("https://example.com/docs/"),
        false,
    );
    assert!(rewritten_single_srcset.contains("srcset=\"https://example.com/docs/plain.png\""));
    let unchanged_empty_srcset = rewrite_html_urls(
        "<img srcset=\" , \">",
        Some("https://example.com/docs/"),
        false,
    );
    assert!(unchanged_empty_srcset.contains("srcset=\" , \""));
    assert_eq!(collapse_blank_lines_for_tests("A\n\n\n\nB"), "A\n\nB");
    assert_eq!(
        rewrite_srcset_for_tests("plain.png, second.png", Some("https://example.com/docs/")),
        "https://example.com/docs/plain.png, https://example.com/docs/second.png"
    );
    assert_eq!(
        rewrite_srcset_for_tests(
            "data:image/svg+xml,<svg></svg> 1x, plain.png 2x",
            Some("https://example.com/docs/"),
        ),
        "data:image/svg+xml,<svg></svg> 1x, https://example.com/docs/plain.png 2x"
    );
    let rewritten_double_quoted_refresh = rewrite_html_urls(
        "<meta http-equiv=\"refresh\" content='0; url=\"next.html\"'>",
        Some("https://example.com/docs/"),
        false,
    );
    assert!(
        rewritten_double_quoted_refresh
            .contains("content=\"0; url=&quot;https://example.com/docs/next.html&quot;\"")
    );
    let rewritten_single_quoted_refresh = rewrite_html_urls(
        "<meta http-equiv=\"refresh\" content=\"0; url='other.html'\">",
        Some("https://example.com/docs/"),
        false,
    );
    assert!(
        rewritten_single_quoted_refresh
            .contains("content=\"0; url='https://example.com/docs/other.html'\"")
    );
    let non_meta_content = rewrite_html_urls(
        "<div content=\"0; url=next.html\"></div>",
        Some("https://example.com/docs/"),
        false,
    );
    assert!(non_meta_content.contains("content=\"0; url=next.html\""));
    let meta_without_refresh = rewrite_html_urls(
        "<meta content=\"0; url=next.html\">",
        Some("https://example.com/docs/"),
        false,
    );
    assert!(meta_without_refresh.contains("content=\"0; url=next.html\""));
    let mut mutable_document =
        parse_wrapped_fragment("<img src=\"asset.png\"><a href=\"guide.html\">Guide</a>");
    rewrite_urls_in_document(&mut mutable_document, "https://example.com/docs/");
    let rewritten_body = first_body(&mutable_document).expect("body");
    assert!(serialize_children(&rewritten_body).contains("https://example.com/docs/asset.png"));
    assert!(serialize_children(&rewritten_body).contains("https://example.com/docs/guide.html"));

    let source_meta = source_metadata(
        &LoadedSource {
            kind: SourceKind::Memory,
            value: "inline".to_owned(),
            bytes_read: 5,
            text: "Hello".to_owned(),
            input_base_url: None,
        },
        false,
        None,
    );
    assert!(source_meta.text.is_none());
}

#[test]
fn document_base_resolution_covers_absolute_relative_and_fallback_paths() {
    let absolute_document = parse_document_node(
        "<html><head><base href=\"https://cdn.example.com/shared/\"></head><body></body></html>",
    );
    assert_eq!(
        resolve_document_base_url(
            &absolute_document,
            Some("https://example.com/docs/start.html")
        )
        .as_deref(),
        Some("https://cdn.example.com/shared/")
    );

    let relative_document =
        parse_document_node("<html><head><base href=\"../shared/\"></head><body></body></html>");
    assert_eq!(
        resolve_document_base_url(
            &relative_document,
            Some("https://example.com/docs/start.html")
        )
        .as_deref(),
        Some("https://example.com/shared/")
    );

    assert_eq!(
        resolve_document_base_url(&relative_document, Some("not a url")).as_deref(),
        Some("not a url")
    );
}

#[test]
fn document_base_resolution_ignores_fragment_only_base_hrefs() {
    let document = parse_document_node(
        "<html><head><base href=\"#chapter-1\"></head><body><a href=\"guide.html\">Guide</a></body></html>",
    );

    assert_eq!(
        resolve_document_base_url(&document, Some("https://example.com/docs/start.html"))
            .as_deref(),
        Some("https://example.com/docs/start.html")
    );
}

#[test]
fn document_base_resolution_rejects_unsupported_absolute_schemes() {
    let document = parse_document_node(
        "<html><head><base href=\"mailto:owner@example.com\"></head><body></body></html>",
    );

    assert_eq!(
        resolve_document_base_url(&document, Some("https://example.com/docs/start.html"))
            .as_deref(),
        Some("https://example.com/docs/start.html")
    );
}

#[test]
fn selector_and_slice_runs_collect_builder_errors() {
    let selector_request = selector_request("<article data-id=\"7\">Hello</article>");
    let selector_loaded =
        load_source(&selector_request.source, &RuntimeOptions::default()).expect("loaded");
    let mut invalid_selector_request = selector_request.clone();
    invalid_selector_request.extraction = ExtractionSpec::selector(selector_query("["));
    let selector_run = run_selector_extraction(&invalid_selector_request, &selector_loaded);
    assert!(selector_run.matches.is_empty());
    assert_eq!(selector_run.diagnostics[0].code, "INVALID_SELECTOR");

    let slice_request = ExtractionRequest::new(
        memory_source_with_base(
            "inline",
            "<a href=\"/x\">Hello</a>",
            "https://example.com/base/",
        ),
        ExtractionSpec::slice(slice_spec("<a", "</a>").with_boundary_inclusion(true, true))
            .with_value(attribute_value("title")),
    );
    let selector_loaded =
        load_source(&slice_request.source, &RuntimeOptions::default()).expect("loaded");
    let slice_run = run_slice_extraction(&slice_request, &selector_loaded);
    assert!(slice_run.matches.is_empty());
    assert_eq!(slice_run.diagnostics[0].code, "MISSING_ATTRIBUTE");

    let selector_missing_attribute_request = ExtractionRequest::new(
        memory_source("inline", "<article data-id=\"7\">Hello</article>"),
        ExtractionSpec::selector(selector_query("article")).with_value(attribute_value("title")),
    );
    let selector_missing_attribute_loaded = load_source(
        &selector_missing_attribute_request.source,
        &RuntimeOptions::default(),
    )
    .expect("loaded");
    let selector_missing_attribute_run = run_selector_extraction(
        &selector_missing_attribute_request,
        &selector_missing_attribute_loaded,
    );
    assert!(selector_missing_attribute_run.matches.is_empty());
    assert_eq!(
        selector_missing_attribute_run.diagnostics[0].code,
        "MISSING_ATTRIBUTE"
    );
}

#[test]
fn source_helpers_cover_remaining_unreachable_and_locator_paths() {
    let wrong_url_kind = catch_unwind(AssertUnwindSafe(|| {
        let _ = read_url_source(&file_source("fixture.html"), &RuntimeOptions::default());
    }));
    assert!(wrong_url_kind.is_err());

    let wrong_file_kind = catch_unwind(AssertUnwindSafe(|| {
        let _ = read_file_source(
            &url_source("https://example.com"),
            &RuntimeOptions::default(),
        );
    }));
    assert!(wrong_file_kind.is_err());

    let file_metadata = empty_source_metadata(
        &file_source("fixtures/input.html")
            .with_base_url(Url::parse("https://example.com/base/").expect("base")),
    );
    assert_eq!(file_metadata.value, "fixtures/input.html");
    assert_eq!(
        file_metadata.input_base_url.as_deref(),
        Some("https://example.com/base/")
    );

    let stdin_metadata = empty_source_metadata(&SourceRequest::stdin());
    assert_eq!(stdin_metadata.value, "-");
    assert_eq!(stdin_metadata.kind, SourceKind::Stdin);

    let unnamed_memory_metadata =
        empty_source_metadata(&SourceRequest::memory("   ", "<article>Hello</article>"));
    assert_eq!(unnamed_memory_metadata.value, "memory");
}

#[cfg(unix)]
#[test]
fn read_file_source_reports_permission_denied_reads() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let unreadable_path = tempdir.path().join("unreadable.html");
    std::fs::write(&unreadable_path, "<article>Hello</article>").expect("write html");

    let mut permissions = std::fs::metadata(&unreadable_path)
        .expect("metadata")
        .permissions();
    permissions.set_mode(0o000);
    std::fs::set_permissions(&unreadable_path, permissions).expect("chmod 000");

    let error = read_file_source(&file_source(&unreadable_path), &RuntimeOptions::default())
        .expect_err("permission denied");
    assert_eq!(error.code, "SOURCE_LOAD_FAILED");
    assert!(error.message.contains("Could not read file"));
}
