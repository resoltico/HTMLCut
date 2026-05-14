use serde_json::json;

use crate::contracts::{HttpUrl, RuntimeOptions, SourceKind, SourceRequest};
use crate::diagnostics::{DiagnosticCode, error_diagnostic};

use super::super::metadata::source_load_failure;
use super::super::{LoadedSource, SourceLoadFailure};

pub(crate) fn read_url_source(
    source: &SourceRequest,
    href: &HttpUrl,
    _runtime: &RuntimeOptions,
) -> Result<LoadedSource, SourceLoadFailure> {
    let source_value = href.to_string();
    Err(source_load_failure(
        source,
        SourceKind::Url,
        source_value.clone(),
        Vec::new(),
        error_diagnostic(
            DiagnosticCode::SourceLoadFailed,
            "This build does not include the built-in HTTP(S) source loader. Rebuild with the `htmlcut-core/http-client` feature or fetch the HTML externally and pass it through memory, file, or stdin input."
                .to_owned(),
            Some(json!({
                "source": source_value,
                "requiredFeature": "http-client",
            })),
        ),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::contracts::{ExtractionSpec, RuntimeOptions, SelectorQuery};

    #[test]
    fn disabled_loader_reports_the_http_client_capability_gap_without_leaking_raw_url_state() {
        let href = HttpUrl::parse("https://example.com/docs?token=secret").expect("url");
        let request = crate::ExtractionRequest::new(
            SourceRequest::url(href.clone()),
            ExtractionSpec::selector(SelectorQuery::new("article").expect("selector")),
        );

        let failure = read_url_source(&request.source, &href, &RuntimeOptions::default())
            .expect_err("disabled loader should fail");

        assert_eq!(failure.metadata.kind, SourceKind::Url);
        assert_eq!(
            failure.metadata.value,
            "https://example.com/docs?[redacted]"
        );
        assert!(failure.metadata.load_steps.is_empty());
        assert_eq!(failure.diagnostic.code, DiagnosticCode::SourceLoadFailed);
        assert_eq!(
            failure
                .diagnostic
                .details
                .as_ref()
                .and_then(|details| details.get("requiredFeature")),
            Some(&json!("http-client"))
        );
        assert_eq!(
            failure
                .diagnostic
                .details
                .as_ref()
                .and_then(|details| details.get("source")),
            Some(&json!("https://example.com/docs?[redacted]"))
        );
    }
}
