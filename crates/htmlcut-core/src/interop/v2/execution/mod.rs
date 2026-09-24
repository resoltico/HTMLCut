mod compile;
mod errors;
mod preparation;
mod project;
mod project_helpers;
mod source;

pub use preparation::prepare_document;
pub use source::{PrepareSourceError, PreparedSource, inspect_source, prepare_source};

#[cfg(test)]
pub(crate) use preparation::{
    preparation_parse_count_for_tests, reset_preparation_parse_count_for_tests,
};
#[cfg(test)]
pub(crate) use source::prepare_loaded_source_for_tests;
#[cfg(test)]
pub(crate) use source::source_inspection_from_prepared_for_tests;

use std::fmt;

use std::time::Instant;

use compile::{CompiledStrategy, compile_request, compile_strategy, exact_plan_digest_sha256};
use errors::{core_execution_error, plan_digest_error, plan_invalid_error};
use project::adapt_successful_extraction;

use crate::{
    OperationId, SelectorDomCanonicalization, SourceKind, SourceLoadStep, SourceMetadata,
    extract::{
        ExecutionLimits, FinalizedExtraction, finalize_result, run_prepared_selector_extraction,
        run_prepared_slice_extraction,
    },
    source::memory_label,
};

#[cfg(test)]
use super::HtmlInput;
use super::{InteropError, InteropResult, Output, Plan, PreparedDocument};

#[cfg(test)]
const TEST_PLAN_DIGEST_SHA256: &str =
    "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

/// One plan that has already passed interop validation.
pub struct CompiledPlan {
    plan: Plan,
    plan_digest_sha256: String,
    request: crate::ExtractionRequest,
    compiled_strategy: CompiledStrategy,
}

impl fmt::Debug for CompiledPlan {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CompiledPlan")
            .field("plan", &self.plan)
            .field("plan_digest_sha256", &self.plan_digest_sha256)
            .finish_non_exhaustive()
    }
}

impl CompiledPlan {
    /// Returns the validated plan document.
    pub fn plan(&self) -> &Plan {
        &self.plan
    }

    /// Returns the exact SHA-256 digest of the validated plan document.
    pub fn plan_digest_sha256(&self) -> &str {
        &self.plan_digest_sha256
    }

    fn strategy_kind(&self) -> super::StrategyKind {
        self.compiled_strategy.kind()
    }
}

/// Compiles one plan into a reusable source-independent execution input.
pub fn compile_plan(plan: &Plan) -> Result<CompiledPlan, Box<InteropError>> {
    let plan_digest_sha256 =
        exact_plan_digest_sha256(plan).map_err(|error| Box::new(plan_digest_error(plan, error)))?;
    plan.validate()
        .map_err(|error| Box::new(plan_invalid_error(plan, &plan_digest_sha256, error)))?;
    let request = compile_request(plan);
    let compiled_strategy = compile_strategy(plan, &request).map_err(|diagnostic| {
        Box::new(core_execution_error(
            plan,
            &plan_digest_sha256,
            0,
            &[diagnostic],
        ))
    })?;

    Ok(CompiledPlan {
        plan: plan.clone(),
        plan_digest_sha256,
        request,
        compiled_strategy,
    })
}

/// Executes one previously validated plan directly against in-memory HTML input.
pub fn execute(
    document: &PreparedDocument,
    compiled_plan: &CompiledPlan,
) -> Result<InteropResult, Box<InteropError>> {
    let started_at = Instant::now();
    debug_assert_eq!(
        compiled_plan.strategy_kind(),
        compiled_plan.plan().strategy.kind(),
        "compiled plan grammar must match its declarative strategy"
    );
    let execution_limits = execution_limits(compiled_plan);
    let dom_canonicalization = match &compiled_plan.plan().output {
        Output::Text | Output::PlainText | Output::Structured => compiled_plan
            .plan()
            .dom_canonicalization
            .as_ref()
            .map(|canonicalization| {
                SelectorDomCanonicalization::new(
                    canonicalization
                        .ignore_attributes
                        .iter()
                        .map(ToString::to_string),
                    canonicalization.strip_whitespace_nodes,
                )
            }),
        Output::InnerHtml | Output::OuterHtml | Output::Attribute { .. } | Output::SelectedHtml => {
            None
        }
    };
    let run = match &compiled_plan.compiled_strategy {
        CompiledStrategy::CssSelector(selector) => run_prepared_selector_extraction(
            &compiled_plan.request,
            document.document(),
            document
                .input()
                .input_base_url
                .as_ref()
                .map(ToString::to_string)
                .as_deref(),
            selector,
            dom_canonicalization.as_ref(),
            Some(execution_limits),
        ),
        CompiledStrategy::DelimiterPair { slice, patterns } => run_prepared_slice_extraction(
            &compiled_plan.request,
            &document.input().html,
            document.document(),
            document
                .input()
                .input_base_url
                .as_ref()
                .map(ToString::to_string)
                .as_deref(),
            slice,
            patterns,
            Some(execution_limits),
        ),
    };
    let source = SourceMetadata {
        kind: SourceKind::Memory,
        value: memory_label(&document.input().label),
        input_base_url: document
            .input()
            .input_base_url
            .as_ref()
            .map(ToString::to_string),
        effective_base_url: run.effective_base_url.clone(),
        bytes_read: document.input().html.len(),
        load_steps: Vec::<SourceLoadStep>::new(),
        text: None,
    };
    let extraction = finalize_result(
        &compiled_plan.request,
        FinalizedExtraction {
            operation_id: match compiled_plan.plan().strategy.kind() {
                super::StrategyKind::CssSelector => OperationId::SelectExtract,
                super::StrategyKind::DelimiterPair => OperationId::SliceExtract,
            },
            source,
            document_title: run.document_title,
            diagnostics: run.diagnostics,
            matches: run.matches,
            candidate_count: run.candidate_count,
        },
        started_at,
    );
    if !extraction.ok {
        return Err(Box::new(core_execution_error(
            compiled_plan.plan(),
            compiled_plan.plan_digest_sha256(),
            extraction.stats.candidate_count,
            &extraction.diagnostics,
        )));
    }
    adapt_successful_extraction(
        document.input(),
        document.input_digest_sha256().to_owned(),
        compiled_plan.plan(),
        compiled_plan.plan_digest_sha256.clone(),
        extraction,
    )
}

fn execution_limits(compiled_plan: &CompiledPlan) -> ExecutionLimits {
    // Every supported Rust target represents at least 32 bits in `usize`, while all v2
    // execution-budget fields are validated `u32` values. The casts are therefore exact.
    const _: () = assert!(usize::BITS >= u32::BITS);
    let budget = &compiled_plan.plan().execution_budget;
    ExecutionLimits {
        max_selector_work_units: budget.max_selector_work_units.get(),
        max_candidates: budget.max_candidates.get() as usize,
        max_selected_matches: budget.max_selected_matches.get() as usize,
        max_match_output_bytes: budget.max_match_output_bytes.get() as usize,
        max_total_output_bytes: budget.max_total_output_bytes.get() as usize,
    }
}

#[cfg(test)]
/// Executes one plan through the public v2 preparation boundary for test ergonomics only.
pub(crate) fn execute_one_shot_for_tests(
    source: &HtmlInput,
    plan: &Plan,
) -> Result<InteropResult, Box<InteropError>> {
    let compiled_plan = compile_plan(plan)?;
    let document = super::prepare_document(source.clone(), super::PreparationLimits::default())
        .expect("test one-shot inputs must satisfy default preparation limits");
    execute(&document, &compiled_plan)
}

#[cfg(test)]
pub(crate) fn compile_request_for_tests(
    _source: &HtmlInput,
    plan: &Plan,
) -> crate::ExtractionRequest {
    compile::compile_request(plan)
}

#[cfg(test)]
pub(crate) fn exact_plan_digest_sha256_for_tests(
    plan: &Plan,
) -> Result<String, super::ContractError> {
    exact_plan_digest_sha256(plan)
}

#[cfg(test)]
pub(crate) fn compile_regex_flags_for_tests(flags: &[super::RegexFlag]) -> String {
    compile::compile_regex_flags(flags)
}

#[cfg(test)]
pub(crate) fn project_structured_match_for_tests(
    matched: &crate::result::ExtractionMatch,
    strategy_kind: super::StrategyKind,
    diagnostics: &[crate::Diagnostic],
) -> Result<(), Box<InteropError>> {
    project::project_structured_match(matched, strategy_kind, TEST_PLAN_DIGEST_SHA256, diagnostics)
        .map(|_| ())
}

#[cfg(test)]
pub(crate) fn project_plain_text_for_tests(
    matched: &crate::result::ExtractionMatch,
    strategy_kind: super::StrategyKind,
    diagnostics: &[crate::Diagnostic],
) -> Result<(), Box<InteropError>> {
    let projected = project::project_structured_match(
        matched,
        strategy_kind,
        TEST_PLAN_DIGEST_SHA256,
        diagnostics,
    )?;
    project::project_output_value(
        &super::Output::PlainText,
        &projected,
        TEST_PLAN_DIGEST_SHA256,
        strategy_kind,
        diagnostics,
    )
    .map(|_| ())
}

#[cfg(test)]
pub(crate) fn parse_optional_url_for_tests(
    value: Option<&str>,
    field: &'static str,
    diagnostics: &[crate::Diagnostic],
) -> Result<Option<crate::DisplayedHttpUrl>, Box<InteropError>> {
    project_helpers::parse_optional_url(
        value,
        TEST_PLAN_DIGEST_SHA256,
        super::StrategyKind::CssSelector,
        field,
        diagnostics,
    )
}

#[cfg(test)]
pub(crate) fn core_execution_error_for_tests(
    plan: &Plan,
    diagnostics: &[crate::Diagnostic],
) -> InteropError {
    let plan_digest_sha256 = exact_plan_digest_sha256(plan).expect("plan digest");
    errors::core_execution_error(plan, &plan_digest_sha256, 0, diagnostics)
}

#[cfg(test)]
pub(crate) fn internal_adapter_error_for_tests(
    message: impl Into<String>,
    failure: super::InteropAdapterFailure,
    diagnostics: Vec<crate::Diagnostic>,
) -> InteropError {
    errors::internal_adapter_error(
        TEST_PLAN_DIGEST_SHA256,
        Some(super::StrategyKind::CssSelector),
        message,
        failure,
        &diagnostics,
    )
}

#[cfg(test)]
pub(crate) fn internal_adapter_error_with_plan_digest_for_tests(
    plan_digest_sha256: &str,
    message: impl Into<String>,
    failure: super::InteropAdapterFailure,
    diagnostics: Vec<crate::Diagnostic>,
) -> InteropError {
    errors::internal_adapter_error(
        plan_digest_sha256,
        Some(super::StrategyKind::CssSelector),
        message,
        failure,
        &diagnostics,
    )
}

#[cfg(test)]
pub(crate) fn finalize_error_for_tests(error: InteropError) -> InteropError {
    errors::finalize_error(error)
}

#[cfg(test)]
pub(crate) fn finalize_sanitized_fallback_for_tests(error: InteropError) -> InteropError {
    errors::finalize_sanitized_fallback_for_tests(error)
}

#[cfg(test)]
pub(crate) fn plan_digest_error_for_tests(
    plan: &Plan,
    error: super::ContractError,
) -> InteropError {
    errors::plan_digest_error(plan, error)
}

#[cfg(test)]
pub(crate) fn adapt_successful_extraction_for_tests(
    source: &HtmlInput,
    plan: &Plan,
    extraction: crate::ExtractionResult,
) -> Result<InteropResult, Box<InteropError>> {
    project::adapt_successful_extraction(
        source,
        TEST_PLAN_DIGEST_SHA256.to_owned(),
        plan,
        exact_plan_digest_sha256(plan).expect("plan digest"),
        extraction,
    )
}
