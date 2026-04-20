use super::*;

#[test]
fn request_file_recovery_hints_cover_preview_and_slice_variants() {
    let tempdir = tempdir().expect("tempdir");
    let invalid_json_path = write_fixture_file(tempdir.path(), "invalid.json", "{not json");

    let inspect_select_error = expect_cli_error(
        load_extraction_definition_for_tests(
            &invalid_json_path,
            ExtractionStrategy::Selector,
            "inspect select",
        ),
        "inspect select invalid request file json",
    );
    assert_eq!(inspect_select_error.code, "CLI_REQUEST_FILE_INVALID");
    assert!(
        inspect_select_error
            .message
            .contains("htmlcut catalog --operation select.preview --output json")
    );

    let inspect_slice_error = expect_cli_error(
        load_extraction_definition_for_tests(
            &invalid_json_path,
            ExtractionStrategy::Slice,
            "inspect slice",
        ),
        "inspect slice invalid request file json",
    );
    assert_eq!(inspect_slice_error.code, "CLI_REQUEST_FILE_INVALID");
    assert!(
        inspect_slice_error
            .message
            .contains("htmlcut catalog --operation slice.preview --output json")
    );
    assert!(
        inspect_slice_error.message.contains(
            "Slice request files use plain JSON strings for `request.extraction.from` and `request.extraction.to`."
        )
    );

    let fallback_selector_error = expect_cli_error(
        load_extraction_definition_for_tests(
            &invalid_json_path,
            ExtractionStrategy::Selector,
            "custom selector command",
        ),
        "fallback selector operation id",
    );
    assert!(
        fallback_selector_error
            .message
            .contains("htmlcut catalog --operation select.extract --output json")
    );

    let fallback_slice_error = expect_cli_error(
        load_extraction_definition_for_tests(
            &invalid_json_path,
            ExtractionStrategy::Slice,
            "custom slice command",
        ),
        "fallback slice operation id",
    );
    assert!(
        fallback_slice_error
            .message
            .contains("htmlcut catalog --operation slice.extract --output json")
    );

    let invalid_slice_shape_path = write_fixture_file(
        tempdir.path(),
        "invalid-slice-shape.json",
        r#"{
  "schema_name": "htmlcut.extraction_definition",
  "schema_version": 1,
  "request": {
    "source": { "input": { "type": "stdin" } },
    "extraction": {
      "kind": "slice",
      "from": { "literal": "<article>" },
      "to": "</article>"
    }
  }
}"#,
    );
    let invalid_slice_shape_error = expect_cli_error(
        load_extraction_definition_for_tests(
            &invalid_slice_shape_path,
            ExtractionStrategy::Slice,
            "inspect slice",
        ),
        "invalid slice request file shape",
    );
    assert_eq!(invalid_slice_shape_error.code, "CLI_REQUEST_FILE_INVALID");
    assert!(
        invalid_slice_shape_error
            .message
            .contains("request.extraction.from` as a plain JSON string, not an object")
    );

    let invalid_selector_array_path = write_fixture_file(
        tempdir.path(),
        "invalid-selector-array.json",
        r#"{
  "schema_name": "htmlcut.extraction_definition",
  "schema_version": 1,
  "request": {
    "source": { "input": { "type": "stdin" } },
    "extraction": {
      "kind": "selector",
      "selector": ["article"]
    }
  }
}"#,
    );
    let invalid_selector_array_error = expect_cli_error(
        load_extraction_definition_for_tests(
            &invalid_selector_array_path,
            ExtractionStrategy::Selector,
            "inspect select",
        ),
        "invalid selector array shape",
    );
    assert!(
        invalid_selector_array_error
            .message
            .contains("request.extraction.selector` as a plain JSON string, not an object")
    );

    let invalid_slice_array_path = write_fixture_file(
        tempdir.path(),
        "invalid-slice-array.json",
        r#"{
  "schema_name": "htmlcut.extraction_definition",
  "schema_version": 1,
  "request": {
    "source": { "input": { "type": "stdin" } },
    "extraction": {
      "kind": "slice",
      "from": ["<article>"],
      "to": "</article>"
    }
  }
}"#,
    );
    let invalid_slice_array_error = expect_cli_error(
        load_extraction_definition_for_tests(
            &invalid_slice_array_path,
            ExtractionStrategy::Slice,
            "inspect slice",
        ),
        "invalid slice array shape",
    );
    assert!(
        invalid_slice_array_error
            .message
            .contains("request.extraction.from` as a plain JSON string, not an object")
    );
}

#[test]
fn json_error_path_formatter_covers_root_and_dot_prefixed_shapes() {
    assert_eq!(format_json_error_path_for_tests(""), "$");
    assert_eq!(format_json_error_path_for_tests("$"), "$");
    assert_eq!(
        format_json_error_path_for_tests(".request.extraction.selector"),
        "$.request.extraction.selector"
    );
    assert_eq!(
        format_json_error_path_for_tests("$.request.extraction.selector"),
        "$.request.extraction.selector"
    );
    assert_eq!(
        format_json_error_path_for_tests("request.extraction.selector"),
        "$.request.extraction.selector"
    );
}
