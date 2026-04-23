use std::collections::BTreeMap;
use std::num::NonZeroUsize;

use serde_json::Value;
use url::Url;

use crate::{
    Diagnostic,
    result::{ExtractionMatch, ExtractionMatchMetadata},
};

use super::super::{
    HtmlInput, InteropError, InteropResult, OutputKind, Plan, ResultExecution, ResultSource,
    SelectedMatch, SelectedMatchMetadata, StrategyKind,
};
use super::errors::internal_adapter_error;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ProjectedStructuredMatch {
    candidate_index: NonZeroUsize,
    selected_html: String,
    comparison_input_text: String,
    inner_html: String,
    outer_html: String,
    metadata: SelectedMatchMetadata,
}

pub(super) fn adapt_successful_extraction(
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

pub(super) fn project_structured_match(
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

pub(super) fn parse_optional_url(
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

fn finalize_result(result: InteropResult) -> InteropResult {
    result
        .with_computed_digest()
        .expect("results should always validate and serialize")
}
