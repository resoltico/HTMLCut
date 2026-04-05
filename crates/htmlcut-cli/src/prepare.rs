use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};

use htmlcut_core::{
    AttributeName, DEFAULT_REGEX_FLAGS, ExtractionRequest, ExtractionResult, ExtractionSpec,
    HTMLCUT_JSON_SCHEMA_PROFILE, InspectionOptions, NormalizationOptions, OutputOptions,
    RuntimeOptions, SchemaStability, SelectionSpec, SelectorQuery, SliceBoundary, SlicePatternSpec,
    SliceSpec, SourceInput, SourceInspectionResult, SourceRequest, ValueSpec, ValueType,
    WhitespaceMode,
};
use schemars::schema_for;
use serde_json::Value;
use url::Url;

use crate::args::{
    CliMatchMode, CliOutputMode, CliPatternMode, CliValueMode, CliWhitespaceMode,
    ExtractOutputArgs, InspectSelectArgs, InspectSliceArgs, InspectSourceArgs, SelectArgs,
    SelectionArgs, SliceArgs, SourceArgs,
};
use crate::error::{CliError, usage_error};
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
    pub(crate) command: &'static str,
    pub(crate) request: ExtractionRequest,
    pub(crate) runtime: RuntimeOptions,
    pub(crate) output: CliOutputMode,
    pub(crate) bundle: Option<PathBuf>,
    pub(crate) verbose: u8,
}

pub(crate) struct PreparedSourceInspection {
    pub(crate) command: &'static str,
    pub(crate) source: SourceRequest,
    pub(crate) runtime: RuntimeOptions,
    pub(crate) options: InspectionOptions,
    pub(crate) output: crate::args::CliInspectOutputMode,
    pub(crate) preview_chars: usize,
}

pub(crate) struct PreparedPreview {
    pub(crate) command: &'static str,
    pub(crate) request: ExtractionRequest,
    pub(crate) runtime: RuntimeOptions,
    pub(crate) output: crate::args::CliInspectOutputMode,
}

pub(crate) struct RequestBuildOptions {
    pub(crate) value: ValueSpec,
    pub(crate) whitespace: CliWhitespaceMode,
    pub(crate) rewrite_urls: bool,
    pub(crate) preview_chars: NonZeroUsize,
    pub(crate) include_source_text: bool,
}

impl PreparedExtraction {
    #[cfg(test)]
    pub(crate) fn from_select(args: SelectArgs) -> Result<Self, CliError> {
        Self::from_select_with_verbose(args, 0)
    }

    pub(crate) fn from_select_with_verbose(
        args: SelectArgs,
        verbose: u8,
    ) -> Result<Self, CliError> {
        let value = resolve_value_spec(args.output.value, args.output.attribute.clone())?;
        let value_type = value.value_type();
        let preview_chars = validate_preview_chars(args.output.preview_chars)?;
        let output = resolve_extract_output_mode(
            args.output.output,
            &value_type,
            args.output.bundle.as_deref(),
        )?;
        let strategy_args = StrategyArgs::Select { css: args.css };
        let options = RequestBuildOptions {
            value,
            whitespace: args.output.whitespace,
            rewrite_urls: args.output.rewrite_urls,
            preview_chars,
            include_source_text: args.output.include_source_text,
        };
        let request =
            build_extraction_request(strategy_args, &args.source, &args.selection, options)?;

        Ok(Self {
            command: "select",
            runtime: build_runtime(&args.source)?,
            request,
            output,
            bundle: args.output.bundle,
            verbose,
        })
    }

    #[cfg(test)]
    pub(crate) fn from_slice(args: SliceArgs) -> Result<Self, CliError> {
        Self::from_slice_with_verbose(args, 0)
    }

    pub(crate) fn from_slice_with_verbose(args: SliceArgs, verbose: u8) -> Result<Self, CliError> {
        let value = resolve_value_spec(args.output.value, args.output.attribute.clone())?;
        let value_type = value.value_type();
        let preview_chars = validate_preview_chars(args.output.preview_chars)?;
        let output = resolve_extract_output_mode(
            args.output.output,
            &value_type,
            args.output.bundle.as_deref(),
        )?;
        let strategy_args = StrategyArgs::Slice {
            from: args.from,
            to: args.to,
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
        let request =
            build_extraction_request(strategy_args, &args.source, &args.selection, options)?;

        Ok(Self {
            command: "slice",
            runtime: build_runtime(&args.source)?,
            request,
            output,
            bundle: args.output.bundle,
            verbose,
        })
    }
}

impl PreparedSourceInspection {
    pub(crate) fn new(args: InspectSourceArgs) -> Result<Self, CliError> {
        let preview_chars = validate_preview_chars(args.preview_chars)?;
        let runtime = build_runtime(&args.source)?;
        let source = build_source_request(&args.source)?;
        Ok(Self {
            command: "inspect-source",
            source,
            runtime,
            output: args.output,
            preview_chars: preview_chars.get(),
            options: InspectionOptions {
                include_source_text: args.include_source_text,
                sample_limit: args.sample_limit,
            },
        })
    }
}

impl PreparedPreview {
    pub(crate) fn from_select(args: InspectSelectArgs) -> Result<Self, CliError> {
        let preview_chars = validate_preview_chars(args.output.preview_chars)?;
        Ok(Self {
            command: "inspect-select",
            runtime: build_runtime(&args.source)?,
            request: build_extraction_request(
                StrategyArgs::Select { css: args.css },
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
            output: args.output.output,
        })
    }

    pub(crate) fn from_slice(args: InspectSliceArgs) -> Result<Self, CliError> {
        let preview_chars = validate_preview_chars(args.output.preview_chars)?;
        Ok(Self {
            command: "inspect-slice",
            runtime: build_runtime(&args.source)?,
            request: build_extraction_request(
                StrategyArgs::Slice {
                    from: args.from,
                    to: args.to,
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
            output: args.output.output,
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
    let base_url = validate_base_url(args.base_url.as_deref())?;
    let mut source = if args.input == "-" {
        SourceRequest::stdin()
    } else if args.input.starts_with("http://") || args.input.starts_with("https://") {
        SourceRequest::url(validate_input_url(&args.input)?)
    } else {
        SourceRequest {
            input: SourceInput::File {
                path: PathBuf::from(&args.input),
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
        CliValueMode::Html => {
            reject_attribute_conflict(attribute)?;
            Ok(ValueSpec::Html)
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

pub(crate) fn resolve_extract_output_mode(
    requested: Option<CliOutputMode>,
    value_type: &ValueType,
    bundle: Option<&Path>,
) -> Result<CliOutputMode, CliError> {
    let output = requested.unwrap_or(default_output_for_value(value_type));
    if output == CliOutputMode::None && bundle.is_none() {
        return Err(usage_error(
            "CLI_OUTPUT_NONE_WITHOUT_BUNDLE",
            "--output none requires --bundle so the command still produces artifacts.",
        ));
    }

    if output == CliOutputMode::Html {
        match value_type {
            ValueType::Html | ValueType::OuterHtml => {}
            _ => {
                return Err(usage_error(
                    "CLI_OUTPUT_HTML_INVALID",
                    "--output html can only be used with --value html or --value outer-html.",
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
        ValueType::Html | ValueType::OuterHtml => CliOutputMode::Html,
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
    command: &'static str,
    result: ExtractionResult,
    bundle: Option<BundlePaths>,
) -> ExtractionCommandReport {
    ExtractionCommandReport {
        tool: TOOL_NAME.to_owned(),
        engine: ENGINE_NAME.to_owned(),
        version: HTMLCUT_VERSION.to_owned(),
        schema_name: EXTRACTION_COMMAND_REPORT_SCHEMA_NAME.to_owned(),
        schema_version: EXTRACTION_COMMAND_REPORT_SCHEMA_VERSION,
        command: command.to_owned(),
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
    command: &'static str,
    result: SourceInspectionResult,
) -> SourceInspectionCommandReport {
    SourceInspectionCommandReport {
        tool: TOOL_NAME.to_owned(),
        engine: ENGINE_NAME.to_owned(),
        version: HTMLCUT_VERSION.to_owned(),
        schema_name: SOURCE_INSPECTION_COMMAND_REPORT_SCHEMA_NAME.to_owned(),
        schema_version: SOURCE_INSPECTION_COMMAND_REPORT_SCHEMA_VERSION,
        command: command.to_owned(),
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
            operation_id.parse::<htmlcut_core::OperationId>().map_err(|_| {
                usage_error(
                    "CLI_OPERATION_ID_UNKNOWN",
                    format!(
                        "Unknown operation ID: {operation_id}. Use `htmlcut catalog` to list the valid operation IDs."
                    ),
                )
            })
        })
        .transpose()?;

    let operations = htmlcut_core::operation_catalog()
        .iter()
        .filter(|descriptor| {
            requested_operation.is_none_or(|operation_id| descriptor.id == operation_id)
        })
        .map(|descriptor| CatalogOperationReport {
            operation_id: descriptor.id,
            command: descriptor.cli_surface.map(str::to_owned),
            availability: match descriptor.cli_surface {
                Some(_) => CatalogAvailability::Cli,
                None => CatalogAvailability::CoreOnly,
            },
            summary: descriptor.description.to_owned(),
            core_surface: descriptor.core_surface.to_owned(),
            request_contract: build_contract_surface(&descriptor.request_contract),
            result_contract: build_contract_surface(&descriptor.result_contract),
            command_contract: build_catalog_command_contract(descriptor.id),
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
        .map(build_core_schema_document_report)
        .chain(
            cli_schema_catalog()
                .iter()
                .map(build_cli_schema_document_report),
        )
        .collect::<Vec<_>>();

    schemas.sort_by(|left, right| {
        left.schema_name
            .cmp(&right.schema_name)
            .then(left.schema_version.cmp(&right.schema_version))
    });

    let filtered = schemas
        .into_iter()
        .filter(|schema| name_filter.is_none_or(|name| schema.schema_name == name))
        .filter(|schema| version_filter.is_none_or(|version| schema.schema_version == version))
        .collect::<Vec<_>>();

    if filtered.is_empty() {
        let name = name_filter.expect("version-only filters return earlier");
        let requested = version_filter
            .map(|version| format!("{name}@{version}"))
            .unwrap_or_else(|| name.to_owned());
        return Err(usage_error(
            "CLI_SCHEMA_UNKNOWN",
            format!("Unknown schema: {requested}. Use `htmlcut schema` to list the valid schemas."),
        ));
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

#[derive(Clone, Copy)]
struct CliSchemaDescriptor {
    schema_name: &'static str,
    schema_version: u32,
    owner_surface: &'static str,
    rust_shape: &'static str,
    stability: SchemaStability,
    json_schema: fn() -> Value,
}

const CLI_SCHEMA_CATALOG: &[CliSchemaDescriptor] = &[
    CliSchemaDescriptor {
        schema_name: EXTRACTION_COMMAND_REPORT_SCHEMA_NAME,
        schema_version: EXTRACTION_COMMAND_REPORT_SCHEMA_VERSION,
        owner_surface: "htmlcut-cli",
        rust_shape: "ExtractionCommandReport",
        stability: SchemaStability::Versioned,
        json_schema: extraction_command_report_schema,
    },
    CliSchemaDescriptor {
        schema_name: SOURCE_INSPECTION_COMMAND_REPORT_SCHEMA_NAME,
        schema_version: SOURCE_INSPECTION_COMMAND_REPORT_SCHEMA_VERSION,
        owner_surface: "htmlcut-cli",
        rust_shape: "SourceInspectionCommandReport",
        stability: SchemaStability::Versioned,
        json_schema: source_inspection_command_report_schema,
    },
    CliSchemaDescriptor {
        schema_name: CATALOG_REPORT_SCHEMA_NAME,
        schema_version: CATALOG_SCHEMA_VERSION,
        owner_surface: "htmlcut-cli",
        rust_shape: "CatalogCommandReport",
        stability: SchemaStability::Versioned,
        json_schema: catalog_command_report_schema,
    },
    CliSchemaDescriptor {
        schema_name: SCHEMA_COMMAND_REPORT_SCHEMA_NAME,
        schema_version: SCHEMA_COMMAND_REPORT_SCHEMA_VERSION,
        owner_surface: "htmlcut-cli",
        rust_shape: "SchemaCommandReport",
        stability: SchemaStability::Versioned,
        json_schema: schema_command_report_schema,
    },
];

fn cli_schema_catalog() -> &'static [CliSchemaDescriptor] {
    CLI_SCHEMA_CATALOG
}

fn build_core_schema_document_report(
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

fn build_cli_schema_document_report(descriptor: &CliSchemaDescriptor) -> SchemaDocumentReport {
    SchemaDocumentReport {
        schema_name: descriptor.schema_name.to_owned(),
        schema_version: descriptor.schema_version,
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
    operation_id: htmlcut_core::OperationId,
) -> Option<CatalogCommandContract> {
    let input_forms = common_input_forms();
    match operation_id {
        htmlcut_core::OperationId::DocumentParse => None,
        htmlcut_core::OperationId::SourceInspect => Some(CatalogCommandContract {
            invocation: "htmlcut inspect source [OPTIONS] <INPUT>".to_owned(),
            inputs: input_forms,
            default_match: None,
            selection_modes: Vec::new(),
            default_value: None,
            value_modes: Vec::new(),
            default_output: Some("json".to_owned()),
            default_output_overrides: Vec::new(),
            output_modes: vec!["text".to_owned(), "json".to_owned()],
            constraints: Vec::new(),
            notes: vec![
                "Use this command to inspect document shape, headings, links, classes, and effective base-URL behavior before choosing selectors or slice boundaries.".to_owned(),
                "--include-source-text stores the full source in JSON output and prints a bounded source preview in text mode.".to_owned(),
                "--sample-limit bounds the sampled headings, links, tags, and classes in the summary.".to_owned(),
            ],
            examples: vec![
                "htmlcut inspect source ./page.html".to_owned(),
                "htmlcut inspect source ./page.html --output text --include-source-text --preview-chars 200".to_owned(),
            ],
            parameters: inspect_source_parameters(),
        }),
        htmlcut_core::OperationId::SelectPreview => Some(CatalogCommandContract {
            invocation: "htmlcut inspect select [OPTIONS] --css <CSS> <INPUT>".to_owned(),
            inputs: input_forms,
            default_match: Some("first".to_owned()),
            selection_modes: common_selection_modes(),
            default_value: Some("structured".to_owned()),
            value_modes: vec!["structured".to_owned()],
            default_output: Some("json".to_owned()),
            default_output_overrides: Vec::new(),
            output_modes: vec!["text".to_owned(), "json".to_owned()],
            constraints: common_selection_constraints(),
            notes: vec![
                "inspect select always previews matches in structured form; it is a preview workflow, not a final extraction surface.".to_owned(),
                "When --rewrite-urls is requested but no effective base URL can be resolved, relative URLs stay unchanged and a warning is emitted.".to_owned(),
            ],
            examples: vec![
                "htmlcut inspect select ./page.html --css article --match single".to_owned(),
                "htmlcut inspect select ./page.html --css '.card' --match all --output text".to_owned(),
            ],
            parameters: inspect_select_parameters(),
        }),
        htmlcut_core::OperationId::SlicePreview => Some(CatalogCommandContract {
            invocation: "htmlcut inspect slice [OPTIONS] --from <FROM> --to <TO> <INPUT>"
                .to_owned(),
            inputs: input_forms,
            default_match: Some("first".to_owned()),
            selection_modes: common_selection_modes(),
            default_value: Some("structured".to_owned()),
            value_modes: vec!["structured".to_owned()],
            default_output: Some("json".to_owned()),
            default_output_overrides: Vec::new(),
            output_modes: vec!["text".to_owned(), "json".to_owned()],
            constraints: {
                let mut constraints = common_selection_constraints();
                constraints.extend(slice_pattern_constraints());
                constraints
            },
            notes: vec![
                "Literal boundaries are raw substring matching, not tag-aware; `<a` also matches `<article>`.".to_owned(),
                "Previews exclude both matched boundaries by default unless --include-start and/or --include-end are supplied.".to_owned(),
                "Text output shows fragment context when it adds signal so boundary-consumption mistakes are easier to spot.".to_owned(),
            ],
            examples: vec![
                "htmlcut inspect slice ./page.html --from '<article>' --to '</article>'".to_owned(),
                "htmlcut inspect slice ./page.html --from 'START::' --to '::END' --pattern regex --output text".to_owned(),
            ],
            parameters: inspect_slice_parameters(),
        }),
        htmlcut_core::OperationId::SelectExtract => Some(CatalogCommandContract {
            invocation: "htmlcut select [OPTIONS] --css <CSS> <INPUT>".to_owned(),
            inputs: input_forms,
            default_match: Some("first".to_owned()),
            selection_modes: common_selection_modes(),
            default_value: Some("text".to_owned()),
            value_modes: vec![
                "text".to_owned(),
                "html".to_owned(),
                "outer-html".to_owned(),
                "attribute".to_owned(),
                "structured".to_owned(),
            ],
            default_output: Some("text".to_owned()),
            default_output_overrides: vec![conditional_default(
                "json",
                condition("--value", &["structured"]),
            )],
            output_modes: common_output_modes(),
            constraints: {
                let mut constraints = common_selection_constraints();
                constraints.extend(common_extract_constraints());
                constraints
            },
            notes: vec![
                "Structured extraction only supports --output json or --output none.".to_owned(),
                "--output html is only valid with --value html or --value outer-html.".to_owned(),
                "When --rewrite-urls is requested but no effective base URL can be resolved, relative URLs stay unchanged and a warning is emitted.".to_owned(),
            ],
            examples: vec![
                "htmlcut select ./page.html --css article --match single".to_owned(),
                "htmlcut select ./page.html --css '.card' --match all --value outer-html".to_owned(),
                "htmlcut select ./page.html --css 'article a.more' --value attribute --attribute href --rewrite-urls".to_owned(),
            ],
            parameters: select_extract_parameters(),
        }),
        htmlcut_core::OperationId::SliceExtract => Some(CatalogCommandContract {
            invocation: "htmlcut slice [OPTIONS] --from <FROM> --to <TO> <INPUT>".to_owned(),
            inputs: input_forms,
            default_match: Some("first".to_owned()),
            selection_modes: common_selection_modes(),
            default_value: Some("text".to_owned()),
            value_modes: vec![
                "text".to_owned(),
                "html".to_owned(),
                "outer-html".to_owned(),
                "attribute".to_owned(),
                "structured".to_owned(),
            ],
            default_output: Some("text".to_owned()),
            default_output_overrides: vec![conditional_default(
                "json",
                condition("--value", &["structured"]),
            )],
            output_modes: common_output_modes(),
            constraints: {
                let mut constraints = common_selection_constraints();
                constraints.extend(slice_pattern_constraints());
                constraints.extend(common_extract_constraints());
                constraints
            },
            notes: vec![
                "Literal boundaries are raw substring matching, not tag-aware; `<a` also matches `<article>`.".to_owned(),
                "The selected fragment excludes both matched boundaries by default; --include-start and --include-end control that selected fragment precisely.".to_owned(),
                "For --value html, HTMLCut returns the selected fragment as HTML. For --value outer-html, HTMLCut returns the full outer matched range including both boundaries.".to_owned(),
                "When extracting --value attribute from sliced HTML, use --include-start when the opening tag lives in the start boundary.".to_owned(),
                "Structured extraction only supports --output json or --output none.".to_owned(),
            ],
            examples: vec![
                "htmlcut slice ./page.html --from '<article>' --to '</article>'".to_owned(),
                "htmlcut slice ./page.html --from 'START::' --to '::END' --pattern regex --match all --output json".to_owned(),
                "htmlcut slice ./page.html --from '<a ' --to '</a>' --include-start --include-end --value attribute --attribute href".to_owned(),
            ],
            parameters: slice_extract_parameters(),
        }),
    }
}

fn common_input_forms() -> Vec<String> {
    vec![
        "local file path".to_owned(),
        "http:// or https:// URL".to_owned(),
        "- for stdin".to_owned(),
    ]
}

fn common_selection_modes() -> Vec<String> {
    vec![
        "single".to_owned(),
        "first".to_owned(),
        "nth".to_owned(),
        "all".to_owned(),
    ]
}

fn common_output_modes() -> Vec<String> {
    vec![
        "text".to_owned(),
        "html".to_owned(),
        "json".to_owned(),
        "none".to_owned(),
    ]
}

fn condition(parameter: &str, values: &[&str]) -> CatalogCondition {
    CatalogCondition {
        parameter: parameter.to_owned(),
        values: values.iter().map(|value| (*value).to_owned()).collect(),
    }
}

fn conditional_default(value: &str, when: CatalogCondition) -> CatalogConditionalDefault {
    CatalogConditionalDefault {
        value: value.to_owned(),
        when,
    }
}

fn requires_parameter(parameter: &str, when: CatalogCondition) -> CatalogConstraint {
    CatalogConstraint::RequiresParameter {
        parameter: parameter.to_owned(),
        when,
    }
}

fn allowed_only_when(parameter: &str, when: CatalogCondition) -> CatalogConstraint {
    CatalogConstraint::AllowedOnlyWhen {
        parameter: parameter.to_owned(),
        when,
    }
}

fn restricts_parameter_values(
    parameter: &str,
    allowed_values: &[&str],
    when: CatalogCondition,
) -> CatalogConstraint {
    CatalogConstraint::RestrictsParameterValues {
        parameter: parameter.to_owned(),
        allowed_values: allowed_values
            .iter()
            .map(|value| (*value).to_owned())
            .collect(),
        when,
    }
}

fn common_selection_constraints() -> Vec<CatalogConstraint> {
    vec![requires_parameter(
        "--index",
        condition("--match", &["nth"]),
    )]
}

fn common_extract_constraints() -> Vec<CatalogConstraint> {
    vec![
        requires_parameter("--attribute", condition("--value", &["attribute"])),
        requires_parameter("--bundle", condition("--output", &["none"])),
        restricts_parameter_values(
            "--output",
            &["json", "none"],
            condition("--value", &["structured"]),
        ),
        restricts_parameter_values(
            "--value",
            &["html", "outer-html"],
            condition("--output", &["html"]),
        ),
    ]
}

fn slice_pattern_constraints() -> Vec<CatalogConstraint> {
    vec![allowed_only_when(
        "--regex-flags",
        condition("--pattern", &["regex"]),
    )]
}

fn param_positional(
    section: &str,
    name: &str,
    requirement: CatalogParameterRequirement,
    summary: &str,
) -> CatalogParameterSpec {
    CatalogParameterSpec {
        section: section.to_owned(),
        name: name.to_owned(),
        kind: CatalogParameterKind::Positional,
        requirement,
        requirement_note: None,
        value_hint: None,
        default: None,
        allowed_values: Vec::new(),
        summary: summary.to_owned(),
    }
}

struct OptionParameterSpec<'a> {
    section: &'a str,
    name: &'a str,
    requirement: CatalogParameterRequirement,
    requirement_note: Option<&'a str>,
    value_hint: &'a str,
    default: Option<&'a str>,
    allowed_values: Vec<String>,
    summary: &'a str,
}

fn param_option(spec: OptionParameterSpec<'_>) -> CatalogParameterSpec {
    CatalogParameterSpec {
        section: spec.section.to_owned(),
        name: spec.name.to_owned(),
        kind: CatalogParameterKind::Option,
        requirement: spec.requirement,
        requirement_note: spec.requirement_note.map(str::to_owned),
        value_hint: Some(spec.value_hint.to_owned()),
        default: spec.default.map(str::to_owned),
        allowed_values: spec.allowed_values,
        summary: spec.summary.to_owned(),
    }
}

fn param_flag(section: &str, name: &str, summary: &str) -> CatalogParameterSpec {
    CatalogParameterSpec {
        section: section.to_owned(),
        name: name.to_owned(),
        kind: CatalogParameterKind::Flag,
        requirement: CatalogParameterRequirement::Optional,
        requirement_note: None,
        value_hint: None,
        default: Some("false".to_owned()),
        allowed_values: Vec::new(),
        summary: summary.to_owned(),
    }
}

fn common_source_parameters() -> Vec<CatalogParameterSpec> {
    vec![
        param_option(OptionParameterSpec {
            section: "Source",
            name: "--base-url",
            requirement: CatalogParameterRequirement::Optional,
            requirement_note: None,
            value_hint: "URL",
            default: None,
            allowed_values: Vec::new(),
            summary: "Override the input base URL used for relative-link resolution.",
        }),
        param_option(OptionParameterSpec {
            section: "Source",
            name: "--max-bytes",
            requirement: CatalogParameterRequirement::Optional,
            requirement_note: None,
            value_hint: "SIZE",
            default: Some("52428800"),
            allowed_values: Vec::new(),
            summary: "Refuse sources larger than this limit. Accepts raw bytes or KB, MB, and GB.",
        }),
        param_option(OptionParameterSpec {
            section: "Source",
            name: "--fetch-timeout-ms",
            requirement: CatalogParameterRequirement::Optional,
            requirement_note: None,
            value_hint: "MILLISECONDS",
            default: Some("15000"),
            allowed_values: Vec::new(),
            summary: "HTTP fetch timeout in milliseconds for URL inputs.",
        }),
        param_positional(
            "Source",
            "<INPUT>",
            CatalogParameterRequirement::Required,
            "HTML input source: a local file path, an http(s) URL, or - for stdin.",
        ),
    ]
}

fn common_selection_parameters() -> Vec<CatalogParameterSpec> {
    vec![
        param_option(OptionParameterSpec {
            section: "Selection",
            name: "--match",
            requirement: CatalogParameterRequirement::Optional,
            requirement_note: None,
            value_hint: "MATCH",
            default: Some("first"),
            allowed_values: common_selection_modes(),
            summary: "Require exactly one match, keep the first match, keep one 1-based match, or keep every match.",
        }),
        param_option(OptionParameterSpec {
            section: "Selection",
            name: "--index",
            requirement: CatalogParameterRequirement::Conditional,
            requirement_note: Some("required when --match nth is used"),
            value_hint: "INDEX",
            default: None,
            allowed_values: Vec::new(),
            summary: "The 1-based match index when --match nth is used.",
        }),
    ]
}

fn common_extract_parameters() -> Vec<CatalogParameterSpec> {
    vec![
        param_option(OptionParameterSpec {
            section: "Extraction",
            name: "--value",
            requirement: CatalogParameterRequirement::Optional,
            requirement_note: None,
            value_hint: "VALUE",
            default: Some("text"),
            allowed_values: vec![
                "text".to_owned(),
                "html".to_owned(),
                "outer-html".to_owned(),
                "attribute".to_owned(),
                "structured".to_owned(),
            ],
            summary: "What each selected match should produce before stdout formatting is applied.",
        }),
        param_option(OptionParameterSpec {
            section: "Extraction",
            name: "--attribute",
            requirement: CatalogParameterRequirement::Conditional,
            requirement_note: Some("required when --value attribute is used"),
            value_hint: "ATTRIBUTE",
            default: None,
            allowed_values: Vec::new(),
            summary: "Attribute name to extract when --value attribute is used.",
        }),
        param_option(OptionParameterSpec {
            section: "Extraction",
            name: "--whitespace",
            requirement: CatalogParameterRequirement::Optional,
            requirement_note: None,
            value_hint: "WHITESPACE",
            default: Some("preserve"),
            allowed_values: vec!["preserve".to_owned(), "normalize".to_owned()],
            summary: "Preserve source whitespace or normalize it for text-like values.",
        }),
        param_flag(
            "Extraction",
            "--rewrite-urls",
            "Rewrite relative URLs in extracted HTML and attributes with the effective base URL.",
        ),
        param_option(OptionParameterSpec {
            section: "Extraction",
            name: "--output",
            requirement: CatalogParameterRequirement::Optional,
            requirement_note: None,
            value_hint: "OUTPUT",
            default: None,
            allowed_values: common_output_modes(),
            summary: "How stdout should be rendered after extraction.",
        }),
        param_option(OptionParameterSpec {
            section: "Extraction",
            name: "--bundle",
            requirement: CatalogParameterRequirement::Optional,
            requirement_note: None,
            value_hint: "BUNDLE",
            default: None,
            allowed_values: Vec::new(),
            summary: "Write report.json, selection.html, and selection.txt to this directory.",
        }),
        param_option(OptionParameterSpec {
            section: "Extraction",
            name: "--preview-chars",
            requirement: CatalogParameterRequirement::Optional,
            requirement_note: None,
            value_hint: "PREVIEW_CHARS",
            default: Some("160"),
            allowed_values: Vec::new(),
            summary: "Maximum preview length stored in structured reports.",
        }),
        param_flag(
            "Extraction",
            "--include-source-text",
            "Include the full source text inside structured reports and bundles.",
        ),
    ]
}

fn common_inspect_output_parameters() -> Vec<CatalogParameterSpec> {
    vec![
        param_option(OptionParameterSpec {
            section: "Inspection Output",
            name: "--output",
            requirement: CatalogParameterRequirement::Optional,
            requirement_note: None,
            value_hint: "OUTPUT",
            default: Some("json"),
            allowed_values: vec!["text".to_owned(), "json".to_owned()],
            summary: "Render the inspection as compact text or structured JSON.",
        }),
        param_option(OptionParameterSpec {
            section: "Inspection Output",
            name: "--preview-chars",
            requirement: CatalogParameterRequirement::Optional,
            requirement_note: None,
            value_hint: "PREVIEW_CHARS",
            default: Some("160"),
            allowed_values: Vec::new(),
            summary: "Maximum preview length stored in structured preview reports.",
        }),
        param_flag(
            "Inspection Output",
            "--include-source-text",
            "Include the full source text inside structured inspection reports.",
        ),
    ]
}

fn inspect_source_parameters() -> Vec<CatalogParameterSpec> {
    let mut parameters = common_source_parameters();
    parameters.push(param_option(OptionParameterSpec {
        section: "Source",
        name: "--sample-limit",
        requirement: CatalogParameterRequirement::Optional,
        requirement_note: None,
        value_hint: "SAMPLE_LIMIT",
        default: Some("8"),
        allowed_values: Vec::new(),
        summary: "Maximum number of headings, links, tags, and classes to sample in the summary.",
    }));
    parameters.push(param_option(OptionParameterSpec {
        section: "Source",
        name: "--output",
        requirement: CatalogParameterRequirement::Optional,
        requirement_note: None,
        value_hint: "OUTPUT",
        default: Some("json"),
        allowed_values: vec!["text".to_owned(), "json".to_owned()],
        summary: "Render the inspection as compact text or structured JSON.",
    }));
    parameters.push(param_flag(
        "Source",
        "--include-source-text",
        "Include the full source text in JSON output and a bounded preview in text output.",
    ));
    parameters.push(param_option(OptionParameterSpec {
        section: "Source",
        name: "--preview-chars",
        requirement: CatalogParameterRequirement::Optional,
        requirement_note: None,
        value_hint: "PREVIEW_CHARS",
        default: Some("160"),
        allowed_values: Vec::new(),
        summary: "Maximum length of the source preview shown in text mode when --include-source-text is used.",
    }));
    parameters
}

fn inspect_select_parameters() -> Vec<CatalogParameterSpec> {
    let mut parameters = common_source_parameters();
    parameters.push(param_option(OptionParameterSpec {
        section: "Source",
        name: "--css",
        requirement: CatalogParameterRequirement::Required,
        requirement_note: None,
        value_hint: "CSS",
        default: None,
        allowed_values: Vec::new(),
        summary: "CSS selector that chooses the candidate nodes to preview.",
    }));
    parameters.extend(common_selection_parameters());
    parameters.push(param_option(OptionParameterSpec {
        section: "Selection",
        name: "--whitespace",
        requirement: CatalogParameterRequirement::Optional,
        requirement_note: None,
        value_hint: "WHITESPACE",
        default: Some("preserve"),
        allowed_values: vec!["preserve".to_owned(), "normalize".to_owned()],
        summary: "Preserve source whitespace or normalize preview text.",
    }));
    parameters.push(param_flag(
        "Selection",
        "--rewrite-urls",
        "Rewrite relative URLs in preview HTML and attribute data with the effective base URL.",
    ));
    parameters.extend(common_inspect_output_parameters());
    parameters
}

fn inspect_slice_parameters() -> Vec<CatalogParameterSpec> {
    let mut parameters = common_source_parameters();
    parameters.extend(slice_strategy_parameters("Source"));
    parameters.extend(common_selection_parameters());
    parameters.push(param_option(OptionParameterSpec {
        section: "Selection",
        name: "--whitespace",
        requirement: CatalogParameterRequirement::Optional,
        requirement_note: None,
        value_hint: "WHITESPACE",
        default: Some("preserve"),
        allowed_values: vec!["preserve".to_owned(), "normalize".to_owned()],
        summary: "Preserve source whitespace or normalize preview text.",
    }));
    parameters.push(param_flag(
        "Selection",
        "--rewrite-urls",
        "Rewrite relative URLs in preview HTML and attribute data with the effective base URL.",
    ));
    parameters.extend(common_inspect_output_parameters());
    parameters
}

fn select_extract_parameters() -> Vec<CatalogParameterSpec> {
    let mut parameters = common_source_parameters();
    parameters.push(param_option(OptionParameterSpec {
        section: "Source",
        name: "--css",
        requirement: CatalogParameterRequirement::Required,
        requirement_note: None,
        value_hint: "CSS",
        default: None,
        allowed_values: Vec::new(),
        summary: "CSS selector that chooses the candidate nodes to extract.",
    }));
    parameters.extend(common_selection_parameters());
    parameters.extend(common_extract_parameters());
    parameters
}

fn slice_extract_parameters() -> Vec<CatalogParameterSpec> {
    let mut parameters = common_source_parameters();
    parameters.extend(slice_strategy_parameters("Source"));
    parameters.extend(common_selection_parameters());
    parameters.extend(common_extract_parameters());
    parameters
}

fn slice_strategy_parameters(section: &str) -> Vec<CatalogParameterSpec> {
    vec![
        param_option(OptionParameterSpec {
            section,
            name: "--from",
            requirement: CatalogParameterRequirement::Required,
            requirement_note: None,
            value_hint: "FROM",
            default: None,
            allowed_values: Vec::new(),
            summary: "Start boundary used to locate each candidate slice.",
        }),
        param_option(OptionParameterSpec {
            section,
            name: "--to",
            requirement: CatalogParameterRequirement::Required,
            requirement_note: None,
            value_hint: "TO",
            default: None,
            allowed_values: Vec::new(),
            summary: "End boundary used to locate each candidate slice.",
        }),
        param_option(OptionParameterSpec {
            section,
            name: "--pattern",
            requirement: CatalogParameterRequirement::Optional,
            requirement_note: None,
            value_hint: "PATTERN",
            default: Some("literal"),
            allowed_values: vec!["literal".to_owned(), "regex".to_owned()],
            summary: "Interpret --from and --to as literal text or regex patterns.",
        }),
        param_option(OptionParameterSpec {
            section,
            name: "--regex-flags",
            requirement: CatalogParameterRequirement::Conditional,
            requirement_note: Some("allowed only when --pattern regex is used"),
            value_hint: "REGEX_FLAGS",
            default: None,
            allowed_values: Vec::new(),
            summary: "Regex flags for --pattern regex. Accepts i, m, s, u, and x.",
        }),
        param_flag(
            section,
            "--include-start",
            "Include the matched --from boundary in the selected fragment.",
        ),
        param_flag(
            section,
            "--include-end",
            "Include the matched --to boundary in the selected fragment.",
        ),
    ]
}
