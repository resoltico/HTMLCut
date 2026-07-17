use super::*;

#[test]
fn successful_adaptation_projects_selector_results_and_selection_modes() {
    let selector_source = selector_source();
    let adapted_text = v1::adapt_successful_extraction_for_tests(
        &selector_source
            .clone()
            .with_input_base_url(http_url("https://example.com/start.html")),
        &selector_plan(),
        successful_selector_extraction(
            vec![selector_core_match(1, 1, 1)],
            1,
            Some("https://example.com/base.html"),
        ),
    )
    .expect("adapted text result");
    assert_eq!(adapted_text.selected_matches.len(), 1);
    assert_eq!(adapted_text.output.kind(), OutputKind::Text);
    assert_eq!(
        adapted_text.selected_matches[0].output_value,
        Value::String("Hello".to_owned())
    );
    assert_eq!(
        adapted_text
            .source
            .effective_base_url
            .as_ref()
            .map(DisplayedHttpUrl::as_str),
        Some("https://example.com/base.html")
    );

    let adapted_comparison_text = v1::adapt_successful_extraction_for_tests(
        &selector_source,
        &selector_plan(),
        successful_selector_extraction(
            vec![ExtractionMatch {
                value: json!({
                    "textOutput": "Raw evidence",
                    "comparisonTextOutput": "Canonical comparison",
                    "innerHtmlOutput": "Raw evidence",
                    "outerHtmlOutput": "<article>Raw evidence</article>"
                }),
                ..selector_core_match(1, 1, 1)
            }],
            1,
            None,
        ),
    )
    .expect("comparison text projection");
    assert_eq!(
        adapted_comparison_text.selected_matches[0].text_output,
        "Raw evidence"
    );
    assert_eq!(
        adapted_comparison_text.selected_matches[0]
            .comparison_text_output
            .as_deref(),
        Some("Canonical comparison")
    );
    assert_eq!(
        adapted_comparison_text.selected_matches[0].output_value,
        Value::String("Canonical comparison".to_owned())
    );

    let adapted_comparison_structured = v1::adapt_successful_extraction_for_tests(
        &selector_source,
        &Plan::new(
            PlanStrategy::css_selector(css_selector("article")),
            Selection::single(),
            Output::structured(),
            Rendering::new(TextWhitespace::Normalize, false),
        ),
        successful_selector_extraction(
            vec![ExtractionMatch {
                value: json!({
                    "textOutput": "Raw evidence",
                    "comparisonTextOutput": "Canonical comparison",
                    "innerHtmlOutput": "Raw evidence",
                    "outerHtmlOutput": "<article>Raw evidence</article>"
                }),
                ..selector_core_match(1, 1, 1)
            }],
            1,
            None,
        ),
    )
    .expect("comparison structured projection");
    let selected = &adapted_comparison_structured.selected_matches[0];
    assert_eq!(
        selected.comparison_text_output.as_deref(),
        Some("Canonical comparison")
    );
    assert_eq!(
        selected.output_value,
        json!({
            "textOutput": "Raw evidence",
            "innerHtmlOutput": "Raw evidence",
            "outerHtmlOutput": "<article>Raw evidence</article>"
        })
    );

    let adapted_inner = v1::adapt_successful_extraction_for_tests(
        &selector_source,
        &Plan::new(
            PlanStrategy::css_selector(css_selector("article")),
            Selection::single(),
            Output::inner_html(),
            Rendering::new(TextWhitespace::Normalize, false),
        ),
        successful_selector_extraction(vec![selector_core_match(1, 1, 1)], 1, None),
    )
    .expect("adapted inner-html result");
    assert_eq!(
        adapted_inner.selected_matches[0].output_value,
        Value::String("Hello".to_owned())
    );

    let adapted_outer = v1::adapt_successful_extraction_for_tests(
        &selector_source,
        &Plan::new(
            PlanStrategy::css_selector(css_selector("article")),
            Selection::single(),
            Output::outer_html(),
            Rendering::new(TextWhitespace::Normalize, false),
        ),
        successful_selector_extraction(vec![selector_core_match(1, 1, 1)], 1, None),
    )
    .expect("adapted outer-html result");
    assert!(
        adapted_outer.selected_matches[0]
            .output_value
            .as_str()
            .is_some_and(|html| html.contains("<article"))
    );

    let adapted_all = v1::adapt_successful_extraction_for_tests(
        &selector_source,
        &Plan::new(
            PlanStrategy::css_selector(css_selector("article")),
            Selection::all(),
            Output::text(),
            Rendering::new(TextWhitespace::Normalize, false),
        ),
        successful_selector_extraction(
            vec![selector_core_match(1, 1, 2), selector_core_match(2, 2, 2)],
            2,
            None,
        ),
    )
    .expect("adapted all-selection result");
    assert_eq!(adapted_all.selected_matches.len(), 2);
    assert_eq!(adapted_all.selection_mode, SelectionMode::All);
    assert_eq!(adapted_all.selected_matches[0].candidate_index.get(), 1);
    assert_eq!(adapted_all.selected_matches[1].candidate_index.get(), 2);

    let adapted_projection_error = v1::adapt_successful_extraction_for_tests(
        &selector_source,
        &selector_plan(),
        successful_selector_extraction(
            vec![ExtractionMatch {
                value: json!({"textOutput": "Hello", "outerHtmlOutput": "<article>Hello</article>"}),
                ..selector_core_match(1, 1, 1)
            }],
            1,
            None,
        ),
    )
    .expect_err("projection failure should surface as interop error");
    assert_eq!(
        adapted_projection_error.error_code,
        ErrorCode::InternalError
    );
    assert!(
        adapted_projection_error
            .message
            .contains("\"innerHtmlOutput\"")
    );

    let adapted_url_error = v1::adapt_successful_extraction_for_tests(
        &selector_source,
        &selector_plan(),
        successful_selector_extraction(vec![selector_core_match(1, 1, 1)], 1, Some("not a url")),
    )
    .expect_err("invalid effective base URL should surface as interop error");
    assert_eq!(adapted_url_error.error_code, ErrorCode::InternalError);
    assert!(adapted_url_error.message.contains("invalid URL"));

    let no_match_adapter_error = v1::adapt_successful_extraction_for_tests(
        &selector_source,
        &selector_plan(),
        successful_selector_extraction(Vec::new(), 0, None),
    )
    .expect_err("missing selected match");
    assert_eq!(no_match_adapter_error.error_code, ErrorCode::InternalError);
    assert!(
        no_match_adapter_error
            .message
            .contains("did not produce a selected match")
    );

    let multi_match_adapter_error = v1::adapt_successful_extraction_for_tests(
        &selector_source,
        &selector_plan(),
        successful_selector_extraction(
            vec![selector_core_match(1, 1, 2), selector_core_match(2, 2, 2)],
            2,
            None,
        ),
    )
    .expect_err("multiple selected matches");
    assert_eq!(
        multi_match_adapter_error.error_code,
        ErrorCode::InternalError
    );
    assert!(
        multi_match_adapter_error
            .message
            .contains("invalid interop result")
    );
}
