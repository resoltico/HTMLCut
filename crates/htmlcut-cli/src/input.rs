//! Adapter-only source specifications and strict charset decoding.

use std::fs::File;
use std::io::Read;
use std::path::Path;

use encoding_rs::{DecoderResult, Encoding, UTF_8};
use htmlcut_core::{ErrorCode, ExtractionError, ExtractionPlan, SnapshotMetadata, SourceSnapshot};
use serde::{Deserialize, Serialize};

#[path = "input/http.rs"]
mod http;

pub(crate) const MAX_SOURCE_BYTES: usize = 50 * 1024 * 1024;
pub(crate) const MAX_CONFIG_BYTES: usize = 256 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum SourceSpec {
    File {
        path: String,
    },
    Stdin,
    Http {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        url: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        url_env: Option<String>,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct RunSpec {
    #[schemars(extend("const" = "htmlcut.run"))]
    pub(crate) schema: String,
    #[schemars(extend("const" = 1))]
    pub(crate) version: u32,
    pub(crate) source: SourceSpec,
    pub(crate) plan: ExtractionPlan,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) encoding: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) base_url: Option<String>,
}

impl RunSpec {
    pub(crate) fn read(path: &Path) -> Result<Self, ExtractionError> {
        let bytes = read_file(path, MAX_CONFIG_BYTES)?;
        let value = htmlcut_core::parse_closed_json(&bytes)?;
        let run: Self = serde_json::from_value(value)
            .map_err(|_| options("The saved run contains invalid or unknown fields."))?;
        if run.schema != "htmlcut.run" || run.version != 1 {
            return Err(options("Unsupported saved-run schema or version."));
        }
        run.source.validate()?;
        if let Some(base) = &run.base_url {
            SourceSnapshot::new(
                "",
                SnapshotMetadata {
                    base_url: Some(base.clone()),
                },
            )?;
        }
        if let Some(encoding) = &run.encoding {
            encoding_for(Some(encoding))?;
        }
        Ok(run)
    }
}

impl SourceSpec {
    pub(crate) fn validate(&self) -> Result<(), ExtractionError> {
        match self {
            Self::File { path } if path.is_empty() => {
                Err(options("A file source requires a nonempty path."))
            }
            Self::Http {
                url: Some(url),
                url_env: None,
            } => {
                http::validated_url(url)?;
                Ok(())
            }
            Self::Http {
                url: None,
                url_env: Some(name),
            } if valid_env_name(name) => Ok(()),
            Self::Http { .. } => Err(options(
                "An HTTP source requires exactly one literal URL or environment-variable name.",
            )),
            _ => Ok(()),
        }
    }
    pub(crate) fn acquire(
        &self,
        stdin: &mut dyn Read,
        encoding: Option<&str>,
        base: Option<&str>,
        relative_to: &Path,
    ) -> Result<SourceSnapshot, ExtractionError> {
        self.validate()?;
        if let Some(encoding) = encoding {
            encoding_for(Some(encoding))?;
        }
        if let Some(base) = base {
            SourceSnapshot::new(
                "",
                SnapshotMetadata {
                    base_url: Some(base.into()),
                },
            )?;
        }
        let (html, effective_base) = match self {
            Self::File { path } => (
                decode(
                    &read_file(&relative_to.join(path), MAX_SOURCE_BYTES)?,
                    encoding,
                )?,
                base.map(str::to_owned),
            ),
            Self::Stdin => (
                decode(&read_bounded(stdin, MAX_SOURCE_BYTES)?, encoding)?,
                base.map(str::to_owned),
            ),
            Self::Http { url, url_env } => {
                let runtime_url = match (url, url_env) {
                    (Some(url), None) => url.clone(),
                    (None, Some(name)) => std::env::var(name).map_err(|_| {
                        options("The source URL environment variable is unset or not UTF-8.")
                    })?,
                    _ => return Err(options("Invalid HTTP source specification.")),
                };
                let (bytes, response_charset, final_url) = http::fetch(&runtime_url)?;
                (
                    decode(&bytes, encoding.or(response_charset.as_deref()))?,
                    base.map(str::to_owned).or(Some(final_url)),
                )
            }
        };
        SourceSnapshot::new(
            html,
            SnapshotMetadata {
                base_url: effective_base,
            },
        )
    }
}

fn valid_env_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 256
        && name.bytes().enumerate().all(|(index, c)| {
            c.is_ascii_alphabetic() || c == b'_' || (index > 0 && c.is_ascii_digit())
        })
}

pub(crate) fn options(message: &'static str) -> ExtractionError {
    ExtractionError::new(ErrorCode::InvalidOptions, "options", message)
}

pub(crate) fn acquisition() -> ExtractionError {
    ExtractionError::new(
        ErrorCode::Acquisition,
        "acquisition",
        "The source could not be acquired.",
    )
}

pub(crate) fn read_file(path: &Path, maximum: usize) -> Result<Vec<u8>, ExtractionError> {
    read_bounded(&mut File::open(path).map_err(|_| acquisition())?, maximum)
}

pub(crate) fn read_bounded(
    reader: &mut dyn Read,
    maximum: usize,
) -> Result<Vec<u8>, ExtractionError> {
    let mut value = Vec::new();
    let mut chunk = [0_u8; 8192];
    loop {
        let size = reader.read(&mut chunk).map_err(|_| acquisition())?;
        if size == 0 {
            return Ok(value);
        }
        if size > maximum.saturating_sub(value.len()) {
            return Err(limit("acquisition"));
        }
        value.extend_from_slice(&chunk[..size]);
    }
}

pub(crate) fn limit(stage: &'static str) -> ExtractionError {
    ExtractionError::new(
        ErrorCode::ResourceLimit,
        stage,
        "The operation exceeded its configured resource limit.",
    )
}

fn encoding_for(label: Option<&str>) -> Result<&'static Encoding, ExtractionError> {
    match label {
        None => Ok(UTF_8),
        Some(label) => Encoding::for_label(label.as_bytes()).ok_or_else(decoding),
    }
}

fn decoding() -> ExtractionError {
    ExtractionError::new(
        ErrorCode::Decoding,
        "decoding",
        "Source bytes, declared encoding and BOM must form valid, consistent Unicode.",
    )
}

pub(crate) fn decode(bytes: &[u8], label: Option<&str>) -> Result<String, ExtractionError> {
    decode_with_limit(bytes, label, MAX_SOURCE_BYTES)
}

pub(crate) fn decode_with_limit(
    bytes: &[u8],
    label: Option<&str>,
    maximum: usize,
) -> Result<String, ExtractionError> {
    let encoding = encoding_for(label)?;
    let bytes = if let Some((bom_encoding, length)) = Encoding::for_bom(bytes) {
        if bom_encoding != encoding {
            return Err(decoding());
        }
        &bytes[length..]
    } else {
        bytes
    };
    let mut decoder = encoding.new_decoder_without_bom_handling();
    let mut value = String::new();
    let mut offset = 0;
    let mut output = [0_u8; 8192];
    loop {
        let (result, read, written) =
            decoder.decode_to_utf8_without_replacement(&bytes[offset..], &mut output, true);
        if written > maximum.saturating_sub(value.len()) {
            return Err(limit("decoding"));
        }
        value.push_str(std::str::from_utf8(&output[..written]).map_err(|_| decoding())?);
        offset += read;
        match result {
            DecoderResult::InputEmpty => return Ok(value),
            DecoderResult::Malformed(_, _) => return Err(decoding()),
            DecoderResult::OutputFull => (),
        }
    }
}
