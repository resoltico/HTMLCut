use super::*;

fn selector_source() -> HtmlInput {
    HtmlInput::new("inline", "<article>Hello</article>").expect("selector source")
}

fn selector_match() -> ExtractionMatch {
    ExtractionMatch {
        index: 1,
        path: Some("article:nth-of-type(1)".to_owned()),
        value_type: ValueType::Structured,
        value: json!({
            "textOutput": "Hello",
            "plainTextOutput": "Hello",
            "innerHtmlOutput": "Hello",
            "outerHtmlOutput": "<article>Hello</article>"
        }),
        html: Some("<article>Hello</article>".to_owned()),
        text: Some("Hello".to_owned()),
        preview: "Hello".to_owned(),
        metadata: ExtractionMatchMetadata::Selector(SelectorMatchMetadata {
            candidate_count: 1,
            candidate_index: 1,
            path: "article:nth-of-type(1)".to_owned(),
            tag_name: "article".to_owned(),
            attributes: BTreeMap::new(),
        }),
    }
}

fn delimiter_match() -> ExtractionMatch {
    ExtractionMatch {
        index: 1,
        path: None,
        value_type: ValueType::Structured,
        value: json!({
            "textOutput": "Hello",
            "selectedHtmlOutput": "<article>Hello</article>",
            "innerHtmlOutput": "Hello",
            "outerHtmlOutput": "<article>Hello</article>",
            "attributes": {}
        }),
        html: Some("<article>Hello</article>".to_owned()),
        text: Some("Hello".to_owned()),
        preview: "Hello".to_owned(),
        metadata: ExtractionMatchMetadata::DelimiterPair(DelimiterPairMatchMetadata {
            candidate_count: 1,
            candidate_index: 1,
            selected_range: Range { start: 0, end: 22 },
            inner_range: Range { start: 9, end: 14 },
            outer_range: Range { start: 0, end: 22 },
            include_start: true,
            include_end: false,
            matched_start: "<article>".to_owned(),
            matched_end: "</article>".to_owned(),
        }),
    }
}

mod adapter_failures;
mod execution_errors;
mod request_compilation;
mod structured_projection_errors;
mod successful_adaptation;
