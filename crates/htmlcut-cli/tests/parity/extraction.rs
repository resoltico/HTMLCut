use super::helpers::*;

#[test]
fn extraction_and_preview_commands_stay_in_lockstep_with_core() {
    let tempdir = tempdir().expect("tempdir");
    let card_path = write_fixture(
        tempdir.path(),
        "matrix cards.html",
        "<html><head><title>Matrix Cards</title></head><body><article class=\"card primary\"><p>Alpha   Beta</p></article><article class=\"card secondary\"><p>Gamma</p></article><div>START::First::END</div><div>START::Second::END</div></body></html>",
    );
    let base_path = write_fixture(
        tempdir.path(),
        "matrix base.html",
        "<html><head><title>Matrix Base</title><base href=\"../content/\"></head><body><article><a class=\"cta\" href=\"guide/start.html\">Guide</a></article></body></html>",
    );

    let cases = vec![
        ExtractionParityCase {
            name: "select nth normalized text",
            args: vec![
                "select".to_owned(),
                card_path.to_string_lossy().into_owned(),
                "--css".to_owned(),
                "article.card p".to_owned(),
                "--match".to_owned(),
                "nth".to_owned(),
                "--index".to_owned(),
                "1".to_owned(),
                "--whitespace".to_owned(),
                "normalize".to_owned(),
                "--output".to_owned(),
                "json".to_owned(),
            ],
            command: "select",
            request: {
                let mut request = ExtractionRequest::new(
                    source_request(&card_path, None),
                    selector_extraction("article.card p")
                        .with_selection(SelectionSpec::nth(
                            NonZeroUsize::new(1).expect("match index"),
                        ))
                        .with_value(ValueSpec::Text),
                );
                request.normalization = NormalizationOptions {
                    whitespace: WhitespaceMode::Normalize,
                    rewrite_urls: false,
                };
                request.output = extraction_output(false, DEFAULT_PREVIEW_CHARS);
                request
            },
            runtime: runtime_options(),
            execution: ExtractionExecution::Extract,
        },
        ExtractionParityCase {
            name: "select attribute rewrite honors effective document base",
            args: vec![
                "select".to_owned(),
                base_path.to_string_lossy().into_owned(),
                "--css".to_owned(),
                "article a.cta".to_owned(),
                "--value".to_owned(),
                "attribute".to_owned(),
                "--attribute".to_owned(),
                "href".to_owned(),
                "--rewrite-urls".to_owned(),
                "--base-url".to_owned(),
                "https://example.com/docs/start.html".to_owned(),
                "--output".to_owned(),
                "json".to_owned(),
            ],
            command: "select",
            request: {
                let mut request = ExtractionRequest::new(
                    source_request(&base_path, Some("https://example.com/docs/start.html")),
                    selector_extraction("article a.cta")
                        .with_selection(SelectionSpec::First)
                        .with_value(ValueSpec::Attribute {
                            name: AttributeName::new("href").expect("attribute name"),
                        }),
                );
                request.normalization = NormalizationOptions {
                    whitespace: WhitespaceMode::Preserve,
                    rewrite_urls: true,
                };
                request.output = extraction_output(false, DEFAULT_PREVIEW_CHARS);
                request
            },
            runtime: runtime_options(),
            execution: ExtractionExecution::Extract,
        },
        ExtractionParityCase {
            name: "select single exact one keeps core parity",
            args: vec![
                "select".to_owned(),
                base_path.to_string_lossy().into_owned(),
                "--css".to_owned(),
                "article".to_owned(),
                "--match".to_owned(),
                "single".to_owned(),
                "--output".to_owned(),
                "json".to_owned(),
            ],
            command: "select",
            request: {
                let mut request = ExtractionRequest::new(
                    source_request(&base_path, None),
                    selector_extraction("article")
                        .with_selection(SelectionSpec::single())
                        .with_value(ValueSpec::Text),
                );
                request.normalization = NormalizationOptions {
                    whitespace: WhitespaceMode::Preserve,
                    rewrite_urls: false,
                };
                request.output = extraction_output(false, DEFAULT_PREVIEW_CHARS);
                request
            },
            runtime: runtime_options(),
            execution: ExtractionExecution::Extract,
        },
        ExtractionParityCase {
            name: "slice regex outer all",
            args: vec![
                "slice".to_owned(),
                card_path.to_string_lossy().into_owned(),
                "--from".to_owned(),
                "START::[A-Za-z]+".to_owned(),
                "--to".to_owned(),
                "::END".to_owned(),
                "--pattern".to_owned(),
                "regex".to_owned(),
                "--include-start".to_owned(),
                "--include-end".to_owned(),
                "--match".to_owned(),
                "all".to_owned(),
                "--output".to_owned(),
                "json".to_owned(),
            ],
            command: "slice",
            request: {
                let mut request = ExtractionRequest::new(
                    source_request(&card_path, None),
                    slice_extraction("START::[A-Za-z]+", "::END", PatternMode::Regex, true, true)
                        .with_selection(SelectionSpec::All)
                        .with_value(ValueSpec::Text),
                );
                request.normalization = NormalizationOptions {
                    whitespace: WhitespaceMode::Preserve,
                    rewrite_urls: false,
                };
                request.output = extraction_output(false, DEFAULT_PREVIEW_CHARS);
                request
            },
            runtime: runtime_options(),
            execution: ExtractionExecution::Extract,
        },
        ExtractionParityCase {
            name: "slice literal start-inclusive parity",
            args: vec![
                "slice".to_owned(),
                card_path.to_string_lossy().into_owned(),
                "--from".to_owned(),
                "START::".to_owned(),
                "--to".to_owned(),
                "::END".to_owned(),
                "--include-start".to_owned(),
                "--match".to_owned(),
                "nth".to_owned(),
                "--index".to_owned(),
                "2".to_owned(),
                "--output".to_owned(),
                "json".to_owned(),
            ],
            command: "slice",
            request: {
                let mut request = ExtractionRequest::new(
                    source_request(&card_path, None),
                    slice_extraction("START::", "::END", PatternMode::Literal, true, false)
                        .with_selection(SelectionSpec::nth(
                            NonZeroUsize::new(2).expect("match index"),
                        ))
                        .with_value(ValueSpec::Text),
                );
                request.normalization = NormalizationOptions {
                    whitespace: WhitespaceMode::Preserve,
                    rewrite_urls: false,
                };
                request.output = extraction_output(false, DEFAULT_PREVIEW_CHARS);
                request
            },
            runtime: runtime_options(),
            execution: ExtractionExecution::Extract,
        },
        ExtractionParityCase {
            name: "inspect select preview keeps core metadata",
            args: vec![
                "inspect".to_owned(),
                "select".to_owned(),
                base_path.to_string_lossy().into_owned(),
                "--css".to_owned(),
                "article a.cta".to_owned(),
                "--match".to_owned(),
                "all".to_owned(),
                "--rewrite-urls".to_owned(),
                "--base-url".to_owned(),
                "https://example.com/docs/start.html".to_owned(),
                "--preview-chars".to_owned(),
                "48".to_owned(),
                "--include-source-text".to_owned(),
            ],
            command: "inspect-select",
            request: {
                let mut request = ExtractionRequest::new(
                    source_request(&base_path, Some("https://example.com/docs/start.html")),
                    selector_extraction("article a.cta")
                        .with_selection(SelectionSpec::All)
                        .with_value(ValueSpec::Structured),
                );
                request.normalization = NormalizationOptions {
                    whitespace: WhitespaceMode::Preserve,
                    rewrite_urls: true,
                };
                request.output = extraction_output(true, 48);
                request
            },
            runtime: runtime_options(),
            execution: ExtractionExecution::Preview,
        },
        ExtractionParityCase {
            name: "inspect slice preview keeps start-only inclusion parity",
            args: vec![
                "inspect".to_owned(),
                "slice".to_owned(),
                card_path.to_string_lossy().into_owned(),
                "--from".to_owned(),
                "START::".to_owned(),
                "--to".to_owned(),
                "::END".to_owned(),
                "--include-start".to_owned(),
                "--match".to_owned(),
                "nth".to_owned(),
                "--index".to_owned(),
                "1".to_owned(),
                "--preview-chars".to_owned(),
                "48".to_owned(),
                "--include-source-text".to_owned(),
            ],
            command: "inspect-slice",
            request: {
                let mut request = ExtractionRequest::new(
                    source_request(&card_path, None),
                    slice_extraction("START::", "::END", PatternMode::Literal, true, false)
                        .with_selection(SelectionSpec::nth(
                            NonZeroUsize::new(1).expect("match index"),
                        ))
                        .with_value(ValueSpec::Structured),
                );
                request.normalization = NormalizationOptions {
                    whitespace: WhitespaceMode::Preserve,
                    rewrite_urls: false,
                };
                request.output = extraction_output(true, 48);
                request
            },
            runtime: runtime_options(),
            execution: ExtractionExecution::Preview,
        },
        ExtractionParityCase {
            name: "inspect slice preview keeps core metadata",
            args: vec![
                "inspect".to_owned(),
                "slice".to_owned(),
                card_path.to_string_lossy().into_owned(),
                "--from".to_owned(),
                "START::[A-Za-z]+".to_owned(),
                "--to".to_owned(),
                "::END".to_owned(),
                "--pattern".to_owned(),
                "regex".to_owned(),
                "--include-start".to_owned(),
                "--include-end".to_owned(),
                "--match".to_owned(),
                "all".to_owned(),
                "--preview-chars".to_owned(),
                "48".to_owned(),
                "--include-source-text".to_owned(),
            ],
            command: "inspect-slice",
            request: {
                let mut request = ExtractionRequest::new(
                    source_request(&card_path, None),
                    slice_extraction("START::[A-Za-z]+", "::END", PatternMode::Regex, true, true)
                        .with_selection(SelectionSpec::All)
                        .with_value(ValueSpec::Structured),
                );
                request.normalization = NormalizationOptions {
                    whitespace: WhitespaceMode::Preserve,
                    rewrite_urls: false,
                };
                request.output = extraction_output(true, 48);
                request
            },
            runtime: runtime_options(),
            execution: ExtractionExecution::Preview,
        },
    ];

    for case in &cases {
        assert_extraction_parity(case);
    }
}
