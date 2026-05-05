use super::*;

#[test]
fn run_covers_extraction_error_json_and_bundle_failure_modes() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture_file(
        tempdir.path(),
        "input.html",
        "<article><p>Hello</p></article>",
    );
    let input = input_path.to_string_lossy().into_owned();
    let bundle_path = tempdir.path().join("not-a-dir");
    fs::write(&bundle_path, "file").expect("bundle sentinel");

    let (exit_code, stdout, stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "select".to_owned(),
        input.clone(),
        "--css".to_owned(),
        "[".to_owned(),
        "--output".to_owned(),
        "json".to_owned(),
    ]);
    assert_eq!(exit_code, EXIT_CODE_USAGE);
    assert!(stdout.contains("\"code\": \"INVALID_SELECTOR\""));
    assert!(stdout.contains("Invalid selector"));
    assert!(stderr.is_empty());

    let (exit_code, _, stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "slice".to_owned(),
        input.clone(),
        "--from".to_owned(),
        "<article>".to_owned(),
        "--to".to_owned(),
        "</article>".to_owned(),
        "--regex-flags".to_owned(),
        "i".to_owned(),
    ]);
    assert_eq!(exit_code, EXIT_CODE_USAGE);
    assert!(stderr.contains("--regex-flags can only be used with --pattern regex."));

    let (exit_code, stdout, stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "select".to_owned(),
        input.clone(),
        "--css".to_owned(),
        "article".to_owned(),
        "--output".to_owned(),
        "json".to_owned(),
        "--bundle".to_owned(),
        bundle_path.to_string_lossy().into_owned(),
    ]);
    assert_eq!(exit_code, EXIT_CODE_OUTPUT);
    assert!(stdout.contains("\"category\": \"output\""));
    assert!(stderr.is_empty());

    let bundle_dir = tempdir.path().join("bundle-write-fails");
    fs::create_dir(&bundle_dir).expect("bundle dir");
    fs::create_dir(bundle_dir.join("selection.html")).expect("blocking html dir");
    let (exit_code, stdout, stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "select".to_owned(),
        input.clone(),
        "--css".to_owned(),
        "article".to_owned(),
        "--output".to_owned(),
        "json".to_owned(),
        "--bundle".to_owned(),
        bundle_dir.to_string_lossy().into_owned(),
        "--overwrite".to_owned(),
    ]);
    assert_eq!(exit_code, EXIT_CODE_OUTPUT);
    assert!(stdout.contains("\"code\": \"CLI_BUNDLE_HTML_WRITE_FAILED\""));
    assert!(stderr.is_empty());

    let (exit_code, stdout, _) = run_vec(vec![
        "htmlcut".to_owned(),
        "inspect".to_owned(),
        "select".to_owned(),
        input.clone(),
        "--css".to_owned(),
        "[".to_owned(),
    ]);
    assert_eq!(exit_code, EXIT_CODE_USAGE);
    assert!(stdout.contains("\"command\": \"inspect-select\""));

    let (exit_code, _, stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "inspect".to_owned(),
        "slice".to_owned(),
        input,
        "--from".to_owned(),
        "[".to_owned(),
        "--to".to_owned(),
        "]".to_owned(),
        "--pattern".to_owned(),
        "regex".to_owned(),
        "--output".to_owned(),
        "text".to_owned(),
    ]);
    assert_eq!(exit_code, EXIT_CODE_USAGE);
    assert!(stderr.contains("Invalid regular expression"));
}

#[test]
fn run_refuses_existing_output_targets_until_overwrite_is_explicit() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture_file(
        tempdir.path(),
        "input.html",
        "<article><a class=\"more\" href=\"/guide\">Guide</a></article>",
    );
    let input = input_path.to_string_lossy().into_owned();

    let output_file = tempdir.path().join("selection.json");
    fs::write(&output_file, "existing output").expect("existing output file");
    let (exit_code, stdout, stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "select".to_owned(),
        input.clone(),
        "--css".to_owned(),
        "article".to_owned(),
        "--output".to_owned(),
        "json".to_owned(),
        "--output-file".to_owned(),
        output_file.to_string_lossy().into_owned(),
    ]);
    assert_eq!(exit_code, EXIT_CODE_OUTPUT);
    assert!(stdout.contains("\"code\": \"CLI_OUTPUT_FILE_EXISTS\""));
    assert!(stderr.is_empty());
    assert_eq!(
        fs::read_to_string(&output_file).expect("existing output file contents"),
        "existing output"
    );

    let request_file = tempdir.path().join("request.json");
    fs::write(&request_file, "existing request").expect("existing request file");
    let (exit_code, stdout, stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "select".to_owned(),
        input.clone(),
        "--css".to_owned(),
        "article".to_owned(),
        "--output".to_owned(),
        "json".to_owned(),
        "--emit-request-file".to_owned(),
        request_file.to_string_lossy().into_owned(),
    ]);
    assert_eq!(exit_code, EXIT_CODE_OUTPUT);
    assert!(stdout.contains("\"code\": \"CLI_REQUEST_FILE_EXISTS\""));
    assert!(stderr.is_empty());
    assert_eq!(
        fs::read_to_string(&request_file).expect("existing request file contents"),
        "existing request"
    );

    let bundle_dir = tempdir.path().join("bundle");
    fs::create_dir(&bundle_dir).expect("existing bundle dir");
    let (exit_code, stdout, stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "select".to_owned(),
        input.clone(),
        "--css".to_owned(),
        "article".to_owned(),
        "--output".to_owned(),
        "json".to_owned(),
        "--bundle".to_owned(),
        bundle_dir.to_string_lossy().into_owned(),
    ]);
    assert_eq!(exit_code, EXIT_CODE_OUTPUT);
    assert!(stdout.contains("\"code\": \"CLI_BUNDLE_PATH_EXISTS\""));
    assert!(stderr.is_empty());
}

#[test]
fn run_overwrite_replaces_existing_outputs_when_requested() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture_file(
        tempdir.path(),
        "input.html",
        "<article><a class=\"more\" href=\"/guide\">Guide</a></article>",
    );
    let input = input_path.to_string_lossy().into_owned();

    let output_file = tempdir.path().join("selection.json");
    fs::write(&output_file, "stale output").expect("stale output");
    let (exit_code, stdout, stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "select".to_owned(),
        input.clone(),
        "--css".to_owned(),
        "article".to_owned(),
        "--output".to_owned(),
        "json".to_owned(),
        "--output-file".to_owned(),
        output_file.to_string_lossy().into_owned(),
        "--overwrite".to_owned(),
    ]);
    assert_eq!(exit_code, 0);
    assert!(stdout.is_empty());
    assert!(stderr.is_empty());
    assert!(
        fs::read_to_string(&output_file)
            .expect("overwritten output file")
            .contains("\"ok\": true")
    );

    let request_file = tempdir.path().join("request.json");
    fs::write(&request_file, "stale request").expect("stale request");
    let (exit_code, stdout, stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "select".to_owned(),
        input.clone(),
        "--css".to_owned(),
        "article".to_owned(),
        "--output".to_owned(),
        "json".to_owned(),
        "--emit-request-file".to_owned(),
        request_file.to_string_lossy().into_owned(),
        "--overwrite".to_owned(),
    ]);
    assert_eq!(exit_code, 0);
    assert!(stdout.contains("\"ok\": true"));
    assert!(stderr.is_empty());
    assert!(
        fs::read_to_string(&request_file)
            .expect("overwritten request file")
            .contains("\"schema_name\": \"htmlcut.extraction_definition\"")
    );

    let bundle_dir = tempdir.path().join("bundle");
    fs::create_dir(&bundle_dir).expect("bundle dir");
    fs::write(bundle_dir.join("selection.txt"), "stale bundle text").expect("stale bundle text");
    let (exit_code, stdout, stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "select".to_owned(),
        input,
        "--css".to_owned(),
        "article".to_owned(),
        "--output".to_owned(),
        "json".to_owned(),
        "--bundle".to_owned(),
        bundle_dir.to_string_lossy().into_owned(),
        "--overwrite".to_owned(),
    ]);
    assert_eq!(exit_code, 0);
    assert!(stdout.contains("\"ok\": true"));
    assert!(stderr.is_empty());
    assert!(
        fs::read_to_string(bundle_dir.join("selection.txt"))
            .expect("overwritten bundle text")
            .contains("Guide")
    );
    assert!(bundle_dir.join("report.json").is_file());
    assert!(bundle_dir.join("selection.html").is_file());
}

#[test]
fn run_renders_human_text_for_html_valued_extractions_and_bundles() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture_file(
        tempdir.path(),
        "input.html",
        "<article><h2>Heading</h2><ul><li><a href=\"/guide\">Guide</a></li></ul></article>",
    );
    let input = input_path.to_string_lossy().into_owned();

    let output_file = tempdir.path().join("selection.txt");
    let (exit_code, stdout, stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "select".to_owned(),
        input.clone(),
        "--css".to_owned(),
        "article".to_owned(),
        "--value".to_owned(),
        "outer-html".to_owned(),
        "--output".to_owned(),
        "text".to_owned(),
        "--base-url".to_owned(),
        "https://example.com/page".to_owned(),
        "--rewrite-urls".to_owned(),
        "--output-file".to_owned(),
        output_file.to_string_lossy().into_owned(),
    ]);
    assert_eq!(exit_code, 0);
    assert!(stdout.is_empty());
    assert!(stderr.is_empty());
    assert_eq!(
        fs::read_to_string(&output_file).expect("text output file"),
        "## Heading\n- Guide [https://example.com/guide]\n"
    );

    let bundle_dir = tempdir.path().join("bundle");
    let (exit_code, stdout, stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "slice".to_owned(),
        input,
        "--from".to_owned(),
        "<article>".to_owned(),
        "--to".to_owned(),
        "</article>".to_owned(),
        "--include-start".to_owned(),
        "--include-end".to_owned(),
        "--value".to_owned(),
        "outer-html".to_owned(),
        "--output".to_owned(),
        "none".to_owned(),
        "--base-url".to_owned(),
        "https://example.com/page".to_owned(),
        "--rewrite-urls".to_owned(),
        "--bundle".to_owned(),
        bundle_dir.to_string_lossy().into_owned(),
    ]);
    assert_eq!(exit_code, 0);
    assert!(stdout.is_empty());
    assert!(stderr.is_empty());
    assert_eq!(
        fs::read_to_string(bundle_dir.join("selection.txt")).expect("bundle text"),
        "## Heading\n- Guide [https://example.com/guide]"
    );
    assert!(
        fs::read_to_string(bundle_dir.join("selection.html"))
            .expect("bundle html")
            .contains("<article><h2>Heading</h2><ul><li><a href=\"https://example.com/guide\">Guide</a></li></ul></article>")
    );
}
