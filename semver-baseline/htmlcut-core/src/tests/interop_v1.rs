use super::*;
use crate::interop::v1::{
    self, AttributeName, ByteRange, ContractError, CssSelectorText, DelimiterBoundaryRetention,
    DelimiterBoundaryText, DelimiterMode, DisplayedHttpUrl, ErrorCode, HtmlInput, HttpUrl,
    InteropDiagnosticCode, InteropDiagnosticLevel, InteropError, InteropResult, Output, OutputKind,
    Plan, PlanStrategy, RegexFlag, Rendering, ResultExecution, ResultSource, SelectedMatch,
    SelectedMatchMetadata, Selection, SelectionMode, StrategyKind, TextWhitespace,
};
use crate::result::{
    DelimiterPairMatchMetadata, ExtractionMatch, ExtractionStats, Range, SelectorMatchMetadata,
};
use std::collections::BTreeMap;

const TEST_PLAN_DIGEST_SHA256: &str =
    "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn selector_plan() -> Plan {
    Plan::new(
        PlanStrategy::css_selector(css_selector("article")),
        Selection::single(),
        Output::text(),
        Rendering::new(TextWhitespace::Normalize, false),
    )
}

fn delimiter_plan() -> Plan {
    Plan::new(
        PlanStrategy::delimiter_pair(
            delimiter_boundary("<article>"),
            delimiter_boundary("</article>"),
            DelimiterMode::Regex,
            DelimiterBoundaryRetention::IncludeStart,
            vec![
                RegexFlag::CaseInsensitive,
                RegexFlag::MultiLine,
                RegexFlag::DotMatchesNewLine,
                RegexFlag::SwapGreed,
                RegexFlag::IgnoreWhitespace,
            ],
        ),
        Selection::nth(NonZeroUsize::new(2).expect("index")),
        Output::outer_html(),
        Rendering::new(TextWhitespace::Rendered, true),
    )
}

fn css_selector(selector: &str) -> CssSelectorText {
    CssSelectorText::new(selector).expect("css selector")
}

fn delimiter_boundary(boundary: &str) -> DelimiterBoundaryText {
    DelimiterBoundaryText::new(boundary).expect("delimiter boundary")
}

fn output_attribute_name(name: &str) -> AttributeName {
    AttributeName::new(name).expect("output attribute name")
}

fn http_url(value: &str) -> HttpUrl {
    HttpUrl::parse(value).expect("http url")
}

fn displayed_http_url(value: &str) -> DisplayedHttpUrl {
    DisplayedHttpUrl::parse(value).expect("displayed http url")
}

fn selector_selected_match_with(candidate_count: usize, candidate_index: usize) -> SelectedMatch {
    let candidate_index = NonZeroUsize::new(candidate_index).expect("candidate index");
    SelectedMatch {
        candidate_index,
        output_value: Value::String("Hello".to_owned()),
        text_output: "Hello".to_owned(),
        selected_html_output: None,
        inner_html_output: "Hello".to_owned(),
        outer_html_output: "<article>Hello</article>".to_owned(),
        metadata: SelectedMatchMetadata::CssSelector {
            candidate_count,
            candidate_index,
            path: "html:nth-of-type(1) > body:nth-of-type(1) > article:nth-of-type(1)".to_owned(),
            tag_name: "article".to_owned(),
            attributes: BTreeMap::new(),
        },
    }
}

fn selector_selected_match() -> SelectedMatch {
    selector_selected_match_with(1, 1)
}

fn selector_selected_matches() -> Vec<SelectedMatch> {
    vec![selector_selected_match()]
}

fn delimiter_selected_match_with(candidate_count: usize, candidate_index: usize) -> SelectedMatch {
    let candidate_index = NonZeroUsize::new(candidate_index).expect("candidate index");
    SelectedMatch {
        candidate_index,
        output_value: Value::String("<article>Hello</article>".to_owned()),
        text_output: "Hello".to_owned(),
        selected_html_output: Some("<article>Hello</article>".to_owned()),
        inner_html_output: "Hello".to_owned(),
        outer_html_output: "<article>Hello</article>".to_owned(),
        metadata: SelectedMatchMetadata::DelimiterPair {
            candidate_count,
            candidate_index,
            selected_range: ByteRange { start: 0, end: 22 },
            inner_range: ByteRange { start: 9, end: 14 },
            outer_range: ByteRange { start: 0, end: 22 },
            include_start: true,
            include_end: true,
            matched_start: "<article>".to_owned(),
            matched_end: "</article>".to_owned(),
        },
    }
}

fn selector_core_match(
    index: usize,
    candidate_index: usize,
    candidate_count: usize,
) -> ExtractionMatch {
    ExtractionMatch {
        index,
        path: Some(format!("article:nth-of-type({index})")),
        value_type: ValueType::Structured,
        value: json!({
            "textOutput": "Hello",
            "innerHtmlOutput": "Hello",
            "outerHtmlOutput": format!("<article data-index=\"{index}\">Hello</article>")
        }),
        html: Some(format!("<article data-index=\"{index}\">Hello</article>")),
        text: Some("Hello".to_owned()),
        preview: "Hello".to_owned(),
        metadata: ExtractionMatchMetadata::Selector(SelectorMatchMetadata {
            candidate_count,
            candidate_index,
            path: format!("article:nth-of-type({index})"),
            tag_name: "article".to_owned(),
            attributes: BTreeMap::new(),
        }),
    }
}

fn successful_selector_extraction(
    matches: Vec<ExtractionMatch>,
    candidate_count: usize,
    effective_base_url: Option<&str>,
) -> ExtractionResult {
    ExtractionResult {
        operation_id: crate::OperationId::SelectExtract,
        schema_name: crate::CORE_RESULT_SCHEMA_NAME.to_owned(),
        schema_version: crate::CORE_RESULT_SCHEMA_VERSION,
        ok: true,
        source: SourceMetadata {
            kind: SourceKind::Memory,
            value: "inline".to_owned(),
            input_base_url: Some("https://example.com/start.html".to_owned()),
            effective_base_url: effective_base_url.map(str::to_owned),
            bytes_read: 22,
            load_steps: Vec::new(),
            text: None,
        },
        document_title: Some("Example".to_owned()),
        extraction: ExtractionSpec::selector(selector_query("article")),
        stats: ExtractionStats {
            duration_ms: 1,
            candidate_count,
            match_count: matches.len(),
        },
        matches,
        diagnostics: Vec::new(),
    }
}

mod acceptance;
mod execution;
mod properties;
mod schema;
mod surface;
mod validation;
