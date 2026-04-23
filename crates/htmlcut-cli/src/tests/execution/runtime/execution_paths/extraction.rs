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
        "u".to_owned(),
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
