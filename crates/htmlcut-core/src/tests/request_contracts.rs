use super::*;
use schemars::{JsonSchema, SchemaGenerator};
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
    assert_eq!(
        PersistedHttpUrl::parse("https://example.com/archive")
            .expect("persisted url")
            .as_http_url()
            .as_fetch_str(),
        "https://example.com/archive"
    );
    assert_eq!(
        DisplayedHttpUrl::from(&url).as_str(),
        "https://example.com/docs/page.html?[redacted]"
    );
    assert_eq!(
        DisplayedHttpUrl::parse("https://example.com/docs/page.html?[redacted]")
            .expect("displayed redacted url")
            .as_str(),
        "https://example.com/docs/page.html?[redacted]"
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
    assert!(matches!(
        PersistedHttpUrl::parse("https://example.com/archive?token=secret"),
        Err(ContractValueError::UrlQueryUnsupported { .. })
    ));
    assert!(matches!(
        PersistedHttpUrl::parse("https://example.com/archive#section"),
        Err(ContractValueError::UrlFragmentUnsupported { .. })
    ));
    assert!(matches!(
        DisplayedHttpUrl::parse("https://example.com/archive?token=secret"),
        Err(ContractValueError::UrlUnredactedQueryUnsupported { .. })
    ));
    assert!(matches!(
        DisplayedHttpUrl::parse("https://example.com/archive#section"),
        Err(ContractValueError::UrlFragmentUnsupported { .. })
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

#[test]
fn request_schemas_expose_runtime_and_url_constraints() {
    let schema = (crate::schema_descriptor(
        crate::EXTRACTION_DEFINITION_SCHEMA_NAME,
        crate::EXTRACTION_DEFINITION_SCHEMA_VERSION,
    )
    .expect("schema descriptor")
    .json_schema)()
    .expect("json schema");

    let defs = schema["$defs"].as_object().expect("schema defs");
    assert_eq!(defs["MaxBytes"]["minimum"], json!(1));
    assert_eq!(defs["FetchTimeoutMs"]["minimum"], json!(1));
    assert_eq!(defs["FetchConnectTimeoutMs"]["minimum"], json!(1));
    assert_eq!(
        defs["RuntimeOptionsDocument"]["properties"]["fetch_timeout_ms"]["$ref"],
        json!("#/$defs/FetchTimeoutMs")
    );
    assert_eq!(
        defs["RuntimeOptionsDocument"]["properties"]["fetch_connect_timeout_ms"]["$ref"],
        json!("#/$defs/FetchConnectTimeoutMs")
    );

    let source_input = defs["SourceInputDocument"]["oneOf"]
        .as_array()
        .expect("source input variants")
        .iter()
        .find(|variant| variant["properties"]["type"]["const"] == json!("url"))
        .expect("url source input variant");
    let href_checks = source_input["properties"]["href"]["allOf"]
        .as_array()
        .expect("href validation checks");
    assert!(
        href_checks
            .iter()
            .any(|check| check["pattern"] == json!("^https?://"))
    );
    assert!(
        href_checks
            .iter()
            .any(|check| check["not"]["pattern"] == json!("\\?"))
    );

    let base_url_variants = defs["SourceRequestDocument"]["properties"]["base_url"]["anyOf"]
        .as_array()
        .expect("base url variants");
    let base_url = base_url_variants
        .iter()
        .find(|variant| variant["type"] == json!("string"))
        .expect("base url string variant");
    let base_url_checks = base_url["allOf"]
        .as_array()
        .expect("base url validation checks");
    assert!(
        base_url_checks
            .iter()
            .any(|check| check["pattern"] == json!("^https?://"))
    );
    assert!(
        base_url_checks
            .iter()
            .any(|check| check["not"]["pattern"] == json!("\\?"))
    );

    let interop_result_schema = (crate::schema_descriptor(
        crate::interop::v1::RESULT_SCHEMA_NAME,
        crate::interop::v1::RESULT_SCHEMA_VERSION,
    )
    .expect("interop result schema descriptor")
    .json_schema)()
    .expect("interop result json schema");
    let interop_defs = interop_result_schema["$defs"]
        .as_object()
        .expect("interop result schema defs");
    let display_url_variants =
        interop_defs["ResultSource"]["properties"]["input_base_url"]["anyOf"]
            .as_array()
            .expect("display url variants");
    let display_url = display_url_variants
        .iter()
        .find(|variant| variant["type"] == json!("string"))
        .expect("display url string variant");
    let display_url_checks = display_url["allOf"]
        .as_array()
        .expect("display url validation checks");
    assert!(
        display_url_checks
            .iter()
            .any(|check| check["pattern"] == json!("^https?://"))
    );
    assert!(
        display_url_checks
            .iter()
            .any(|check| { check["anyOf"][1]["pattern"] == json!("\\?\\[redacted\\]$") })
    );
}

#[test]
fn request_url_value_objects_cover_schema_metadata_and_all_public_conversions() {
    assert!(<HttpUrl as JsonSchema>::inline_schema());
    assert_eq!(<HttpUrl as JsonSchema>::schema_name(), "HttpUrl");
    assert!(
        <HttpUrl as JsonSchema>::schema_id().ends_with("::contracts::request::primitives::HttpUrl")
    );
    let mut http_schema_generator = SchemaGenerator::default();
    let http_schema = serde_json::to_value(<HttpUrl as JsonSchema>::json_schema(
        &mut http_schema_generator,
    ))
    .expect("http url schema");
    assert_eq!(
        http_schema["description"],
        json!("Absolute HTTP or HTTPS URL without URL userinfo.")
    );
    let http_checks = http_schema["allOf"].as_array().expect("http url checks");
    assert!(
        http_checks
            .iter()
            .any(|check| check["pattern"] == json!("^https?://"))
    );
    assert!(http_checks.iter().all(|check| check.get("anyOf").is_none()));
    assert!(
        http_checks
            .iter()
            .all(|check| check["not"]["pattern"] != json!("#"))
    );

    let persisted = PersistedHttpUrl::try_from("https://example.com/replay".to_owned())
        .expect("persisted string url");
    assert_eq!(persisted.to_string(), "https://example.com/replay");
    assert_eq!(
        format!("{persisted:?}"),
        "PersistedHttpUrl(\"https://example.com/replay\")"
    );
    assert_eq!(
        HttpUrl::from(persisted.clone()).as_fetch_str(),
        "https://example.com/replay"
    );
    assert_eq!(
        "https://example.com/replay"
            .parse::<PersistedHttpUrl>()
            .expect("persisted from str")
            .to_string(),
        "https://example.com/replay"
    );
    assert!(matches!(
        PersistedHttpUrl::try_from(http_url("https://example.com/raw?token=secret")),
        Err(ContractValueError::UrlQueryUnsupported { .. })
    ));
    assert!(matches!(
        PersistedHttpUrl::parse("not a url"),
        Err(ContractValueError::InvalidUrl { .. })
    ));
    assert!(<PersistedHttpUrl as JsonSchema>::inline_schema());
    assert_eq!(
        <PersistedHttpUrl as JsonSchema>::schema_name(),
        "PersistedHttpUrl"
    );
    assert!(
        <PersistedHttpUrl as JsonSchema>::schema_id()
            .ends_with("::contracts::request::primitives::PersistedHttpUrl")
    );
    let mut persisted_schema_generator = SchemaGenerator::default();
    let persisted_schema = serde_json::to_value(<PersistedHttpUrl as JsonSchema>::json_schema(
        &mut persisted_schema_generator,
    ))
    .expect("persisted url schema");
    assert_eq!(
        persisted_schema["description"],
        json!("Absolute replayable HTTP or HTTPS URL without userinfo, query, or fragment.")
    );
    let persisted_checks = persisted_schema["allOf"]
        .as_array()
        .expect("persisted url checks");
    assert!(
        persisted_checks
            .iter()
            .any(|check| check["not"]["pattern"] == json!("\\?"))
    );
    assert!(
        persisted_checks
            .iter()
            .any(|check| check["not"]["pattern"] == json!("#"))
    );

    let displayed = DisplayedHttpUrl::try_from("https://example.com/report?[redacted]".to_owned())
        .expect("displayed string url");
    assert_eq!(displayed.as_ref(), "https://example.com/report?[redacted]");
    assert_eq!(
        displayed.to_string(),
        "https://example.com/report?[redacted]"
    );
    assert_eq!(
        format!("{displayed:?}"),
        "DisplayedHttpUrl(\"https://example.com/report?[redacted]\")"
    );
    assert_eq!(
        "https://example.com/report?[redacted]"
            .parse::<DisplayedHttpUrl>()
            .expect("displayed from str")
            .as_str(),
        "https://example.com/report?[redacted]"
    );
    assert!(<DisplayedHttpUrl as JsonSchema>::inline_schema());
    assert_eq!(
        <DisplayedHttpUrl as JsonSchema>::schema_name(),
        "DisplayedHttpUrl"
    );
    assert!(
        <DisplayedHttpUrl as JsonSchema>::schema_id()
            .ends_with("::contracts::request::primitives::DisplayedHttpUrl")
    );
    let mut displayed_schema_generator = SchemaGenerator::default();
    let displayed_schema = serde_json::to_value(<DisplayedHttpUrl as JsonSchema>::json_schema(
        &mut displayed_schema_generator,
    ))
    .expect("displayed url schema");
    assert_eq!(
        displayed_schema["description"],
        json!(
            "Safe display URL for diagnostics and result artifacts. Userinfo and fragments are forbidden, and any query string must be the exact `?[redacted]` marker."
        )
    );
    let displayed_checks = displayed_schema["allOf"]
        .as_array()
        .expect("displayed url checks");
    assert!(
        displayed_checks
            .iter()
            .any(|check| check["anyOf"][1]["pattern"] == json!("\\?\\[redacted\\]$"))
    );
    assert!(
        displayed_checks
            .iter()
            .any(|check| check["not"]["pattern"] == json!("#"))
    );
}
