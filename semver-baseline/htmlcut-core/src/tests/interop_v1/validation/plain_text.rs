use super::*;

fn selector_result_for(output: Output) -> InteropResult {
    let output_value = match &output {
        Output::Structured => serde_json::json!({"plainTextOutput": "Hello"}),
        _ => serde_json::json!("Hello"),
    };
    let mut selected_matches = selector_selected_matches();
    selected_matches[0].output_value = output_value;
    InteropResult::new(
        ResultExecution::new(
            TEST_PLAN_DIGEST_SHA256,
            StrategyKind::CssSelector,
            SelectionMode::Single,
            output,
            1,
        ),
        ResultSource {
            input_base_url: None,
            effective_base_url: None,
            document_title: None,
        },
        selected_matches,
        Vec::new(),
    )
    .with_computed_digest()
    .expect("valid selector result digest")
}

fn delimiter_result_for(output: Output) -> InteropResult {
    InteropResult::new(
        ResultExecution::new(
            TEST_PLAN_DIGEST_SHA256,
            StrategyKind::DelimiterPair,
            SelectionMode::Single,
            output,
            1,
        ),
        ResultSource {
            input_base_url: None,
            effective_base_url: None,
            document_title: None,
        },
        vec![delimiter_selected_match_with(1, 1)],
        Vec::new(),
    )
    .with_computed_digest()
    .expect("valid delimiter result digest")
}

#[test]
fn interop_result_validation_closes_plain_text_evidence_for_every_strategy_and_output_kind() {
    assert_eq!(OutputKind::PlainText.as_str(), "plain_text");

    let mut missing_css_plain_text = selector_result_for(Output::text());
    missing_css_plain_text.selected_matches[0].plain_text_output = None;
    assert!(matches!(
        missing_css_plain_text
            .validate()
            .expect_err("CSS selected matches require plain-text evidence"),
        ContractError::MissingPlainTextOutput
    ));

    let mut delimiter_with_plain_text = delimiter_result_for(Output::inner_html());
    delimiter_with_plain_text.selected_matches[0].plain_text_output = Some("invented".to_owned());
    assert!(matches!(
        delimiter_with_plain_text
            .validate()
            .expect_err("delimiter selected matches must not carry DOM plain text"),
        ContractError::UnexpectedPlainTextOutput
    ));

    let mut delimiter_with_plain_comparison = delimiter_result_for(Output::inner_html());
    delimiter_with_plain_comparison.selected_matches[0].comparison_plain_text_output =
        Some("invented".to_owned());
    assert!(matches!(
        delimiter_with_plain_comparison
            .validate()
            .expect_err("delimiter selected matches must not carry a plain-text comparison"),
        ContractError::UnexpectedComparisonPlainTextOutput
    ));

    let mut raw_output_with_plain_comparison =
        selector_result_for(Output::attribute(output_attribute_name("href")));
    raw_output_with_plain_comparison.selected_matches[0].comparison_plain_text_output =
        Some("invented".to_owned());
    assert!(matches!(
        raw_output_with_plain_comparison
            .validate()
            .expect_err("raw output kinds must not carry a plain-text comparison"),
        ContractError::UnexpectedComparisonPlainTextOutputForOutput {
            output_kind: OutputKind::Attribute
        }
    ));

    let valid_plain_text = selector_result_for(Output::plain_text());
    let plain_text_validation = valid_plain_text.validate();
    assert!(
        plain_text_validation.is_ok(),
        "unexpected plain-text validation error: {plain_text_validation:?}"
    );

    let mut non_string_plain_text = valid_plain_text.clone();
    non_string_plain_text.selected_matches[0].output_value = serde_json::json!(7);
    assert!(matches!(
        non_string_plain_text
            .validate()
            .expect_err("plain-text output must be a string"),
        ContractError::NonStringOutputValue {
            output_kind: OutputKind::PlainText
        }
    ));

    let mut mismatched_plain_text = valid_plain_text.clone();
    mismatched_plain_text.selected_matches[0].output_value = serde_json::json!("invented");
    assert!(matches!(
        mismatched_plain_text
            .validate()
            .expect_err("plain-text output must equal its retained evidence"),
        ContractError::PlainTextOutputValueMismatch
    ));

    let mut structured_with_plain_comparison_leak = selector_result_for(Output::structured());
    structured_with_plain_comparison_leak.selected_matches[0].output_value = serde_json::json!({
        "plainTextOutput": "Hello",
        "comparisonPlainTextOutput": "invented"
    });
    assert!(matches!(
        structured_with_plain_comparison_leak
            .validate()
            .expect_err("structured raw evidence must not carry a plain-text comparison"),
        ContractError::StructuredOutputContainsComparisonPlainText
    ));

    let mut structured_with_wrong_plain_text = selector_result_for(Output::structured());
    structured_with_wrong_plain_text.selected_matches[0].output_value = serde_json::json!({
        "plainTextOutput": "invented"
    });
    assert!(matches!(
        structured_with_wrong_plain_text
            .validate()
            .expect_err("structured CSS evidence must retain matching plain text"),
        ContractError::StructuredOutputPlainTextMismatch
    ));

    let mut valid_structured_plain_text = selector_result_for(Output::structured());
    valid_structured_plain_text.selected_matches[0].comparison_plain_text_output =
        Some("canonical".to_owned());
    valid_structured_plain_text.selected_matches[0].output_value = serde_json::json!({
        "plainTextOutput": "Hello"
    });
    let valid_structured_plain_text = valid_structured_plain_text
        .with_computed_digest()
        .expect("valid structured plain-text result digest");
    assert!(valid_structured_plain_text.validate().is_ok());
}
