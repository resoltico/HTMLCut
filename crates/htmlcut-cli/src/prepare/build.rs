use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};

use htmlcut_core::{
    AttributeName, ExtractionRequest, ExtractionSpec, NormalizationOptions, OutputOptions,
    RuntimeOptions, SelectionSpec, SelectorQuery, SliceBoundary, SlicePatternSpec, SliceSpec,
    SourceInput, SourceRequest, ValueSpec, ValueType,
};
use url::Url;

use crate::args::{
    CliMatchMode, CliOutputMode, CliPatternMode, CliValueMode, ExtractOutputArgs, SelectionArgs,
    SourceArgs,
};
use crate::error::{CliError, usage_error};

use super::{RequestBuildOptions, default_regex_flags};

const KIBIBYTE: usize = 1024;
const MEBIBYTE: usize = KIBIBYTE * KIBIBYTE;
const GIBIBYTE: usize = MEBIBYTE * KIBIBYTE;

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
        whitespace: options.whitespace,
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
        fetch_preflight: args.fetch_preflight,
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
    if bytes > usize::MAX as f64 {
        return Err(usage_error(
            "CLI_BYTE_SIZE_INVALID",
            format!("Byte size is too large: {value}"),
        ));
    }

    Ok(bytes as usize)
}

fn required_cli_value(value: Option<String>, parameter: &'static str) -> Result<String, CliError> {
    value.ok_or_else(|| {
        usage_error(
            "CLI_REQUIRED_PARAMETER_MISSING",
            format!("{parameter} is required unless --request-file is used."),
        )
    })
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
