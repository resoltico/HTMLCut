use super::helpers::*;

#[test]
fn source_inspection_commands_stay_in_lockstep_with_core() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture(
        tempdir.path(),
        "inspect source matrix.html",
        "<html><head><title>Inspect Matrix</title><base href=\"../content/\"></head><body><main><h1>Heading</h1><h2>Details</h2><a href=\"guide.html\">Guide</a><a href=\"/docs\">Docs</a><p>Alpha Beta Gamma</p></main></body></html>",
    );

    let case = SourceInspectionParityCase {
        name: "inspect source with effective base and source text",
        args: vec![
            "inspect".to_owned(),
            "source".to_owned(),
            input_path.to_string_lossy().into_owned(),
            "--base-url".to_owned(),
            "https://example.com/docs/start.html".to_owned(),
            "--sample-limit".to_owned(),
            "3".to_owned(),
            "--include-source-text".to_owned(),
        ],
        command: "inspect-source",
        source: source_request(&input_path, Some("https://example.com/docs/start.html")),
        runtime: runtime_options(),
        sample_limit: 3,
        include_source_text: true,
    };

    assert_source_inspection_parity(&case);
}
