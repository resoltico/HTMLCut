use super::*;
use url::Url;

#[test]
fn request_value_objects_cover_http_urls_attributes_and_boundary_retention() {
    let url = http_url("https://example.com/docs/page.html?token=secret#frag");
    assert_eq!(
        url.as_url().as_str(),
        "https://example.com/docs/page.html?token=secret#frag"
    );
    assert_eq!(
        url.as_fetch_str(),
        "https://example.com/docs/page.html?token=secret#frag"
    );
    assert_eq!(
        url.display_url(),
        "https://example.com/docs/page.html?[redacted]"
    );
    assert_eq!(
        url.to_string(),
        "https://example.com/docs/page.html?[redacted]"
    );
    assert_eq!(
        format!("{url:?}"),
        "HttpUrl(\"https://example.com/docs/page.html?[redacted]\")"
    );

    let from_string =
        HttpUrl::try_from("https://example.com/next".to_owned()).expect("string http url");
    assert_eq!(from_string.as_fetch_str(), "https://example.com/next");
    let from_url =
        HttpUrl::try_from(Url::parse("http://example.com/raw").expect("url")).expect("http url");
    assert_eq!(String::from(from_url.clone()), "http://example.com/raw");
    let fragment_only =
        HttpUrl::parse("https://example.com/docs/page.html#section").expect("fragment only url");
    assert_eq!(
        fragment_only.display_url(),
        "https://example.com/docs/page.html"
    );
    assert_eq!(
        "https://example.com/from-str"
            .parse::<HttpUrl>()
            .expect("from str")
            .as_fetch_str(),
        "https://example.com/from-str"
    );

    assert!(matches!(
        HttpUrl::parse("ftp://example.com/archive"),
        Err(ContractValueError::UnsupportedUrlScheme { scheme, .. }) if scheme == "ftp"
    ));
    assert!(matches!(
        HttpUrl::parse("https://alice:secret@example.com/private"),
        Err(ContractValueError::UrlUserInfoUnsupported { .. })
    ));
    assert!(matches!(
        HttpUrl::parse("https://:secret@example.com/private"),
        Err(ContractValueError::UrlUserInfoUnsupported { .. })
    ));
    assert!(matches!(
        HttpUrl::parse("::not-a-url::"),
        Err(ContractValueError::InvalidUrl { .. })
    ));

    let attribute = AttributeName::new("HREF").expect("attribute");
    assert_eq!(attribute.as_str(), "href");
    assert_eq!(attribute.as_ref(), "href");
    assert_eq!(attribute.to_string(), "href");

    assert_eq!(
        BoundaryRetention::from_flags(false, false),
        BoundaryRetention::ExcludeBoth
    );
    assert_eq!(
        BoundaryRetention::from_flags(true, false),
        BoundaryRetention::IncludeStart
    );
    assert_eq!(
        BoundaryRetention::from_flags(false, true),
        BoundaryRetention::IncludeEnd
    );
    assert_eq!(
        BoundaryRetention::from_flags(true, true),
        BoundaryRetention::IncludeBoth
    );
    assert!(BoundaryRetention::IncludeBoth.includes_start());
    assert!(BoundaryRetention::IncludeEnd.includes_end());
    assert!(!BoundaryRetention::ExcludeBoth.includes_start());
    assert!(!BoundaryRetention::ExcludeBoth.includes_end());
}
