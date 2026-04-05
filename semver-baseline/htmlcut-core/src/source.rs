use std::fs;
use std::io::{self, Read};
use std::time::Duration;

use serde_json::json;
use ureq::tls::{RootCerts, TlsConfig};

use crate::contracts::{
    Diagnostic, RuntimeOptions, SourceInput, SourceKind, SourceMetadata, SourceRequest,
};
use crate::diagnostics::error_diagnostic;
use crate::format_byte_size;

#[derive(Clone, Debug)]
pub(crate) struct LoadedSource {
    pub(crate) kind: SourceKind,
    pub(crate) value: String,
    pub(crate) text: String,
    pub(crate) bytes_read: usize,
    pub(crate) input_base_url: Option<String>,
}

pub(crate) fn load_source(
    source: &SourceRequest,
    runtime: &RuntimeOptions,
) -> Result<LoadedSource, Diagnostic> {
    match &source.input {
        SourceInput::Memory { label, text } => {
            if text.len() > runtime.max_bytes {
                return Err(error_diagnostic(
                    "SOURCE_LOAD_FAILED",
                    format!(
                        "Preloaded source exceeds {} limit.",
                        format_byte_size(runtime.max_bytes)
                    ),
                    None,
                ));
            }

            Ok(LoadedSource {
                kind: SourceKind::Memory,
                value: memory_label(label),
                bytes_read: text.len(),
                text: text.clone(),
                input_base_url: source.base_url.as_ref().map(ToString::to_string),
            })
        }
        SourceInput::Url { .. } => read_url_source(source, runtime),
        SourceInput::File { .. } => read_file_source(source, runtime),
        SourceInput::Stdin => read_stdin_source(source, runtime),
    }
}

pub(crate) fn read_url_source(
    source: &SourceRequest,
    runtime: &RuntimeOptions,
) -> Result<LoadedSource, Diagnostic> {
    let SourceInput::Url { href } = &source.input else {
        unreachable!("read_url_source should only be called for URL sources");
    };
    let source_value = href.to_string();
    let agent = build_http_agent(runtime);
    let mut response = agent.get(&source_value).call().map_err(|error| {
        error_diagnostic(
            "SOURCE_LOAD_FAILED",
            format!("Could not fetch {source_value}: {error}"),
            Some(json!({ "source": source_value })),
        )
    })?;

    if response
        .headers()
        .get("content-length")
        .and_then(|content_length| content_length.to_str().ok())
        .and_then(|content_length| content_length.parse::<usize>().ok())
        .is_some_and(|bytes| bytes > runtime.max_bytes)
    {
        return Err(error_diagnostic(
            "SOURCE_LOAD_FAILED",
            format!(
                "Response exceeds {} limit.",
                format_byte_size(runtime.max_bytes)
            ),
            Some(json!({ "source": source_value })),
        ));
    }

    let mut reader = response.body_mut().as_reader();
    let text = read_limited_to_string(&mut reader, runtime.max_bytes, "Response")?;

    Ok(LoadedSource {
        kind: SourceKind::Url,
        value: source_value.clone(),
        bytes_read: text.len(),
        text,
        input_base_url: source
            .base_url
            .as_ref()
            .map(ToString::to_string)
            .or(Some(source_value)),
    })
}

pub(crate) fn build_http_agent(runtime: &RuntimeOptions) -> ureq::Agent {
    let tls_config = TlsConfig::builder()
        .root_certs(RootCerts::PlatformVerifier)
        .build();

    ureq::Agent::config_builder()
        .tls_config(tls_config)
        .timeout_global(Some(Duration::from_millis(runtime.fetch_timeout_ms)))
        .build()
        .into()
}

pub(crate) fn read_file_source(
    source: &SourceRequest,
    runtime: &RuntimeOptions,
) -> Result<LoadedSource, Diagnostic> {
    let SourceInput::File { path } = &source.input else {
        unreachable!("read_file_source should only be called for file sources");
    };
    let source_value = path.to_string_lossy().into_owned();
    let metadata = fs::metadata(path).map_err(|error| {
        error_diagnostic(
            "SOURCE_LOAD_FAILED",
            format!("Could not access file {source_value}: {error}"),
            Some(json!({ "source": source_value })),
        )
    })?;

    if metadata.is_dir() {
        return Err(error_diagnostic(
            "SOURCE_LOAD_FAILED",
            format!("Input path is a directory, not a file: {source_value}"),
            Some(json!({ "source": source_value, "kind": "directory" })),
        ));
    }

    if metadata.len() as usize > runtime.max_bytes {
        return Err(error_diagnostic(
            "SOURCE_LOAD_FAILED",
            format!(
                "File exceeds {} limit.",
                format_byte_size(runtime.max_bytes)
            ),
            Some(json!({ "source": source_value })),
        ));
    }

    let bytes = fs::read(path).map_err(|error| {
        error_diagnostic(
            "SOURCE_LOAD_FAILED",
            format!("Could not read file {source_value}: {error}"),
            Some(json!({ "source": source_value })),
        )
    })?;

    let text = String::from_utf8(bytes).map_err(|error| {
        error_diagnostic(
            "SOURCE_LOAD_FAILED",
            format!("File is not valid UTF-8: {source_value}"),
            Some(json!({
                "source": source_value,
                "utf8_error": error.to_string(),
            })),
        )
    })?;

    let resolved_path = path
        .canonicalize()
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or(source_value);

    Ok(LoadedSource {
        kind: SourceKind::File,
        value: resolved_path,
        bytes_read: text.len(),
        text,
        input_base_url: source.base_url.as_ref().map(ToString::to_string),
    })
}

pub(crate) fn read_stdin_source(
    source: &SourceRequest,
    runtime: &RuntimeOptions,
) -> Result<LoadedSource, Diagnostic> {
    let mut stdin = io::stdin().lock();
    let text = read_limited_to_string(&mut stdin, runtime.max_bytes, "Stdin")?;

    Ok(LoadedSource {
        kind: SourceKind::Stdin,
        value: "-".to_owned(),
        bytes_read: text.len(),
        text,
        input_base_url: source.base_url.as_ref().map(ToString::to_string),
    })
}

pub(crate) fn read_limited_to_string(
    reader: &mut impl Read,
    max_bytes: usize,
    label: &str,
) -> Result<String, Diagnostic> {
    let mut buffer = Vec::new();
    let mut chunk = [0u8; 8192];

    loop {
        let read = reader.read(&mut chunk).map_err(|error| {
            error_diagnostic(
                "SOURCE_LOAD_FAILED",
                format!("Could not read {label}: {error}"),
                None,
            )
        })?;

        if read == 0 {
            break;
        }

        buffer.extend_from_slice(&chunk[..read]);
        if buffer.len() > max_bytes {
            return Err(error_diagnostic(
                "SOURCE_LOAD_FAILED",
                format!("{label} exceeds {} limit.", format_byte_size(max_bytes)),
                None,
            ));
        }
    }

    String::from_utf8(buffer).map_err(|error| {
        error_diagnostic(
            "SOURCE_LOAD_FAILED",
            format!("{label} is not valid UTF-8: {error}"),
            None,
        )
    })
}

pub(crate) fn source_metadata(
    source: &LoadedSource,
    include_text: bool,
    effective_base_url: Option<String>,
) -> SourceMetadata {
    SourceMetadata {
        kind: source.kind,
        value: source.value.clone(),
        input_base_url: source.input_base_url.clone(),
        effective_base_url,
        bytes_read: source.bytes_read,
        text: include_text.then_some(source.text.clone()),
    }
}

pub(crate) fn empty_source_metadata(source: &SourceRequest) -> SourceMetadata {
    let kind = source.kind();
    let value = source_locator_value(&source.input);
    let input_base_url = source
        .base_url
        .as_ref()
        .map(ToString::to_string)
        .or_else(|| matches!(source.input, SourceInput::Url { .. }).then(|| value.clone()));
    SourceMetadata {
        kind,
        value,
        input_base_url: input_base_url.clone(),
        effective_base_url: input_base_url,
        bytes_read: 0,
        text: None,
    }
}

fn source_locator_value(input: &SourceInput) -> String {
    match input {
        SourceInput::Url { href } => href.to_string(),
        SourceInput::File { path } => path.to_string_lossy().into_owned(),
        SourceInput::Stdin => "-".to_owned(),
        SourceInput::Memory { label, .. } => memory_label(label),
    }
}

fn memory_label(label: &str) -> String {
    if label.trim().is_empty() {
        "memory".to_owned()
    } else {
        label.to_owned()
    }
}
