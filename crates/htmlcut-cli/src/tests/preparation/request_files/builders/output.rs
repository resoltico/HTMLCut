use super::*;

#[test]
fn request_file_output_mode_rejects_none_when_writing_stdout_payloads() {
    let fixture = request_file_fixture();

    assert_eq!(
        resolve_extract_output_mode_with_output_file(
            Some(CliOutputMode::None),
            &ValueType::Text,
            Some(fixture.tempdir.path()),
            Some(&fixture.tempdir.path().join("selection.txt")),
        )
        .expect_err("output file requires stdout payload")
        .code,
        "CLI_OUTPUT_FILE_REQUIRES_STDOUT_PAYLOAD"
    );
}

#[test]
fn request_file_output_helpers_create_parent_dirs_and_preserve_stderr_ordering() {
    let fixture = request_file_fixture();
    let nested_output = fixture.tempdir.path().join("nested/output/selection.txt");
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let exit_code = write_outcome(
        ExecutionOutcome {
            stdout: Some("Hello".to_owned()),
            output_file: Some(nested_output.clone()),
            post_write_stderr: Vec::new(),
            stderr: Vec::new(),
            exit_code: 0,
        },
        &mut stdout,
        &mut stderr,
    )
    .expect("write outcome");
    assert_eq!(exit_code, 0);
    assert!(stdout.is_empty());
    assert!(stderr.is_empty());
    assert_eq!(
        fs::read_to_string(&nested_output).expect("nested output file"),
        "Hello
"
    );

    let ordered_output = fixture.tempdir.path().join("ordered/output/report.txt");
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let exit_code = write_outcome(
        ExecutionOutcome {
            stdout: Some("Hello".to_owned()),
            output_file: Some(ordered_output.clone()),
            post_write_stderr: vec![
                "htmlcut: wrote output file to ordered/output/report.txt".to_owned(),
            ],
            stderr: vec![
                "htmlcut: request normalized".to_owned(),
                "htmlcut: preview complete".to_owned(),
            ],
            exit_code: 0,
        },
        &mut stdout,
        &mut stderr,
    )
    .expect("write outcome");
    assert_eq!(exit_code, 0);
    assert_eq!(
        String::from_utf8(stderr).expect("stderr"),
        "htmlcut: request normalized
htmlcut: preview complete
htmlcut: wrote output file to ordered/output/report.txt
"
    );
    assert_eq!(
        fs::read_to_string(&ordered_output).expect("ordered output file"),
        "Hello
"
    );
}

#[test]
fn request_file_output_helpers_cover_direct_and_failing_writes() {
    let fixture = request_file_fixture();
    let direct_nested_output = fixture.tempdir.path().join("direct/output/report.txt");
    write_stdout_payload_for_tests(&direct_nested_output, "Hello")
        .expect("write stdout payload with nested parent");
    assert_eq!(
        fs::read_to_string(&direct_nested_output).expect("direct nested output file"),
        "Hello
"
    );

    let relative_output =
        PathBuf::from(format!(".htmlcut-write-payload-{}.txt", std::process::id()));
    write_stdout_payload_for_tests(&relative_output, "Hello")
        .expect("write stdout payload without parent directory");
    assert_eq!(
        fs::read_to_string(&relative_output).expect("relative output file"),
        "Hello
"
    );
    fs::remove_file(&relative_output).expect("remove relative output file");
    assert!(
        write_stdout_payload_for_tests(Path::new(""), "Hello")
            .expect_err("empty path write should fail")
            .kind()
            != std::io::ErrorKind::AlreadyExists
    );

    let directory_output = fixture.tempdir.path().join("directory-output");
    fs::create_dir(&directory_output).expect("create directory output placeholder");
    assert!(
        write_stdout_payload_for_tests(&directory_output, "Hello")
            .expect_err("directory write should fail")
            .kind()
            != std::io::ErrorKind::NotFound
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let exit_code = write_outcome(
        ExecutionOutcome {
            stdout: Some("Hello".to_owned()),
            output_file: Some(fixture.tempdir.path().to_path_buf()),
            post_write_stderr: Vec::new(),
            stderr: Vec::new(),
            exit_code: 0,
        },
        &mut stdout,
        &mut stderr,
    )
    .expect("write outcome");
    assert_eq!(exit_code, EXIT_CODE_OUTPUT);
    assert!(stdout.is_empty());
    assert!(
        String::from_utf8(stderr)
            .expect("stderr")
            .contains("Could not write")
    );
}

#[test]
fn request_file_output_helpers_propagate_stderr_failures_when_output_reporting_fails() {
    let fixture = request_file_fixture();
    let error = write_outcome(
        ExecutionOutcome {
            stdout: Some("Hello".to_owned()),
            output_file: Some(fixture.tempdir.path().to_path_buf()),
            post_write_stderr: Vec::new(),
            stderr: Vec::new(),
            exit_code: 0,
        },
        &mut Vec::new(),
        &mut BrokenPipeWriter,
    )
    .expect_err("stderr write should fail");

    assert_eq!(error.kind(), std::io::ErrorKind::BrokenPipe);
}
