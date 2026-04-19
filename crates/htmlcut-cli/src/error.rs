use htmlcut_core::{Diagnostic, DiagnosticCode, DiagnosticLevel};
use serde::{Deserialize, Serialize};

use crate::{
    EXIT_CODE_EXTRACTION, EXIT_CODE_INTERNAL, EXIT_CODE_OUTPUT, EXIT_CODE_SOURCE, EXIT_CODE_USAGE,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CliErrorCategory {
    Usage,
    Source,
    Extraction,
    Output,
    Internal,
}

#[derive(Debug)]
pub(crate) struct CliError {
    pub(crate) category: CliErrorCategory,
    pub(crate) code: String,
    pub(crate) message: String,
    pub(crate) diagnostics: Vec<Diagnostic>,
    pub(crate) source_load_steps: Vec<htmlcut_core::SourceLoadStep>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub(crate) struct CliErrorBody {
    pub(crate) category: String,
    pub(crate) code: String,
    pub(crate) message: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub(crate) struct CliErrorReport {
    pub(crate) tool: String,
    pub(crate) engine: String,
    pub(crate) version: String,
    pub(crate) command: String,
    pub(crate) ok: bool,
    pub(crate) exit_code: i32,
    pub(crate) error: CliErrorBody,
    pub(crate) diagnostics: Vec<Diagnostic>,
}

pub(crate) fn primary_extraction_error(diagnostics: &[Diagnostic]) -> CliError {
    let Some(diagnostic) = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.level == DiagnosticLevel::Error)
    else {
        return internal_error(
            "CLI_PRIMARY_DIAGNOSTIC_MISSING",
            "Extraction failed without a primary diagnostic.",
        );
    };

    match diagnostic.code.parse::<DiagnosticCode>() {
        Ok(DiagnosticCode::SourceLoadFailed) => source_error(
            diagnostic.code.clone(),
            diagnostic.message.clone(),
            diagnostics.to_vec(),
        ),
        Ok(
            DiagnosticCode::InvalidRequest
            | DiagnosticCode::InvalidSelector
            | DiagnosticCode::InvalidSlicePattern
            | DiagnosticCode::UnsupportedSpecVersion,
        ) => usage_error_with_diagnostics(
            diagnostic.code.clone(),
            diagnostic.message.clone(),
            diagnostics.to_vec(),
        ),
        Ok(
            DiagnosticCode::NoMatch
            | DiagnosticCode::AmbiguousMatch
            | DiagnosticCode::MatchIndexOutOfRange
            | DiagnosticCode::MissingAttribute
            | DiagnosticCode::ParseFailed,
        ) => extraction_error(
            diagnostic.code.clone(),
            diagnostic.message.clone(),
            diagnostics.to_vec(),
        ),
        _ => internal_error_with_diagnostics(
            diagnostic.code.clone(),
            diagnostic.message.clone(),
            diagnostics.to_vec(),
        ),
    }
}
pub(crate) fn primary_source_inspection_error(diagnostics: &[Diagnostic]) -> CliError {
    match diagnostics
        .iter()
        .find(|diagnostic| diagnostic.level == DiagnosticLevel::Error)
    {
        Some(diagnostic) => match diagnostic.code.parse::<DiagnosticCode>() {
            Ok(DiagnosticCode::SourceLoadFailed) => source_error(
                diagnostic.code.clone(),
                diagnostic.message.clone(),
                diagnostics.to_vec(),
            ),
            _ => internal_error_with_diagnostics(
                diagnostic.code.clone(),
                diagnostic.message.clone(),
                diagnostics.to_vec(),
            ),
        },
        None => internal_error(
            "CLI_PRIMARY_DIAGNOSTIC_MISSING",
            "Inspection failed without a primary diagnostic.",
        ),
    }
}

pub(crate) fn usage_error(code: impl Into<String>, message: impl Into<String>) -> CliError {
    CliError {
        category: CliErrorCategory::Usage,
        code: code.into(),
        message: message.into(),
        diagnostics: Vec::new(),
        source_load_steps: Vec::new(),
    }
}

pub(crate) fn usage_error_with_diagnostics(
    code: impl Into<String>,
    message: impl Into<String>,
    diagnostics: Vec<Diagnostic>,
) -> CliError {
    CliError {
        category: CliErrorCategory::Usage,
        code: code.into(),
        message: message.into(),
        diagnostics,
        source_load_steps: Vec::new(),
    }
}

pub(crate) fn source_error(
    code: impl Into<String>,
    message: impl Into<String>,
    diagnostics: Vec<Diagnostic>,
) -> CliError {
    CliError {
        category: CliErrorCategory::Source,
        code: code.into(),
        message: message.into(),
        diagnostics,
        source_load_steps: Vec::new(),
    }
}

pub(crate) fn extraction_error(
    code: impl Into<String>,
    message: impl Into<String>,
    diagnostics: Vec<Diagnostic>,
) -> CliError {
    CliError {
        category: CliErrorCategory::Extraction,
        code: code.into(),
        message: message.into(),
        diagnostics,
        source_load_steps: Vec::new(),
    }
}

pub(crate) fn output_error(code: impl Into<String>, message: impl Into<String>) -> CliError {
    CliError {
        category: CliErrorCategory::Output,
        code: code.into(),
        message: message.into(),
        diagnostics: Vec::new(),
        source_load_steps: Vec::new(),
    }
}

pub(crate) fn internal_error(code: impl Into<String>, message: impl Into<String>) -> CliError {
    CliError {
        category: CliErrorCategory::Internal,
        code: code.into(),
        message: message.into(),
        diagnostics: Vec::new(),
        source_load_steps: Vec::new(),
    }
}

pub(crate) fn internal_error_with_diagnostics(
    code: impl Into<String>,
    message: impl Into<String>,
    diagnostics: Vec<Diagnostic>,
) -> CliError {
    CliError {
        category: CliErrorCategory::Internal,
        code: code.into(),
        message: message.into(),
        diagnostics,
        source_load_steps: Vec::new(),
    }
}

pub(crate) fn with_source_load_steps(
    mut error: CliError,
    source: &htmlcut_core::SourceMetadata,
) -> CliError {
    error.source_load_steps = source.load_steps.clone();
    error
}

pub(crate) fn json_error_diagnostics(error: &CliError) -> Vec<Diagnostic> {
    match error.diagnostics.is_empty() {
        true => vec![Diagnostic {
            level: DiagnosticLevel::Error,
            code: error.code.clone(),
            message: error.message.clone(),
            details: None,
        }],
        false => error.diagnostics.clone(),
    }
}

pub(crate) fn render_error_category(category: CliErrorCategory) -> &'static str {
    match category {
        CliErrorCategory::Usage => "usage",
        CliErrorCategory::Source => "source",
        CliErrorCategory::Extraction => "extraction",
        CliErrorCategory::Output => "output",
        CliErrorCategory::Internal => "internal",
    }
}

pub(crate) fn exit_code_for_error(error: &CliError) -> i32 {
    match error.category {
        CliErrorCategory::Usage => EXIT_CODE_USAGE,
        CliErrorCategory::Source => EXIT_CODE_SOURCE,
        CliErrorCategory::Extraction => EXIT_CODE_EXTRACTION,
        CliErrorCategory::Output => EXIT_CODE_OUTPUT,
        CliErrorCategory::Internal => EXIT_CODE_INTERNAL,
    }
}
