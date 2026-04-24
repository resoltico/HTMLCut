use super::*;

#[test]
fn diagnostic_codes_reject_unknown_strings_with_a_stable_error() {
    for code in DiagnosticCode::ALL {
        assert_eq!(
            DiagnosticCode::from_str(code.as_str()).expect("round-trip parse"),
            *code
        );
    }

    let error = DiagnosticCode::from_str("NOT_A_REAL_CODE").expect_err("invalid diagnostic code");
    assert_eq!(error.to_string(), "unknown HTMLCut diagnostic code");
}
#[test]
fn cli_choice_and_non_empty_contract_values_reject_drift_and_blank_inputs() {
    assert_eq!(
        <ValueType as crate::CliChoice>::parse_cli_str("inner-html"),
        Some(ValueType::InnerHtml)
    );
    assert_eq!(
        <WhitespaceMode as crate::CliChoice>::parse_cli_str("normalize"),
        Some(WhitespaceMode::Normalize)
    );
    assert_eq!(
        <PatternMode as crate::CliChoice>::parse_cli_str("literal"),
        Some(PatternMode::Literal)
    );
    assert_eq!(
        <FetchPreflightMode as crate::CliChoice>::parse_cli_str("head-first"),
        Some(FetchPreflightMode::HeadFirst)
    );
    assert_eq!(
        <ValueType as crate::CliChoice>::parse_cli_str("not-real"),
        None
    );

    let attribute = AttributeName::new("href").expect("attribute");
    assert_eq!(attribute.as_ref(), "href");
    let blank = AttributeName::new("   ").expect_err("blank attribute");
    assert_eq!(
        blank,
        crate::contracts::ContractValueError::Empty {
            field: "attribute name"
        }
    );
}
#[test]
fn render_cli_value_covers_every_public_variant() {
    assert_eq!(
        crate::cli_contract::render_cli_value(crate::cli_contract::CliValue::SelectionMode(
            crate::cli_contract::CliSelectionMode::Nth
        )),
        "nth"
    );
    assert_eq!(
        crate::cli_contract::render_cli_value(crate::cli_contract::CliValue::ValueType(
            ValueType::InnerHtml
        )),
        "inner-html"
    );
    assert_eq!(
        crate::cli_contract::render_cli_value(crate::cli_contract::CliValue::OutputMode(
            crate::cli_contract::CliOutputMode::Html
        )),
        "html"
    );
    assert_eq!(
        crate::cli_contract::render_cli_value(crate::cli_contract::CliValue::WhitespaceMode(
            WhitespaceMode::Normalize
        )),
        "normalize"
    );
    assert_eq!(
        crate::cli_contract::render_cli_value(crate::cli_contract::CliValue::PatternMode(
            PatternMode::Regex
        )),
        "regex"
    );
    assert_eq!(
        crate::cli_contract::render_cli_value(crate::cli_contract::CliValue::FetchPreflightMode(
            FetchPreflightMode::GetOnly
        )),
        "get-only"
    );
    assert_eq!(
        crate::cli_contract::render_cli_value(crate::cli_contract::CliValue::Boolean(true)),
        "true"
    );
    assert_eq!(
        crate::cli_contract::render_cli_value(crate::cli_contract::CliValue::Usize(12)),
        "12"
    );
    assert_eq!(
        crate::cli_contract::render_cli_value(crate::cli_contract::CliValue::U64(64)),
        "64"
    );
}
