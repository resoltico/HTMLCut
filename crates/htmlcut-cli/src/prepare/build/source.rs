use std::num::NonZeroUsize;
use std::path::PathBuf;

use htmlcut_core::{RuntimeOptions, SourceInput, SourceRequest};
use url::Url;

use crate::args::SourceArgs;
use crate::error::{CliError, usage_error};
use crate::model::CliErrorCode;

use crate::prepare::required_cli_value;

const KIBIBYTE: u128 = 1024;
const MEBIBYTE: u128 = KIBIBYTE * KIBIBYTE;
const GIBIBYTE: u128 = MEBIBYTE * KIBIBYTE;

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
        fetch_connect_timeout_ms: args.fetch_connect_timeout_ms,
        fetch_preflight: args.fetch_preflight,
    })
}

pub(crate) fn validate_base_url(base_url: Option<&str>) -> Result<Option<Url>, CliError> {
    let Some(value) = base_url else {
        return Ok(None);
    };

    Ok(Some(validate_http_url(
        value,
        CliErrorCode::BaseUrlInvalid,
        CliErrorCode::BaseUrlSchemeInvalid,
    )?))
}

pub(crate) fn validate_preview_chars(preview_chars: usize) -> Result<NonZeroUsize, CliError> {
    NonZeroUsize::new(preview_chars).ok_or_else(|| {
        usage_error(
            CliErrorCode::PreviewCharsInvalid,
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
    let multiplier = match unit_text.trim() {
        "" | "b" => 1u128,
        "kib" => KIBIBYTE,
        "mib" => MEBIBYTE,
        "gib" => GIBIBYTE,
        _ => return Err(invalid_byte_size(value)),
    };

    parse_scaled_bytes(amount_text, multiplier, value)
}

fn parse_scaled_bytes(
    amount_text: &str,
    multiplier: u128,
    original: &str,
) -> Result<usize, CliError> {
    let (whole_text, fractional_text) = parse_decimal_parts(amount_text, original)?;
    let scale = decimal_scale(fractional_text.len(), original)?;
    let whole = parse_decimal_digits(whole_text, original)?;
    let fractional = parse_decimal_digits(fractional_text, original)?;
    let scaled_amount = whole
        .checked_mul(scale)
        .and_then(|value| value.checked_add(fractional))
        .ok_or_else(|| too_large_byte_size(original))?;
    let scaled_bytes = scaled_amount
        .checked_mul(multiplier)
        .ok_or_else(|| too_large_byte_size(original))?;

    if scaled_bytes == 0 || scaled_bytes % scale != 0 {
        return Err(invalid_byte_size(original));
    }

    usize::try_from(scaled_bytes / scale).map_err(|_| too_large_byte_size(original))
}

fn parse_decimal_parts<'a>(
    amount_text: &'a str,
    original: &str,
) -> Result<(&'a str, &'a str), CliError> {
    let mut parts = amount_text.split('.');
    let whole = parts.next().unwrap_or_default();
    let fractional = parts.next().unwrap_or_default();

    if parts.next().is_some() {
        return Err(invalid_byte_size(original));
    }

    if whole.is_empty() && fractional.is_empty() {
        return Err(invalid_byte_size(original));
    }

    Ok((whole, fractional))
}

fn parse_decimal_digits(value: &str, original: &str) -> Result<u128, CliError> {
    if value.is_empty() {
        return Ok(0);
    }

    value
        .parse::<u128>()
        .map_err(|_| invalid_byte_size(original))
}

fn decimal_scale(fractional_digits: usize, original: &str) -> Result<u128, CliError> {
    10u128
        .checked_pow(u32::try_from(fractional_digits).unwrap_or(u32::MAX))
        .ok_or_else(|| too_large_byte_size(original))
}

fn invalid_byte_size(value: &str) -> CliError {
    usage_error(
        CliErrorCode::ByteSizeInvalid,
        format!("Invalid byte size: {value}"),
    )
}

fn too_large_byte_size(value: &str) -> CliError {
    usage_error(
        CliErrorCode::ByteSizeInvalid,
        format!("Byte size is too large: {value}"),
    )
}

fn validate_input_url(value: &str) -> Result<Url, CliError> {
    validate_http_url(
        value,
        CliErrorCode::SourceUrlInvalid,
        CliErrorCode::SourceUrlSchemeInvalid,
    )
}

fn validate_http_url(
    value: &str,
    invalid_code: CliErrorCode,
    invalid_scheme_code: CliErrorCode,
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
