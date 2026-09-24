use std::collections::BTreeMap;
use std::num::NonZeroU32;

use serde_json::Value;

use crate::{
    Diagnostic, DisplayedHttpUrl,
    result::{ExtractionMatch, ExtractionMatchMetadata},
};

use super::super::{
    ByteRange, ErrorCode, HtmlInput, InteropAdapterFailure, InteropDiagnostic, InteropError,
    InteropErrorDetail, InteropResult, Output, Plan, ResultExecution, ResultSource, SelectedMatch,
    SelectedMatchMetadata, StrategyKind,
};
use super::errors::internal_adapter_error;
use super::project_helpers::{finalize_interop_result, parse_optional_url};

#[derive(Clone, Debug, PartialEq)]
pub(super) struct ProjectedStructuredMatch {
    candidate_index: NonZeroU32,
    structured_output: Value,
    text_output: String,
    comparison_text_output: Option<String>,
    plain_text_output: Option<String>,
    comparison_plain_text_output: Option<String>,
    selected_html_output: Option<String>,
    inner_html_output: String,
    outer_html_output: String,
    attribute_values: BTreeMap<String, String>,
    metadata: SelectedMatchMetadata,
}

pub(super) fn adapt_successful_extraction(
    source: &HtmlInput,
    prepared_source_digest_sha256: String,
    plan: &Plan,
    plan_digest_sha256: String,
    extraction: crate::ExtractionResult,
) -> Result<InteropResult, Box<InteropError>> {
    let strategy_kind = plan.strategy.kind();
    if extraction.matches.is_empty() {
        return Err(Box::new(internal_adapter_error(
            &plan_digest_sha256,
            Some(strategy_kind),
            "successful extraction did not produce a selected match",
            InteropAdapterFailure::EmptySelectedMatchSet,
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
        prepared_source_digest_sha256,
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
                comparison_text_output: projected.comparison_text_output,
                plain_text_output: projected.plain_text_output,
                comparison_plain_text_output: projected.comparison_plain_text_output,
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
        candidate_count_from_core(extraction.stats.candidate_count),
    );

    finalize_interop_result(
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
        Box::new(internal_adapter_error(
            plan_digest_sha256,
            Some(strategy_kind),
            "execution expected a structured core match payload",
            InteropAdapterFailure::ExpectedStructuredMatchObject,
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
            let comparison_text_output = optional_string_field(
                structured,
                "comparisonTextOutput",
                plan_digest_sha256,
                strategy_kind,
                diagnostics,
            )?;
            let comparison_plain_text_output = optional_string_field(
                structured,
                "comparisonPlainTextOutput",
                plan_digest_sha256,
                strategy_kind,
                diagnostics,
            )?;
            // The core structured payload transports this interop-only projection, but structured
            // output is raw evidence and must never expose it as part of that payload.
            let mut structured_output = matched.value.clone();
            structured_output
                .as_object_mut()
                .expect("validated structured core match payload must stay an object")
                .remove("comparisonTextOutput");
            structured_output
                .as_object_mut()
                .expect("validated structured core match payload must stay an object")
                .remove("comparisonPlainTextOutput");
            Ok(ProjectedStructuredMatch {
                candidate_index,
                structured_output,
                text_output: required_string_field(
                    structured,
                    "textOutput",
                    plan_digest_sha256,
                    strategy_kind,
                    diagnostics,
                )?,
                comparison_text_output,
                plain_text_output: Some(required_string_field(
                    structured,
                    "plainTextOutput",
                    plan_digest_sha256,
                    strategy_kind,
                    diagnostics,
                )?),
                comparison_plain_text_output,
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
                    candidate_count: candidate_count_from_core(metadata.candidate_count),
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
                comparison_text_output: None,
                plain_text_output: None,
                comparison_plain_text_output: None,
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
                    candidate_count: candidate_count_from_core(metadata.candidate_count),
                    candidate_index,
                    selected_range: wire_byte_range(&metadata.selected_range),
                    inner_range: wire_byte_range(&metadata.inner_range),
                    outer_range: wire_byte_range(&metadata.outer_range),
                    include_start: metadata.include_start,
                    include_end: metadata.include_end,
                    matched_start: metadata.matched_start.clone(),
                    matched_end: metadata.matched_end.clone(),
                },
            })
        }
    }
}

pub(super) fn project_output_value(
    output: &Output,
    projected: &ProjectedStructuredMatch,
    plan_digest_sha256: &str,
    strategy_kind: StrategyKind,
    diagnostics: &[Diagnostic],
) -> Result<Value, Box<InteropError>> {
    match output {
        Output::Text => Ok(Value::String(
            projected
                .comparison_text_output
                .as_ref()
                .unwrap_or(&projected.text_output)
                .clone(),
        )),
        Output::PlainText => projected
            .plain_text_output
            .as_ref()
            .map(|plain_text_output| {
                Value::String(
                    projected
                        .comparison_plain_text_output
                        .as_ref()
                        .unwrap_or(plain_text_output)
                        .clone(),
                )
            })
            .ok_or_else(|| {
                Box::new(internal_adapter_error(
                    plan_digest_sha256,
                    Some(strategy_kind),
                    "execution could not project plain_text for this strategy",
                    InteropAdapterFailure::UnsupportedOutputProjection,
                    diagnostics,
                ))
            }),
        Output::InnerHtml => Ok(Value::String(projected.inner_html_output.clone())),
        Output::OuterHtml => Ok(Value::String(projected.outer_html_output.clone())),
        Output::SelectedHtml => projected
            .selected_html_output
            .as_ref()
            .map(|value| Value::String(value.clone()))
            .ok_or_else(|| {
                Box::new(internal_adapter_error(
                    plan_digest_sha256,
                    Some(strategy_kind),
                    "execution could not project selected_html for this strategy",
                    InteropAdapterFailure::UnsupportedOutputProjection,
                    diagnostics,
                ))
            }),
        Output::Attribute { name } => projected
            .attribute_values
            .get(name.as_str())
            .map(|value| Value::String(value.clone()))
            .ok_or_else(|| {
                Box::new(
                    InteropError::new(
                        plan_digest_sha256.to_owned(),
                        ErrorCode::MissingAttribute,
                        format!("Selected candidate is missing attribute \"{name}\"."),
                        Some(strategy_kind),
                        InteropErrorDetail::MissingRequestedAttribute,
                        diagnostics.iter().map(InteropDiagnostic::from).collect(),
                    )
                    .with_computed_digest()
                    .expect("missing-attribute interop error payload must digest"),
                )
            }),
        Output::Structured => Ok(projected.structured_output.clone()),
    }
}

fn optional_string_field(
    structured: &serde_json::Map<String, Value>,
    field: &'static str,
    plan_digest_sha256: &str,
    strategy_kind: StrategyKind,
    diagnostics: &[Diagnostic],
) -> Result<Option<String>, Box<InteropError>> {
    match structured.get(field) {
        None => Ok(None),
        Some(Value::String(value)) => Ok(Some(value.clone())),
        Some(_) => Err(Box::new(internal_adapter_error(
            plan_digest_sha256,
            Some(strategy_kind),
            format!("execution produced a non-string structured field {field:?}"),
            InteropAdapterFailure::InvalidStructuredProjection,
            diagnostics,
        ))),
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
            Box::new(internal_adapter_error(
                plan_digest_sha256,
                Some(strategy_kind),
                format!("execution could not project structured field {field:?}"),
                InteropAdapterFailure::InvalidStructuredProjection,
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
        return Err(Box::new(internal_adapter_error(
            plan_digest_sha256,
            Some(strategy_kind),
            format!("execution could not project structured field {field:?}"),
            InteropAdapterFailure::InvalidStructuredProjection,
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
                    Box::new(internal_adapter_error(
                        plan_digest_sha256,
                        Some(strategy_kind),
                        format!(
                            "execution produced a non-string attribute value in structured field {field:?}"
                        ),
                        InteropAdapterFailure::InvalidStructuredProjection,
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
) -> Result<NonZeroU32, Box<InteropError>> {
    let candidate_index = candidate_count_from_core(candidate_index);
    NonZeroU32::new(candidate_index).ok_or_else(|| {
        Box::new(internal_adapter_error(
            plan_digest_sha256,
            Some(strategy_kind),
            "execution received an invalid zero candidate index from core metadata",
            InteropAdapterFailure::InvalidCandidateIndex,
            diagnostics,
        ))
    })
}

fn candidate_count_from_core(value: usize) -> u32 {
    // Core execution has already bounded every candidate and selected-match cardinality by the
    // v2 `ExecutionBudget`, whose fixed-width maximum is `u32`. Adapter input is therefore not a
    // foreign wire value and this conversion cannot lose information.
    u32::try_from(value).expect("bounded core execution counts must fit the v2 wire range")
}

fn wire_byte_range(value: &crate::result::Range) -> ByteRange {
    // All supported Rust targets have `usize` no wider than 64 bits, while the v2 wire contract
    // carries ranges as `u64`; these offsets are therefore exact by construction.
    const _: () = assert!(usize::BITS <= u64::BITS);
    ByteRange {
        start: value.start as u64,
        end: value.end as u64,
    }
}
