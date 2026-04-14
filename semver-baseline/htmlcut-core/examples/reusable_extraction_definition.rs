use htmlcut_core::{
    ExtractionDefinition, extract,
    request::{
        AttributeName, ExtractionRequest, ExtractionSpec, NormalizationOptions, RuntimeOptions,
        SelectionSpec, SelectorQuery, SourceRequest, ValueSpec,
    },
};
use url::Url;

fn main() {
    let source = SourceRequest::memory(
        "inline",
        "<article><a href=\"../guide.html\">Guide</a></article>",
    )
    .with_base_url(Url::parse("https://example.com/docs/start.html").expect("base url"));

    let mut request = ExtractionRequest::new(
        source,
        ExtractionSpec::selector(SelectorQuery::new("article a").expect("selector"))
            .with_selection(SelectionSpec::single())
            .with_value(ValueSpec::Attribute {
                name: AttributeName::new("href").expect("attribute"),
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

    let encoded = serde_json::to_string_pretty(&definition).expect("serialize definition");
    let decoded: ExtractionDefinition =
        serde_json::from_str(&encoded).expect("deserialize definition");

    let result = extract(&decoded.request, &decoded.runtime);
    assert!(result.ok);
    assert_eq!(
        result.matches[0].value.as_str(),
        Some("https://example.com/guide.html")
    );

    println!("{encoded}");
}
