mod http;
mod io;
mod metadata;
mod types;

use http as url_loader;

pub(crate) use io::read_stdin_source;
#[cfg(test)]
pub(crate) use io::{
    finish_url_source_from_reader_for_tests, read_file_source as read_file_source_from_path,
    read_limited_to_string, read_stdin_source_from_reader_for_tests,
};
use metadata::source_load_failure;
pub(crate) use metadata::{empty_source_metadata, memory_label, source_metadata};
pub(crate) use types::{LoadedSource, SourceLoadFailure};
#[cfg(all(test, feature = "http-client"))]
pub(crate) use url_loader::read_url_source as read_url_source_from_href;
#[cfg(all(test, feature = "http-client"))]
pub(crate) use url_loader::{
    build_http_agent, content_type_is_obviously_non_html_for_tests,
    head_error_allows_get_fallback_for_tests,
};

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
        SourceInput::Url { href } => url_loader::read_url_source(source, href, runtime),
        SourceInput::File { path } => io::read_file_source(source, path, runtime),
        SourceInput::Stdin => read_stdin_source(source, runtime),
    }
}
