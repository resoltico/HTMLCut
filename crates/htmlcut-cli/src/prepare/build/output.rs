use std::path::Path;

use htmlcut_core::{AttributeName, ValueSpec, ValueType};

use crate::args::{CliOutputMode, CliPatternMode, CliValueMode, ExtractOutputArgs};
use crate::error::{CliError, usage_error};

use crate::prepare::default_regex_flags;

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

fn reject_attribute_conflict(attribute: Option<String>) -> Result<(), CliError> {
    if attribute.is_some() {
        return Err(usage_error(
            "CLI_ATTRIBUTE_CONFLICT",
            "--attribute can only be used with --value attribute.",
        ));
    }

    Ok(())
}
