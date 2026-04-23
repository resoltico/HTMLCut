use super::*;

#[test]
fn interop_public_helpers_cover_selection_modes_and_html_input_paths() {
    assert!(selector_plan().validate().is_ok());
    assert_eq!(Selection::single().mode(), SelectionMode::Single);
    assert_eq!(Selection::first().mode(), SelectionMode::First);
    assert_eq!(
        Selection::nth(NonZeroUsize::new(3).expect("index")).mode(),
        SelectionMode::Nth
    );

    let source = HtmlInput::new("inline", "<article>Hello</article>").expect("source");
    assert_eq!(source.to_source_request().kind(), SourceKind::Memory);
    assert_eq!(
        source.clone().into_source_request().kind(),
        SourceKind::Memory
    );

    let with_base =
        source.with_input_base_url(Url::parse("https://example.com/start.html").expect("url"));
    assert_eq!(
        with_base
            .clone()
            .into_source_request()
            .base_url
            .as_ref()
            .map(Url::as_str),
        Some("https://example.com/start.html")
    );
}
#[test]
fn interop_schema_identity_helpers_reject_name_version_and_profile_drift() {
    let name_error = v1::validate_schema_identity_for_tests(
        "wrong",
        v1::PLAN_SCHEMA_NAME,
        v1::PLAN_SCHEMA_VERSION,
        v1::PLAN_SCHEMA_VERSION,
        v1::INTEROP_V1_PROFILE,
        v1::INTEROP_V1_PROFILE,
    )
    .expect_err("schema name drift");
    assert!(matches!(
        name_error,
        ContractError::InvalidIdentity {
            field: "schema_name",
            ..
        }
    ));

    let version_error = v1::validate_schema_identity_for_tests(
        v1::PLAN_SCHEMA_NAME,
        v1::PLAN_SCHEMA_NAME,
        99,
        v1::PLAN_SCHEMA_VERSION,
        v1::INTEROP_V1_PROFILE,
        v1::INTEROP_V1_PROFILE,
    )
    .expect_err("schema version drift");
    assert!(matches!(
        version_error,
        ContractError::InvalidVersion {
            field: "schema_version",
            ..
        }
    ));

    let profile_error = v1::validate_schema_identity_for_tests(
        v1::PLAN_SCHEMA_NAME,
        v1::PLAN_SCHEMA_NAME,
        v1::PLAN_SCHEMA_VERSION,
        v1::PLAN_SCHEMA_VERSION,
        "wrong-profile",
        v1::INTEROP_V1_PROFILE,
    )
    .expect_err("interop profile drift");
    assert!(matches!(
        profile_error,
        ContractError::InvalidIdentity {
            field: "interop_profile",
            ..
        }
    ));

    let mut plan = selector_plan();
    plan.schema_name = "wrong".to_owned();
    let plan_error = plan
        .validate()
        .expect_err("public plan validate should reject drift");
    assert!(matches!(
        plan_error,
        ContractError::InvalidIdentity {
            field: "schema_name",
            ..
        }
    ));
}
