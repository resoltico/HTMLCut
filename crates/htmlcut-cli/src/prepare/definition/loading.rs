use std::fs;
use std::path::Path;

use htmlcut_core::{ExtractionDefinition, ExtractionRequest, ExtractionStrategy, RuntimeOptions};
use serde_json::Value;

use crate::error::{CliError, usage_error};
use crate::model::CliErrorCode;

use super::super::{MaterializedDefinition, PendingExtractionDefinitionWrite};
use crate::args::DefinitionArgs;

pub(in crate::prepare) fn materialize_extraction_definition<Build>(
    definition_args: &DefinitionArgs,
    expected_strategy: ExtractionStrategy,
    command: &str,
    operation_id: htmlcut_core::OperationId,
    build_inline: Build,
) -> Result<MaterializedDefinition, CliError>
where
    Build: FnOnce() -> Result<(ExtractionRequest, RuntimeOptions), CliError>,
{
    let (request, runtime) = if let Some(path) = definition_args.request_file.as_deref() {
        let definition =
            load_extraction_definition(path, expected_strategy, command, operation_id)?;
        (definition.request, definition.runtime)
    } else {
        build_inline()?
    };

    Ok(MaterializedDefinition {
        request_definition_output: definition_args.emit_request_file.clone().map(|path| {
            PendingExtractionDefinitionWrite {
                path,
                definition: ExtractionDefinition {
                    schema_name: htmlcut_core::EXTRACTION_DEFINITION_SCHEMA_NAME.to_owned(),
                    schema_version: htmlcut_core::EXTRACTION_DEFINITION_SCHEMA_VERSION,
                    request: request.clone(),
                    runtime: runtime.clone(),
                },
            }
        }),
        request,
        runtime,
    })
}

fn load_extraction_definition(
    path: &Path,
    expected_strategy: ExtractionStrategy,
    command: &str,
    operation_id: htmlcut_core::OperationId,
) -> Result<ExtractionDefinition, CliError> {
    let raw = fs::read_to_string(path).map_err(|error| {
        usage_error(
            CliErrorCode::RequestFileReadFailed,
            format!(
                "Could not read extraction definition {}: {error}. {}",
                path.display(),
                request_file_recovery_hint(operation_id, expected_strategy, None)
            ),
        )
    })?;
    let value: Value = serde_json::from_str(&raw).map_err(|error| {
        usage_error(
            CliErrorCode::RequestFileInvalid,
            format!(
                "Could not parse extraction definition {} as JSON: {error}. {}",
                path.display(),
                request_file_recovery_hint(operation_id, expected_strategy, None)
            ),
        )
    })?;
    let shape_hint = request_file_shape_hint(&value, expected_strategy);
    let definition: ExtractionDefinition =
        serde_path_to_error::deserialize(value).map_err(|error| {
            let json_path = render_json_error_path(&error);
            usage_error(
                CliErrorCode::RequestFileInvalid,
                format!(
                    "Could not parse extraction definition {} as {}@{} at JSON path {}: {}. {}",
                    path.display(),
                    htmlcut_core::EXTRACTION_DEFINITION_SCHEMA_NAME,
                    htmlcut_core::EXTRACTION_DEFINITION_SCHEMA_VERSION,
                    json_path,
                    error.inner(),
                    request_file_recovery_hint(
                        operation_id,
                        expected_strategy,
                        shape_hint.as_deref()
                    )
                ),
            )
        })?;

    if definition.schema_name != htmlcut_core::EXTRACTION_DEFINITION_SCHEMA_NAME
        || definition.schema_version != htmlcut_core::EXTRACTION_DEFINITION_SCHEMA_VERSION
    {
        return Err(usage_error(
            CliErrorCode::RequestFileSchemaUnsupported,
            format!(
                "Unsupported extraction definition schema in {}: expected {}@{}, got {}@{}. {} Re-emit a current definition with `htmlcut {} ... --emit-request-file <PATH>` or hand-author one that matches the maintained contract.",
                path.display(),
                htmlcut_core::EXTRACTION_DEFINITION_SCHEMA_NAME,
                htmlcut_core::EXTRACTION_DEFINITION_SCHEMA_VERSION,
                definition.schema_name,
                definition.schema_version,
                request_file_recovery_hint(operation_id, expected_strategy, None),
                command,
            ),
        ));
    }

    if definition.request.extraction.strategy() != expected_strategy {
        return Err(usage_error(
            CliErrorCode::RequestFileStrategyMismatch,
            format!(
                "{} cannot execute a {} extraction definition from {} because it only accepts {} extraction definitions. {} Re-emit a matching definition with `htmlcut {} ... --emit-request-file <PATH>` or hand-author one that matches the maintained contract.",
                command,
                strategy_label(definition.request.extraction.strategy()),
                path.display(),
                strategy_label(expected_strategy),
                request_file_recovery_hint(operation_id, expected_strategy, None),
                command,
            ),
        ));
    }

    Ok(definition)
}

#[cfg(test)]
pub(crate) fn load_extraction_definition_for_tests(
    path: &Path,
    expected_strategy: ExtractionStrategy,
    command: &str,
) -> Result<ExtractionDefinition, CliError> {
    let operation_id = match (command, expected_strategy) {
        (command, ExtractionStrategy::Selector)
            if command
                == htmlcut_core::cli_contract::cli_operation_display_command(
                    htmlcut_core::OperationId::SelectExtract,
                )
                .expect("select extract should stay CLI-visible") =>
        {
            htmlcut_core::OperationId::SelectExtract
        }
        (command, ExtractionStrategy::Slice)
            if command
                == htmlcut_core::cli_contract::cli_operation_display_command(
                    htmlcut_core::OperationId::SliceExtract,
                )
                .expect("slice extract should stay CLI-visible") =>
        {
            htmlcut_core::OperationId::SliceExtract
        }
        (command, ExtractionStrategy::Selector)
            if command
                == htmlcut_core::cli_contract::cli_operation_display_command(
                    htmlcut_core::OperationId::SelectPreview,
                )
                .expect("select preview should stay CLI-visible") =>
        {
            htmlcut_core::OperationId::SelectPreview
        }
        (command, ExtractionStrategy::Slice)
            if command
                == htmlcut_core::cli_contract::cli_operation_display_command(
                    htmlcut_core::OperationId::SlicePreview,
                )
                .expect("slice preview should stay CLI-visible") =>
        {
            htmlcut_core::OperationId::SlicePreview
        }
        (_, ExtractionStrategy::Selector) => htmlcut_core::OperationId::SelectExtract,
        (_, ExtractionStrategy::Slice) => htmlcut_core::OperationId::SliceExtract,
    };
    load_extraction_definition(path, expected_strategy, command, operation_id)
}

fn render_json_error_path(error: &serde_path_to_error::Error<serde_json::Error>) -> String {
    format_json_error_path(&error.path().to_string())
}

fn format_json_error_path(path: &str) -> String {
    match path {
        "" | "$" => "$".to_owned(),
        path if path.starts_with("$.") => path.to_owned(),
        path => {
            let stripped = path.strip_prefix('.').unwrap_or(path);
            format!("$.{stripped}")
        }
    }
}

#[cfg(test)]
pub(crate) fn format_json_error_path_for_tests(path: &str) -> String {
    format_json_error_path(path)
}

fn request_file_recovery_hint(
    operation_id: htmlcut_core::OperationId,
    expected_strategy: ExtractionStrategy,
    shape_hint: Option<&str>,
) -> String {
    let schema_command = htmlcut_core::cli_contract::cli_aux_command_display_command(
        htmlcut_core::cli_contract::CliAuxCommandId::Schema,
    );
    let catalog_command = htmlcut_core::cli_contract::cli_aux_command_display_command(
        htmlcut_core::cli_contract::CliAuxCommandId::Catalog,
    );
    let mut hint = format!(
        "Use `htmlcut {schema_command} --name {} --output json` for the exact request-file shape and `htmlcut {catalog_command} --operation {} --output json` for the command contract.",
        htmlcut_core::EXTRACTION_DEFINITION_SCHEMA_NAME,
        operation_id
    );
    if let Some(shape_hint) = shape_hint {
        hint.push(' ');
        hint.push_str(shape_hint);
    } else {
        let generic = match expected_strategy {
            ExtractionStrategy::Selector => {
                "Selector request files use `request.extraction.selector` as a plain JSON string."
            }
            ExtractionStrategy::Slice => {
                "Slice request files use plain JSON strings for `request.extraction.from` and `request.extraction.to`."
            }
        };
        hint.push(' ');
        hint.push_str(generic);
    }

    hint
}

fn request_file_shape_hint(value: &Value, expected_strategy: ExtractionStrategy) -> Option<String> {
    let extraction = value.get("request")?.get("extraction")?;

    match expected_strategy {
        ExtractionStrategy::Selector => extraction
            .get("selector")
            .filter(|selector| matches!(selector, Value::Object(_) | Value::Array(_)))
            .map(|_| {
                "Selector request files use `request.extraction.selector` as a plain JSON string, not an object."
                    .to_owned()
            }),
        ExtractionStrategy::Slice => ["from", "to"].iter().find_map(|field| {
            extraction
                .get(field)
                .filter(|boundary| matches!(boundary, Value::Object(_) | Value::Array(_)))
                .map(|_| {
                    format!(
                        "Slice request files use `request.extraction.{field}` as a plain JSON string, not an object."
                    )
                })
        }),
    }
}

fn strategy_label(strategy: ExtractionStrategy) -> &'static str {
    match strategy {
        ExtractionStrategy::Selector => "selector",
        ExtractionStrategy::Slice => "slice",
    }
}
