use std::io::{self, Write};

use htmlcut_core::{
    extract,
    request::{
        AttributeName, ExtractionRequest, ExtractionSpec, NormalizationOptions, SelectionSpec,
        SelectorQuery, SourceRequest, ValueSpec,
    },
    result::ExtractionMatchMetadata,
};
use serde_json::json;
use url::Url;

pub fn write_request_and_result_namespace_summary<W: Write>(writer: &mut W) -> io::Result<()> {
    let source = SourceRequest::memory(
        "inline",
        "<article><a href=\"../guide.html\">Guide</a></article>",
    )
    .with_base_url(Url::parse("https://example.com/docs/start.html").map_err(io::Error::other)?);
    let request = ExtractionRequest {
        normalization: NormalizationOptions {
            rewrite_urls: true,
            ..Default::default()
        },
        ..ExtractionRequest::new(
            source,
            ExtractionSpec::selector(SelectorQuery::new("article a").map_err(io::Error::other)?)
                .with_selection(SelectionSpec::single())
                .with_value(ValueSpec::Attribute {
                    name: AttributeName::new("href").map_err(io::Error::other)?,
                }),
        )
    };

    let result = extract(&request, &Default::default());
    assert!(result.ok);
    assert_eq!(
        result.matches[0].value.as_str(),
        Some("https://example.com/guide.html")
    );

    let (tag_name, path) = match &result.matches[0].metadata {
        ExtractionMatchMetadata::Selector(metadata) => {
            assert_eq!(metadata.tag_name, "a");
            assert_eq!(
                metadata.path,
                "html:nth-of-type(1) > body:nth-of-type(1) > article:nth-of-type(1) > a:nth-of-type(1)"
            );
            (metadata.tag_name.clone(), metadata.path.clone())
        }
        ExtractionMatchMetadata::DelimiterPair(_) => {
            return Err(io::Error::other(
                "selector extraction should yield selector metadata",
            ));
        }
    };

    serde_json::to_writer_pretty(
        &mut *writer,
        &json!({
            "request_namespace": "htmlcut_core::request",
            "result_namespace": "htmlcut_core::result",
            "value": result.matches[0].value,
            "tag_name": tag_name,
            "path": path,
        }),
    )
    .map_err(io::Error::other)?;
    writer.write_all(b"\n")
}
