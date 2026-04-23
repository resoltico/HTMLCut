use std::io::{Read, Write};
use std::net::TcpListener;
use std::num::NonZeroUsize;
use std::thread;
use std::time::{Duration, Instant};

use htmlcut_core::{
    AttributeName, ExtractionRequest, ExtractionSpec, NormalizationOptions, OutputOptions,
    RuntimeOptions, SelectionSpec, SelectorQuery, SliceBoundary, SliceSpec, SourceRequest,
    ValueSpec, WhitespaceMode, extract, format_byte_size, parse_document, preview_extraction,
};
use url::Url;

fn output_options(preview_chars: usize) -> OutputOptions {
    OutputOptions {
        preview_chars: NonZeroUsize::new(preview_chars).expect("preview chars"),
        ..OutputOptions::default()
    }
}

fn selector_request(html: &str, selector: &str) -> ExtractionRequest {
    let mut request = ExtractionRequest::new(
        SourceRequest::memory("inline", html),
        ExtractionSpec::selector(SelectorQuery::new(selector).expect("selector")),
    );
    request.normalization = NormalizationOptions {
        whitespace: WhitespaceMode::Normalize,
        rewrite_urls: false,
    };
    request.output = output_options(80);
    request
}

fn slice_request(html: &str, from: &str, to: &str) -> ExtractionRequest {
    ExtractionRequest::new(
        SourceRequest::memory("inline", html),
        ExtractionSpec::slice(SliceSpec::new(
            SliceBoundary::new(from).expect("slice boundary"),
            SliceBoundary::new(to).expect("slice boundary"),
        )),
    )
}

#[test]
fn parse_document_reads_memory_source() {
    let source = SourceRequest::memory("inline", "<article><p>Hello</p></article>");

    let parsed = parse_document(&source, &RuntimeOptions::default());
    assert!(parsed.ok);
    assert!(parsed.document.is_some());
}

#[test]
fn selector_extraction_returns_normalized_text() {
    let request = selector_request(
        "<article><p>Hello <strong>world</strong></p><p>Again</p></article>",
        "article",
    );

    let result = extract(&request, &RuntimeOptions::default());
    assert!(result.ok);
    assert_eq!(result.stats.match_count, 1);
    assert_eq!(
        result.matches[0].value.as_str(),
        Some("Hello world\n\nAgain")
    );
    assert!(
        result.matches[0]
            .path
            .as_deref()
            .is_some_and(|path| path.contains("article:nth-of-type(1)"))
    );
}

#[test]
fn selector_attribute_extraction_rewrites_relative_urls() {
    let mut request = selector_request(
        "<article><a href=\"../guide.html\">Guide</a></article>",
        "article a",
    );
    request.source = request
        .source
        .clone()
        .with_base_url(Url::parse("https://example.com/docs/start.html").expect("base url"));
    request.normalization.rewrite_urls = true;
    request.extraction = request.extraction.clone().with_value(ValueSpec::Attribute {
        name: AttributeName::new("href").expect("attribute name"),
    });

    let result = extract(&request, &RuntimeOptions::default());
    assert!(result.ok);
    assert_eq!(
        result.matches[0].value.as_str(),
        Some("https://example.com/guide.html")
    );
}

#[test]
fn selector_attribute_extraction_honors_document_base_href() {
    let mut request = selector_request(
        "<html><head><base href=\"https://fixture.example/base/\"></head><body><article><a href=\"guide.html\">Guide</a></article></body></html>",
        "article a",
    );
    request.normalization.rewrite_urls = true;
    request.extraction = request.extraction.clone().with_value(ValueSpec::Attribute {
        name: AttributeName::new("href").expect("attribute name"),
    });

    let result = extract(&request, &RuntimeOptions::default());
    assert!(result.ok);
    assert_eq!(result.source.input_base_url.as_deref(), None);
    assert_eq!(
        result.source.effective_base_url.as_deref(),
        Some("https://fixture.example/base/")
    );
    assert_eq!(
        result.matches[0].value.as_str(),
        Some("https://fixture.example/base/guide.html")
    );
}

#[test]
fn slice_attribute_extraction_rewrites_relative_urls_with_input_base() {
    let mut request = slice_request("<a href=\"guide.html\">Guide</a>", "<a ", "</a>");
    request.source = request
        .source
        .clone()
        .with_base_url(Url::parse("https://example.com/docs/start.html").expect("base url"));
    request.extraction = request.extraction.clone().with_value(ValueSpec::Attribute {
        name: AttributeName::new("href").expect("attribute name"),
    });
    request.extraction = ExtractionSpec::slice(SliceSpec {
        include_start: true,
        include_end: true,
        ..request.extraction.slice_spec().expect("slice spec").clone()
    })
    .with_selection(request.extraction.selection().clone())
    .with_value(request.extraction.value().clone());
    request.normalization = NormalizationOptions {
        whitespace: WhitespaceMode::Preserve,
        rewrite_urls: true,
    };

    let result = extract(&request, &RuntimeOptions::default());
    assert!(result.ok);
    assert!(result.diagnostics.is_empty());
    assert_eq!(
        result.source.effective_base_url.as_deref(),
        Some("https://example.com/docs/start.html")
    );
    assert_eq!(
        result.matches[0].value.as_str(),
        Some("https://example.com/docs/guide.html")
    );
}

#[test]
fn selector_reports_invalid_selector() {
    let request = selector_request("<div>Hello</div>", "[");
    let result = extract(&request, &RuntimeOptions::default());

    assert!(!result.ok);
    assert_eq!(result.diagnostics[0].code, "INVALID_SELECTOR");
}

#[test]
fn selector_reports_multiple_matches_as_warning_for_first() {
    let request = selector_request("<p>One</p><p>Two</p>", "p");
    let result = preview_extraction(&request, &RuntimeOptions::default());

    assert!(result.ok);
    assert_eq!(result.stats.candidate_count, 2);
    assert_eq!(result.diagnostics[0].code, "MULTIPLE_MATCHES");
}

#[test]
fn selector_single_reports_ambiguity_as_a_hard_error() {
    let mut request = selector_request("<p>One</p><p>Two</p>", "p");
    request.extraction = request
        .extraction
        .clone()
        .with_selection(SelectionSpec::single());

    let result = extract(&request, &RuntimeOptions::default());
    assert!(!result.ok);
    assert_eq!(result.diagnostics[0].code, "AMBIGUOUS_MATCH");
}

#[test]
fn selector_reports_nth_out_of_range() {
    let mut request = selector_request("<p>One</p>", "p");
    request.extraction = request
        .extraction
        .clone()
        .with_selection(SelectionSpec::nth(
            NonZeroUsize::new(3).expect("match index"),
        ));

    let result = extract(&request, &RuntimeOptions::default());
    assert!(!result.ok);
    assert_eq!(result.diagnostics[0].code, "MATCH_INDEX_OUT_OF_RANGE");
}

#[test]
fn selector_structured_mode_returns_metadata() {
    let mut request = selector_request("<section data-id=\"x\"><p>Hello</p></section>", "section");
    request.extraction = request.extraction.clone().with_value(ValueSpec::Structured);

    let result = extract(&request, &RuntimeOptions::default());
    assert!(result.ok);
    assert_eq!(result.matches[0].value["tagName"], "section");
    assert_eq!(result.matches[0].value["attributes"]["data-id"], "x");
    assert_eq!(result.matches[0].metadata.candidate_index(), 1);
}

#[test]
fn slice_literal_outer_html_returns_all_matches() {
    let mut request = slice_request("<p>One</p><p>Two</p>", "<p>", "</p>");
    request.extraction = ExtractionSpec::slice(SliceSpec {
        include_start: true,
        include_end: true,
        ..request.extraction.slice_spec().expect("slice spec").clone()
    })
    .with_selection(SelectionSpec::All)
    .with_value(ValueSpec::OuterHtml);
    request.normalization = NormalizationOptions {
        whitespace: WhitespaceMode::Preserve,
        rewrite_urls: false,
    };

    let result = extract(&request, &RuntimeOptions::default());
    assert!(result.ok);
    assert_eq!(result.stats.match_count, 2);
    assert_eq!(result.matches[0].value.as_str(), Some("<p>One</p>"));
    assert_eq!(result.matches[1].value.as_str(), Some("<p>Two</p>"));
}

#[test]
fn slice_boundary_inclusion_supports_start_only_and_end_only() {
    let mut request = slice_request("<p>One</p>", "<p>", "</p>");
    request.extraction = ExtractionSpec::slice(
        request
            .extraction
            .slice_spec()
            .expect("slice spec")
            .clone()
            .with_boundary_inclusion(true, false),
    )
    .with_value(ValueSpec::InnerHtml);

    let start_only = extract(&request, &RuntimeOptions::default());
    assert!(start_only.ok);
    assert_eq!(start_only.matches[0].value.as_str(), Some("<p>One"));

    request.extraction = ExtractionSpec::slice(
        request
            .extraction
            .slice_spec()
            .expect("slice spec")
            .clone()
            .with_boundary_inclusion(false, true),
    )
    .with_value(ValueSpec::InnerHtml);

    let end_only = extract(&request, &RuntimeOptions::default());
    assert!(end_only.ok);
    assert_eq!(end_only.matches[0].value.as_str(), Some("One</p>"));
}

#[test]
fn slice_regex_supports_flags() {
    let mut request = slice_request("<DIV>Caps</DIV>", "<div>", "</div>");
    let mut slice = request.extraction.slice_spec().expect("slice spec").clone();
    slice.pattern = htmlcut_core::SlicePatternSpec::regex(
        SliceBoundary::new("<div>").expect("slice boundary"),
        SliceBoundary::new("</div>").expect("slice boundary"),
        "i",
    );
    request.extraction = ExtractionSpec::slice(slice);

    let result = extract(&request, &RuntimeOptions::default());
    assert!(result.ok);
    assert_eq!(result.matches[0].value.as_str(), Some("Caps"));
}

#[test]
fn slice_reports_invalid_regex_flags() {
    let mut request = slice_request("<p>One</p>", "<p>", "</p>");
    let mut slice = request.extraction.slice_spec().expect("slice spec").clone();
    slice.pattern = htmlcut_core::SlicePatternSpec::regex(
        SliceBoundary::new("<p>").expect("slice boundary"),
        SliceBoundary::new("</p>").expect("slice boundary"),
        "q",
    );
    request.extraction = ExtractionSpec::slice(slice);

    let result = extract(&request, &RuntimeOptions::default());
    assert!(!result.ok);
    assert_eq!(result.diagnostics[0].code, "INVALID_SLICE_PATTERN");
}

#[test]
fn source_file_loading_returns_absolute_path() {
    let tempdir = htmlcut_tempdir::tempdir().expect("tempdir");
    let input_path = tempdir.path().join("input.html");
    std::fs::write(&input_path, "<p>Hello</p>").expect("write fixture");

    let mut request = selector_request("", "p");
    request.source = SourceRequest::file(&input_path);

    let result = extract(&request, &RuntimeOptions::default());
    assert!(result.ok);
    assert!(result.source.value.ends_with("input.html"));
}

#[test]
fn source_url_loading_works() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind server");
    listener
        .set_nonblocking(true)
        .expect("make listener nonblocking");
    let address = listener.local_addr().expect("server addr");
    let url = format!("http://{address}");

    let handle = thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(2);
        let mut methods = Vec::new();

        while Instant::now() < deadline {
            match listener.accept() {
                Ok((mut stream, _)) => {
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
                    methods.push(method.to_owned());

                    let body = "<article>Hello</article>";
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

                    if method == "GET" {
                        return methods;
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(10));
                }
                Err(error) => panic!("accept connection: {error}"),
            }
        }

        panic!("timed out waiting for HEAD/GET sequence, saw methods: {methods:?}");
    });

    let mut request = selector_request("", "article");
    request.source = SourceRequest::url(Url::parse(&url).expect("url"));

    let result = extract(&request, &RuntimeOptions::default());
    let methods = handle.join().expect("server join");
    assert!(result.ok);
    assert_eq!(methods, vec!["HEAD".to_owned(), "GET".to_owned()]);
    let expected_url = format!("{url}/");
    assert_eq!(
        result.source.input_base_url.as_deref(),
        Some(expected_url.as_str())
    );
    assert_eq!(
        result.source.effective_base_url.as_deref(),
        Some(expected_url.as_str())
    );
}

#[test]
fn source_size_limits_are_enforced() {
    let mut request = selector_request("", "p");
    request.source = SourceRequest::memory("memory", "<p>abcdefghij</p>");

    let runtime = RuntimeOptions {
        max_bytes: 5,
        fetch_timeout_ms: 1000,
        ..RuntimeOptions::default()
    };

    let result = extract(&request, &runtime);
    assert!(!result.ok);
    assert_eq!(result.diagnostics[0].code, "SOURCE_LOAD_FAILED");
}

#[test]
fn format_byte_size_has_friendly_units() {
    assert_eq!(format_byte_size(5), "5 bytes");
    assert_eq!(format_byte_size(1024), "1 KB");
}
