//! Delimiter execution resource-limit scenarios.

use super::*;

#[test]
fn prepared_delimiter_execution_rejects_selected_match_overflow_after_selection() {
    let document = prepare_document(
        HtmlInput::new(
            "delimiter-selected-limit",
            "<article>One</article><article>Two</article>",
        )
        .expect("source"),
        PreparationLimits::default(),
    )
    .expect("prepared");
    let budget = ExecutionBudget {
        max_selected_matches: NonZeroU32::new(1).expect("non-zero"),
        ..ExecutionBudget::default()
    };
    let plan = Plan::new(
        PlanStrategy::delimiter_pair(
            delimiter_boundary("<article>"),
            delimiter_boundary("</article>"),
            DelimiterMode::Literal,
            DelimiterBoundaryRetention::IncludeBoth,
            Vec::new(),
        ),
        Selection::all(),
        Output::selected_html(),
        Rendering::new(TextWhitespace::Normalize, false),
    )
    .with_execution_budget(budget);
    let error = execute(&document, &compile_plan(&plan).expect("compiled plan"))
        .expect_err("selected-match limit");
    assert_eq!(error.error_code, ErrorCode::ExecutionLimitExceeded);
    assert_eq!(
        error.diagnostics[0].code,
        InteropDiagnosticCode::SelectedMatchLimitExceeded
    );
}

#[test]
fn prepared_delimiter_execution_enforces_post_projection_and_total_output_budgets() {
    let document = prepare_document(
        HtmlInput::new("delimiter-output-limits", "STARTOneENDSTARTTwoEND").expect("source"),
        PreparationLimits::default(),
    )
    .expect("prepared");
    let strategy = || {
        PlanStrategy::delimiter_pair(
            delimiter_boundary("START"),
            delimiter_boundary("END"),
            DelimiterMode::Literal,
            DelimiterBoundaryRetention::ExcludeBoth,
            Vec::new(),
        )
    };

    let post_projection_budget = ExecutionBudget {
        max_match_output_bytes: NonZeroU32::new(32).expect("non-zero"),
        ..ExecutionBudget::default()
    };
    let post_projection_plan = Plan::new(
        strategy(),
        Selection::first(),
        Output::structured(),
        Rendering::new(TextWhitespace::Normalize, false),
    )
    .with_execution_budget(post_projection_budget);
    let post_projection_error = execute(
        &document,
        &compile_plan(&post_projection_plan).expect("compiled plan"),
    )
    .expect_err("structured projection exceeds match budget");
    assert_eq!(
        post_projection_error.diagnostics[0]
            .details
            .as_ref()
            .expect("details")["limitName"],
        "maxMatchOutputBytes"
    );

    let total_budget = ExecutionBudget {
        max_match_output_bytes: NonZeroU32::new(1_024).expect("non-zero"),
        max_total_output_bytes: NonZeroU32::new(100).expect("non-zero"),
        ..ExecutionBudget::default()
    };
    let total_plan = Plan::new(
        strategy(),
        Selection::all(),
        Output::structured(),
        Rendering::new(TextWhitespace::Normalize, false),
    )
    .with_execution_budget(total_budget);
    let total_error = execute(
        &document,
        &compile_plan(&total_plan).expect("compiled plan"),
    )
    .expect_err("aggregate structured output exceeds total budget");
    assert_eq!(
        total_error.diagnostics[0]
            .details
            .as_ref()
            .expect("details")["limitName"],
        "maxTotalOutputBytes"
    );
}
