use std::collections::BTreeSet;

use clap::error::ErrorKind;

mod parsing;
mod runtime;
mod sandbox;
#[cfg(test)]
pub(crate) mod testing;

pub(crate) fn command_example_errors(
    display_path: &str,
    text: &str,
    schema_names: &BTreeSet<&'static str>,
    operation_ids: &BTreeSet<&'static str>,
) -> Vec<String> {
    command_example_errors_with_prepared_sandbox(
        display_path,
        text,
        schema_names,
        operation_ids,
        sandbox::prepare_sandbox(display_path),
    )
}

fn command_example_errors_with_prepared_sandbox(
    display_path: &str,
    text: &str,
    schema_names: &BTreeSet<&'static str>,
    operation_ids: &BTreeSet<&'static str>,
    prepared_sandbox: Result<(sandbox::ExampleSandbox, sandbox::CurrentDirGuard), Vec<String>>,
) -> Vec<String> {
    let mut errors = Vec::new();
    let command = htmlcut_cli::command();
    let (sandbox, _cwd) = match prepared_sandbox {
        Ok(parts) => parts,
        Err(errors) => return errors,
    };

    for example in parsing::extract_htmlcut_examples(text) {
        let tokens = match parsing::shell_words(&example) {
            Ok(tokens) => tokens,
            Err(error) => {
                errors.push(format!(
                    "{display_path} contains a non-parsing htmlcut example: {example} ({error})"
                ));
                continue;
            }
        };
        if let Err(error) = command.clone().try_get_matches_from(tokens.clone()) {
            if !matches!(
                error.kind(),
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
            ) {
                errors.push(format!(
                    "{display_path} contains a non-parsing htmlcut example: {example} ({})",
                    parsing::clap_error_message(&error)
                ));
            }
            continue;
        }

        if let Some(error) =
            command_reference_error(display_path, &tokens, schema_names, operation_ids)
        {
            errors.push(error);
            continue;
        }

        if let Some(error) = sandbox.command_runtime_error(display_path, &example, &tokens) {
            errors.push(error);
        }
    }

    errors
}

#[cfg(test)]
pub(crate) use parsing::{command_path, extract_htmlcut_examples, shell_words};

pub(crate) fn command_reference_error(
    display_path: &str,
    tokens: &[String],
    schema_names: &BTreeSet<&'static str>,
    operation_ids: &BTreeSet<&'static str>,
) -> Option<String> {
    match tokens.get(1).map(String::as_str) {
        Some("catalog") => {
            let operation_id = parsing::option_value(tokens, "--operation")?;
            (!operation_ids.contains(operation_id)).then(|| {
                format!("{display_path} example references unknown operation ID: {operation_id}")
            })
        }
        Some("schema") => {
            let schema_name = parsing::option_value(tokens, "--name")?;
            (!schema_names.contains(schema_name)).then(|| {
                format!("{display_path} example references unknown schema name: {schema_name}")
            })
        }
        Some("inspect") | Some("select") | Some("slice") => {
            let command_path = parsing::command_path(tokens);
            htmlcut_core::find_cli_operation_by_command_path(&command_path)
                .is_none()
                .then(|| {
                    format!(
                        "{display_path} example references unknown CLI command path: {}",
                        command_path.join(" ")
                    )
                })
        }
        Some(_) | None => None,
    }
}
