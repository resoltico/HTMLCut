use std::collections::BTreeSet;

use clap::error::ErrorKind;

pub(super) fn command_example_errors(
    display_path: &str,
    text: &str,
    schema_names: &BTreeSet<&'static str>,
    operation_ids: &BTreeSet<&'static str>,
) -> Vec<String> {
    let mut errors = Vec::new();
    let command = htmlcut_cli::command();

    for example in extract_htmlcut_examples(text) {
        let tokens = match shell_words(&example) {
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
                    clap_error_message(&error)
                ));
            }
            continue;
        }

        if let Some(error) =
            command_reference_error(display_path, &tokens, schema_names, operation_ids)
        {
            errors.push(error);
        }
    }

    errors
}

pub(super) fn extract_htmlcut_examples(text: &str) -> Vec<String> {
    let mut examples = Vec::new();
    let mut in_fence = false;
    let mut current = String::new();

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            in_fence = !in_fence;
            if !in_fence {
                current.clear();
            }
            continue;
        }

        if !in_fence {
            continue;
        }

        if current.is_empty() && (!trimmed.starts_with("htmlcut ") || !is_concrete_example(trimmed))
        {
            continue;
        }

        if current.is_empty() {
            current.push_str(trimmed.trim_end_matches('\\').trim_end());
        } else {
            current.push(' ');
            current.push_str(trimmed.trim_end_matches('\\').trim_end());
        }

        if !trimmed.ends_with('\\') {
            examples.push(std::mem::take(&mut current));
        }
    }

    examples
}

fn is_concrete_example(line: &str) -> bool {
    !line.contains('[') && !line.contains("...")
}

pub(super) fn shell_words(command: &str) -> Result<Vec<String>, String> {
    shell_words::split(command).map_err(|error| error.to_string())
}

fn option_value<'a>(tokens: &'a [String], flag: &str) -> Option<&'a str> {
    tokens.iter().enumerate().find_map(|(index, token)| {
        token.strip_prefix(&format!("{flag}=")).or_else(|| {
            if token == flag {
                tokens.get(index + 1).map(String::as_str)
            } else {
                None
            }
        })
    })
}

pub(super) fn command_reference_error(
    display_path: &str,
    tokens: &[String],
    schema_names: &BTreeSet<&'static str>,
    operation_ids: &BTreeSet<&'static str>,
) -> Option<String> {
    match tokens.get(1).map(String::as_str) {
        Some("catalog") => {
            let operation_id = option_value(tokens, "--operation")?;
            (!operation_ids.contains(operation_id)).then(|| {
                format!("{display_path} example references unknown operation ID: {operation_id}")
            })
        }
        Some("schema") => {
            let schema_name = option_value(tokens, "--name")?;
            (!schema_names.contains(schema_name)).then(|| {
                format!("{display_path} example references unknown schema name: {schema_name}")
            })
        }
        Some("inspect") | Some("select") | Some("slice") => {
            let command_path = command_path(tokens);
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

pub(super) fn command_path(tokens: &[String]) -> Vec<&str> {
    match tokens.get(1).map(String::as_str) {
        Some("inspect") => vec![
            "inspect",
            tokens
                .get(2)
                .map(String::as_str)
                .expect("inspect examples should include a subcommand"),
        ],
        Some(top_level) => vec![top_level],
        None => Vec::new(),
    }
}

fn clap_error_message(error: &clap::Error) -> String {
    let rendered = error.to_string();
    rendered
        .lines()
        .find_map(|line| line.strip_prefix("error: ").map(ToOwned::to_owned))
        .unwrap_or_else(|| rendered.trim().to_owned())
}
