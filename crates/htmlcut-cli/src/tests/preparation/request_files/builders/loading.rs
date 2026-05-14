use super::*;

#[test]
fn request_file_loading_reports_read_shape_and_schema_failures() {
    let fixture = request_file_fixture();

    assert_eq!(
        expect_cli_error(
            load_extraction_definition_for_tests(
                &fixture.tempdir.path().join("missing-request.json"),
                ExtractionStrategy::Selector,
                "select",
            ),
            "missing request file",
        )
        .code,
        "CLI_REQUEST_FILE_READ_FAILED"
    );

    let invalid_json_path =
        write_fixture_file(fixture.tempdir.path(), "invalid-request.json", "{not json");
    let invalid_json_error = expect_cli_error(
        load_extraction_definition_for_tests(
            &invalid_json_path,
            ExtractionStrategy::Selector,
            "select",
        ),
        "invalid request file json",
    );
    assert_eq!(invalid_json_error.code, "CLI_REQUEST_FILE_INVALID");
    assert!(
        invalid_json_error
            .message
            .contains("htmlcut schema --name htmlcut.extraction_definition --output json")
    );
    assert!(
        invalid_json_error
            .message
            .contains("htmlcut catalog --operation select.extract --output json")
    );

    let invalid_shape_path = write_fixture_file(
        fixture.tempdir.path(),
        "invalid-shape.json",
        r#"{
  "schema_name": "htmlcut.extraction_definition",
  "schema_version": 4,
  "request": {
    "spec_version": 7,
    "source": { "input": { "type": "stdin" } },
    "extraction": {
      "kind": "selector",
      "selector": { "css": "article" }
    }
  }
}"#,
    );
    let invalid_shape_error = expect_cli_error(
        load_extraction_definition_for_tests(
            &invalid_shape_path,
            ExtractionStrategy::Selector,
            "select",
        ),
        "invalid request file shape",
    );
    assert_eq!(invalid_shape_error.code, "CLI_REQUEST_FILE_INVALID");
    assert!(invalid_shape_error.message.contains("JSON path $"));
    assert!(invalid_shape_error.message.contains("selector"));
    assert!(
        invalid_shape_error
            .message
            .contains("request.extraction.selector` as a plain JSON string")
    );

    let selector_definition = ExtractionDefinition::new(ExtractionRequest::new(
        SourceRequest::file(&fixture.input_path),
        ExtractionSpec::selector(SelectorQuery::new("article").expect("selector"))
            .with_selection(SelectionSpec::single())
            .with_value(ValueSpec::Text),
    ));
    let mut unsupported_schema =
        serde_json::to_value(&selector_definition).expect("definition json");
    unsupported_schema["schema_name"] = Value::String("synthetic.request".to_owned());
    unsupported_schema["schema_version"] = Value::from(99);
    let unsupported_schema_path = fixture.tempdir.path().join("unsupported-schema.json");
    fs::write(
        &unsupported_schema_path,
        serde_json::to_string_pretty(&unsupported_schema).expect("serialize unsupported schema"),
    )
    .expect("write unsupported schema");
    assert_eq!(
        expect_cli_error(
            load_extraction_definition_for_tests(
                &unsupported_schema_path,
                ExtractionStrategy::Selector,
                "select",
            ),
            "unsupported schema",
        )
        .code,
        "CLI_REQUEST_FILE_SCHEMA_UNSUPPORTED"
    );

    let mut unsupported_version =
        serde_json::to_value(&selector_definition).expect("definition json");
    unsupported_version["schema_version"] = Value::from(99);
    let unsupported_version_path = fixture.tempdir.path().join("unsupported-version.json");
    fs::write(
        &unsupported_version_path,
        serde_json::to_string_pretty(&unsupported_version).expect("serialize unsupported version"),
    )
    .expect("write unsupported version");
    assert_eq!(
        expect_cli_error(
            load_extraction_definition_for_tests(
                &unsupported_version_path,
                ExtractionStrategy::Selector,
                "select",
            ),
            "unsupported version",
        )
        .code,
        "CLI_REQUEST_FILE_SCHEMA_UNSUPPORTED"
    );
}

#[test]
fn request_file_loading_reports_strategy_mismatches() {
    let fixture = request_file_fixture();

    assert_eq!(
        expect_cli_error(
            load_extraction_definition_for_tests(
                &fixture.selector_definition_path,
                ExtractionStrategy::Slice,
                "slice",
            ),
            "strategy mismatch",
        )
        .code,
        "CLI_REQUEST_FILE_STRATEGY_MISMATCH"
    );
    assert_eq!(
        expect_cli_error(
            load_extraction_definition_for_tests(
                &fixture.slice_definition_path,
                ExtractionStrategy::Selector,
                "select",
            ),
            "slice strategy mismatch",
        )
        .code,
        "CLI_REQUEST_FILE_STRATEGY_MISMATCH"
    );
}

#[test]
fn request_file_loading_rejects_schema_less_and_legacy_slice_definitions() {
    let fixture = request_file_fixture();

    let schema_less_path = write_fixture_file(
        fixture.tempdir.path(),
        "schema-less.json",
        r#"{
  "request": {
    "spec_version": 7,
    "source": { "input": { "type": "stdin" } },
    "extraction": {
      "kind": "selector",
      "selector": "article"
    }
  }
}"#,
    );
    let schema_less_error = expect_cli_error(
        load_extraction_definition_for_tests(
            &schema_less_path,
            ExtractionStrategy::Selector,
            "select",
        ),
        "schema-less request file",
    );
    assert_eq!(schema_less_error.code, "CLI_REQUEST_FILE_INVALID");
    assert!(
        schema_less_error
            .message
            .contains("missing field `schema_name`")
    );

    let legacy_slice_path = write_fixture_file(
        fixture.tempdir.path(),
        "legacy-slice.json",
        r#"{
  "schema_name": "htmlcut.extraction_definition",
  "schema_version": 4,
  "request": {
    "spec_version": 7,
    "source": { "input": { "type": "stdin" } },
    "extraction": {
      "kind": "slice",
      "pattern": {
        "mode": "literal",
        "from": "<article>",
        "to": "</article>"
      },
      "include_start": true,
      "include_end": false
    }
  }
}"#,
    );
    let legacy_slice_error = expect_cli_error(
        load_extraction_definition_for_tests(
            &legacy_slice_path,
            ExtractionStrategy::Slice,
            "slice",
        ),
        "legacy slice request file",
    );
    assert_eq!(legacy_slice_error.code, "CLI_REQUEST_FILE_INVALID");
    assert!(
        legacy_slice_error.message.contains("include_start")
            || legacy_slice_error.message.contains("include_end")
    );
}
