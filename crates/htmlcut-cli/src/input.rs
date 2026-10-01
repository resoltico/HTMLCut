//! Adapter-only source specifications and strict charset decoding.

use std::fs::File;
use std::io::Read;
use std::path::Path;

use encoding_rs::{DecoderResult, Encoding, UTF_8};
use htmlcut_core::{ErrorCode, ExtractionError, ExtractionPlan, SnapshotMetadata, SourceSnapshot};
use serde::{Deserialize, Serialize};

#[path = "input/http.rs"]
mod http;
#[path = "input/regular_file.rs"]
mod regular_file;

pub(crate) const MAX_SOURCE_BYTES: usize = 50 * 1024 * 1024;
pub(crate) const MAX_CONFIG_BYTES: usize = 256 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum SourceSpec {
    File {
        path: String,
    },
    Stdin {},
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
    #[schemars(extend("const" = htmlcut_core::SCHEMA_VERSION))]
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
        if run.schema != "htmlcut.run" || run.version != htmlcut_core::SCHEMA_VERSION {
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
            configured_encoding(encoding)?;
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
        self.acquire_with(
            stdin,
            encoding,
            base,
            relative_to,
            &mut Acquisition {
                fetch: http::fetch,
                now: std::time::Instant::now,
            },
        )
    }

    fn acquire_with<F, C>(
        &self,
        stdin: &mut dyn Read,
        encoding: Option<&str>,
        base: Option<&str>,
        relative_to: &Path,
        adapter: &mut Acquisition<F, C>,
    ) -> Result<SourceSnapshot, ExtractionError>
    where
        F: FnMut(&str, bool) -> Result<http::Acquired, ExtractionError>,
        C: Fn() -> std::time::Instant,
    {
        self.validate()?;
        if let Some(encoding) = encoding {
            configured_encoding(encoding)?;
        }
        if let Some(base) = base {
            SourceSnapshot::new(
                "",
                SnapshotMetadata {
                    base_url: Some(base.into()),
                },
            )?;
        }
        let mut deadline = None;
        let (html, effective_base) = match self {
            Self::File { path } => (
                decode(
                    &read_file(&relative_to.join(path), MAX_SOURCE_BYTES)?,
                    encoding,
                )?,
                base.map(str::to_owned),
            ),
            Self::Stdin {} => (
                decode(&read_bounded(stdin, MAX_SOURCE_BYTES)?, encoding)?,
                base.map(str::to_owned),
            ),
            Self::Http { url, url_env } => {
                // validate() proves exactly one source reference is present.
                let runtime_url = if let Some(url) = url {
                    url.clone()
                } else {
                    std::env::var(
                        url_env
                            .as_ref()
                            .expect("validated URL environment reference"),
                    )
                    .map_err(|_| {
                        options("The source URL environment variable is unset or not UTF-8.")
                    })?
                };
                let fetched = (adapter.fetch)(&runtime_url, encoding.is_some())?;
                deadline = Some(fetched.deadline);
                let html = decode_until(
                    &fetched.bytes,
                    encoding.or(fetched.charset.as_deref()),
                    MAX_SOURCE_BYTES,
                    Some(fetched.deadline),
                )?;
                if (adapter.now)() >= fetched.deadline {
                    return Err(limit("acquisition"));
                }
                (html, base.map(str::to_owned).or(Some(fetched.final_url)))
            }
        };
        let snapshot = SourceSnapshot::new(
            html,
            SnapshotMetadata {
                base_url: effective_base,
            },
        )?;
        if deadline.is_some_and(|deadline| (adapter.now)() >= deadline) {
            return Err(limit("acquisition"));
        }
        Ok(snapshot)
    }
}

// Private adapter seam covers the deadline through decode and accepted snapshot hashing.
struct Acquisition<F, C> {
    fetch: F,
    now: C,
}

fn valid_env_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 256
        && name.bytes().enumerate().all(|(index, c)| {
            c.is_ascii_alphabetic() || c == b'_' || (index > 0 && c.is_ascii_digit())
        })
}

pub(crate) fn options(message: &'static str) -> ExtractionError {
    ExtractionError::new(ErrorCode::InvalidOptions, "options", message).with_cause(
        htmlcut_core::FailureCause::Configuration {
            role: htmlcut_core::ConfigurationRole::Arguments,
            problem: htmlcut_core::ConfigurationProblem::InvalidValue,
        },
    )
}

pub(crate) fn acquisition() -> ExtractionError {
    ExtractionError::new(
        ErrorCode::Acquisition,
        "acquisition",
        "The source could not be acquired.",
    )
}

pub(crate) fn io_failure(error: std::io::Error) -> ExtractionError {
    let problem = match error.kind() {
        std::io::ErrorKind::NotFound => htmlcut_core::IoProblem::NotFound,
        std::io::ErrorKind::PermissionDenied => htmlcut_core::IoProblem::PermissionDenied,
        std::io::ErrorKind::BrokenPipe => htmlcut_core::IoProblem::BrokenPipe,
        _ => {
            #[cfg(unix)]
            if error.raw_os_error() == Some(rustix::io::Errno::BADF.raw_os_error()) {
                return acquisition().with_cause(htmlcut_core::FailureCause::Io {
                    operation: htmlcut_core::IoOperation::Input,
                    problem: htmlcut_core::IoProblem::InvalidDescriptor,
                });
            }
            htmlcut_core::IoProblem::Other
        }
    };
    acquisition().with_cause(htmlcut_core::FailureCause::Io {
        operation: htmlcut_core::IoOperation::Input,
        problem,
    })
}

pub(crate) fn read_file(path: &Path, maximum: usize) -> Result<Vec<u8>, ExtractionError> {
    read_bounded(&mut regular_file::open(path)?, maximum)
}

pub(crate) fn read_bounded(
    reader: &mut dyn Read,
    maximum: usize,
) -> Result<Vec<u8>, ExtractionError> {
    let mut value = Vec::new();
    let mut chunk = [0_u8; 8192];
    loop {
        let size = reader.read(&mut chunk).map_err(io_failure)?;
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
    .with_cause(htmlcut_core::FailureCause::Resource {})
}

fn configured_encoding(label: &str) -> Result<&'static Encoding, ExtractionError> {
    Encoding::for_label(label.as_bytes()).ok_or_else(|| {
        options("The configured encoding is unsupported.").with_cause(
            htmlcut_core::FailureCause::Configuration {
                role: htmlcut_core::ConfigurationRole::Encoding,
                problem: htmlcut_core::ConfigurationProblem::InvalidValue,
            },
        )
    })
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
    .with_cause(htmlcut_core::FailureCause::Charset {})
}

pub(crate) fn decode(bytes: &[u8], label: Option<&str>) -> Result<String, ExtractionError> {
    decode_with_limit(bytes, label, MAX_SOURCE_BYTES)
}

pub(crate) fn decode_with_limit(
    bytes: &[u8],
    label: Option<&str>,
    maximum: usize,
) -> Result<String, ExtractionError> {
    decode_until(bytes, label, maximum, None)
}

fn decode_until(
    bytes: &[u8],
    label: Option<&str>,
    maximum: usize,
    deadline: Option<std::time::Instant>,
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
        if deadline.is_some_and(|deadline| std::time::Instant::now() >= deadline) {
            return Err(limit("decoding"));
        }
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

#[cfg(test)]
#[path = "input/tests.rs"]
mod tests;
