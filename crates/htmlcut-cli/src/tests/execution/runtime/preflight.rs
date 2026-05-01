use super::*;

fn parsed_command(args: &[&str]) -> (u8, bool, Commands) {
    let cli = Cli::try_parse_from(args).expect("parse cli");
    (cli.global.verbose, cli.global.quiet, cli.command)
}

#[test]
fn command_preflights_reject_existing_paths_before_preparation_runs() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture_file(
        tempdir.path(),
        "input.html",
        "<article><p>Hello</p></article>",
    );
    let input = input_path.to_string_lossy().into_owned();

    let existing_catalog = tempdir.path().join("catalog.txt");
    fs::write(&existing_catalog, "catalog").expect("catalog output");
    let (exit_code, stdout, stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "catalog".to_owned(),
        "--output-file".to_owned(),
        existing_catalog.to_string_lossy().into_owned(),
    ]);
    assert_eq!(exit_code, EXIT_CODE_OUTPUT);
    assert!(stdout.is_empty());
    assert!(stderr.contains("Refusing to overwrite existing output file"));

    let existing_schema = tempdir.path().join("schema.json");
    fs::write(&existing_schema, "schema").expect("schema output");
    let (exit_code, stdout, stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "schema".to_owned(),
        "--output".to_owned(),
        "json".to_owned(),
        "--output-file".to_owned(),
        existing_schema.to_string_lossy().into_owned(),
    ]);
    assert_eq!(exit_code, EXIT_CODE_OUTPUT);
    assert!(stdout.contains("\"code\": \"CLI_OUTPUT_FILE_EXISTS\""));
    assert!(stderr.is_empty());

    let existing_request = tempdir.path().join("request.json");
    fs::write(&existing_request, "request").expect("request output");
    let (exit_code, stdout, stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "slice".to_owned(),
        input.clone(),
        "--from".to_owned(),
        "<article>".to_owned(),
        "--to".to_owned(),
        "</article>".to_owned(),
        "--output".to_owned(),
        "json".to_owned(),
        "--emit-request-file".to_owned(),
        existing_request.to_string_lossy().into_owned(),
    ]);
    assert_eq!(exit_code, EXIT_CODE_OUTPUT);
    assert!(stdout.contains("\"code\": \"CLI_REQUEST_FILE_EXISTS\""));
    assert!(stderr.is_empty());

    let existing_slice_output = tempdir.path().join("slice.txt");
    fs::write(&existing_slice_output, "slice").expect("slice output");
    let (exit_code, stdout, stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "slice".to_owned(),
        input.clone(),
        "--from".to_owned(),
        "<article>".to_owned(),
        "--to".to_owned(),
        "</article>".to_owned(),
        "--output-file".to_owned(),
        existing_slice_output.to_string_lossy().into_owned(),
    ]);
    assert_eq!(exit_code, EXIT_CODE_OUTPUT);
    assert!(stdout.is_empty());
    assert!(stderr.contains("Refusing to overwrite existing output file"));

    let existing_bundle = tempdir.path().join("bundle");
    fs::create_dir(&existing_bundle).expect("bundle dir");
    let (exit_code, stdout, stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "slice".to_owned(),
        input.clone(),
        "--from".to_owned(),
        "<article>".to_owned(),
        "--to".to_owned(),
        "</article>".to_owned(),
        "--output".to_owned(),
        "json".to_owned(),
        "--bundle".to_owned(),
        existing_bundle.to_string_lossy().into_owned(),
    ]);
    assert_eq!(exit_code, EXIT_CODE_OUTPUT);
    assert!(stdout.contains("\"code\": \"CLI_BUNDLE_PATH_EXISTS\""));
    assert!(stderr.is_empty());

    let existing_inspect_source = tempdir.path().join("inspect-source.txt");
    fs::write(&existing_inspect_source, "source").expect("inspect source output");
    let (exit_code, stdout, stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "inspect".to_owned(),
        "source".to_owned(),
        input.clone(),
        "--output".to_owned(),
        "text".to_owned(),
        "--output-file".to_owned(),
        existing_inspect_source.to_string_lossy().into_owned(),
    ]);
    assert_eq!(exit_code, EXIT_CODE_OUTPUT);
    assert!(stdout.is_empty());
    assert!(stderr.contains("Refusing to overwrite existing output file"));

    let existing_inspect_select_request = tempdir.path().join("inspect-select-request.json");
    fs::write(&existing_inspect_select_request, "request").expect("inspect select request");
    let (exit_code, stdout, stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "inspect".to_owned(),
        "select".to_owned(),
        input.clone(),
        "--css".to_owned(),
        "article".to_owned(),
        "--emit-request-file".to_owned(),
        existing_inspect_select_request
            .to_string_lossy()
            .into_owned(),
    ]);
    assert_eq!(exit_code, EXIT_CODE_OUTPUT);
    assert!(stdout.contains("\"code\": \"CLI_REQUEST_FILE_EXISTS\""));
    assert!(stderr.is_empty());

    let existing_inspect_select_output = tempdir.path().join("inspect-select.txt");
    fs::write(&existing_inspect_select_output, "select").expect("inspect select output");
    let (exit_code, stdout, stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "inspect".to_owned(),
        "select".to_owned(),
        input.clone(),
        "--css".to_owned(),
        "article".to_owned(),
        "--output".to_owned(),
        "text".to_owned(),
        "--output-file".to_owned(),
        existing_inspect_select_output
            .to_string_lossy()
            .into_owned(),
    ]);
    assert_eq!(exit_code, EXIT_CODE_OUTPUT);
    assert!(stdout.is_empty());
    assert!(stderr.contains("Refusing to overwrite existing output file"));

    let existing_inspect_slice_request = tempdir.path().join("inspect-slice-request.json");
    fs::write(&existing_inspect_slice_request, "request").expect("inspect slice request");
    let (exit_code, stdout, stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "inspect".to_owned(),
        "slice".to_owned(),
        input.clone(),
        "--from".to_owned(),
        "<article>".to_owned(),
        "--to".to_owned(),
        "</article>".to_owned(),
        "--emit-request-file".to_owned(),
        existing_inspect_slice_request
            .to_string_lossy()
            .into_owned(),
    ]);
    assert_eq!(exit_code, EXIT_CODE_OUTPUT);
    assert!(stdout.contains("\"code\": \"CLI_REQUEST_FILE_EXISTS\""));
    assert!(stderr.is_empty());

    let existing_inspect_slice_output = tempdir.path().join("inspect-slice.txt");
    fs::write(&existing_inspect_slice_output, "slice").expect("inspect slice output");
    let (exit_code, stdout, stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "inspect".to_owned(),
        "slice".to_owned(),
        input,
        "--from".to_owned(),
        "<article>".to_owned(),
        "--to".to_owned(),
        "</article>".to_owned(),
        "--output".to_owned(),
        "text".to_owned(),
        "--output-file".to_owned(),
        existing_inspect_slice_output.to_string_lossy().into_owned(),
    ]);
    assert_eq!(exit_code, EXIT_CODE_OUTPUT);
    assert!(stdout.is_empty());
    assert!(stderr.contains("Refusing to overwrite existing output file"));
}

#[test]
fn command_preflight_success_paths_cover_optional_filesystem_targets() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture_file(
        tempdir.path(),
        "input.html",
        "<article><p>Hello</p></article>",
    );
    let input = input_path.to_string_lossy().into_owned();

    let catalog_output = tempdir.path().join("catalog/output.json");
    let (verbose, quiet, command) = parsed_command(&[
        "htmlcut",
        "catalog",
        "--output",
        "json",
        "--output-file",
        catalog_output.to_string_lossy().as_ref(),
    ]);
    let Commands::Catalog(args) = command else {
        panic!("catalog command");
    };
    let outcome = run_catalog(args, verbose, quiet);
    assert_eq!(outcome.exit_code, 0);
    assert_eq!(
        outcome.output_file.as_deref(),
        Some(catalog_output.as_path())
    );

    let schema_output = tempdir.path().join("schema/output.json");
    let (verbose, quiet, command) = parsed_command(&[
        "htmlcut",
        "schema",
        "--output",
        "json",
        "--output-file",
        schema_output.to_string_lossy().as_ref(),
    ]);
    let Commands::Schema(args) = command else {
        panic!("schema command");
    };
    let outcome = run_schema(args, verbose, quiet);
    assert_eq!(outcome.exit_code, 0);
    assert_eq!(
        outcome.output_file.as_deref(),
        Some(schema_output.as_path())
    );

    let slice_request = tempdir.path().join("slice/request.json");
    let (verbose, quiet, command) = parsed_command(&[
        "htmlcut",
        "slice",
        input.as_str(),
        "--from",
        "<article>",
        "--to",
        "</article>",
        "--output",
        "json",
        "--emit-request-file",
        slice_request.to_string_lossy().as_ref(),
    ]);
    let Commands::Slice(args) = command else {
        panic!("slice command");
    };
    let outcome = run_slice(args, verbose, quiet);
    assert_eq!(outcome.exit_code, 0);

    let slice_output = tempdir.path().join("slice/output.json");
    let (verbose, quiet, command) = parsed_command(&[
        "htmlcut",
        "slice",
        input.as_str(),
        "--from",
        "<article>",
        "--to",
        "</article>",
        "--output",
        "json",
        "--output-file",
        slice_output.to_string_lossy().as_ref(),
    ]);
    let Commands::Slice(args) = command else {
        panic!("slice command");
    };
    let outcome = run_slice(args, verbose, quiet);
    assert_eq!(outcome.exit_code, 0);
    assert_eq!(outcome.output_file.as_deref(), Some(slice_output.as_path()));

    let slice_bundle = tempdir.path().join("slice/bundle");
    let (verbose, quiet, command) = parsed_command(&[
        "htmlcut",
        "slice",
        input.as_str(),
        "--from",
        "<article>",
        "--to",
        "</article>",
        "--output",
        "json",
        "--bundle",
        slice_bundle.to_string_lossy().as_ref(),
    ]);
    let Commands::Slice(args) = command else {
        panic!("slice command");
    };
    let outcome = run_slice(args, verbose, quiet);
    assert_eq!(outcome.exit_code, 0);

    let inspect_source_output = tempdir.path().join("inspect/source.json");
    let (verbose, quiet, command) = parsed_command(&[
        "htmlcut",
        "--quiet",
        "inspect",
        "source",
        input.as_str(),
        "--output",
        "json",
        "--output-file",
        inspect_source_output.to_string_lossy().as_ref(),
    ]);
    let Commands::Inspect(args) = command else {
        panic!("inspect command");
    };
    let InspectCommands::Source(args) = args.command else {
        panic!("inspect source command");
    };
    let outcome = run_inspect_source(args, verbose, quiet);
    assert_eq!(outcome.exit_code, 0);
    assert!(outcome.stderr.is_empty());

    let inspect_select_request = tempdir.path().join("inspect/select-request.json");
    let inspect_select_output = tempdir.path().join("inspect/select.json");
    let (verbose, quiet, command) = parsed_command(&[
        "htmlcut",
        "inspect",
        "select",
        input.as_str(),
        "--css",
        "article",
        "--emit-request-file",
        inspect_select_request.to_string_lossy().as_ref(),
        "--output-file",
        inspect_select_output.to_string_lossy().as_ref(),
    ]);
    let Commands::Inspect(args) = command else {
        panic!("inspect command");
    };
    let InspectCommands::Select(args) = args.command else {
        panic!("inspect select command");
    };
    let outcome = run_inspect_select(args, verbose, quiet);
    assert_eq!(outcome.exit_code, 0);
    assert_eq!(
        outcome.output_file.as_deref(),
        Some(inspect_select_output.as_path())
    );

    let inspect_slice_request = tempdir.path().join("inspect/slice-request.json");
    let inspect_slice_output = tempdir.path().join("inspect/slice.json");
    let (verbose, quiet, command) = parsed_command(&[
        "htmlcut",
        "inspect",
        "slice",
        input.as_str(),
        "--from",
        "<article>",
        "--to",
        "</article>",
        "--emit-request-file",
        inspect_slice_request.to_string_lossy().as_ref(),
        "--output-file",
        inspect_slice_output.to_string_lossy().as_ref(),
    ]);
    let Commands::Inspect(args) = command else {
        panic!("inspect command");
    };
    let InspectCommands::Slice(args) = args.command else {
        panic!("inspect slice command");
    };
    let outcome = run_inspect_slice(args, verbose, quiet);
    assert_eq!(outcome.exit_code, 0);
    assert_eq!(
        outcome.output_file.as_deref(),
        Some(inspect_slice_output.as_path())
    );
}

#[test]
fn direct_inspect_source_failure_and_quiet_success_cover_remaining_branches() {
    let tempdir = tempdir().expect("tempdir");
    let missing_input = tempdir.path().join("missing.html");
    let (verbose, quiet, command) = parsed_command(&[
        "htmlcut",
        "inspect",
        "source",
        missing_input.to_string_lossy().as_ref(),
        "--output",
        "json",
    ]);
    let Commands::Inspect(args) = command else {
        panic!("inspect command");
    };
    let InspectCommands::Source(args) = args.command else {
        panic!("inspect source command");
    };
    let failure = run_inspect_source(args, verbose, quiet);
    assert_eq!(failure.exit_code, EXIT_CODE_SOURCE);
    assert!(
        failure
            .stdout
            .as_deref()
            .expect("json failure")
            .contains("\"ok\": false")
    );
}
