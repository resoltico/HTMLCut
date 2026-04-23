use crate::{
    DEFAULT_FETCH_TIMEOUT_MS, DEFAULT_REGEX_FLAGS, ExtractionRequest, ExtractionSpec,
    NormalizationOptions, OutputOptions, RuntimeOptions, SelectionSpec, SlicePatternSpec,
    SliceSpec, ValueSpec, WhitespaceMode,
};

use super::super::stable_json::digest_stable_json;
use super::super::{
    DelimiterMode, HtmlInput, Plan, PlanStrategy, RegexFlag, Selection, TextWhitespace,
};

pub(super) fn exact_plan_digest_sha256(plan: &Plan) -> String {
    digest_stable_json(plan).expect("plans should always serialize to stable JSON")
}

pub(super) fn runtime_options(source: &HtmlInput) -> RuntimeOptions {
    RuntimeOptions {
        max_bytes: source.html.len(),
        fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
        fetch_preflight: crate::FetchPreflightMode::HeadFirst,
    }
}

pub(super) fn compile_request(source: &HtmlInput, plan: &Plan) -> ExtractionRequest {
    let extraction = match &plan.strategy {
        PlanStrategy::CssSelector { selector } => ExtractionSpec::selector(selector.clone()),
        PlanStrategy::DelimiterPair {
            start,
            end,
            mode,
            include_start,
            include_end,
            flags,
        } => ExtractionSpec::slice(SliceSpec {
            pattern: match mode {
                DelimiterMode::Literal => SlicePatternSpec::literal(start.clone(), end.clone()),
                DelimiterMode::Regex => {
                    SlicePatternSpec::regex(start.clone(), end.clone(), compile_regex_flags(flags))
                }
            },
            include_start: *include_start,
            include_end: *include_end,
        }),
    }
    .with_selection(compile_selection(&plan.selection))
    .with_value(ValueSpec::Structured);

    let mut request = ExtractionRequest::new(source.to_source_request(), extraction);
    request.normalization = NormalizationOptions {
        whitespace: match plan.normalization.whitespace {
            TextWhitespace::Preserve => WhitespaceMode::Preserve,
            TextWhitespace::Normalize => WhitespaceMode::Normalize,
        },
        rewrite_urls: plan.normalization.rewrite_urls,
    };
    request.output = OutputOptions {
        include_source_text: false,
        include_html: false,
        include_text: false,
        ..OutputOptions::default()
    };
    request
}

fn compile_selection(selection: &Selection) -> SelectionSpec {
    match selection {
        Selection::Single => SelectionSpec::single(),
        Selection::First => SelectionSpec::First,
        Selection::Nth { index } => SelectionSpec::nth(*index),
    }
}

pub(super) fn compile_regex_flags(flags: &[RegexFlag]) -> String {
    let mut compiled = DEFAULT_REGEX_FLAGS.to_owned();
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
