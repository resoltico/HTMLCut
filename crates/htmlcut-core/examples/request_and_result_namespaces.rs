use htmlcut_core::{
    extract,
    request::{
        AttributeName, ExtractionRequest, ExtractionSpec, NormalizationOptions, SelectionSpec,
        SelectorQuery, SourceRequest, ValueSpec,
    },
    result::ExtractionMatchMetadata,
};
use url::Url;

fn main() {
    let source = SourceRequest::memory(
        "inline",
        "<article><a href=\"../guide.html\">Guide</a></article>",
    )
    .with_base_url(Url::parse("https://example.com/docs/start.html").expect("base url"));

    let request = ExtractionRequest {
        normalization: NormalizationOptions {
            rewrite_urls: true,
            ..Default::default()
        },
        ..ExtractionRequest::new(
            source,
            ExtractionSpec::selector(SelectorQuery::new("article a").expect("selector"))
                .with_selection(SelectionSpec::single())
                .with_value(ValueSpec::Attribute {
                    name: AttributeName::new("href").expect("attribute"),
                }),
        )
    };

    let result = extract(&request, &Default::default());
    assert!(result.ok);
    assert_eq!(
        result.matches[0].value.as_str(),
        Some("https://example.com/guide.html")
    );

    match &result.matches[0].metadata {
        ExtractionMatchMetadata::Selector(metadata) => {
            assert_eq!(metadata.tag_name, "a");
            assert_eq!(
                metadata.path,
                "html:nth-of-type(1) > body:nth-of-type(1) > article:nth-of-type(1) > a:nth-of-type(1)"
            );
        }
        ExtractionMatchMetadata::DelimiterPair(_) => unreachable!("selector extraction"),
    }
}
