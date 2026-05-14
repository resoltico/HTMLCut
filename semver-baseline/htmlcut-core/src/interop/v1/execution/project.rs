use std::collections::BTreeMap;
use std::num::NonZeroUsize;

use serde_json::Value;

use crate::{
    Diagnostic, DisplayedHttpUrl,
    result::{ExtractionMatch, ExtractionMatchMetadata},
};

use super::super::{
    ByteRange, ErrorCode, HtmlInput, InteropDiagnostic, InteropError, InteropResult, Output, Plan,
    ResultExecution, ResultSource, SelectedMatch, SelectedMatchMetadata, StrategyKind,
};
use super::errors::internal_adapter_error;

#[derive(Clone, Debug, PartialEq)]
pub(super) struct ProjectedStructuredMatch {
    candidate_index: NonZeroUsize,
    structured_output: Value,
    text_output: String,
    selected_html_output: Option<String>,
    inner_html_output: String,
    outer_html_output: String,
    attribute_values: BTreeMap<String, String>,
    metadata: SelectedMatchMetadata,
}

pub(super) fn adapt_successful_extraction(
    source: &HtmlInput,
    plan: &Plan,
    plan_digest_sha256: String,
    extraction: crate::ExtractionResult,
) -> Result<InteropResult, Box<InteropError>> {
    let strategy_kind = plan.strategy.kind();
    if extraction.matches.is_empty() {
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
            &extraction.diagnostics,
        )));
    }

    let projected_matches = extraction
        .matches
        .iter()
        .map(|matched| {
            project_structured_match(
                matched,
                strategy_kind,
                &plan_digest_sha256,
                &extraction.diagnostics,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    let source_summary = ResultSource {
        input_base_url: source.input_base_url.as_ref().map(DisplayedHttpUrl::from),
        effective_base_url: parse_optional_url(
            extraction.source.effective_base_url.as_deref(),
            &plan_digest_sha256,
            strategy_kind,
            "effective_base_url",
            &extraction.diagnostics,
        )?,
        document_title: extraction.document_title.clone(),
    };
    let selected_matches = projected_matches
        .into_iter()
        .map(|projected| {
            let output_value = project_output_value(
                &plan.output,
                &projected,
                &plan_digest_sha256,
                strategy_kind,
                &extraction.diagnostics,
            )?;
            Ok(SelectedMatch {
                candidate_index: projected.candidate_index,
                output_value,
                text_output: projected.text_output,
                selected_html_output: projected.selected_html_output,
                inner_html_output: projected.inner_html_output,
                outer_html_output: projected.outer_html_output,
                metadata: projected.metadata,
            })
        })
        .collect::<Result<Vec<_>, Box<InteropError>>>()?;
    let execution = ResultExecution::new(
        plan_digest_sha256.clone(),
        strategy_kind,
        plan.selection.mode(),
        plan.output.clone(),
        extraction.stats.candidate_count,
    );

    finalize_result(
        InteropResult::new(
            execution,
            source_summary,
            selected_matches,
            extraction
                .diagnostics
                .iter()
                .map(InteropDiagnostic::from)
                .collect(),
        ),
        &plan_digest_sha256,
        strategy_kind,
        &extraction.diagnostics,
    )
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
            diagnostics,
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
            Ok(ProjectedStructuredMatch {
                candidate_index,
                structured_output: matched.value.clone(),
                text_output: required_string_field(
                    structured,
                    "textOutput",
                    plan_digest_sha256,
                    strategy_kind,
                    diagnostics,
                )?,
                selected_html_output: None,
                inner_html_output: required_string_field(
                    structured,
                    "innerHtmlOutput",
                    plan_digest_sha256,
                    strategy_kind,
                    diagnostics,
                )?,
                outer_html_output: required_string_field(
                    structured,
                    "outerHtmlOutput",
                    plan_digest_sha256,
                    strategy_kind,
                    diagnostics,
                )?,
                attribute_values: metadata.attributes.clone(),
                metadata: SelectedMatchMetadata::CssSelector {
                    candidate_count: metadata.candidate_count,
                    candidate_index,
                    path: metadata.path.clone(),
                    tag_name: metadata.tag_name.clone(),
                    attributes: metadata.attributes.clone(),
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
                structured_output: matched.value.clone(),
                text_output: required_string_field(
                    structured,
                    "textOutput",
                    plan_digest_sha256,
                    strategy_kind,
                    diagnostics,
                )?,
                selected_html_output: Some(required_string_field(
                    structured,
                    "selectedHtmlOutput",
                    plan_digest_sha256,
                    strategy_kind,
                    diagnostics,
                )?),
                inner_html_output: required_string_field(
                    structured,
                    "innerHtmlOutput",
                    plan_digest_sha256,
                    strategy_kind,
                    diagnostics,
                )?,
                outer_html_output: required_string_field(
                    structured,
                    "outerHtmlOutput",
                    plan_digest_sha256,
                    strategy_kind,
                    diagnostics,
                )?,
                attribute_values: required_string_map_field(
                    structured,
                    "attributes",
                    plan_digest_sha256,
                    strategy_kind,
                    diagnostics,
                )?,
                metadata: SelectedMatchMetadata::DelimiterPair {
                    candidate_count: metadata.candidate_count,
                    candidate_index,
                    selected_range: ByteRange::from(&metadata.selected_range),
                    inner_range: ByteRange::from(&metadata.inner_range),
                    outer_range: ByteRange::from(&metadata.outer_range),
                    include_start: metadata.include_start,
                    include_end: metadata.include_end,
                    matched_start: metadata.matched_start.clone(),
                    matched_end: metadata.matched_end.clone(),
                },
            })
        }
    }
}

fn project_output_value(
    output: &Output,
    projected: &ProjectedStructuredMatch,
    plan_digest_sha256: &str,
    strategy_kind: StrategyKind,
    diagnostics: &[Diagnostic],
) -> Result<Value, Box<InteropError>> {
    match output {
        Output::Text => Ok(Value::String(projected.text_output.clone())),
        Output::InnerHtml => Ok(Value::String(projected.inner_html_output.clone())),
        Output::OuterHtml => Ok(Value::String(projected.outer_html_output.clone())),
        Output::SelectedHtml => projected
            .selected_html_output
            .as_ref()
            .map(|value| Value::String(value.clone()))
            .ok_or_else(|| {
                let mut details = BTreeMap::new();
                details.insert(
                    "output_kind".to_owned(),
                    Value::from(output.kind().to_string()),
                );
                Box::new(internal_adapter_error(
                    plan_digest_sha256,
                    Some(strategy_kind),
                    "execution could not project selected_html for this strategy",
                    details,
                    diagnostics,
                ))
            }),
        Output::Attribute { name } => projected
            .attribute_values
            .get(name.as_str())
            .map(|value| Value::String(value.clone()))
            .ok_or_else(|| {
                let mut details = BTreeMap::new();
                details.insert("attribute".to_owned(), Value::from(name.as_str()));
                Box::new(
                    InteropError::new(
                        plan_digest_sha256.to_owned(),
                        ErrorCode::MissingAttribute,
                        format!("Selected candidate is missing attribute \"{name}\"."),
                        Some(strategy_kind),
                        details,
                        diagnostics.iter().map(InteropDiagnostic::from).collect(),
                    )
                    .with_computed_digest()
                    .expect("missing-attribute interop error payload must digest"),
                )
            }),
        Output::Structured => Ok(projected.structured_output.clone()),
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
                diagnostics,
            ))
        })
}

fn required_string_map_field(
    structured: &serde_json::Map<String, Value>,
    field: &'static str,
    plan_digest_sha256: &str,
    strategy_kind: StrategyKind,
    diagnostics: &[Diagnostic],
) -> Result<BTreeMap<String, String>, Box<InteropError>> {
    let Some(Value::Object(entries)) = structured.get(field) else {
        let mut details = BTreeMap::new();
        details.insert("field".to_owned(), Value::from(field));
        return Err(Box::new(internal_adapter_error(
            plan_digest_sha256,
            Some(strategy_kind),
            format!("execution could not project structured field {field:?}"),
            details,
            diagnostics,
        )));
    };

    entries
        .iter()
        .map(|(key, value)| {
            value
                .as_str()
                .map(|text| (key.clone(), text.to_owned()))
                .ok_or_else(|| {
                    let mut details = BTreeMap::new();
                    details.insert("field".to_owned(), Value::from(field));
                    details.insert("attribute".to_owned(), Value::from(key.as_str()));
                    Box::new(internal_adapter_error(
                        plan_digest_sha256,
                        Some(strategy_kind),
                        format!(
                            "execution produced a non-string attribute value in structured field {field:?}"
                        ),
                        details,
                        diagnostics,
                    ))
                })
        })
        .collect()
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
            diagnostics,
        ))
    })
}

pub(super) fn parse_optional_url(
    value: Option<&str>,
    plan_digest_sha256: &str,
    strategy_kind: StrategyKind,
    field: &'static str,
    diagnostics: &[Diagnostic],
) -> Result<Option<DisplayedHttpUrl>, Box<InteropError>> {
    value
        .map(|value| {
            DisplayedHttpUrl::parse(value).map_err(|_| {
                let mut details = BTreeMap::new();
                details.insert("field".to_owned(), Value::from(field));
                details.insert("value".to_owned(), Value::from(value));
                Box::new(internal_adapter_error(
                    plan_digest_sha256,
                    Some(strategy_kind),
                    format!("execution produced an invalid URL in {field}"),
                    details,
                    diagnostics,
                ))
            })
        })
        .transpose()
}

fn finalize_result(
    result: InteropResult,
    plan_digest_sha256: &str,
    strategy_kind: StrategyKind,
    diagnostics: &[Diagnostic],
) -> Result<InteropResult, Box<InteropError>> {
    result.with_computed_digest().map_err(|error| {
        let mut details = BTreeMap::new();
        details.insert("contract_error".to_owned(), Value::from(error.to_string()));
        Box::new(internal_adapter_error(
            plan_digest_sha256,
            Some(strategy_kind),
            "execution produced an invalid interop result during finalization",
            details,
            diagnostics,
        ))
    })
}
