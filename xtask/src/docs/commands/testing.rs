use std::collections::BTreeSet;

use super::command_example_errors_with_prepared_sandbox;
use super::runtime::{documented_artifact_error, render_execution_failure};
use super::sandbox::{ExampleSandbox, prepare_sandbox_with_hooks};

pub(crate) fn documented_artifact_error_for_tests(
    display_path: &str,
    example: &str,
    tokens: &[String],
) -> Option<String> {
    documented_artifact_error(display_path, example, tokens)
}

pub(crate) fn render_execution_failure_for_tests(
    exit_code: i32,
    stdout: &[u8],
    stderr: &[u8],
) -> String {
    render_execution_failure(exit_code, stdout, stderr)
}

pub(crate) fn prepare_sandbox_errors_for_tests(
    display_path: &str,
    init_error: Option<&str>,
    enter_error: Option<&str>,
) -> Vec<String> {
    match prepare_sandbox_with_hooks(
        display_path,
        || {
            if let Some(error) = init_error {
                Err(error.to_owned())
            } else {
                ExampleSandbox::new().map_err(|error| error.to_string())
            }
        },
        |sandbox| {
            if let Some(error) = enter_error {
                Err(error.to_owned())
            } else {
                sandbox.enter().map_err(|error| error.to_string())
            }
        },
    ) {
        Ok(_) => Vec::new(),
        Err(errors) => errors,
    }
}

pub(crate) fn injected_sandbox_error_for_tests(message: &str) -> Vec<String> {
    command_example_errors_with_prepared_sandbox(
        "README.md",
        "```bash\nhtmlcut select page.html --css article\n```\n",
        &BTreeSet::new(),
        &BTreeSet::new(),
        Err(vec![message.to_owned()]),
    )
}
