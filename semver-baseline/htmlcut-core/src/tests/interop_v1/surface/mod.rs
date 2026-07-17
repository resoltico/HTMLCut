use std::collections::BTreeMap;
use std::num::NonZeroUsize;

use super::{displayed_http_url, http_url};
use crate::DEFAULT_MAX_BYTES;
use crate::interop::v1::{
    ByteRange, ContractError, CssSelectorText, DelimiterBoundaryRetention, DelimiterBoundaryText,
    DelimiterMode, ERROR_SCHEMA_NAME, ErrorCode, HTMLCUT_EXTRACTION_SEMANTICS_VERSION, HtmlInput,
    InteropDiagnostic, InteropDiagnosticCode, InteropDiagnosticLevel, InteropError, InteropResult,
    Output, OutputKind, PLAN_SCHEMA_NAME, Plan, PlanStrategy, RESULT_SCHEMA_NAME, RegexFlag,
    Rendering, ResultExecution, ResultSource, SelectedMatch, SelectedMatchMetadata, Selection,
    SelectionMode, StrategyKind, TextWhitespace, execute_plan, execute_validated_plan,
    prepare_plan, stable_json_v1,
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

fn selector_match() -> SelectedMatch {
    SelectedMatch {
        candidate_index: NonZeroUsize::new(1).expect("candidate index"),
        output_value: json!("<article>Hello</article>"),
        text_output: "Hello".to_owned(),
        comparison_text_output: None,
        selected_html_output: None,
        inner_html_output: "Hello".to_owned(),
        outer_html_output: "<article>Hello</article>".to_owned(),
        metadata: SelectedMatchMetadata::CssSelector {
            candidate_count: 1,
            candidate_index: NonZeroUsize::new(1).expect("candidate index"),
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
mod identity;
mod preparation;
