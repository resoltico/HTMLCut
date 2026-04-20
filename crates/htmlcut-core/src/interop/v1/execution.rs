use std::collections::BTreeMap;
use std::num::NonZeroUsize;

use serde_json::Value;
use url::Url;

use crate::{
    DEFAULT_FETCH_TIMEOUT_MS, DEFAULT_REGEX_FLAGS, Diagnostic, DiagnosticCode, ExtractionRequest,
    ExtractionSpec, NormalizationOptions, OutputOptions, RuntimeOptions, SelectionSpec,
    SlicePatternSpec, SliceSpec, ValueSpec, WhitespaceMode, extract,
    result::{ExtractionMatch, ExtractionMatchMetadata},
};

use super::stable_json::digest_stable_json;
use super::{
    ContractError, DelimiterMode, ErrorCode, HtmlInput, InteropError, InteropResult, OutputKind,
    Plan, PlanStrategy, RegexFlag, ResultExecution, ResultSource, SelectedMatch,
    SelectedMatchMetadata, Selection, StrategyKind, TextWhitespace,
};

#[derive(Clone, Debug, PartialEq, Eq)]
struct ProjectedStructuredMatch {
    candidate_index: NonZeroUsize,
    selected_html: String,
    comparison_input_text: String,
    inner_html: String,
    outer_html: String,
    metadata: SelectedMatchMetadata,
}

/// Validates one plan and returns a typed interop error on failure.
pub fn validate_plan(plan: &Plan) -> Result<(), Box<InteropError>> {
    let plan_digest_sha256 = exact_plan_digest_sha256(plan);
    plan.validate()
        .map_err(|error| Box::new(plan_invalid_error(plan, &plan_digest_sha256, error)))
}

/// Executes one plan directly against in-memory HTML input.
pub fn execute_plan(source: &HtmlInput, plan: &Plan) -> Result<InteropResult, Box<InteropError>> {
    let plan_digest_sha256 = exact_plan_digest_sha256(plan);
    plan.validate()
        .map_err(|error| Box::new(plan_invalid_error(plan, &plan_digest_sha256, error)))?;

    let request = compile_request(source, plan);
    let runtime = runtime_options(source);
    let extraction = extract(&request, &runtime);

    if !extraction.ok {
        return Err(Box::new(core_execution_error(
            plan,
            &plan_digest_sha256,
            &extraction.diagnostics,
        )));
    }

    adapt_successful_extraction(source, plan, plan_digest_sha256, extraction)
}

fn adapt_successful_extraction(
    source: &HtmlInput,
    plan: &Plan,
    plan_digest_sha256: String,
    extraction: crate::ExtractionResult,
) -> Result<InteropResult, Box<InteropError>> {
    let strategy_kind = plan.strategy.kind();
    let Some(selected) = extraction.matches.first() else {
        let mut details = BTreeMap::new();
        details.insert(
            "match_count".to_owned(),
            Value::from(extraction.matches.len() as u64),
        );
        return Err(Box::new(internal_adapter_error(
            &plan_digest_sha256,
            Some(strategy_kind),
            "successful extraction did not produce a selected match",
            details,
            extraction.diagnostics,
        )));
    };

    if extraction.matches.len() != 1 {
        let mut details = BTreeMap::new();
        details.insert(
            "match_count".to_owned(),
            Value::from(extraction.matches.len() as u64),
        );
        details.insert(
            "candidate_count".to_owned(),
            Value::from(extraction.stats.candidate_count as u64),
        );
        return Err(Box::new(internal_adapter_error(
            &plan_digest_sha256,
            Some(strategy_kind),
            "successful execution must produce exactly one selected match",
            details,
            extraction.diagnostics,
        )));
    }

    let projected = project_structured_match(
        selected,
        strategy_kind,
        &plan_digest_sha256,
        &extraction.diagnostics,
    )?;
    let source_summary = ResultSource {
        input_base_url: source.input_base_url.clone(),
        effective_base_url: parse_optional_url(
            extraction.source.effective_base_url.as_deref(),
            &plan_digest_sha256,
            strategy_kind,
            "effective_base_url",
            &extraction.diagnostics,
        )?,
        document_title: extraction.document_title.clone(),
    };
    let selected_match = SelectedMatch {
        candidate_index: projected.candidate_index,
        value_kind: plan.output.kind,
        value: match plan.output.kind {
            OutputKind::Text => projected.comparison_input_text.clone(),
            OutputKind::InnerHtml => projected.selected_html.clone(),
            OutputKind::OuterHtml => projected.outer_html.clone(),
        },
        comparison_input_text: projected.comparison_input_text,
        inner_html: Some(projected.inner_html),
        outer_html: Some(projected.outer_html),
        metadata: projected.metadata,
    };
    let execution = ResultExecution::new(
        plan_digest_sha256,
        strategy_kind,
        plan.selection.mode(),
        extraction.stats.candidate_count,
    );

    Ok(finalize_result(InteropResult::new(
        execution,
        source_summary,
        selected_match,
        extraction.diagnostics,
    )))
}

fn exact_plan_digest_sha256(plan: &Plan) -> String {
    digest_stable_json(plan).expect("plans should always serialize to stable JSON")
}

fn runtime_options(source: &HtmlInput) -> RuntimeOptions {
    RuntimeOptions {
        max_bytes: source.html.len(),
        fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
        fetch_preflight: crate::FetchPreflightMode::HeadFirst,
    }
}

fn compile_request(source: &HtmlInput, plan: &Plan) -> ExtractionRequest {
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

fn compile_regex_flags(flags: &[RegexFlag]) -> String {
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

fn project_structured_match(
    matched: &ExtractionMatch,
    strategy_kind: StrategyKind,
    plan_digest_sha256: &str,
    diagnostics: &[Diagnostic],
) -> Result<ProjectedStructuredMatch, Box<InteropError>> {
    let structured = matched.value.as_object().ok_or_else(|| {
        let mut details = BTreeMap::new();
        details.insert("value_type".to_owned(), Value::from("structured"));
        Box::new(internal_adapter_error(
            plan_digest_sha256,
            Some(strategy_kind),
            "execution expected a structured core match payload",
            details,
            diagnostics.to_vec(),
        ))
    })?;

    match &matched.metadata {
        ExtractionMatchMetadata::Selector(metadata) => {
            let candidate_index = non_zero_candidate_index(
                metadata.candidate_index,
                plan_digest_sha256,
                strategy_kind,
                diagnostics,
            )?;
            let selected_html = required_string_field(
                structured,
                "html",
                plan_digest_sha256,
                strategy_kind,
                diagnostics,
            )?;
            Ok(ProjectedStructuredMatch {
                candidate_index,
                selected_html: selected_html.clone(),
                comparison_input_text: required_string_field(
                    structured,
                    "text",
                    plan_digest_sha256,
                    strategy_kind,
                    diagnostics,
                )?,
                inner_html: selected_html,
                outer_html: required_string_field(
                    structured,
                    "outerHtml",
                    plan_digest_sha256,
                    strategy_kind,
                    diagnostics,
                )?,
                metadata: SelectedMatchMetadata::CssSelector {
                    candidate_count: metadata.candidate_count,
                    candidate_index,
                    path: metadata.path.clone(),
                    tag_name: metadata.tag_name.clone(),
                },
            })
        }
        ExtractionMatchMetadata::DelimiterPair(metadata) => {
            let candidate_index = non_zero_candidate_index(
                metadata.candidate_index,
                plan_digest_sha256,
                strategy_kind,
                diagnostics,
            )?;
            Ok(ProjectedStructuredMatch {
                candidate_index,
                selected_html: required_string_field(
                    structured,
                    "html",
                    plan_digest_sha256,
                    strategy_kind,
                    diagnostics,
                )?,
                comparison_input_text: required_string_field(
                    structured,
                    "text",
                    plan_digest_sha256,
                    strategy_kind,
                    diagnostics,
                )?,
                inner_html: required_string_field(
                    structured,
                    "innerHtml",
                    plan_digest_sha256,
                    strategy_kind,
                    diagnostics,
                )?,
                outer_html: required_string_field(
                    structured,
                    "outerHtml",
                    plan_digest_sha256,
                    strategy_kind,
                    diagnostics,
                )?,
                metadata: SelectedMatchMetadata::DelimiterPair {
                    candidate_count: metadata.candidate_count,
                    candidate_index,
                    selected_range: metadata.selected_range.clone(),
                    inner_range: metadata.inner_range.clone(),
                    outer_range: metadata.outer_range.clone(),
                    include_start: metadata.include_start,
                    include_end: metadata.include_end,
                },
            })
        }
    }
}

fn required_string_field(
    structured: &serde_json::Map<String, Value>,
    field: &'static str,
    plan_digest_sha256: &str,
    strategy_kind: StrategyKind,
    diagnostics: &[Diagnostic],
) -> Result<String, Box<InteropError>> {
    structured
        .get(field)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| {
            let mut details = BTreeMap::new();
            details.insert("field".to_owned(), Value::from(field));
            Box::new(internal_adapter_error(
                plan_digest_sha256,
                Some(strategy_kind),
                format!("execution could not project structured field {field:?}"),
                details,
                diagnostics.to_vec(),
            ))
        })
}

fn non_zero_candidate_index(
    candidate_index: usize,
    plan_digest_sha256: &str,
    strategy_kind: StrategyKind,
    diagnostics: &[Diagnostic],
) -> Result<NonZeroUsize, Box<InteropError>> {
    NonZeroUsize::new(candidate_index).ok_or_else(|| {
        let mut details = BTreeMap::new();
        details.insert(
            "candidate_index".to_owned(),
            Value::from(candidate_index as u64),
        );
        Box::new(internal_adapter_error(
            plan_digest_sha256,
            Some(strategy_kind),
            "execution received an invalid zero candidate index from core metadata",
            details,
            diagnostics.to_vec(),
        ))
    })
}

fn parse_optional_url(
    value: Option<&str>,
    plan_digest_sha256: &str,
    strategy_kind: StrategyKind,
    field: &'static str,
    diagnostics: &[Diagnostic],
) -> Result<Option<Url>, Box<InteropError>> {
    value
        .map(|value| {
            Url::parse(value).map_err(|_| {
                let mut details = BTreeMap::new();
                details.insert("field".to_owned(), Value::from(field));
                details.insert("value".to_owned(), Value::from(value));
                Box::new(internal_adapter_error(
                    plan_digest_sha256,
                    Some(strategy_kind),
                    format!("execution produced an invalid URL in {field}"),
                    details,
                    diagnostics.to_vec(),
                ))
            })
        })
        .transpose()
}

fn plan_invalid_error(plan: &Plan, plan_digest_sha256: &str, error: ContractError) -> InteropError {
    let mut details = BTreeMap::new();
    details.insert("contract_error".to_owned(), Value::from(error.to_string()));
    finalize_error(InteropError::new(
        plan_digest_sha256.to_owned(),
        ErrorCode::PlanInvalid,
        error.to_string(),
        Some(plan.strategy.kind()),
        details,
        Vec::new(),
    ))
}

fn core_execution_error(
    plan: &Plan,
    plan_digest_sha256: &str,
    diagnostics: &[Diagnostic],
) -> InteropError {
    let Some(primary) = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.level == crate::DiagnosticLevel::Error)
    else {
        return internal_adapter_error(
            plan_digest_sha256,
            Some(plan.strategy.kind()),
            "execution failed without an error diagnostic",
            BTreeMap::new(),
            diagnostics.to_vec(),
        );
    };

    let error_code = match primary.code.parse::<DiagnosticCode>() {
        Ok(
            DiagnosticCode::UnsupportedSpecVersion
            | DiagnosticCode::InvalidSelector
            | DiagnosticCode::InvalidSlicePattern
            | DiagnosticCode::InvalidRequest,
        ) => ErrorCode::PlanInvalid,
        Ok(DiagnosticCode::NoMatch | DiagnosticCode::MatchIndexOutOfRange) => ErrorCode::NoMatch,
        Ok(DiagnosticCode::AmbiguousMatch) => ErrorCode::AmbiguousMatch,
        _ => ErrorCode::InternalError,
    };
    let mut details = BTreeMap::new();
    details.insert(
        "core_diagnostic_code".to_owned(),
        Value::from(primary.code.clone()),
    );
    if let Some(core_details) = &primary.details {
        details.insert("core_details".to_owned(), core_details.clone());
    }

    finalize_error(InteropError::new(
        plan_digest_sha256.to_owned(),
        error_code,
        primary.message.clone(),
        Some(plan.strategy.kind()),
        details,
        diagnostics.to_vec(),
    ))
}

fn internal_adapter_error(
    plan_digest_sha256: &str,
    strategy_kind: Option<StrategyKind>,
    message: impl Into<String>,
    details: BTreeMap<String, Value>,
    diagnostics: Vec<Diagnostic>,
) -> InteropError {
    finalize_error(InteropError::new(
        plan_digest_sha256.to_owned(),
        ErrorCode::InternalError,
        message,
        strategy_kind,
        details,
        diagnostics,
    ))
}

fn finalize_result(result: InteropResult) -> InteropResult {
    result
        .with_computed_digest()
        .expect("results should always validate and serialize")
}

fn finalize_error(error: InteropError) -> InteropError {
    error
        .with_computed_digest()
        .expect("errors should always validate and serialize")
}

#[cfg(test)]
pub(crate) fn compile_request_for_tests(source: &HtmlInput, plan: &Plan) -> ExtractionRequest {
    compile_request(source, plan)
}

#[cfg(test)]
pub(crate) fn compile_regex_flags_for_tests(flags: &[RegexFlag]) -> String {
    compile_regex_flags(flags)
}

#[cfg(test)]
pub(crate) fn project_structured_match_for_tests(
    matched: &ExtractionMatch,
    strategy_kind: StrategyKind,
    diagnostics: &[Diagnostic],
) -> Result<(), Box<InteropError>> {
    project_structured_match(matched, strategy_kind, "plan-digest", diagnostics).map(|_| ())
}

#[cfg(test)]
pub(crate) fn parse_optional_url_for_tests(
    value: Option<&str>,
    field: &'static str,
    diagnostics: &[Diagnostic],
) -> Result<Option<Url>, Box<InteropError>> {
    parse_optional_url(
        value,
        "plan-digest",
        StrategyKind::CssSelector,
        field,
        diagnostics,
    )
}

#[cfg(test)]
pub(crate) fn core_execution_error_for_tests(
    plan: &Plan,
    diagnostics: &[Diagnostic],
) -> InteropError {
    let plan_digest_sha256 = exact_plan_digest_sha256(plan);
    core_execution_error(plan, &plan_digest_sha256, diagnostics)
}

#[cfg(test)]
pub(crate) fn internal_adapter_error_for_tests(
    message: impl Into<String>,
    details: BTreeMap<String, Value>,
    diagnostics: Vec<Diagnostic>,
) -> InteropError {
    internal_adapter_error(
        "plan-digest",
        Some(StrategyKind::CssSelector),
        message,
        details,
        diagnostics,
    )
}

#[cfg(test)]
pub(crate) fn adapt_successful_extraction_for_tests(
    source: &HtmlInput,
    plan: &Plan,
    extraction: crate::ExtractionResult,
) -> Result<InteropResult, Box<InteropError>> {
    adapt_successful_extraction(source, plan, exact_plan_digest_sha256(plan), extraction)
}
