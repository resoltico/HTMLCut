use super::*;

#[test]
fn request_definition_write_paths_cover_execution_failures_and_preview_success() {
    let tempdir = tempdir().expect("tempdir");
    let request = ExtractionRequest::new(
        SourceRequest::memory("inline", "<article>Hello</article>"),
        ExtractionSpec::selector(SelectorQuery::new("article").expect("selector")),
    );
    let definition = ExtractionDefinition::new(request.clone());

    assert_eq!(
        request_definition_parent_dir_for_tests(Path::new("request.json")),
        None
    );
    assert_eq!(
        request_definition_parent_dir_for_tests(Path::new("/")),
        None
    );
    assert_eq!(
        request_definition_parent_dir_for_tests(Path::new("saved/request.json")),
        Some(Path::new("saved"))
    );

    let request_dir = tempdir.path().join("request-dir");
    fs::create_dir_all(&request_dir).expect("request directory");
    let extraction_failure = execute_extraction(PreparedExtraction {
        command: "select".to_owned(),
        request: request.clone(),
        runtime: htmlcut_core::RuntimeOptions::default(),
        request_definition_output: Some(PendingExtractionDefinitionWrite {
            path: request_dir,
            definition: definition.clone(),
        }),
        output: CliOutputMode::Json,
        bundle: None,
        output_file: None,
        verbose: 0,
        quiet: false,
    });
    assert_eq!(extraction_failure.exit_code, EXIT_CODE_OUTPUT);
    assert!(
        extraction_failure
            .stdout
            .as_deref()
            .expect("json error payload")
            .contains("\"code\": \"CLI_REQUEST_FILE_WRITE_FAILED\"")
    );
    assert!(extraction_failure.stderr.is_empty());

    let bad_parent = tempdir.path().join("not a directory");
    fs::write(&bad_parent, "sentinel").expect("sentinel parent file");
    let preview_failure = execute_preview(PreparedPreview {
        command: "inspect-select".to_owned(),
        request: request.clone(),
        runtime: htmlcut_core::RuntimeOptions::default(),
        request_definition_output: Some(PendingExtractionDefinitionWrite {
            path: bad_parent.join("request.json"),
            definition: definition.clone(),
        }),
        output: CliInspectOutputMode::Json,
        output_file: None,
        verbose: 0,
        quiet: false,
    });
    assert_eq!(preview_failure.exit_code, EXIT_CODE_OUTPUT);
    assert!(
        preview_failure
            .stdout
            .as_deref()
            .expect("json error payload")
            .contains("\"code\": \"CLI_REQUEST_FILE_WRITE_FAILED\"")
    );
    assert!(preview_failure.stderr.is_empty());

    let root_path_failure = execute_preview(PreparedPreview {
        command: "inspect-select".to_owned(),
        request: request.clone(),
        runtime: htmlcut_core::RuntimeOptions::default(),
        request_definition_output: Some(PendingExtractionDefinitionWrite {
            path: PathBuf::from("/"),
            definition: definition.clone(),
        }),
        output: CliInspectOutputMode::Json,
        output_file: None,
        verbose: 0,
        quiet: false,
    });
    assert_eq!(root_path_failure.exit_code, EXIT_CODE_OUTPUT);
    assert!(
        root_path_failure
            .stdout
            .as_deref()
            .expect("json error payload")
            .contains("\"code\": \"CLI_REQUEST_FILE_WRITE_FAILED\"")
    );

    let preview_request_path = tempdir
        .path()
        .join("saved preview defs")
        .join("request [inspect].json");
    let preview_success = execute_preview(PreparedPreview {
        command: "inspect-select".to_owned(),
        request,
        runtime: htmlcut_core::RuntimeOptions::default(),
        request_definition_output: Some(PendingExtractionDefinitionWrite {
            path: preview_request_path.clone(),
            definition,
        }),
        output: CliInspectOutputMode::Text,
        output_file: None,
        verbose: 1,
        quiet: false,
    });
    assert_eq!(preview_success.exit_code, 0);
    assert!(preview_request_path.exists());
    assert!(
        preview_success
            .stderr
            .iter()
            .any(|line| line.contains("wrote request file"))
    );

    let preview_without_request_file = execute_preview(PreparedPreview {
        command: "inspect-select".to_owned(),
        request: ExtractionRequest::new(
            SourceRequest::memory("inline", "<article>Hello</article>"),
            ExtractionSpec::selector(SelectorQuery::new("article").expect("selector")),
        ),
        runtime: htmlcut_core::RuntimeOptions::default(),
        request_definition_output: None,
        output: CliInspectOutputMode::Text,
        output_file: None,
        verbose: 1,
        quiet: false,
    });
    assert_eq!(preview_without_request_file.exit_code, 0);
    assert!(
        preview_without_request_file
            .stderr
            .iter()
            .all(|line| !line.contains("wrote request file"))
    );
}

#[test]
fn catalog_and_schema_output_files_report_verbose_success() {
    let tempdir = tempdir().expect("tempdir");
    let catalog_output = tempdir.path().join("catalog report.json");
    let schema_output = tempdir.path().join("schema report.json");

    let (catalog_exit_code, catalog_stdout, catalog_stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "--verbose".to_owned(),
        "catalog".to_owned(),
        "--output".to_owned(),
        "json".to_owned(),
        "--output-file".to_owned(),
        catalog_output.to_string_lossy().into_owned(),
    ]);
    assert_eq!(catalog_exit_code, 0);
    assert!(catalog_stdout.is_empty());
    assert!(catalog_stderr.contains("wrote output file"));
    assert!(
        fs::read_to_string(&catalog_output)
            .expect("catalog output")
            .contains("\"command\": \"catalog\"")
    );

    let (schema_exit_code, schema_stdout, schema_stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "--verbose".to_owned(),
        "schema".to_owned(),
        "--name".to_owned(),
        "htmlcut.result".to_owned(),
        "--schema-version".to_owned(),
        "1".to_owned(),
        "--output".to_owned(),
        "json".to_owned(),
        "--output-file".to_owned(),
        schema_output.to_string_lossy().into_owned(),
    ]);
    assert_eq!(schema_exit_code, 0);
    assert!(schema_stdout.is_empty());
    assert!(schema_stderr.contains("wrote output file"));
    assert!(
        fs::read_to_string(&schema_output)
            .expect("schema output")
            .contains("\"schema_name\": \"htmlcut.schema_report\"")
    );
}

#[test]
fn human_error_outcome_renders_source_load_traces() {
    let error = with_source_load_steps(
        source_error("SOURCE_LOAD_FAILED", "Could not fetch source.", Vec::new()),
        &SourceMetadata {
            kind: SourceKind::Url,
            value: "https://example.com".to_owned(),
            input_base_url: Some("https://example.com".to_owned()),
            effective_base_url: Some("https://example.com".to_owned()),
            bytes_read: 0,
            load_steps: vec![
                SourceLoadStep {
                    action: SourceLoadAction::HeadPreflight,
                    outcome: SourceLoadOutcome::Fallback,
                    status: Some(405),
                    message: "HEAD returned 405, so HTMLCut fell back to GET.".to_owned(),
                },
                SourceLoadStep {
                    action: SourceLoadAction::Get,
                    outcome: SourceLoadOutcome::Failed,
                    status: Some(500),
                    message: "GET failed validation with status 500.".to_owned(),
                },
            ],
            text: None,
        },
    );
    let outcome = human_error_outcome(error);
    assert!(
        outcome
            .stderr
            .iter()
            .any(|line| line.contains("source load trace"))
    );
    assert!(
        outcome
            .stderr
            .iter()
            .any(|line| line.contains("head preflight fallback"))
    );
    assert!(
        outcome
            .stderr
            .iter()
            .any(|line| line.contains("get failed (500)"))
    );
}

#[test]
fn inspect_slice_text_warns_when_boundaries_split_markup() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture_file(
        tempdir.path(),
        "page.html",
        "<article class=\"card\">Hello <a href=\"/guide\">Guide</a></article>",
    );

    let (exit_code, stdout, stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "inspect".to_owned(),
        "slice".to_owned(),
        input_path.to_string_lossy().into_owned(),
        "--from".to_owned(),
        "<a".to_owned(),
        "--to".to_owned(),
        "</a>".to_owned(),
        "--output".to_owned(),
        "text".to_owned(),
    ]);
    assert_eq!(exit_code, 0);
    assert!(stderr.is_empty());
    assert!(stdout.contains("SLICE_SPLITS_MARKUP"));
    assert!(stdout.contains("fragment:"));
}
