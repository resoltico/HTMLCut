use std::collections::BTreeMap;

use htmlcut_core::{
    DEFAULT_PREVIEW_CHARS, Diagnostic, DiagnosticLevel,
    result::{InspectionCount, Range},
};

pub(crate) fn render_attribute_summary(attributes: &BTreeMap<String, String>) -> Option<String> {
    if attributes.is_empty() {
        return None;
    }

    Some(
        attributes
            .iter()
            .map(|(name, value)| format!("{name}={value:?}"))
            .collect::<Vec<_>>()
            .join(", "),
    )
}

pub(crate) fn format_range_summary(range: &Range) -> String {
    format!("{}..{}", range.start, range.end)
}

#[cfg(test)]
pub(crate) fn render_range_summary(range: Option<&Range>) -> Option<String> {
    range.map(format_range_summary)
}

pub(crate) fn compact_inline_preview(input: &str, preview_chars: usize) -> String {
    let normalized = input.split_whitespace().collect::<Vec<_>>().join(" ");
    let char_count = normalized.chars().count();
    if char_count <= preview_chars {
        return normalized;
    }

    let preview = normalized.chars().take(preview_chars).collect::<String>();
    format!("{preview}...")
}

pub(crate) fn render_fragment_preview(fragment: Option<&str>) -> Option<String> {
    let fragment = fragment?;
    (!fragment.is_empty()).then(|| compact_inline_preview(fragment, DEFAULT_PREVIEW_CHARS))
}

/// Builds stderr lines for non-fatal diagnostics when stdout is reserved for the payload.
pub(crate) fn build_human_diagnostic_stderr_lines(diagnostics: &[Diagnostic]) -> Vec<String> {
    diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.level != DiagnosticLevel::Error)
        .map(|diagnostic| {
            format!(
                "htmlcut: {} {}: {}",
                render_diagnostic_level(diagnostic.level),
                diagnostic.code,
                diagnostic.message
            )
        })
        .collect()
}

pub(crate) fn render_count_list(entries: &[InspectionCount]) -> String {
    entries
        .iter()
        .map(|entry| format!("{} ({})", entry.name, entry.count))
        .collect::<Vec<_>>()
        .join(", ")
}

pub(crate) fn render_diagnostic_line(diagnostic: &Diagnostic) -> String {
    format!(
        "- {} {}: {}",
        render_diagnostic_level(diagnostic.level),
        diagnostic.code,
        diagnostic.message
    )
}

pub(crate) fn render_diagnostic_level(level: DiagnosticLevel) -> &'static str {
    match level {
        DiagnosticLevel::Error => "error",
        DiagnosticLevel::Warning => "warning",
        DiagnosticLevel::Info => "info",
    }
}

pub(crate) fn render_source_kind(kind: &htmlcut_core::SourceKind) -> &'static str {
    match kind {
        htmlcut_core::SourceKind::Url => "url",
        htmlcut_core::SourceKind::File => "file",
        htmlcut_core::SourceKind::Stdin => "stdin",
        htmlcut_core::SourceKind::Memory => "memory",
    }
}

pub(crate) fn render_source_load_trace_lines(source: &htmlcut_core::SourceMetadata) -> Vec<String> {
    source
        .load_steps
        .iter()
        .map(|step| format!("- {}", render_source_load_step(step)))
        .collect()
}

pub(crate) fn build_source_load_verbose_lines(
    source: &htmlcut_core::SourceMetadata,
) -> Vec<String> {
    source
        .load_steps
        .iter()
        .map(|step| format!("htmlcut: source load {}", render_source_load_step(step)))
        .collect()
}

pub(crate) fn render_source_load_step(step: &htmlcut_core::SourceLoadStep) -> String {
    let action = match step.action {
        htmlcut_core::SourceLoadAction::HeadPreflight => "head preflight",
        htmlcut_core::SourceLoadAction::Get => "get",
    };
    let outcome = match step.outcome {
        htmlcut_core::SourceLoadOutcome::Succeeded => "succeeded",
        htmlcut_core::SourceLoadOutcome::Skipped => "skipped",
        htmlcut_core::SourceLoadOutcome::Fallback => "fallback",
        htmlcut_core::SourceLoadOutcome::Failed => "failed",
    };
    let status = step
        .status
        .map(|status| format!(" ({status})"))
        .unwrap_or_default();

    format!("{action} {outcome}{status}: {}", step.message)
}
