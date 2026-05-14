use std::path::Path;

use htmlcut_core::{AttributeName, ValueSpec, ValueType};

use crate::args::{CliOutputMode, CliPatternMode, ExtractOutputArgs, SliceExtractOutputArgs};
use crate::error::{CliError, usage_error};
use crate::model::CliErrorCode;

pub(crate) trait ExtractOutputLike {
    fn requested_output(&self) -> Option<CliOutputMode>;
    fn default_value_type(&self) -> ValueType;
}

impl ExtractOutputLike for ExtractOutputArgs {
    fn requested_output(&self) -> Option<CliOutputMode> {
        self.output
    }

    fn default_value_type(&self) -> ValueType {
        ValueType::from(self.value)
    }
}

impl ExtractOutputLike for SliceExtractOutputArgs {
    fn requested_output(&self) -> Option<CliOutputMode> {
        self.output
    }

    fn default_value_type(&self) -> ValueType {
        ValueType::from(self.value)
    }
}

pub(crate) fn resolve_value_spec(
    value_mode: ValueType,
    attribute: Option<String>,
) -> Result<ValueSpec, CliError> {
    match value_mode {
        ValueType::Text => {
            reject_attribute_conflict(attribute)?;
            Ok(ValueSpec::Text)
        }
        ValueType::SelectedHtml => {
            reject_attribute_conflict(attribute)?;
            Ok(ValueSpec::SelectedHtml)
        }
        ValueType::InnerHtml => {
            reject_attribute_conflict(attribute)?;
            Ok(ValueSpec::InnerHtml)
        }
        ValueType::OuterHtml => {
            reject_attribute_conflict(attribute)?;
            Ok(ValueSpec::OuterHtml)
        }
        ValueType::Attribute => {
            let Some(attribute) = attribute else {
                return Err(usage_error(
                    CliErrorCode::AttributeRequired,
                    "--attribute is required with --value attribute.",
                ));
            };
            Ok(ValueSpec::Attribute {
                name: AttributeName::new(attribute).map_err(|error| {
                    usage_error(CliErrorCode::AttributeInvalid, error.to_string())
                })?,
            })
        }
        ValueType::Structured => {
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
            CliErrorCode::OutputNoneWithoutBundle,
            "--output none requires --bundle so the command still produces artifacts.",
        ));
    }

    if output == CliOutputMode::None && output_file.is_some() {
        return Err(usage_error(
            CliErrorCode::OutputFileRequiresStdoutPayload,
            "--output-file cannot be used with --output none because no stdout payload is produced.",
        ));
    }

    if output == CliOutputMode::Html {
        match value_type {
            ValueType::SelectedHtml | ValueType::InnerHtml | ValueType::OuterHtml => {}
            _ => {
                return Err(usage_error(
                    CliErrorCode::OutputHtmlInvalid,
                    "--output html can only be used with --value selected-html, --value inner-html, or --value outer-html.",
                ));
            }
        }
    }

    if *value_type == ValueType::Structured {
        match output {
            CliOutputMode::Json | CliOutputMode::None => {}
            _ => {
                return Err(usage_error(
                    CliErrorCode::StructuredOutputInvalid,
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
                    CliErrorCode::RegexFlagsConflict,
                    "--regex-flags can only be used with --pattern regex.",
                ));
            }
            Ok(None)
        }
        CliPatternMode::Regex => Ok(Some(regex_flags.unwrap_or_default())),
    }
}

pub(crate) fn default_output_for_value(value_type: &ValueType) -> CliOutputMode {
    match value_type {
        ValueType::SelectedHtml | ValueType::InnerHtml | ValueType::OuterHtml => {
            CliOutputMode::Html
        }
        ValueType::Structured => CliOutputMode::Json,
        _ => CliOutputMode::Text,
    }
}

pub(crate) fn extract_prefers_json<T>(args: &T) -> bool
where
    T: ExtractOutputLike,
{
    args.requested_output() == Some(CliOutputMode::Json)
        || (args.requested_output().is_none() && args.default_value_type() == ValueType::Structured)
}

fn reject_attribute_conflict(attribute: Option<String>) -> Result<(), CliError> {
    if attribute.is_some() {
        return Err(usage_error(
            CliErrorCode::AttributeConflict,
            "--attribute can only be used with --value attribute.",
        ));
    }

    Ok(())
}
