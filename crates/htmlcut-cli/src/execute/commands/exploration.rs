//! Bounded exploration and target-resolution CLI execution.

use super::super::{ExecutionOutcome, error_outcome, output_file_notice};
use crate::file_output::{FileWriteMode, validate_output_file_target};
use crate::render::to_pretty_json;

const MAX_CURSOR_JSON_BYTES: usize = 1_024;

pub(crate) fn run_inspect_elements(
    args: crate::args::InspectElementsArgs,
    _verbose: u8,
    quiet: bool,
) -> ExecutionOutcome {
    let write_mode = FileWriteMode::from_overwrite_flag(args.file_write.overwrite);
    if let Some(path) = args.output_file.as_deref()
        && let Err(error) = validate_output_file_target(path, write_mode)
    {
        return error_outcome("inspect elements".to_owned(), true, None, write_mode, error);
    }
    let cursor = match args.cursor.as_deref().map(parse_cursor).transpose() {
        Ok(cursor) => cursor,
        Err(error) => {
            return error_outcome("inspect elements".to_owned(), true, None, write_mode, error);
        }
    };
    let source = match crate::prepare::build_source_request(&args.source) {
        Ok(source) => source,
        Err(error) => {
            return error_outcome("inspect elements".to_owned(), true, None, write_mode, error);
        }
    };
    let runtime = match crate::prepare::build_runtime(&args.source) {
        Ok(runtime) => runtime,
        Err(error) => {
            return error_outcome("inspect elements".to_owned(), true, None, write_mode, error);
        }
    };
    let Some(max_elements) = std::num::NonZeroU32::new(args.max_elements) else {
        return error_outcome(
            "inspect elements".to_owned(),
            true,
            None,
            write_mode,
            crate::error::usage_error(
                crate::model::CliErrorCode::ExplorationLimitInvalid,
                "--max-elements must be greater than zero.",
            ),
        );
    };
    let Some(max_work_units) = std::num::NonZeroU32::new(args.max_work_units) else {
        return error_outcome(
            "inspect elements".to_owned(),
            true,
            None,
            write_mode,
            crate::error::usage_error(
                crate::model::CliErrorCode::ExplorationLimitInvalid,
                "--max-work-units must be greater than zero.",
            ),
        );
    };
    let Some(max_proposals_per_element) = std::num::NonZeroU32::new(args.max_proposals_per_element)
    else {
        return error_outcome(
            "inspect elements".to_owned(),
            true,
            None,
            write_mode,
            crate::error::usage_error(
                crate::model::CliErrorCode::ExplorationLimitInvalid,
                "--max-proposals-per-element must be greater than zero.",
            ),
        );
    };
    let Some(preview_bytes) = std::num::NonZeroU32::new(args.preview_bytes) else {
        return error_outcome(
            "inspect elements".to_owned(),
            true,
            None,
            write_mode,
            crate::error::usage_error(
                crate::model::CliErrorCode::ExplorationLimitInvalid,
                "--preview-bytes must be greater than zero.",
            ),
        );
    };
    let prepared = match prepared_source_or_outcome(
        "inspect elements",
        htmlcut_core::interop::v2::prepare_source(
            &source,
            &runtime,
            htmlcut_core::interop::v2::PreparationLimits::default(),
        ),
        write_mode,
    ) {
        Ok(prepared) => prepared,
        Err(outcome) => return outcome,
    };
    let options = htmlcut_core::interop::v2::ExplorationOptions {
        cursor,
        max_elements,
        max_work_units,
        max_proposals_per_element,
        preview_bytes,
    };
    let result = match htmlcut_core::interop::v2::explore(prepared.document(), &options) {
        Ok(result) => result,
        Err(error) => {
            return error_outcome(
                "inspect elements".to_owned(),
                true,
                None,
                write_mode,
                exploration_cli_error(*error),
            );
        }
    };
    let stdout = match to_pretty_json(&result) {
        Ok(stdout) => stdout,
        Err(error) => {
            return error_outcome("inspect elements".to_owned(), true, None, write_mode, error);
        }
    };
    let post_write_stderr = output_file_notice(args.output_file.as_deref(), quiet);
    ExecutionOutcome {
        stdout: Some(stdout),
        output_file: args.output_file,
        write_mode,
        post_write_stderr,
        stderr: Vec::new(),
        exit_code: 0,
    }
}

fn parse_cursor(
    json: &str,
) -> Result<htmlcut_core::interop::v2::ExplorationCursor, crate::error::CliError> {
    if json.len() > MAX_CURSOR_JSON_BYTES {
        return Err(crate::error::usage_error(
            crate::model::CliErrorCode::ExplorationCursorInvalid,
            "--cursor exceeds the 1024-byte JSON limit.",
        ));
    }
    serde_json::from_str(json).map_err(|_| {
        crate::error::usage_error(
            crate::model::CliErrorCode::ExplorationCursorInvalid,
            "--cursor must be the exact next_cursor JSON object from an exploration page.",
        )
    })
}

fn exploration_cli_error(
    error: htmlcut_core::interop::v2::ExplorationError,
) -> crate::error::CliError {
    use htmlcut_core::interop::v2::ExplorationErrorCode;

    match error.error_code {
        ExplorationErrorCode::InvalidCursor => crate::error::usage_error(
            crate::model::CliErrorCode::ExplorationCursorInvalid,
            error.message,
        ),
        ExplorationErrorCode::CursorSnapshotMismatch => crate::error::extraction_error(
            crate::model::CliErrorCode::ExplorationCursorSnapshotMismatch,
            error.message,
            Vec::new(),
        ),
        ExplorationErrorCode::CursorOptionsMismatch => crate::error::usage_error(
            crate::model::CliErrorCode::ExplorationCursorOptionsMismatch,
            error.message,
        ),
        _ => crate::error::extraction_error(
            crate::model::CliErrorCode::ExplorationFailed,
            error.message,
            Vec::new(),
        ),
    }
}

pub(crate) fn run_inspect_propose(
    args: crate::args::InspectProposeArgs,
    _verbose: u8,
    quiet: bool,
) -> ExecutionOutcome {
    let write_mode = FileWriteMode::from_overwrite_flag(args.file_write.overwrite);
    if let Some(path) = args.output_file.as_deref()
        && let Err(error) = validate_output_file_target(path, write_mode)
    {
        return error_outcome("inspect propose".to_owned(), true, None, write_mode, error);
    }
    let source = match crate::prepare::build_source_request(&args.source) {
        Ok(source) => source,
        Err(error) => {
            return error_outcome("inspect propose".to_owned(), true, None, write_mode, error);
        }
    };
    let runtime = match crate::prepare::build_runtime(&args.source) {
        Ok(runtime) => runtime,
        Err(error) => {
            return error_outcome("inspect propose".to_owned(), true, None, write_mode, error);
        }
    };
    let namespace = match args.namespace.as_str() {
        "html" => htmlcut_core::interop::v2::ElementNamespace::Html,
        "svg" => htmlcut_core::interop::v2::ElementNamespace::Svg,
        "mathml" => htmlcut_core::interop::v2::ElementNamespace::Mathml,
        _ => {
            return error_outcome(
                "inspect propose".to_owned(),
                true,
                None,
                write_mode,
                crate::error::usage_error(
                    crate::model::CliErrorCode::TargetHintInvalid,
                    "--namespace must be html, svg, or mathml.",
                ),
            );
        }
    };
    let semantic_attributes = match args
        .attributes
        .iter()
        .map(|attribute| {
            let Some((name, value)) = attribute.split_once('=') else {
                return Err(crate::error::usage_error(
                    crate::model::CliErrorCode::TargetHintInvalid,
                    "--attribute must use NAME=VALUE form.",
                ));
            };
            if name.is_empty() || value.is_empty() {
                return Err(crate::error::usage_error(
                    crate::model::CliErrorCode::TargetHintInvalid,
                    "--attribute NAME and VALUE must both be non-empty.",
                ));
            }
            Ok(htmlcut_core::interop::v2::ExplorationAttribute {
                name: name.to_owned(),
                value: value.to_owned(),
                value_truncated: false,
            })
        })
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(attributes) => attributes,
        Err(error) => {
            return error_outcome("inspect propose".to_owned(), true, None, write_mode, error);
        }
    };
    let Some(max_work_units) = std::num::NonZeroU32::new(args.max_work_units) else {
        return error_outcome(
            "inspect propose".to_owned(),
            true,
            None,
            write_mode,
            crate::error::usage_error(
                crate::model::CliErrorCode::ExplorationLimitInvalid,
                "--max-work-units must be greater than zero.",
            ),
        );
    };
    let Some(max_proposals) = std::num::NonZeroU32::new(args.max_proposals) else {
        return error_outcome(
            "inspect propose".to_owned(),
            true,
            None,
            write_mode,
            crate::error::usage_error(
                crate::model::CliErrorCode::ExplorationLimitInvalid,
                "--max-proposals must be greater than zero.",
            ),
        );
    };
    let prepared = match prepared_source_or_outcome(
        "inspect propose",
        htmlcut_core::interop::v2::prepare_source(
            &source,
            &runtime,
            htmlcut_core::interop::v2::PreparationLimits::default(),
        ),
        write_mode,
    ) {
        Ok(prepared) => prepared,
        Err(outcome) => return outcome,
    };
    let hint = htmlcut_core::interop::v2::ElementTargetHint {
        path_profile: "htmlcut.dom_path.v1".to_owned(),
        path: args.path,
        element_name: htmlcut_core::interop::v2::ElementName {
            namespace,
            local_name: args.local_name,
        },
        normalized_text_digest_sha256: args.text_digest_sha256,
        semantic_attributes,
    };
    let options = htmlcut_core::interop::v2::TargetResolutionOptions {
        max_work_units,
        max_proposals,
    };
    let result = match htmlcut_core::interop::v2::resolve_target_and_propose(
        prepared.document(),
        &hint,
        &options,
    ) {
        Ok(result) => result,
        Err(error) => {
            return error_outcome(
                "inspect propose".to_owned(),
                true,
                None,
                write_mode,
                crate::error::extraction_error(
                    crate::model::CliErrorCode::ExplorationFailed,
                    error.message,
                    Vec::new(),
                ),
            );
        }
    };
    let stdout = match to_pretty_json(&result) {
        Ok(stdout) => stdout,
        Err(error) => {
            return error_outcome("inspect propose".to_owned(), true, None, write_mode, error);
        }
    };
    let post_write_stderr = output_file_notice(args.output_file.as_deref(), quiet);
    ExecutionOutcome {
        stdout: Some(stdout),
        output_file: args.output_file,
        write_mode,
        post_write_stderr,
        stderr: Vec::new(),
        exit_code: 0,
    }
}

fn preparation_error_outcome(
    command: &str,
    source: &htmlcut_core::SourceMetadata,
    error: htmlcut_core::interop::v2::PreparationError,
    write_mode: FileWriteMode,
) -> ExecutionOutcome {
    let error = crate::error::with_source_load_steps(
        crate::error::extraction_error(
            crate::model::CliErrorCode::ExplorationFailed,
            error.message,
            Vec::new(),
        ),
        source,
    );
    error_outcome(command.to_owned(), true, None, write_mode, error)
}

fn prepared_source_or_outcome(
    command: &str,
    result: Result<
        htmlcut_core::interop::v2::PreparedSource,
        htmlcut_core::interop::v2::PrepareSourceError,
    >,
    write_mode: FileWriteMode,
) -> Result<htmlcut_core::interop::v2::PreparedSource, ExecutionOutcome> {
    match result {
        Ok(prepared) => Ok(prepared),
        Err(htmlcut_core::interop::v2::PrepareSourceError::SourceLoad { source, diagnostic }) => {
            let error = crate::error::with_source_load_steps(
                crate::error::source_error(
                    diagnostic.code,
                    diagnostic.message.clone(),
                    vec![*diagnostic],
                ),
                &source,
            );
            Err(error_outcome(
                command.to_owned(),
                true,
                None,
                write_mode,
                error,
            ))
        }
        Err(htmlcut_core::interop::v2::PrepareSourceError::Preparation { source, error }) => Err(
            preparation_error_outcome(command, &source, *error, write_mode),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preparation_failure_preserves_source_context_and_uses_exploration_error_code() {
        let source = htmlcut_core::SourceMetadata {
            kind: htmlcut_core::SourceKind::Memory,
            value: "prepared-source".to_owned(),
            input_base_url: None,
            effective_base_url: None,
            bytes_read: 12,
            load_steps: Vec::new(),
            text: None,
        };
        let outcome = preparation_error_outcome(
            "inspect elements",
            &source,
            htmlcut_core::interop::v2::PreparationError {
                schema_name: "htmlcut.preparation_error".to_owned(),
                schema_version: 1,
                error_code: htmlcut_core::interop::v2::PreparationErrorCode::InputTooLarge,
                input_digest_sha256: "0".repeat(64),
                message: "HTML input exceeds the configured preparation byte limit.".to_owned(),
            },
            FileWriteMode::CreateFresh,
        );
        assert_eq!(outcome.exit_code, crate::EXIT_CODE_EXTRACTION);
        assert!(outcome.stdout.is_some());
        assert!(outcome.stderr.is_empty());
    }

    #[test]
    fn prepared_source_resolution_projects_each_typed_failure_once() {
        let source = htmlcut_core::SourceMetadata {
            kind: htmlcut_core::SourceKind::Memory,
            value: "missing-source".to_owned(),
            input_base_url: None,
            effective_base_url: None,
            bytes_read: 0,
            load_steps: Vec::new(),
            text: None,
        };
        let source_failure = prepared_source_or_outcome(
            "inspect propose",
            Err(htmlcut_core::interop::v2::PrepareSourceError::SourceLoad {
                source: Box::new(source.clone()),
                diagnostic: Box::new(htmlcut_core::Diagnostic {
                    level: htmlcut_core::DiagnosticLevel::Error,
                    code: htmlcut_core::DiagnosticCode::SourceLoadFailed,
                    message: "Could not load source.".to_owned(),
                    details: None,
                }),
            }),
            FileWriteMode::CreateFresh,
        );
        assert!(source_failure.is_err());
        let source_failure = source_failure.err().expect("source failure outcome");
        assert_eq!(source_failure.exit_code, crate::EXIT_CODE_SOURCE);

        let preparation_failure = prepared_source_or_outcome(
            "inspect propose",
            Err(htmlcut_core::interop::v2::PrepareSourceError::Preparation {
                source: Box::new(source),
                error: Box::new(htmlcut_core::interop::v2::PreparationError {
                    schema_name: "htmlcut.preparation_error".to_owned(),
                    schema_version: 1,
                    error_code: htmlcut_core::interop::v2::PreparationErrorCode::DocumentTooDeep,
                    input_digest_sha256: "0".repeat(64),
                    message: "Parsed HTML exceeds the configured depth limit.".to_owned(),
                }),
            }),
            FileWriteMode::CreateFresh,
        );
        assert!(preparation_failure.is_err());
        let preparation_failure = preparation_failure
            .err()
            .expect("preparation failure outcome");
        assert_eq!(preparation_failure.exit_code, crate::EXIT_CODE_EXTRACTION);
    }
}
