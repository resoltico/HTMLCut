use super::*;

#[test]
fn emit_request_file_round_trips_and_reports_verbose_success() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture_file(
        tempdir.path(),
        "page [draft].html",
        "<article><a href=\"/guide\">Guide</a></article>",
    );
    let emitted_request_path = tempdir
        .path()
        .join("saved defs")
        .join("request [weird].json");

    let (exit_code, stdout, stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "--verbose".to_owned(),
        "select".to_owned(),
        input_path.to_string_lossy().into_owned(),
        "--css".to_owned(),
        "a".to_owned(),
        "--value".to_owned(),
        "attribute".to_owned(),
        "--attribute".to_owned(),
        "href".to_owned(),
        "--output".to_owned(),
        "json".to_owned(),
        "--emit-request-file".to_owned(),
        emitted_request_path.to_string_lossy().into_owned(),
    ]);
    assert_eq!(exit_code, 0);
    assert!(stdout.contains("\"ok\": true"));
    assert!(stderr.contains("wrote request file"));

    let emitted_definition: ExtractionDefinition = serde_json::from_str(
        &fs::read_to_string(&emitted_request_path).expect("emitted request file"),
    )
    .expect("parse emitted definition");
    assert_eq!(
        emitted_definition.request.extraction.strategy(),
        ExtractionStrategy::Selector
    );
    assert_eq!(
        emitted_definition.request.extraction.value().value_type(),
        ValueType::Attribute
    );

    let (round_trip_exit_code, round_trip_stdout, round_trip_stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "select".to_owned(),
        "--request-file".to_owned(),
        emitted_request_path.to_string_lossy().into_owned(),
        "--output".to_owned(),
        "json".to_owned(),
    ]);
    assert_eq!(round_trip_exit_code, 0);
    assert!(round_trip_stdout.contains("\"ok\": true"));
    assert!(round_trip_stderr.is_empty());
}
