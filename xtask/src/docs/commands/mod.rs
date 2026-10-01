use std::collections::BTreeSet;

mod http_fixture;
mod parsing;
mod process;
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
pub(crate) use parsing::{extract_htmlcut_examples, shell_words};

pub(crate) fn command_reference_error(
    display_path: &str,
    tokens: &[String],
    schema_names: &BTreeSet<&'static str>,
    operation_ids: &BTreeSet<&'static str>,
) -> Option<String> {
    match tokens.get(1).map(String::as_str) {
        Some("describe") => {
            let operation_id = tokens.get(2)?.as_str();
            if operation_id.starts_with('-') {
                return None;
            }
            (!operation_ids.contains(operation_id)).then(|| {
                format!("{display_path} example references unknown operation ID: {operation_id}")
            })
        }
        Some("schema") => {
            let schema_name = tokens.get(2)?.as_str();
            if schema_name.starts_with('-') {
                return None;
            }
            (!schema_names.contains(schema_name)).then(|| {
                format!("{display_path} example references unknown schema name: {schema_name}")
            })
        }
        Some(_) | None => None,
    }
}
