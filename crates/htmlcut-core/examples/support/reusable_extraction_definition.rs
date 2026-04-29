use std::io::{self, Write};

use htmlcut_core::{
    ExtractionDefinition, extract,
    request::{
        AttributeName, ExtractionRequest, ExtractionSpec, NormalizationOptions, RuntimeOptions,
        SelectionSpec, SelectorQuery, SourceRequest, ValueSpec,
    },
};
use url::Url;

pub fn write_reusable_extraction_definition<W: Write>(writer: &mut W) -> io::Result<()> {
    let source = SourceRequest::memory(
        "inline",
        "<article><a href=\"../guide.html\">Guide</a></article>",
    )
    .with_base_url(Url::parse("https://example.com/docs/start.html").map_err(io::Error::other)?);

    let mut request = ExtractionRequest::new(
        source,
        ExtractionSpec::selector(SelectorQuery::new("article a").map_err(io::Error::other)?)
            .with_selection(SelectionSpec::single())
            .with_value(ValueSpec::Attribute {
                name: AttributeName::new("href").map_err(io::Error::other)?,
            }),
    );
    request.normalization = NormalizationOptions {
        rewrite_urls: true,
        ..Default::default()
    };

    let definition = ExtractionDefinition {
        runtime: RuntimeOptions::default(),
        ..ExtractionDefinition::new(request)
    };

    let encoded = serde_json::to_string_pretty(&definition).map_err(io::Error::other)?;
    let decoded: ExtractionDefinition = serde_json::from_str(&encoded).map_err(io::Error::other)?;

    let result = extract(&decoded.request, &decoded.runtime);
    assert!(result.ok);
    assert_eq!(
        result.matches[0].value.as_str(),
        Some("https://example.com/guide.html")
    );

    writer.write_all(encoded.as_bytes())?;
    writer.write_all(b"\n")
}
