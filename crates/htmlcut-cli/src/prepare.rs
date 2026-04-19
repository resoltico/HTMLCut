use std::fs;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};

use htmlcut_core::{
    AttributeName, DEFAULT_FETCH_TIMEOUT_MS, DEFAULT_MAX_BYTES, DEFAULT_PREVIEW_CHARS,
    DEFAULT_REGEX_FLAGS, ExtractionDefinition, ExtractionRequest, ExtractionResult, ExtractionSpec,
    ExtractionStrategy, FetchPreflightMode, HTMLCUT_JSON_SCHEMA_PROFILE, InspectionOptions,
    NormalizationOptions, OutputOptions, RuntimeOptions, SchemaStability, SelectionSpec,
    SelectorQuery, SliceBoundary, SlicePatternSpec, SliceSpec, SourceInput, SourceInspectionResult,
    SourceRequest, ValueSpec, ValueType, WhitespaceMode,
};
use schemars::schema_for;
use serde_json::Value;
use url::Url;

use crate::args::{
    CliFetchPreflightMode, CliMatchMode, CliOutputMode, CliPatternMode, CliValueMode,
    CliWhitespaceMode, DefinitionArgs, ExtractOutputArgs, InspectSelectArgs, InspectSliceArgs,
    InspectSourceArgs, SelectArgs, SelectionArgs, SliceArgs, SourceArgs,
};
use crate::error::{CliError, usage_error};
use crate::lookup::{unknown_operation_id_error, unknown_schema_error};
use crate::metadata::{ENGINE_NAME, HTMLCUT_DESCRIPTION, HTMLCUT_VERSION, TOOL_NAME};
use crate::model::{
    BundlePaths, CATALOG_REPORT_SCHEMA_NAME, CATALOG_SCHEMA_VERSION, CatalogAvailability,
    CatalogCommandContract, CatalogCommandReport, CatalogCondition, CatalogConditionalDefault,
    CatalogConstraint, CatalogContractSurface, CatalogOperationReport, CatalogParameterKind,
    CatalogParameterRequirement, CatalogParameterSpec, EXTRACTION_COMMAND_REPORT_SCHEMA_NAME,
    EXTRACTION_COMMAND_REPORT_SCHEMA_VERSION, ExtractionCommandReport,
    SCHEMA_COMMAND_REPORT_SCHEMA_NAME, SCHEMA_COMMAND_REPORT_SCHEMA_VERSION,
    SOURCE_INSPECTION_COMMAND_REPORT_SCHEMA_NAME, SOURCE_INSPECTION_COMMAND_REPORT_SCHEMA_VERSION,
    SchemaCommandReport, SchemaDocumentReport, SchemaRefReport, SourceInspectionCommandReport,
};

const KIBIBYTE: usize = 1024;
const MEBIBYTE: usize = KIBIBYTE * KIBIBYTE;
const GIBIBYTE: usize = MEBIBYTE * KIBIBYTE;

pub(crate) struct PreparedExtraction {
    pub(crate) command: String,
    pub(crate) request: ExtractionRequest,
    pub(crate) runtime: RuntimeOptions,
    pub(crate) request_definition_output: Option<PendingExtractionDefinitionWrite>,
    pub(crate) output: CliOutputMode,
    pub(crate) bundle: Option<PathBuf>,
    pub(crate) output_file: Option<PathBuf>,
    pub(crate) verbose: u8,
    pub(crate) quiet: bool,
}

pub(crate) struct PreparedSourceInspection {
    pub(crate) command: String,
    pub(crate) source: SourceRequest,
    pub(crate) runtime: RuntimeOptions,
    pub(crate) options: InspectionOptions,
    pub(crate) output: crate::args::CliInspectOutputMode,
    pub(crate) preview_chars: usize,
    pub(crate) output_file: Option<PathBuf>,
    pub(crate) verbose: u8,
    pub(crate) quiet: bool,
}

pub(crate) struct PreparedPreview {
    pub(crate) command: String,
    pub(crate) request: ExtractionRequest,
    pub(crate) runtime: RuntimeOptions,
    pub(crate) request_definition_output: Option<PendingExtractionDefinitionWrite>,
    pub(crate) output: crate::args::CliInspectOutputMode,
    pub(crate) output_file: Option<PathBuf>,
    pub(crate) verbose: u8,
    pub(crate) quiet: bool,
}

pub(crate) struct RequestBuildOptions {
    pub(crate) value: ValueSpec,
    pub(crate) whitespace: CliWhitespaceMode,
    pub(crate) rewrite_urls: bool,
    pub(crate) preview_chars: NonZeroUsize,
    pub(crate) include_source_text: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct PendingExtractionDefinitionWrite {
    pub(crate) path: PathBuf,
    pub(crate) definition: ExtractionDefinition,
}

struct MaterializedDefinition {
    request: ExtractionRequest,
    runtime: RuntimeOptions,
    request_definition_output: Option<PendingExtractionDefinitionWrite>,
}

impl PreparedExtraction {
    #[cfg(test)]
    pub(crate) fn from_select(args: SelectArgs) -> Result<Self, CliError> {
        Self::from_select_with_logging(args, 0, false)
    }

    pub(crate) fn from_select_with_logging(
        args: SelectArgs,
        verbose: u8,
        quiet: bool,
    ) -> Result<Self, CliError> {
        let display_command =
            htmlcut_core::cli_operation_display_command(htmlcut_core::OperationId::SelectExtract)
                .expect("select extract should stay CLI-visible");
        if args.definition.request_file.is_some() {
            ensure_inline_select_request_is_default(&args)?;
        }
        let materialized = materialize_extraction_definition(
            &args.definition,
            ExtractionStrategy::Selector,
            &display_command,
            htmlcut_core::OperationId::SelectExtract,
            || {
                let value = resolve_value_spec(args.output.value, args.output.attribute.clone())?;
                let preview_chars = validate_preview_chars(args.output.preview_chars)?;
                let strategy_args = StrategyArgs::Select {
                    css: required_cli_value(args.css, "--css")?,
                };
                let options = RequestBuildOptions {
                    value,
                    whitespace: args.output.whitespace,
                    rewrite_urls: args.output.rewrite_urls,
                    preview_chars,
                    include_source_text: args.output.include_source_text,
                };
                Ok((
                    build_extraction_request(
                        strategy_args,
                        &args.source,
                        &args.selection,
                        options,
                    )?,
                    build_runtime(&args.source)?,
                ))
            },
        )?;
        let request = materialized.request;
        let runtime = materialized.runtime;
        let value_type = request.extraction.value().value_type();
        let output = resolve_extract_output_mode_with_output_file(
            args.output.output,
            &value_type,
            args.output.bundle.as_deref(),
            args.output.output_file.as_deref(),
        )?;

        Ok(Self {
            command: htmlcut_core::cli_operation_report_command(
                htmlcut_core::OperationId::SelectExtract,
            )
            .expect("select extract should stay CLI-visible"),
            runtime,
            request,
            request_definition_output: materialized.request_definition_output,
            output,
            bundle: args.output.bundle,
            output_file: args.output.output_file,
            verbose,
            quiet,
        })
    }

    #[cfg(test)]
    pub(crate) fn from_slice(args: SliceArgs) -> Result<Self, CliError> {
        Self::from_slice_with_logging(args, 0, false)
    }

    pub(crate) fn from_slice_with_logging(
        args: SliceArgs,
        verbose: u8,
        quiet: bool,
    ) -> Result<Self, CliError> {
        let display_command =
            htmlcut_core::cli_operation_display_command(htmlcut_core::OperationId::SliceExtract)
                .expect("slice extract should stay CLI-visible");
        if args.definition.request_file.is_some() {
            ensure_inline_slice_request_is_default(&args)?;
        }
        let materialized = materialize_extraction_definition(
            &args.definition,
            ExtractionStrategy::Slice,
            &display_command,
            htmlcut_core::OperationId::SliceExtract,
            || {
                let value = resolve_value_spec(args.output.value, args.output.attribute.clone())?;
                let preview_chars = validate_preview_chars(args.output.preview_chars)?;
                let strategy_args = StrategyArgs::Slice {
                    from: required_cli_value(args.from, "--from")?,
                    to: required_cli_value(args.to, "--to")?,
                    pattern: args.pattern,
                    regex_flags: args.regex_flags,
                    include_start: args.include_start,
                    include_end: args.include_end,
                };
                let options = RequestBuildOptions {
                    value,
                    whitespace: args.output.whitespace,
                    rewrite_urls: args.output.rewrite_urls,
                    preview_chars,
                    include_source_text: args.output.include_source_text,
                };
                Ok((
                    build_extraction_request(
                        strategy_args,
                        &args.source,
                        &args.selection,
                        options,
                    )?,
                    build_runtime(&args.source)?,
                ))
            },
        )?;
        let request = materialized.request;
        let runtime = materialized.runtime;
        let value_type = request.extraction.value().value_type();
        let output = resolve_extract_output_mode_with_output_file(
            args.output.output,
            &value_type,
            args.output.bundle.as_deref(),
            args.output.output_file.as_deref(),
        )?;

        Ok(Self {
            command: htmlcut_core::cli_operation_report_command(
                htmlcut_core::OperationId::SliceExtract,
            )
            .expect("slice extract should stay CLI-visible"),
            runtime,
            request,
            request_definition_output: materialized.request_definition_output,
            output,
            bundle: args.output.bundle,
            output_file: args.output.output_file,
            verbose,
            quiet,
        })
    }
}

impl PreparedSourceInspection {
    #[cfg(test)]
    pub(crate) fn new(args: InspectSourceArgs) -> Result<Self, CliError> {
        Self::new_with_logging(args, 0, false)
    }

    pub(crate) fn new_with_logging(
        args: InspectSourceArgs,
        verbose: u8,
        quiet: bool,
    ) -> Result<Self, CliError> {
        let preview_chars = validate_preview_chars(args.preview_chars)?;
        let runtime = build_runtime(&args.source)?;
        let source = build_source_request(&args.source)?;
        Ok(Self {
            command: htmlcut_core::cli_operation_report_command(
                htmlcut_core::OperationId::SourceInspect,
            )
            .expect("source inspect should stay CLI-visible"),
            source,
            runtime,
            output: args.output,
            preview_chars: preview_chars.get(),
            output_file: args.output_file,
            verbose,
            quiet,
            options: InspectionOptions {
                include_source_text: args.include_source_text,
                sample_limit: args.sample_limit,
            },
        })
    }
}

impl PreparedPreview {
    #[cfg(test)]
    pub(crate) fn from_select(args: InspectSelectArgs) -> Result<Self, CliError> {
        Self::from_select_with_logging(args, 0, false)
    }

    pub(crate) fn from_select_with_logging(
        args: InspectSelectArgs,
        verbose: u8,
        quiet: bool,
    ) -> Result<Self, CliError> {
        let display_command =
            htmlcut_core::cli_operation_display_command(htmlcut_core::OperationId::SelectPreview)
                .expect("select preview should stay CLI-visible");
        if args.definition.request_file.is_some() {
            ensure_inline_inspect_select_request_is_default(&args)?;
        }
        let materialized = materialize_extraction_definition(
            &args.definition,
            ExtractionStrategy::Selector,
            &display_command,
            htmlcut_core::OperationId::SelectPreview,
            || {
                let preview_chars = validate_preview_chars(args.output.preview_chars)?;
                Ok((
                    build_extraction_request(
                        StrategyArgs::Select {
                            css: required_cli_value(args.css, "--css")?,
                        },
                        &args.source,
                        &args.selection,
                        RequestBuildOptions {
                            value: ValueSpec::Structured,
                            whitespace: args.whitespace,
                            rewrite_urls: args.rewrite_urls,
                            preview_chars,
                            include_source_text: args.output.include_source_text,
                        },
                    )?,
                    build_runtime(&args.source)?,
                ))
            },
        )?;
        let mut request = materialized.request;
        let runtime = materialized.runtime;
        request.extraction = request.extraction.clone().with_value(ValueSpec::Structured);
        Ok(Self {
            command: htmlcut_core::cli_operation_report_command(
                htmlcut_core::OperationId::SelectPreview,
            )
            .expect("select preview should stay CLI-visible"),
            runtime,
            request,
            request_definition_output: materialized.request_definition_output,
            output: args.output.output,
            output_file: args.output.output_file,
            verbose,
            quiet,
        })
    }

    #[cfg(test)]
    pub(crate) fn from_slice(args: InspectSliceArgs) -> Result<Self, CliError> {
        Self::from_slice_with_logging(args, 0, false)
    }

    pub(crate) fn from_slice_with_logging(
        args: InspectSliceArgs,
        verbose: u8,
        quiet: bool,
    ) -> Result<Self, CliError> {
        let display_command =
            htmlcut_core::cli_operation_display_command(htmlcut_core::OperationId::SlicePreview)
                .expect("slice preview should stay CLI-visible");
        if args.definition.request_file.is_some() {
            ensure_inline_inspect_slice_request_is_default(&args)?;
        }
        let materialized = materialize_extraction_definition(
            &args.definition,
            ExtractionStrategy::Slice,
            &display_command,
            htmlcut_core::OperationId::SlicePreview,
            || {
                let preview_chars = validate_preview_chars(args.output.preview_chars)?;
                Ok((
                    build_extraction_request(
                        StrategyArgs::Slice {
                            from: required_cli_value(args.from, "--from")?,
                            to: required_cli_value(args.to, "--to")?,
                            pattern: args.pattern,
                            regex_flags: args.regex_flags,
                            include_start: args.include_start,
                            include_end: args.include_end,
                        },
                        &args.source,
                        &args.selection,
                        RequestBuildOptions {
                            value: ValueSpec::Structured,
                            whitespace: args.whitespace,
                            rewrite_urls: args.rewrite_urls,
                            preview_chars,
                            include_source_text: args.output.include_source_text,
                        },
                    )?,
                    build_runtime(&args.source)?,
                ))
            },
        )?;
        let mut request = materialized.request;
        request.extraction = request.extraction.clone().with_value(ValueSpec::Structured);
        Ok(Self {
            command: htmlcut_core::cli_operation_report_command(
                htmlcut_core::OperationId::SlicePreview,
            )
            .expect("slice preview should stay CLI-visible"),
            runtime: materialized.runtime,
            request,
            request_definition_output: materialized.request_definition_output,
            output: args.output.output,
            output_file: args.output.output_file,
            verbose,
            quiet,
        })
    }
}

pub(crate) enum StrategyArgs {
    Select {
        css: String,
    },
    Slice {
        from: String,
        to: String,
        pattern: CliPatternMode,
        regex_flags: Option<String>,
        include_start: bool,
        include_end: bool,
    },
}

fn required_cli_value(value: Option<String>, parameter: &'static str) -> Result<String, CliError> {
    value.ok_or_else(|| {
        usage_error(
            "CLI_REQUIRED_PARAMETER_MISSING",
            format!("{parameter} is required unless --request-file is used."),
        )
    })
}

fn materialize_extraction_definition<Build>(
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
            "CLI_REQUEST_FILE_READ_FAILED",
            format!(
                "Could not read extraction definition {}: {error}",
                path.display()
            ),
        )
    })?;
    let value: Value = serde_json::from_str(&raw).map_err(|error| {
        usage_error(
            "CLI_REQUEST_FILE_INVALID",
            format!(
                "Could not parse extraction definition {} as JSON: {error}. {}",
                path.display(),
                request_file_recovery_hint(operation_id, expected_strategy, None)
            ),
        )
    })?;
    let mut deserializer = serde_json::Deserializer::from_str(&raw);
    let definition: ExtractionDefinition = serde_path_to_error::deserialize(&mut deserializer)
        .map_err(|error| {
            let json_path = render_json_error_path(&error);
            usage_error(
                "CLI_REQUEST_FILE_INVALID",
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
                        request_file_shape_hint(&value, expected_strategy).as_deref()
                    )
                ),
            )
        })?;

    if definition.schema_name != htmlcut_core::EXTRACTION_DEFINITION_SCHEMA_NAME
        || definition.schema_version != htmlcut_core::EXTRACTION_DEFINITION_SCHEMA_VERSION
    {
        return Err(usage_error(
            "CLI_REQUEST_FILE_SCHEMA_UNSUPPORTED",
            format!(
                "Unsupported extraction definition schema in {}: expected {}@{}, got {}@{}.",
                path.display(),
                htmlcut_core::EXTRACTION_DEFINITION_SCHEMA_NAME,
                htmlcut_core::EXTRACTION_DEFINITION_SCHEMA_VERSION,
                definition.schema_name,
                definition.schema_version,
            ),
        ));
    }

    if definition.request.extraction.strategy() != expected_strategy {
        return Err(usage_error(
            "CLI_REQUEST_FILE_STRATEGY_MISMATCH",
            format!(
                "{} cannot execute a {} extraction definition from {}.",
                command,
                strategy_label(definition.request.extraction.strategy()),
                path.display(),
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
                == htmlcut_core::cli_operation_display_command(
                    htmlcut_core::OperationId::SelectExtract,
                )
                .expect("select extract should stay CLI-visible") =>
        {
            htmlcut_core::OperationId::SelectExtract
        }
        (command, ExtractionStrategy::Slice)
            if command
                == htmlcut_core::cli_operation_display_command(
                    htmlcut_core::OperationId::SliceExtract,
                )
                .expect("slice extract should stay CLI-visible") =>
        {
            htmlcut_core::OperationId::SliceExtract
        }
        (command, ExtractionStrategy::Selector)
            if command
                == htmlcut_core::cli_operation_display_command(
                    htmlcut_core::OperationId::SelectPreview,
                )
                .expect("select preview should stay CLI-visible") =>
        {
            htmlcut_core::OperationId::SelectPreview
        }
        (command, ExtractionStrategy::Slice)
            if command
                == htmlcut_core::cli_operation_display_command(
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
    if path.is_empty() {
        "$".to_owned()
    } else if path.starts_with('.') {
        format!("${path}")
    } else {
        format!("$.{path}")
    }
}

fn request_file_recovery_hint(
    operation_id: htmlcut_core::OperationId,
    expected_strategy: ExtractionStrategy,
    shape_hint: Option<&str>,
) -> String {
    let mut hint = format!(
        "Use `htmlcut schema --name {} --output json` for the exact request-file shape and `htmlcut catalog --operation {} --output json` for the command contract.",
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

fn ensure_inline_select_request_is_default(args: &SelectArgs) -> Result<(), CliError> {
    let mut conflicts = collect_source_request_file_conflicts(&args.source);
    push_conflict(&mut conflicts, args.css.is_some(), "--css");
    extend_selection_request_file_conflicts(&mut conflicts, &args.selection);
    extend_extract_request_file_conflicts(&mut conflicts, &args.output);
    reject_request_file_conflicts(conflicts)
}

fn ensure_inline_slice_request_is_default(args: &SliceArgs) -> Result<(), CliError> {
    let mut conflicts = collect_source_request_file_conflicts(&args.source);
    push_conflict(&mut conflicts, args.from.is_some(), "--from");
    push_conflict(&mut conflicts, args.to.is_some(), "--to");
    push_conflict(
        &mut conflicts,
        args.pattern != CliPatternMode::Literal,
        "--pattern",
    );
    push_conflict(&mut conflicts, args.regex_flags.is_some(), "--regex-flags");
    push_conflict(&mut conflicts, args.include_start, "--include-start");
    push_conflict(&mut conflicts, args.include_end, "--include-end");
    extend_selection_request_file_conflicts(&mut conflicts, &args.selection);
    extend_extract_request_file_conflicts(&mut conflicts, &args.output);
    reject_request_file_conflicts(conflicts)
}

fn ensure_inline_inspect_select_request_is_default(
    args: &InspectSelectArgs,
) -> Result<(), CliError> {
    let mut conflicts = collect_source_request_file_conflicts(&args.source);
    push_conflict(&mut conflicts, args.css.is_some(), "--css");
    extend_selection_request_file_conflicts(&mut conflicts, &args.selection);
    push_conflict(
        &mut conflicts,
        args.whitespace != CliWhitespaceMode::Preserve,
        "--whitespace",
    );
    push_conflict(&mut conflicts, args.rewrite_urls, "--rewrite-urls");
    extend_inspect_request_file_conflicts(&mut conflicts, &args.output);
    reject_request_file_conflicts(conflicts)
}

fn ensure_inline_inspect_slice_request_is_default(args: &InspectSliceArgs) -> Result<(), CliError> {
    let mut conflicts = collect_source_request_file_conflicts(&args.source);
    push_conflict(&mut conflicts, args.from.is_some(), "--from");
    push_conflict(&mut conflicts, args.to.is_some(), "--to");
    push_conflict(
        &mut conflicts,
        args.pattern != CliPatternMode::Literal,
        "--pattern",
    );
    push_conflict(&mut conflicts, args.regex_flags.is_some(), "--regex-flags");
    push_conflict(&mut conflicts, args.include_start, "--include-start");
    push_conflict(&mut conflicts, args.include_end, "--include-end");
    extend_selection_request_file_conflicts(&mut conflicts, &args.selection);
    push_conflict(
        &mut conflicts,
        args.whitespace != CliWhitespaceMode::Preserve,
        "--whitespace",
    );
    push_conflict(&mut conflicts, args.rewrite_urls, "--rewrite-urls");
    extend_inspect_request_file_conflicts(&mut conflicts, &args.output);
    reject_request_file_conflicts(conflicts)
}

fn collect_source_request_file_conflicts(source: &SourceArgs) -> Vec<&'static str> {
    let mut conflicts = Vec::new();
    push_conflict(&mut conflicts, source.input.is_some(), "<INPUT>");
    push_conflict(&mut conflicts, source.base_url.is_some(), "--base-url");
    push_conflict(
        &mut conflicts,
        source.max_bytes != DEFAULT_MAX_BYTES.to_string(),
        "--max-bytes",
    );
    push_conflict(
        &mut conflicts,
        source.fetch_timeout_ms != DEFAULT_FETCH_TIMEOUT_MS,
        "--fetch-timeout-ms",
    );
    push_conflict(
        &mut conflicts,
        source.fetch_preflight != CliFetchPreflightMode::HeadFirst,
        "--fetch-preflight",
    );
    conflicts
}

fn extend_selection_request_file_conflicts(
    conflicts: &mut Vec<&'static str>,
    selection: &SelectionArgs,
) {
    push_conflict(
        conflicts,
        selection.r#match != CliMatchMode::First,
        "--match",
    );
    push_conflict(conflicts, selection.index.is_some(), "--index");
}

fn extend_extract_request_file_conflicts(
    conflicts: &mut Vec<&'static str>,
    output: &ExtractOutputArgs,
) {
    push_conflict(conflicts, output.value != CliValueMode::Text, "--value");
    push_conflict(conflicts, output.attribute.is_some(), "--attribute");
    push_conflict(
        conflicts,
        output.whitespace != CliWhitespaceMode::Preserve,
        "--whitespace",
    );
    push_conflict(conflicts, output.rewrite_urls, "--rewrite-urls");
    push_conflict(
        conflicts,
        output.preview_chars != DEFAULT_PREVIEW_CHARS,
        "--preview-chars",
    );
    push_conflict(
        conflicts,
        output.include_source_text,
        "--include-source-text",
    );
}

fn extend_inspect_request_file_conflicts(
    conflicts: &mut Vec<&'static str>,
    output: &crate::args::InspectOutputArgs,
) {
    push_conflict(
        conflicts,
        output.preview_chars != DEFAULT_PREVIEW_CHARS,
        "--preview-chars",
    );
    push_conflict(
        conflicts,
        output.include_source_text,
        "--include-source-text",
    );
}

fn push_conflict(conflicts: &mut Vec<&'static str>, condition: bool, flag: &'static str) {
    if condition {
        conflicts.push(flag);
    }
}

fn reject_request_file_conflicts(conflicts: Vec<&'static str>) -> Result<(), CliError> {
    if conflicts.is_empty() {
        return Ok(());
    }

    Err(usage_error(
        "CLI_REQUEST_FILE_CONFLICT",
        format!(
            "--request-file owns the extraction definition; remove the inline request flags: {}. If you want to keep the inline form, drop `--request-file` and use `--emit-request-file <PATH>` to save the canonical definition.",
            conflicts.join(", ")
        ),
    ))
}

pub(crate) fn build_extraction_request(
    strategy_args: StrategyArgs,
    source_args: &SourceArgs,
    selection_args: &SelectionArgs,
    options: RequestBuildOptions,
) -> Result<ExtractionRequest, CliError> {
    let source = build_source_request(source_args)?;
    let selection = resolve_selection_spec(selection_args)?;
    let extraction = match strategy_args {
        StrategyArgs::Select { css } => ExtractionSpec::selector(parse_selector_query(css)?),
        StrategyArgs::Slice {
            from,
            to,
            pattern,
            regex_flags,
            include_start,
            include_end,
        } => {
            let from = parse_slice_boundary(from)?;
            let to = parse_slice_boundary(to)?;
            let pattern = build_slice_pattern(pattern, regex_flags, from, to)?;
            ExtractionSpec::slice(SliceSpec {
                pattern,
                include_start,
                include_end,
            })
        }
    }
    .with_selection(selection)
    .with_value(options.value);

    let mut request = ExtractionRequest::new(source, extraction);
    request.normalization = NormalizationOptions {
        whitespace: match options.whitespace {
            CliWhitespaceMode::Preserve => WhitespaceMode::Preserve,
            CliWhitespaceMode::Normalize => WhitespaceMode::Normalize,
        },
        rewrite_urls: options.rewrite_urls,
    };
    request.output = OutputOptions {
        include_source_text: options.include_source_text,
        include_html: true,
        include_text: true,
        preview_chars: options.preview_chars,
    };
    Ok(request)
}

pub(crate) fn build_source_request(args: &SourceArgs) -> Result<SourceRequest, CliError> {
    let input = required_cli_value(args.input.clone(), "<INPUT>")?;
    let base_url = validate_base_url(args.base_url.as_deref())?;
    let mut source = if input == "-" {
        SourceRequest::stdin()
    } else if input.starts_with("http://") || input.starts_with("https://") {
        SourceRequest::url(validate_input_url(&input)?)
    } else {
        SourceRequest {
            input: SourceInput::File {
                path: PathBuf::from(input),
            },
            base_url: None,
        }
    };
    if let Some(base_url) = base_url {
        source = source.with_base_url(base_url);
    }

    Ok(source)
}

pub(crate) fn build_runtime(args: &SourceArgs) -> Result<RuntimeOptions, CliError> {
    Ok(RuntimeOptions {
        max_bytes: parse_byte_size(&args.max_bytes)?,
        fetch_timeout_ms: args.fetch_timeout_ms,
        fetch_preflight: match args.fetch_preflight {
            CliFetchPreflightMode::HeadFirst => FetchPreflightMode::HeadFirst,
            CliFetchPreflightMode::GetOnly => FetchPreflightMode::GetOnly,
        },
    })
}

pub(crate) fn resolve_selection_spec(args: &SelectionArgs) -> Result<SelectionSpec, CliError> {
    match args.r#match {
        CliMatchMode::Single => {
            if args.index.is_some() {
                return Err(usage_error(
                    "CLI_MATCH_INDEX_CONFLICT",
                    "--index can only be used with --match nth.",
                ));
            }
            Ok(SelectionSpec::single())
        }
        CliMatchMode::First => {
            if args.index.is_some() {
                return Err(usage_error(
                    "CLI_MATCH_INDEX_CONFLICT",
                    "--index can only be used with --match nth.",
                ));
            }
            Ok(SelectionSpec::First)
        }
        CliMatchMode::Nth => {
            let Some(index) = args.index else {
                return Err(usage_error(
                    "CLI_MATCH_INDEX_REQUIRED",
                    "--index is required with --match nth.",
                ));
            };
            let Some(index) = NonZeroUsize::new(index) else {
                return Err(usage_error(
                    "CLI_MATCH_INDEX_INVALID",
                    "--index must be a positive integer.",
                ));
            };
            Ok(SelectionSpec::nth(index))
        }
        CliMatchMode::All => {
            if args.index.is_some() {
                return Err(usage_error(
                    "CLI_MATCH_INDEX_CONFLICT",
                    "--index can only be used with --match nth.",
                ));
            }
            Ok(SelectionSpec::All)
        }
    }
}

pub(crate) fn resolve_value_spec(
    value_mode: CliValueMode,
    attribute: Option<String>,
) -> Result<ValueSpec, CliError> {
    match value_mode {
        CliValueMode::Text => {
            reject_attribute_conflict(attribute)?;
            Ok(ValueSpec::Text)
        }
        CliValueMode::InnerHtml => {
            reject_attribute_conflict(attribute)?;
            Ok(ValueSpec::InnerHtml)
        }
        CliValueMode::OuterHtml => {
            reject_attribute_conflict(attribute)?;
            Ok(ValueSpec::OuterHtml)
        }
        CliValueMode::Attribute => {
            let Some(attribute) = attribute else {
                return Err(usage_error(
                    "CLI_ATTRIBUTE_REQUIRED",
                    "--attribute is required with --value attribute.",
                ));
            };
            Ok(ValueSpec::Attribute {
                name: AttributeName::new(attribute)
                    .map_err(|error| usage_error("CLI_ATTRIBUTE_INVALID", error.to_string()))?,
            })
        }
        CliValueMode::Structured => {
            reject_attribute_conflict(attribute)?;
            Ok(ValueSpec::Structured)
        }
    }
}

#[cfg(test)]
pub(crate) fn resolve_extract_output_mode(
    requested: Option<CliOutputMode>,
    value_type: &ValueType,
    bundle: Option<&Path>,
) -> Result<CliOutputMode, CliError> {
    resolve_extract_output_mode_with_output_file(requested, value_type, bundle, None)
}

pub(crate) fn resolve_extract_output_mode_with_output_file(
    requested: Option<CliOutputMode>,
    value_type: &ValueType,
    bundle: Option<&Path>,
    output_file: Option<&Path>,
) -> Result<CliOutputMode, CliError> {
    let output = requested.unwrap_or(default_output_for_value(value_type));
    if output == CliOutputMode::None && bundle.is_none() {
        return Err(usage_error(
            "CLI_OUTPUT_NONE_WITHOUT_BUNDLE",
            "--output none requires --bundle so the command still produces artifacts.",
        ));
    }

    if output == CliOutputMode::None && output_file.is_some() {
        return Err(usage_error(
            "CLI_OUTPUT_FILE_REQUIRES_STDOUT_PAYLOAD",
            "--output-file cannot be used with --output none because no stdout payload is produced.",
        ));
    }

    if output == CliOutputMode::Html {
        match value_type {
            ValueType::InnerHtml | ValueType::OuterHtml => {}
            _ => {
                return Err(usage_error(
                    "CLI_OUTPUT_HTML_INVALID",
                    "--output html can only be used with --value inner-html or --value outer-html.",
                ));
            }
        }
    }

    if *value_type == ValueType::Structured {
        match output {
            CliOutputMode::Json | CliOutputMode::None => {}
            _ => {
                return Err(usage_error(
                    "CLI_STRUCTURED_OUTPUT_INVALID",
                    "Structured extraction only supports --output json or --output none.",
                ));
            }
        }
    }

    Ok(output)
}

pub(crate) fn resolve_regex_flags(
    pattern: CliPatternMode,
    regex_flags: Option<String>,
) -> Result<Option<String>, CliError> {
    match pattern {
        CliPatternMode::Literal => {
            if regex_flags.is_some() {
                return Err(usage_error(
                    "CLI_REGEX_FLAGS_CONFLICT",
                    "--regex-flags can only be used with --pattern regex.",
                ));
            }
            Ok(None)
        }
        CliPatternMode::Regex => Ok(Some(regex_flags.unwrap_or_else(default_regex_flags))),
    }
}

fn build_slice_pattern(
    pattern: CliPatternMode,
    regex_flags: Option<String>,
    from: SliceBoundary,
    to: SliceBoundary,
) -> Result<SlicePatternSpec, CliError> {
    match resolve_regex_flags(pattern, regex_flags)? {
        Some(flags) => Ok(SlicePatternSpec::regex(from, to, flags)),
        None => Ok(SlicePatternSpec::literal(from, to)),
    }
}

pub(crate) fn default_output_for_value(value_type: &ValueType) -> CliOutputMode {
    match value_type {
        ValueType::InnerHtml | ValueType::OuterHtml => CliOutputMode::Html,
        ValueType::Structured => CliOutputMode::Json,
        _ => CliOutputMode::Text,
    }
}

pub(crate) fn extract_prefers_json(args: &ExtractOutputArgs) -> bool {
    args.output == Some(CliOutputMode::Json)
        || (args.output.is_none() && args.value == CliValueMode::Structured)
}

pub(crate) fn validate_base_url(base_url: Option<&str>) -> Result<Option<Url>, CliError> {
    let Some(value) = base_url else {
        return Ok(None);
    };

    Ok(Some(validate_http_url(
        value,
        "CLI_BASE_URL_INVALID",
        "CLI_BASE_URL_SCHEME_INVALID",
    )?))
}

pub(crate) fn validate_preview_chars(preview_chars: usize) -> Result<NonZeroUsize, CliError> {
    NonZeroUsize::new(preview_chars).ok_or_else(|| {
        usage_error(
            "CLI_PREVIEW_CHARS_INVALID",
            "--preview-chars must be greater than zero.",
        )
    })
}

fn reject_attribute_conflict(attribute: Option<String>) -> Result<(), CliError> {
    if attribute.is_some() {
        return Err(usage_error(
            "CLI_ATTRIBUTE_CONFLICT",
            "--attribute can only be used with --value attribute.",
        ));
    }

    Ok(())
}

fn parse_selector_query(selector: String) -> Result<SelectorQuery, CliError> {
    SelectorQuery::new(selector)
        .map_err(|error| usage_error("CLI_SELECTOR_INVALID", error.to_string()))
}

fn parse_slice_boundary(boundary: String) -> Result<SliceBoundary, CliError> {
    SliceBoundary::new(boundary)
        .map_err(|error| usage_error("CLI_SLICE_BOUNDARY_INVALID", error.to_string()))
}

fn validate_input_url(value: &str) -> Result<Url, CliError> {
    validate_http_url(
        value,
        "CLI_SOURCE_URL_INVALID",
        "CLI_SOURCE_URL_SCHEME_INVALID",
    )
}

fn validate_http_url(
    value: &str,
    invalid_code: &'static str,
    invalid_scheme_code: &'static str,
) -> Result<Url, CliError> {
    let parsed = Url::parse(value)
        .map_err(|_| usage_error(invalid_code, format!("Invalid URL: {value}")))?;
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err(usage_error(
            invalid_scheme_code,
            "URLs must use http or https.",
        ));
    }

    Ok(parsed)
}

pub(crate) fn parse_byte_size(value: &str) -> Result<usize, CliError> {
    let normalized = value.trim().to_ascii_lowercase();
    let split_at = normalized
        .find(|character: char| !(character.is_ascii_digit() || character == '.'))
        .unwrap_or(normalized.len());
    let (amount_text, unit_text) = normalized.split_at(split_at);
    let amount = amount_text.parse::<f64>().map_err(|_| {
        usage_error(
            "CLI_BYTE_SIZE_INVALID",
            format!("Invalid byte size: {value}"),
        )
    })?;
    let multiplier = match unit_text.trim() {
        "" | "b" => 1f64,
        "kb" => KIBIBYTE as f64,
        "mb" => MEBIBYTE as f64,
        "gb" => GIBIBYTE as f64,
        _ => {
            return Err(usage_error(
                "CLI_BYTE_SIZE_INVALID",
                format!("Invalid byte size: {value}"),
            ));
        }
    };

    let bytes = amount * multiplier;
    if !bytes.is_finite() || bytes <= 0.0 {
        return Err(usage_error(
            "CLI_BYTE_SIZE_INVALID",
            format!("Invalid byte size: {value}"),
        ));
    }

    Ok(bytes.floor() as usize)
}

pub(crate) fn build_extraction_report(
    command: impl Into<String>,
    result: ExtractionResult,
    bundle: Option<BundlePaths>,
) -> ExtractionCommandReport {
    ExtractionCommandReport {
        tool: TOOL_NAME.to_owned(),
        engine: ENGINE_NAME.to_owned(),
        version: HTMLCUT_VERSION.to_owned(),
        schema_name: EXTRACTION_COMMAND_REPORT_SCHEMA_NAME.to_owned(),
        schema_version: EXTRACTION_COMMAND_REPORT_SCHEMA_VERSION,
        command: command.into(),
        operation_id: result.operation_id,
        ok: result.ok,
        source: result.source,
        extraction: result.extraction,
        stats: result.stats,
        document_title: result.document_title,
        matches: result.matches,
        diagnostics: result.diagnostics,
        bundle,
    }
}

pub(crate) fn build_source_inspection_report(
    command: impl Into<String>,
    result: SourceInspectionResult,
) -> SourceInspectionCommandReport {
    SourceInspectionCommandReport {
        tool: TOOL_NAME.to_owned(),
        engine: ENGINE_NAME.to_owned(),
        version: HTMLCUT_VERSION.to_owned(),
        schema_name: SOURCE_INSPECTION_COMMAND_REPORT_SCHEMA_NAME.to_owned(),
        schema_version: SOURCE_INSPECTION_COMMAND_REPORT_SCHEMA_VERSION,
        command: command.into(),
        operation_id: result.operation_id,
        ok: result.ok,
        source: result.source,
        document: result.document,
        diagnostics: result.diagnostics,
    }
}

pub(crate) fn build_catalog_report(
    operation_filter: Option<&str>,
) -> Result<CatalogCommandReport, CliError> {
    let requested_operation = operation_filter
        .map(|operation_id| {
            operation_id
                .parse::<htmlcut_core::OperationId>()
                .map_err(|_| unknown_operation_id_error(operation_id))
        })
        .transpose()?;

    let operations = htmlcut_core::operation_catalog()
        .iter()
        .filter(|descriptor| {
            requested_operation.is_none_or(|operation_id| descriptor.id == operation_id)
        })
        .map(|descriptor| {
            let cli_contract = htmlcut_core::cli_operation_contract(descriptor.id);
            CatalogOperationReport {
                operation_id: descriptor.id,
                command: cli_contract.map(|contract| contract.display_command()),
                availability: match cli_contract {
                    Some(_) => CatalogAvailability::Cli,
                    None => CatalogAvailability::CoreOnly,
                },
                summary: descriptor.description.to_owned(),
                core_surface: descriptor.core_surface.to_owned(),
                request_contract: build_contract_surface(&descriptor.request_contract),
                result_contract: build_contract_surface(&descriptor.result_contract),
                command_contract: cli_contract.map(build_catalog_command_contract),
            }
        })
        .collect::<Vec<_>>();

    Ok(CatalogCommandReport {
        tool: TOOL_NAME.to_owned(),
        version: HTMLCUT_VERSION.to_owned(),
        schema_name: CATALOG_REPORT_SCHEMA_NAME.to_owned(),
        schema_version: CATALOG_SCHEMA_VERSION,
        schema_profile: HTMLCUT_JSON_SCHEMA_PROFILE.to_owned(),
        description: HTMLCUT_DESCRIPTION.to_owned(),
        command: "catalog".to_owned(),
        operations,
    })
}

pub(crate) fn build_schema_report(
    name_filter: Option<&str>,
    version_filter: Option<u32>,
) -> Result<SchemaCommandReport, CliError> {
    if let (Some(_), None) = (version_filter, name_filter) {
        return Err(usage_error(
            "CLI_SCHEMA_VERSION_REQUIRES_NAME",
            "`--schema-version` requires `--name`.",
        ));
    }

    let mut schemas = htmlcut_core::schema_catalog()
        .iter()
        .map(build_schema_document_report)
        .chain(
            cli_schema_catalog()
                .iter()
                .map(build_schema_document_report),
        )
        .collect::<Vec<_>>();

    schemas.sort_by(|left, right| {
        left.schema_name
            .cmp(&right.schema_name)
            .then(left.schema_version.cmp(&right.schema_version))
    });

    let filtered = schemas
        .iter()
        .filter(|schema| name_filter.is_none_or(|name| schema.schema_name == name))
        .filter(|schema| version_filter.is_none_or(|version| schema.schema_version == version))
        .cloned()
        .collect::<Vec<_>>();

    if filtered.is_empty() {
        let name = name_filter.expect("version-only filters return earlier");
        return Err(unknown_schema_error(name, version_filter, &schemas));
    }

    Ok(SchemaCommandReport {
        tool: TOOL_NAME.to_owned(),
        version: HTMLCUT_VERSION.to_owned(),
        schema_name: SCHEMA_COMMAND_REPORT_SCHEMA_NAME.to_owned(),
        schema_version: SCHEMA_COMMAND_REPORT_SCHEMA_VERSION,
        schema_profile: HTMLCUT_JSON_SCHEMA_PROFILE.to_owned(),
        description: HTMLCUT_DESCRIPTION.to_owned(),
        command: "schema".to_owned(),
        schemas: filtered,
    })
}

pub(crate) fn default_regex_flags() -> String {
    DEFAULT_REGEX_FLAGS.to_owned()
}

fn build_contract_surface(contract: &htmlcut_core::OperationContract) -> CatalogContractSurface {
    CatalogContractSurface {
        rust_shape: contract.rust_shape.to_owned(),
        schema_refs: contract
            .schema_refs
            .iter()
            .map(build_schema_ref_report)
            .collect(),
    }
}

fn build_schema_ref_report(schema_ref: &htmlcut_core::SchemaRef) -> SchemaRefReport {
    SchemaRefReport {
        schema_name: schema_ref.schema_name.to_owned(),
        schema_version: schema_ref.schema_version,
    }
}

const CLI_SCHEMA_CATALOG: &[htmlcut_core::SchemaDescriptor] = &[
    htmlcut_core::SchemaDescriptor {
        schema_ref: htmlcut_core::SchemaRef::new(
            EXTRACTION_COMMAND_REPORT_SCHEMA_NAME,
            EXTRACTION_COMMAND_REPORT_SCHEMA_VERSION,
        ),
        owner_surface: "htmlcut-cli",
        rust_shape: "ExtractionCommandReport",
        stability: SchemaStability::Versioned,
        json_schema: extraction_command_report_schema,
    },
    htmlcut_core::SchemaDescriptor {
        schema_ref: htmlcut_core::SchemaRef::new(
            SOURCE_INSPECTION_COMMAND_REPORT_SCHEMA_NAME,
            SOURCE_INSPECTION_COMMAND_REPORT_SCHEMA_VERSION,
        ),
        owner_surface: "htmlcut-cli",
        rust_shape: "SourceInspectionCommandReport",
        stability: SchemaStability::Versioned,
        json_schema: source_inspection_command_report_schema,
    },
    htmlcut_core::SchemaDescriptor {
        schema_ref: htmlcut_core::SchemaRef::new(
            CATALOG_REPORT_SCHEMA_NAME,
            CATALOG_SCHEMA_VERSION,
        ),
        owner_surface: "htmlcut-cli",
        rust_shape: "CatalogCommandReport",
        stability: SchemaStability::Versioned,
        json_schema: catalog_command_report_schema,
    },
    htmlcut_core::SchemaDescriptor {
        schema_ref: htmlcut_core::SchemaRef::new(
            SCHEMA_COMMAND_REPORT_SCHEMA_NAME,
            SCHEMA_COMMAND_REPORT_SCHEMA_VERSION,
        ),
        owner_surface: "htmlcut-cli",
        rust_shape: "SchemaCommandReport",
        stability: SchemaStability::Versioned,
        json_schema: schema_command_report_schema,
    },
];

fn cli_schema_catalog() -> &'static [htmlcut_core::SchemaDescriptor] {
    CLI_SCHEMA_CATALOG
}

fn build_schema_document_report(
    descriptor: &htmlcut_core::SchemaDescriptor,
) -> SchemaDocumentReport {
    SchemaDocumentReport {
        schema_name: descriptor.schema_ref.schema_name.to_owned(),
        schema_version: descriptor.schema_ref.schema_version,
        owner_surface: descriptor.owner_surface.to_owned(),
        rust_shape: descriptor.rust_shape.to_owned(),
        stability: descriptor.stability,
        json_schema: (descriptor.json_schema)(),
    }
}

fn schema_json_for<T: schemars::JsonSchema>() -> Value {
    serde_json::to_value(schema_for!(T)).expect("JSON Schema documents should always serialize")
}

fn extraction_command_report_schema() -> Value {
    schema_json_for::<ExtractionCommandReport>()
}

fn source_inspection_command_report_schema() -> Value {
    schema_json_for::<SourceInspectionCommandReport>()
}

fn catalog_command_report_schema() -> Value {
    schema_json_for::<CatalogCommandReport>()
}

fn schema_command_report_schema() -> Value {
    schema_json_for::<SchemaCommandReport>()
}

fn build_catalog_command_contract(
    descriptor: &htmlcut_core::OperationCliContract,
) -> CatalogCommandContract {
    CatalogCommandContract {
        invocation: descriptor.invocation.to_owned(),
        inputs: descriptor
            .inputs
            .iter()
            .copied()
            .map(|input| input.description().to_owned())
            .collect(),
        default_match: descriptor.default_match.map(render_selection_mode),
        selection_modes: descriptor
            .selection_modes
            .iter()
            .copied()
            .map(render_selection_mode)
            .collect(),
        default_value: descriptor.default_value.map(render_value_type),
        value_modes: descriptor
            .value_modes
            .iter()
            .copied()
            .map(render_value_type)
            .collect(),
        default_output: descriptor.default_output.map(render_output_mode),
        default_output_overrides: descriptor
            .default_output_overrides
            .iter()
            .map(build_conditional_default)
            .collect(),
        output_modes: descriptor
            .output_modes
            .iter()
            .copied()
            .map(render_output_mode)
            .collect(),
        constraints: descriptor
            .constraints
            .iter()
            .map(build_constraint)
            .collect(),
        notes: descriptor
            .notes
            .iter()
            .map(|note| (*note).to_owned())
            .collect(),
        examples: descriptor
            .examples
            .iter()
            .map(|example| (*example).to_owned())
            .collect(),
        parameters: descriptor
            .parameters
            .iter()
            .map(build_parameter_spec)
            .collect(),
    }
}

fn build_conditional_default(
    descriptor: &htmlcut_core::CliConditionalDefault,
) -> CatalogConditionalDefault {
    CatalogConditionalDefault {
        value: htmlcut_core::render_cli_value(descriptor.value),
        when: build_condition(&descriptor.when),
    }
}

fn build_constraint(descriptor: &htmlcut_core::CliConstraint) -> CatalogConstraint {
    match descriptor {
        htmlcut_core::CliConstraint::RequiresParameter { parameter, when } => {
            CatalogConstraint::RequiresParameter {
                parameter: parameter.to_string(),
                when: build_condition(when),
            }
        }
        htmlcut_core::CliConstraint::AllowedOnlyWhen { parameter, when } => {
            CatalogConstraint::AllowedOnlyWhen {
                parameter: parameter.to_string(),
                when: build_condition(when),
            }
        }
        htmlcut_core::CliConstraint::RestrictsParameterValues {
            parameter,
            allowed_values,
            when,
        } => CatalogConstraint::RestrictsParameterValues {
            parameter: parameter.to_string(),
            allowed_values: allowed_values
                .iter()
                .copied()
                .map(htmlcut_core::render_cli_value)
                .collect(),
            when: build_condition(when),
        },
    }
}

fn build_condition(condition: &htmlcut_core::CliCondition) -> CatalogCondition {
    CatalogCondition {
        parameter: condition.parameter.to_string(),
        values: condition
            .values
            .iter()
            .copied()
            .map(htmlcut_core::render_cli_value)
            .collect(),
    }
}

fn build_parameter_spec(parameter: &htmlcut_core::CliParameterDescriptor) -> CatalogParameterSpec {
    let (requirement, requirement_note) = render_parameter_requirement(&parameter.requirement);
    CatalogParameterSpec {
        section: parameter.section.to_string(),
        name: parameter.id.to_string(),
        kind: match parameter.kind {
            htmlcut_core::CliParameterKind::Positional => CatalogParameterKind::Positional,
            htmlcut_core::CliParameterKind::Option => CatalogParameterKind::Option,
            htmlcut_core::CliParameterKind::Flag => CatalogParameterKind::Flag,
        },
        requirement,
        requirement_note,
        value_hint: parameter.value_hint.map(str::to_owned),
        default: parameter.default.map(htmlcut_core::render_cli_value),
        allowed_values: parameter
            .allowed_values
            .iter()
            .copied()
            .map(htmlcut_core::render_cli_value)
            .collect(),
        summary: parameter.summary.to_owned(),
    }
}

fn render_parameter_requirement(
    requirement: &htmlcut_core::CliParameterRequirement,
) -> (CatalogParameterRequirement, Option<String>) {
    match requirement {
        htmlcut_core::CliParameterRequirement::Required => {
            (CatalogParameterRequirement::Required, None)
        }
        htmlcut_core::CliParameterRequirement::Optional => {
            (CatalogParameterRequirement::Optional, None)
        }
        htmlcut_core::CliParameterRequirement::RequiredUnless(parameter) => (
            CatalogParameterRequirement::Conditional,
            Some(format!("required unless {parameter} is used")),
        ),
        htmlcut_core::CliParameterRequirement::RequiredWhen(condition) => (
            CatalogParameterRequirement::Conditional,
            Some(format!(
                "required when {}",
                render_condition_expression(condition)
            )),
        ),
        htmlcut_core::CliParameterRequirement::AllowedOnlyWhen(condition) => (
            CatalogParameterRequirement::Conditional,
            Some(format!(
                "allowed only when {}",
                render_condition_expression(condition)
            )),
        ),
    }
}

fn render_condition_expression(condition: &htmlcut_core::CliCondition) -> String {
    let values = condition
        .values
        .iter()
        .copied()
        .map(htmlcut_core::render_cli_value)
        .collect::<Vec<_>>();

    match values.as_slice() {
        [single] => format!("{} {single} is used", condition.parameter),
        _ => format!("{} is one of {}", condition.parameter, values.join(", ")),
    }
}

#[cfg(test)]
pub(crate) fn render_condition_expression_for_tests(
    condition: &htmlcut_core::CliCondition,
) -> String {
    render_condition_expression(condition)
}

#[cfg(test)]
pub(crate) fn format_json_error_path_for_tests(path: &str) -> String {
    format_json_error_path(path)
}

fn render_selection_mode(mode: htmlcut_core::CliSelectionMode) -> String {
    htmlcut_core::render_cli_value(htmlcut_core::CliValue::SelectionMode(mode))
}

fn render_value_type(value_type: htmlcut_core::ValueType) -> String {
    htmlcut_core::render_cli_value(htmlcut_core::CliValue::ValueType(value_type))
}

fn render_output_mode(mode: htmlcut_core::CliOutputMode) -> String {
    htmlcut_core::render_cli_value(htmlcut_core::CliValue::OutputMode(mode))
}
