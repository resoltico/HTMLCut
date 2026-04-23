use super::*;
use regex::Regex;

mod contract_surfaces;
mod help_surfaces;
mod raw_args;

fn command_for_path(command_path: &[&str]) -> clap::Command {
    let mut command = Cli::command();
    for segment in command_path {
        let next = {
            command
                .get_subcommands()
                .find(|subcommand| subcommand.get_name() == *segment)
                .unwrap_or_else(|| panic!("missing command path segment {segment}"))
                .clone()
        };
        command = next;
    }
    command
}

fn assert_surface_identifiers_registered(
    label: &str,
    text: &str,
    known_schemas: &std::collections::BTreeSet<String>,
    known_operations: &std::collections::BTreeSet<String>,
) {
    let schema_pattern = Regex::new(r"\bhtmlcut\.[a-z_]+\b").expect("schema regex");
    let operation_pattern =
        Regex::new(r"\b(?:document|source|select|slice)\.(?:parse|inspect|preview|extract)\b")
            .expect("operation regex");

    for schema_name in schema_pattern
        .find_iter(text)
        .map(|capture| capture.as_str())
    {
        assert!(
            known_schemas.contains(schema_name),
            "{label} referenced unknown schema name {schema_name}: {text}"
        );
    }

    for operation_id in operation_pattern
        .find_iter(text)
        .map(|capture| capture.as_str())
    {
        assert!(
            known_operations.contains(operation_id),
            "{label} referenced unknown operation ID {operation_id}: {text}"
        );
    }
}
