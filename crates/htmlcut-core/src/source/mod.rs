mod http;
mod io;
mod metadata;
mod types;

#[cfg(test)]
pub(crate) use http::{
    build_http_agent, content_type_is_obviously_non_html_for_tests,
    head_error_allows_get_fallback_for_tests, read_url_source,
};
pub(crate) use io::read_stdin_source;
#[cfg(test)]
pub(crate) use io::{
    finish_url_source_from_reader_for_tests, read_file_source, read_limited_to_string,
    read_stdin_source_from_reader_for_tests,
};
use metadata::source_load_failure;
pub(crate) use metadata::{empty_source_metadata, memory_label, source_metadata};
pub(crate) use types::{LoadedSource, SourceLoadFailure};

use crate::contracts::{RuntimeOptions, SourceInput, SourceKind, SourceRequest};
use crate::diagnostics::{DiagnosticCode, error_diagnostic};
use crate::format_byte_size;

pub(crate) fn load_source(
    source: &SourceRequest,
    runtime: &RuntimeOptions,
) -> Result<LoadedSource, SourceLoadFailure> {
    match &source.input {
        SourceInput::Memory { label, text } => {
            if text.len() > runtime.max_bytes {
                return Err(source_load_failure(
                    source,
                    SourceKind::Memory,
                    memory_label(label),
                    Vec::new(),
                    error_diagnostic(
                        DiagnosticCode::SourceLoadFailed,
                        format!(
                            "Preloaded source exceeds {} limit.",
                            format_byte_size(runtime.max_bytes)
                        ),
                        None,
                    ),
                ));
            }

            Ok(LoadedSource {
                kind: SourceKind::Memory,
                value: memory_label(label),
                bytes_read: text.len(),
                text: text.clone(),
                input_base_url: source.base_url.as_ref().map(ToString::to_string),
                load_steps: Vec::new(),
            })
        }
        SourceInput::Url { .. } => http::read_url_source(source, runtime),
        SourceInput::File { .. } => io::read_file_source(source, runtime),
        SourceInput::Stdin => read_stdin_source(source, runtime),
    }
}
