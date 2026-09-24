use std::collections::BTreeMap;
use std::num::NonZeroU32;

use super::{displayed_http_url, http_url};
use crate::interop::v2::{
    AttributeName, ByteRange, ContractError, CssSelectorText, DelimiterBoundaryRetention,
    DelimiterBoundaryText, DelimiterMode, ERROR_SCHEMA_NAME, ErrorCode, ExecutionBudget,
    HTMLCUT_EXTRACTION_SEMANTICS_VERSION, HtmlInput, InteropDiagnostic, InteropDiagnosticCode,
    InteropDiagnosticLevel, InteropError, InteropErrorDetail, InteropResult, Output, OutputKind,
    PLAN_SCHEMA_NAME, PREPARATION_ERROR_SCHEMA_NAME, Plan, PlanStrategy, PreparationErrorCode,
    PreparationLimits, RESULT_SCHEMA_NAME, RegexFlag, Rendering, ResultExecution, ResultSource,
    SelectedMatch, SelectedMatchMetadata, Selection, SelectionMode, StrategyKind, TextWhitespace,
    compile_plan, execute, execute_one_shot_for_tests, preparation_parse_count_for_tests,
    prepare_document, reset_preparation_parse_count_for_tests, stable_json_v2,
};
use serde_json::json;

const TEST_PLAN_DIGEST_SHA256: &str =
    "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn css_selector(selector: &str) -> CssSelectorText {
    CssSelectorText::new(selector).expect("selector")
}

fn delimiter_boundary(boundary: &str) -> DelimiterBoundaryText {
    DelimiterBoundaryText::new(boundary).expect("slice boundary")
}

fn output_attribute_name(name: &str) -> AttributeName {
    AttributeName::new(name).expect("attribute name")
}

fn selector_match() -> SelectedMatch {
    SelectedMatch {
        candidate_index: NonZeroU32::new(1).expect("candidate index"),
        output_value: json!("<article>Hello</article>"),
        text_output: "Hello".to_owned(),
        comparison_text_output: None,
        plain_text_output: Some("Hello".to_owned()),
        comparison_plain_text_output: None,
        selected_html_output: None,
        inner_html_output: "Hello".to_owned(),
        outer_html_output: "<article>Hello</article>".to_owned(),
        metadata: SelectedMatchMetadata::CssSelector {
            candidate_count: 1,
            candidate_index: NonZeroU32::new(1).expect("candidate index"),
            path: "html:nth-of-type(1) > body:nth-of-type(1) > article:nth-of-type(1)".to_owned(),
            tag_name: "article".to_owned(),
            attributes: BTreeMap::new(),
        },
    }
}

fn selected_matches(selected_match: SelectedMatch) -> Vec<SelectedMatch> {
    vec![selected_match]
}

fn only_selected_match(result: &InteropResult) -> &SelectedMatch {
    result
        .selected_matches
        .first()
        .expect("interop result should carry one selected match")
}

mod execution;
mod exploration;
mod identity;
mod preparation;

mod source;
