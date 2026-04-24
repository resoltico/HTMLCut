use super::*;

#[test]
fn clap_error_message_prefers_the_primary_error_line() {
    let error =
        Cli::try_parse_from(["htmlcut", "select", "page.html"]).expect_err("parse error expected");
    assert!(clap_error_message(&error).contains("required arguments"));

    let help = Cli::try_parse_from(["htmlcut", "--help"]).expect_err("help expected");
    assert!(clap_error_message(&help).contains("Usage: htmlcut [OPTIONS] <COMMAND>"));
}

#[test]
fn global_verbose_parses_before_or_after_subcommand() {
    let before = Cli::try_parse_from(["htmlcut", "-vv", "select", "page.html", "--css", "article"])
        .expect("parse");
    assert_eq!(before.global.verbose, 2);

    let after = Cli::try_parse_from(["htmlcut", "select", "-vv", "page.html", "--css", "article"])
        .expect("parse");
    assert_eq!(after.global.verbose, 2);
}

#[test]
fn cargo_manifest_drives_the_public_metadata_constants() {
    let workspace_manifest = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("crate dir")
            .parent()
            .expect("workspace root")
            .join("Cargo.toml"),
    )
    .expect("workspace manifest");
    let workspace_version =
        workspace_package_field(&workspace_manifest, "version").expect("workspace version");
    let workspace_description =
        workspace_package_field(&workspace_manifest, "description").expect("workspace description");

    assert_eq!(HTMLCUT_VERSION, workspace_version);
    assert_eq!(HTMLCUT_DESCRIPTION, workspace_description);
}

#[test]
fn run_covers_root_help_help_version_and_parse_error_modes() {
    let (exit_code, stdout, stderr) = run_vec(vec!["htmlcut".to_owned()]);
    assert_eq!(exit_code, 0);
    assert!(stdout.starts_with(&format!(
        "{DISPLAY_NAME} {HTMLCUT_VERSION}\n{HTMLCUT_DESCRIPTION}\n"
    )));
    assert!(
        stdout
            .find("Usage: htmlcut [OPTIONS] <COMMAND>")
            .expect("usage")
            < stdout.find("Start here:").expect("flow")
    );
    assert!(stdout.contains("Usage: htmlcut [OPTIONS] <COMMAND>"));
    assert!(stderr.is_empty());

    let (exit_code, stdout, _) = run_vec(vec!["htmlcut".to_owned(), "--help".to_owned()]);
    assert_eq!(exit_code, 0);
    assert!(stdout.starts_with(&format!(
        "{DISPLAY_NAME} {HTMLCUT_VERSION}\n{HTMLCUT_DESCRIPTION}\n"
    )));
    assert!(
        stdout
            .find("Usage: htmlcut [OPTIONS] <COMMAND>")
            .expect("usage")
            < stdout.find("Start here:").expect("flow")
    );
    assert!(stdout.contains("inspect"));

    let (exit_code, stdout, stderr) = run_vec(vec!["htmlcut".to_owned(), "-h".to_owned()]);
    assert_eq!(exit_code, 0);
    assert_eq!(stdout.matches(HTMLCUT_DESCRIPTION).count(), 1);
    assert!(!stdout.contains("Start here:"));
    assert!(stderr.is_empty());

    let (exit_code, stdout, stderr) = run_vec(vec!["htmlcut".to_owned(), "help".to_owned()]);
    assert_eq!(exit_code, 0);
    assert!(stdout.starts_with(&format!(
        "{DISPLAY_NAME} {HTMLCUT_VERSION}\n{HTMLCUT_DESCRIPTION}\n"
    )));
    assert!(stdout.contains("Usage: htmlcut [OPTIONS] <COMMAND>"));
    assert!(stderr.is_empty());

    let (exit_code, stdout, stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "--version".to_owned(),
        "--help".to_owned(),
    ]);
    assert_eq!(exit_code, 0);
    assert!(stdout.starts_with(&format!(
        "{DISPLAY_NAME} {HTMLCUT_VERSION}\n{HTMLCUT_DESCRIPTION}\n"
    )));
    assert!(stdout.contains("Usage: htmlcut [OPTIONS] <COMMAND>"));
    assert!(stderr.is_empty());

    let (exit_code, stdout, stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "--help".to_owned(),
        "--version".to_owned(),
    ]);
    assert_eq!(exit_code, 0);
    assert!(stdout.starts_with(&format!(
        "{DISPLAY_NAME} {HTMLCUT_VERSION}\n{HTMLCUT_DESCRIPTION}\n"
    )));
    assert!(stdout.contains("Usage: htmlcut [OPTIONS] <COMMAND>"));
    assert!(stderr.is_empty());

    let (exit_code, stdout, _) = run_vec(vec!["htmlcut".to_owned(), "--version".to_owned()]);
    assert_eq!(exit_code, 0);
    assert_eq!(stdout, format!("{}\n", version_banner()));

    let (exit_code, stdout, stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "select".to_owned(),
        "--version".to_owned(),
    ]);
    assert_eq!(exit_code, EXIT_CODE_USAGE);
    assert!(stdout.is_empty());
    assert!(stderr.contains("unexpected argument '--version'"));

    let (exit_code, stdout, stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "inspect".to_owned(),
        "source".to_owned(),
        "-V".to_owned(),
    ]);
    assert_eq!(exit_code, EXIT_CODE_USAGE);
    assert!(stdout.contains("\"category\": \"usage\""));
    assert!(stdout.contains("\"command\": \"inspect-source\""));
    assert!(stdout.contains("unexpected argument '-V'"));
    assert!(stderr.is_empty());

    let (exit_code, stdout, stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "select".to_owned(),
        "--version".to_owned(),
        "--help".to_owned(),
    ]);
    assert_eq!(exit_code, EXIT_CODE_USAGE);
    assert!(stdout.is_empty());
    assert!(stderr.contains("unexpected argument '--version'"));

    let (exit_code, _, stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "select".to_owned(),
        "page.html".to_owned(),
        "--bogus".to_owned(),
    ]);
    assert_eq!(exit_code, EXIT_CODE_USAGE);
    assert!(stderr.contains("unexpected argument '--bogus'"));

    let (exit_code, stdout, stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "select".to_owned(),
        "page.html".to_owned(),
        "--output".to_owned(),
        "json".to_owned(),
        "--bogus".to_owned(),
    ]);
    assert_eq!(exit_code, EXIT_CODE_USAGE);
    assert!(stdout.contains("\"category\": \"usage\""));
    assert!(stdout.contains("\"command\": \"select\""));
    assert!(stderr.is_empty());
}

#[test]
fn run_propagates_stdout_write_failures_for_help_and_command_output() {
    let mut stderr = Vec::new();
    let help_error = run(
        vec!["htmlcut".to_owned()],
        &mut BrokenPipeWriter,
        &mut stderr,
    )
    .expect_err("help write should fail");
    assert_eq!(help_error.kind(), std::io::ErrorKind::BrokenPipe);
    assert!(stderr.is_empty());

    let command_error = run(
        vec!["htmlcut".to_owned(), "catalog".to_owned()],
        &mut BrokenPipeWriter,
        &mut Vec::new(),
    )
    .expect_err("catalog write should fail");
    assert_eq!(command_error.kind(), std::io::ErrorKind::BrokenPipe);
}

#[test]
fn run_propagates_stderr_write_failures_for_usage_errors() {
    let error = run(
        vec![
            "htmlcut".to_owned(),
            "select".to_owned(),
            "page.html".to_owned(),
            "--bogus".to_owned(),
        ],
        &mut Vec::new(),
        &mut BrokenPipeWriter,
    )
    .expect_err("stderr write should fail");

    assert_eq!(error.kind(), std::io::ErrorKind::BrokenPipe);
}
