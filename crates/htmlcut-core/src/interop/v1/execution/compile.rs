use crate::{
    DEFAULT_FETCH_CONNECT_TIMEOUT_MS, DEFAULT_FETCH_TIMEOUT_MS, DEFAULT_MAX_BYTES,
    ExtractionRequest, ExtractionSpec, OutputOptions, RenderingOptions, RuntimeOptions,
    SelectionSpec, SlicePatternSpec, SliceSpec, SourceRequest, ValueSpec, WhitespaceMode,
};

use super::super::stable_json::digest_stable_json;
use super::super::{
    ContractError, DelimiterMode, HtmlInput, Plan, PlanStrategy, RegexFlag, Selection,
    TextWhitespace,
};

pub(super) fn exact_plan_digest_sha256(plan: &Plan) -> Result<String, ContractError> {
    digest_stable_json(plan)
}

pub(super) fn default_runtime_options() -> RuntimeOptions {
    RuntimeOptions {
        max_bytes: DEFAULT_MAX_BYTES,
        fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
        fetch_connect_timeout_ms: DEFAULT_FETCH_CONNECT_TIMEOUT_MS,
        fetch_preflight: crate::FetchPreflightMode::HeadFirst,
    }
}

pub(super) fn compile_request(source: &HtmlInput, plan: &Plan) -> ExtractionRequest {
    let extraction = match &plan.strategy {
        PlanStrategy::CssSelector { selector } => ExtractionSpec::selector(
            crate::SelectorQuery::new(selector.as_str().to_owned())
                .expect("validated interop selector must compile into a core selector query"),
        ),
        PlanStrategy::DelimiterPair {
            start,
            end,
            mode,
            include_start,
            include_end,
            flags,
        } => ExtractionSpec::slice(SliceSpec {
            pattern: match mode {
                DelimiterMode::Literal => SlicePatternSpec::literal(
                    crate::SliceBoundary::new(start.as_str().to_owned()).expect(
                        "validated interop start boundary must compile into a core slice boundary",
                    ),
                    crate::SliceBoundary::new(end.as_str().to_owned()).expect(
                        "validated interop end boundary must compile into a core slice boundary",
                    ),
                ),
                DelimiterMode::Regex => SlicePatternSpec::regex(
                    crate::SliceBoundary::new(start.as_str().to_owned()).expect(
                        "validated interop start boundary must compile into a core slice boundary",
                    ),
                    crate::SliceBoundary::new(end.as_str().to_owned()).expect(
                        "validated interop end boundary must compile into a core slice boundary",
                    ),
                    compile_regex_flags(flags),
                ),
            },
            include_start: *include_start,
            include_end: *include_end,
        }),
    }
    .with_selection(compile_selection(&plan.selection))
    .with_value(ValueSpec::Structured);

    let mut request = ExtractionRequest::new(compile_source_request(source), extraction);
    request.output = OutputOptions {
        rendering: RenderingOptions {
            whitespace: match plan.rendering.whitespace {
                TextWhitespace::Preserve => WhitespaceMode::Preserve,
                TextWhitespace::Normalize => WhitespaceMode::Normalize,
            },
            rewrite_urls: plan.rendering.rewrite_urls,
        },
        include_source_text: false,
        include_html: false,
        include_text: false,
        ..OutputOptions::default()
    };
    request
}

fn compile_source_request(source: &HtmlInput) -> SourceRequest {
    let mut compiled = SourceRequest::memory(source.label.clone(), source.html.clone());
    if let Some(base_url) = &source.input_base_url {
        compiled = compiled.with_base_url(base_url.clone());
    }

    compiled
}

fn compile_selection(selection: &Selection) -> SelectionSpec {
    match selection {
        Selection::Single => SelectionSpec::single(),
        Selection::First => SelectionSpec::First,
        Selection::Nth { index } => SelectionSpec::nth(*index),
        Selection::All => SelectionSpec::All,
    }
}

pub(super) fn compile_regex_flags(flags: &[RegexFlag]) -> String {
    let mut compiled = String::new();
    for flag in flags {
        compiled.push(match flag {
            RegexFlag::CaseInsensitive => 'i',
            RegexFlag::MultiLine => 'm',
            RegexFlag::DotMatchesNewLine => 's',
            RegexFlag::SwapGreed => 'U',
            RegexFlag::IgnoreWhitespace => 'x',
        });
    }
    compiled
}
