use super::*;
use crate::interop::v1::{
    self, ContractError, DelimiterMode, ErrorCode, HtmlInput, InteropError, InteropResult,
    Normalization, Output, OutputKind, Plan, PlanStrategy, RegexFlag, ResultExecution,
    ResultSource, SelectedMatch, SelectedMatchMetadata, Selection, SelectionMode, StrategyKind,
    TextWhitespace,
};
use crate::result::{
    DelimiterPairMatchMetadata, ExtractionMatch, ExtractionStats, Range, SelectorMatchMetadata,
};
use std::collections::BTreeMap;

fn selector_plan() -> Plan {
    Plan::new(
        PlanStrategy::css_selector(selector_query("article")),
        Selection::single(),
        Output::new(OutputKind::Text),
        Normalization::new(TextWhitespace::Normalize, false),
    )
}

fn delimiter_plan() -> Plan {
    Plan::new(
        PlanStrategy::delimiter_pair(
            slice_boundary("<article>"),
            slice_boundary("</article>"),
            DelimiterMode::Regex,
            true,
            false,
            vec![
                RegexFlag::CaseInsensitive,
                RegexFlag::MultiLine,
                RegexFlag::DotMatchesNewLine,
                RegexFlag::SwapGreed,
                RegexFlag::IgnoreWhitespace,
            ],
        ),
        Selection::nth(NonZeroUsize::new(2).expect("index")),
        Output::new(OutputKind::OuterHtml),
        Normalization::new(TextWhitespace::Preserve, true),
    )
}

fn selector_selected_match() -> SelectedMatch {
    SelectedMatch {
        candidate_index: NonZeroUsize::new(1).expect("candidate index"),
        value_kind: OutputKind::Text,
        value: "Hello".to_owned(),
        comparison_input_text: "Hello".to_owned(),
        inner_html: Some("Hello".to_owned()),
        outer_html: Some("<article>Hello</article>".to_owned()),
        metadata: SelectedMatchMetadata::CssSelector {
            candidate_count: 1,
            candidate_index: NonZeroUsize::new(1).expect("candidate index"),
            path: "html:nth-of-type(1) > body:nth-of-type(1) > article:nth-of-type(1)".to_owned(),
            tag_name: "article".to_owned(),
        },
    }
}

fn selector_core_match(index: usize, candidate_index: usize) -> ExtractionMatch {
    ExtractionMatch {
        index,
        path: Some(format!("article:nth-of-type({index})")),
        value_type: ValueType::Structured,
        value: json!({
            "html": format!("<article data-index=\"{index}\">Hello</article>"),
            "text": "Hello",
            "outerHtml": format!("<article data-index=\"{index}\">Hello</article>")
        }),
        html: Some(format!("<article data-index=\"{index}\">Hello</article>")),
        text: Some("Hello".to_owned()),
        preview: "Hello".to_owned(),
        metadata: ExtractionMatchMetadata::Selector(SelectorMatchMetadata {
            candidate_count: index,
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

mod execution;
mod schema;
mod validation;
