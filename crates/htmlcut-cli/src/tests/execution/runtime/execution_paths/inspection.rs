use super::*;

#[test]
fn run_covers_inspection_text_failure_and_preview_modes() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture_file(
        tempdir.path(),
        "input.html",
        "<html><body><article><h1>Hello</h1><a href=\"/guide\">Guide</a></article></body></html>",
    );
    let input = input_path.to_string_lossy().into_owned();
    let missing = tempdir
        .path()
        .join("missing.html")
        .to_string_lossy()
        .into_owned();

    let (exit_code, stdout, stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "inspect".to_owned(),
        "source".to_owned(),
        input.clone(),
        "--output".to_owned(),
        "text".to_owned(),
    ]);
    assert_eq!(exit_code, 0);
    assert!(stdout.contains("Root tag: html"));
    assert!(stderr.is_empty());

    let (exit_code, stdout, stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "inspect".to_owned(),
        "source".to_owned(),
        input.clone(),
        "--base-url".to_owned(),
        "ftp://example.com".to_owned(),
    ]);
    assert_eq!(exit_code, EXIT_CODE_USAGE);
    assert!(stdout.contains("\"code\": \"CLI_BASE_URL_SCHEME_INVALID\""));
    assert!(stderr.is_empty());

    let (exit_code, stdout, stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "inspect".to_owned(),
        "source".to_owned(),
        missing.clone(),
        "--output".to_owned(),
        "json".to_owned(),
    ]);
    assert_eq!(exit_code, EXIT_CODE_SOURCE);
    assert!(stdout.contains("\"command\": \"inspect-source\""));
    assert!(stderr.is_empty());

    let (exit_code, _, stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "inspect".to_owned(),
        "source".to_owned(),
        missing,
        "--output".to_owned(),
        "text".to_owned(),
    ]);
    assert_eq!(exit_code, EXIT_CODE_SOURCE);
    assert!(stderr.contains("Could not access file"));

    let (exit_code, stdout, stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "inspect".to_owned(),
        "select".to_owned(),
        input.clone(),
        "--css".to_owned(),
        "article".to_owned(),
        "--match".to_owned(),
        "nth".to_owned(),
    ]);
    assert_eq!(exit_code, EXIT_CODE_USAGE);
    assert!(stdout.contains("\"code\": \"CLI_MATCH_INDEX_REQUIRED\""));
    assert!(stderr.is_empty());

    let (exit_code, stdout, stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "inspect".to_owned(),
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
    assert!(stdout.contains("\"code\": \"CLI_REGEX_FLAGS_CONFLICT\""));
    assert!(stderr.is_empty());

    let (exit_code, stdout, stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "inspect".to_owned(),
        "select".to_owned(),
        input,
        "--css".to_owned(),
        "article".to_owned(),
        "--output".to_owned(),
        "text".to_owned(),
    ]);
    assert_eq!(exit_code, 0);
    assert!(stdout.contains("Command: inspect-select"));
    assert!(stderr.is_empty());
}
