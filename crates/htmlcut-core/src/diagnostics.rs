use serde_json::{Value, json};

use crate::contracts::{Diagnostic, DiagnosticLevel};

pub(crate) fn has_errors(diagnostics: &[Diagnostic]) -> bool {
    diagnostics
        .iter()
        .any(|diagnostic| diagnostic.level == DiagnosticLevel::Error)
}

pub(crate) fn error_diagnostic(
    code: &str,
    message: impl Into<String>,
    details: Option<Value>,
) -> Diagnostic {
    Diagnostic {
        level: DiagnosticLevel::Error,
        code: code.to_owned(),
        message: message.into(),
        details,
    }
}

pub(crate) fn warning_diagnostic(
    code: &str,
    message: impl Into<String>,
    details: Option<Value>,
) -> Diagnostic {
    Diagnostic {
        level: DiagnosticLevel::Warning,
        code: code.to_owned(),
        message: message.into(),
        details,
    }
}

/// Reports that an effective base URL could not be determined for a request that depends on it.
pub(crate) fn unresolved_effective_base_diagnostic(
    document_base_href: Option<&str>,
    rewrite_requested: bool,
) -> Diagnostic {
    warning_diagnostic(
        "EFFECTIVE_BASE_URL_UNRESOLVED",
        if rewrite_requested {
            "URL rewriting was requested, but no effective base URL could be resolved. Relative URLs are left unchanged."
        } else {
            "The document declares <base href>, but no effective base URL could be resolved for this input."
        },
        Some(json!({
            "documentBaseHref": document_base_href,
            "rewriteRequested": rewrite_requested,
        })),
    )
}
