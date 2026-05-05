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

pub(crate) fn block_text_preview(
    input: &str,
    preview_chars: usize,
    max_lines: usize,
) -> Vec<String> {
    let mut preview = String::new();
    for (index, character) in input.chars().enumerate() {
        if index >= preview_chars {
            preview.push_str("...");
            break;
        }
        preview.push(character);
    }

    let mut lines = preview
        .trim_matches('\n')
        .lines()
        .map(str::trim_end)
        .skip_while(|line| line.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    while lines.last().is_some_and(|line| line.is_empty()) {
        lines.pop();
    }
    if lines.len() > max_lines {
        lines.truncate(max_lines);
        if let Some(last) = lines.last_mut()
            && !last.ends_with("...")
        {
            last.push_str("...");
        }
    }
    lines
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

#[cfg(test)]
mod tests {
    use super::*;
    use htmlcut_core::{
        Diagnostic, DiagnosticCode, DiagnosticLevel, SourceLoadAction, SourceLoadOutcome,
        SourceLoadStep, result::Range,
    };

    #[test]
    fn block_text_preview_covers_truncation_line_limits_and_blank_edges() {
        assert_eq!(render_attribute_summary(&BTreeMap::new()), None);
        assert_eq!(
            render_attribute_summary(&BTreeMap::from([(
                "href".to_owned(),
                "https://example.test".to_owned(),
            )]))
            .as_deref(),
            Some("href=\"https://example.test\"")
        );
        assert_eq!(
            compact_inline_preview("  alpha \n beta  ", DEFAULT_PREVIEW_CHARS),
            "alpha beta"
        );
        assert_eq!(
            block_text_preview("alpha beta gamma", 5, 6),
            vec!["alpha...".to_owned()]
        );
        assert_eq!(
            block_text_preview("\n\nalpha\nbeta\n\n", 100, 10),
            vec!["alpha".to_owned(), "beta".to_owned()]
        );
        assert_eq!(
            block_text_preview("alpha\nbeta\n   \n", 100, 10),
            vec!["alpha".to_owned(), "beta".to_owned()]
        );
        assert_eq!(
            block_text_preview("one\ntwo\nthree\nfour", 100, 2),
            vec!["one".to_owned(), "two...".to_owned()]
        );
        assert_eq!(
            block_text_preview("one\ntwo...\nthree", 100, 2),
            vec!["one".to_owned(), "two...".to_owned()]
        );
        assert!(block_text_preview("one\ntwo", 100, 0).is_empty());
        assert_eq!(
            render_fragment_preview(Some("  alpha \n beta  ")).as_deref(),
            Some("alpha beta")
        );
        assert_eq!(render_fragment_preview(Some("")), None);
        assert_eq!(render_fragment_preview(None), None);
        assert_eq!(
            render_range_summary(Some(&Range { start: 4, end: 9 })),
            Some("4..9".to_owned())
        );
        assert_eq!(
            build_human_diagnostic_stderr_lines(&[
                Diagnostic {
                    level: DiagnosticLevel::Warning,
                    code: DiagnosticCode::InvalidSelector,
                    message: "warning message".to_owned(),
                    details: None,
                },
                Diagnostic {
                    level: DiagnosticLevel::Error,
                    code: DiagnosticCode::NoMatch,
                    message: "error message".to_owned(),
                    details: None,
                },
                Diagnostic {
                    level: DiagnosticLevel::Info,
                    code: DiagnosticCode::MissingAttribute,
                    message: "info message".to_owned(),
                    details: None,
                },
            ]),
            vec![
                "htmlcut: warning INVALID_SELECTOR: warning message".to_owned(),
                "htmlcut: info MISSING_ATTRIBUTE: info message".to_owned(),
            ]
        );
        assert_eq!(
            render_source_load_step(&SourceLoadStep {
                action: SourceLoadAction::Get,
                outcome: SourceLoadOutcome::Succeeded,
                status: Some(200),
                message: "ok".to_owned(),
            }),
            "get succeeded (200): ok"
        );
        assert_eq!(
            render_source_load_step(&SourceLoadStep {
                action: SourceLoadAction::HeadPreflight,
                outcome: SourceLoadOutcome::Skipped,
                status: None,
                message: "disabled".to_owned(),
            }),
            "head preflight skipped: disabled"
        );
    }
}
