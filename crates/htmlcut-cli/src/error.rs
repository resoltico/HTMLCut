use htmlcut_core::{Diagnostic, DiagnosticCode, DiagnosticLevel};

use crate::model::{
    CliErrorCode, ErrorReportBody, ErrorReportCategory, ErrorReportCode, ErrorReportDiagnostic,
};
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
    pub(crate) code: ErrorReportCode,
    pub(crate) message: String,
    pub(crate) diagnostics: Vec<Diagnostic>,
    pub(crate) source_load_steps: Vec<htmlcut_core::SourceLoadStep>,
}

pub(crate) fn primary_extraction_error(diagnostics: &[Diagnostic]) -> CliError {
    let Some(diagnostic) = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.level == DiagnosticLevel::Error)
    else {
        return internal_error(
            CliErrorCode::PrimaryDiagnosticMissing,
            "Extraction failed without a primary diagnostic.",
        );
    };

    match diagnostic.code {
        DiagnosticCode::SourceLoadFailed => source_error(
            diagnostic.code,
            diagnostic.message.clone(),
            diagnostics.to_vec(),
        ),
        DiagnosticCode::InvalidSelector
        | DiagnosticCode::InvalidSlicePattern
        | DiagnosticCode::UnsupportedSpecVersion => usage_error_with_diagnostics(
            diagnostic.code,
            diagnostic.message.clone(),
            diagnostics.to_vec(),
        ),
        DiagnosticCode::NoMatch
        | DiagnosticCode::AmbiguousMatch
        | DiagnosticCode::MatchIndexOutOfRange
        | DiagnosticCode::MissingAttribute => extraction_error(
            diagnostic.code,
            diagnostic.message.clone(),
            diagnostics.to_vec(),
        ),
        DiagnosticCode::MultipleMatches
        | DiagnosticCode::EffectiveBaseUrlUnresolved
        | DiagnosticCode::SliceSplitsMarkup => internal_error_with_diagnostics(
            diagnostic.code,
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
        Some(diagnostic) => match diagnostic.code {
            DiagnosticCode::SourceLoadFailed => source_error(
                diagnostic.code,
                diagnostic.message.clone(),
                diagnostics.to_vec(),
            ),
            _ => internal_error_with_diagnostics(
                diagnostic.code,
                diagnostic.message.clone(),
                diagnostics.to_vec(),
            ),
        },
        None => internal_error(
            CliErrorCode::PrimaryDiagnosticMissing,
            "Inspection failed without a primary diagnostic.",
        ),
    }
}

pub(crate) fn usage_error(
    code: impl Into<ErrorReportCode>,
    message: impl Into<String>,
) -> CliError {
    CliError {
        category: CliErrorCategory::Usage,
        code: code.into(),
        message: message.into(),
        diagnostics: Vec::new(),
        source_load_steps: Vec::new(),
    }
}

pub(crate) fn usage_error_with_diagnostics(
    code: impl Into<ErrorReportCode>,
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
    code: impl Into<ErrorReportCode>,
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
    code: impl Into<ErrorReportCode>,
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

pub(crate) fn output_error(
    code: impl Into<ErrorReportCode>,
    message: impl Into<String>,
) -> CliError {
    CliError {
        category: CliErrorCategory::Output,
        code: code.into(),
        message: message.into(),
        diagnostics: Vec::new(),
        source_load_steps: Vec::new(),
    }
}

pub(crate) fn internal_error(
    code: impl Into<ErrorReportCode>,
    message: impl Into<String>,
) -> CliError {
    CliError {
        category: CliErrorCategory::Internal,
        code: code.into(),
        message: message.into(),
        diagnostics: Vec::new(),
        source_load_steps: Vec::new(),
    }
}

pub(crate) fn internal_error_with_diagnostics(
    code: impl Into<ErrorReportCode>,
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

impl From<CliErrorCategory> for ErrorReportCategory {
    fn from(category: CliErrorCategory) -> Self {
        match category {
            CliErrorCategory::Usage => Self::Usage,
            CliErrorCategory::Source => Self::Source,
            CliErrorCategory::Extraction => Self::Extraction,
            CliErrorCategory::Output => Self::Output,
            CliErrorCategory::Internal => Self::Internal,
        }
    }
}

pub(crate) fn error_report_body(error: &CliError) -> ErrorReportBody {
    ErrorReportBody {
        category: error.category.into(),
        code: error.code,
        message: error.message.clone(),
    }
}

pub(crate) fn json_error_diagnostics(error: &CliError) -> Vec<ErrorReportDiagnostic> {
    match error.diagnostics.is_empty() {
        true => vec![ErrorReportDiagnostic {
            level: DiagnosticLevel::Error,
            code: error.code,
            message: error.message.clone(),
            details: None,
        }],
        false => error
            .diagnostics
            .iter()
            .map(report_diagnostic_from_core)
            .collect(),
    }
}

fn report_diagnostic_from_core(diagnostic: &Diagnostic) -> ErrorReportDiagnostic {
    ErrorReportDiagnostic {
        level: diagnostic.level,
        code: diagnostic.code.into(),
        message: diagnostic.message.clone(),
        details: diagnostic.details.clone(),
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
