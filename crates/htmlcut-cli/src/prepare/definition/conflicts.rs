use htmlcut_core::{
    DEFAULT_FETCH_CONNECT_TIMEOUT_MS, DEFAULT_FETCH_TIMEOUT_MS, DEFAULT_MAX_BYTES,
    DEFAULT_PREVIEW_CHARS,
};

use crate::args::{
    CliFetchPreflightMode, CliMatchMode, CliPatternMode, CliValueMode, CliWhitespaceMode,
    ExtractOutputArgs, InspectOutputArgs, InspectSelectArgs, InspectSliceArgs, SelectArgs,
    SelectionArgs, SliceArgs, SourceArgs,
};
use crate::error::{CliError, usage_error};
use crate::model::CliErrorCode;

pub(in crate::prepare) fn ensure_inline_select_request_is_default(
    args: &SelectArgs,
) -> Result<(), CliError> {
    let mut conflicts = collect_source_request_file_conflicts(&args.source);
    push_conflict(&mut conflicts, args.css.is_some(), "--css");
    extend_selection_request_file_conflicts(&mut conflicts, &args.selection);
    extend_extract_request_file_conflicts(&mut conflicts, &args.output);
    reject_request_file_conflicts(conflicts)
}

pub(in crate::prepare) fn ensure_inline_slice_request_is_default(
    args: &SliceArgs,
) -> Result<(), CliError> {
    let mut conflicts = collect_source_request_file_conflicts(&args.source);
    push_conflict(&mut conflicts, args.from.is_some(), "--from");
    push_conflict(&mut conflicts, args.to.is_some(), "--to");
    push_conflict(
        &mut conflicts,
        args.pattern != CliPatternMode::Literal,
        "--pattern",
    );
    push_conflict(&mut conflicts, args.regex_flags.is_some(), "--regex-flags");
    push_conflict(&mut conflicts, args.include_start, "--include-start");
    push_conflict(&mut conflicts, args.include_end, "--include-end");
    extend_selection_request_file_conflicts(&mut conflicts, &args.selection);
    extend_extract_request_file_conflicts(&mut conflicts, &args.output);
    reject_request_file_conflicts(conflicts)
}

pub(in crate::prepare) fn ensure_inline_inspect_select_request_is_default(
    args: &InspectSelectArgs,
) -> Result<(), CliError> {
    let mut conflicts = collect_source_request_file_conflicts(&args.source);
    push_conflict(&mut conflicts, args.css.is_some(), "--css");
    extend_selection_request_file_conflicts(&mut conflicts, &args.selection);
    extend_inspect_request_file_conflicts(&mut conflicts, &args.output);
    push_conflict(&mut conflicts, args.rewrite_urls, "--rewrite-urls");
    push_conflict(
        &mut conflicts,
        args.whitespace != CliWhitespaceMode::Preserve,
        "--whitespace",
    );
    reject_request_file_conflicts(conflicts)
}

pub(in crate::prepare) fn ensure_inline_inspect_slice_request_is_default(
    args: &InspectSliceArgs,
) -> Result<(), CliError> {
    let mut conflicts = collect_source_request_file_conflicts(&args.source);
    push_conflict(&mut conflicts, args.from.is_some(), "--from");
    push_conflict(&mut conflicts, args.to.is_some(), "--to");
    push_conflict(
        &mut conflicts,
        args.pattern != CliPatternMode::Literal,
        "--pattern",
    );
    push_conflict(&mut conflicts, args.regex_flags.is_some(), "--regex-flags");
    push_conflict(&mut conflicts, args.include_start, "--include-start");
    push_conflict(&mut conflicts, args.include_end, "--include-end");
    extend_selection_request_file_conflicts(&mut conflicts, &args.selection);
    extend_inspect_request_file_conflicts(&mut conflicts, &args.output);
    push_conflict(&mut conflicts, args.rewrite_urls, "--rewrite-urls");
    push_conflict(
        &mut conflicts,
        args.whitespace != CliWhitespaceMode::Preserve,
        "--whitespace",
    );
    reject_request_file_conflicts(conflicts)
}

fn collect_source_request_file_conflicts(source: &SourceArgs) -> Vec<&'static str> {
    let mut conflicts = Vec::new();
    push_conflict(&mut conflicts, source.input.is_some(), "<INPUT>");
    push_conflict(&mut conflicts, source.base_url.is_some(), "--base-url");
    push_conflict(
        &mut conflicts,
        source.max_bytes != DEFAULT_MAX_BYTES.to_string(),
        "--max-bytes",
    );
    push_conflict(
        &mut conflicts,
        source.fetch_timeout_ms != DEFAULT_FETCH_TIMEOUT_MS,
        "--fetch-timeout-ms",
    );
    push_conflict(
        &mut conflicts,
        source.fetch_connect_timeout_ms != DEFAULT_FETCH_CONNECT_TIMEOUT_MS,
        "--fetch-connect-timeout-ms",
    );
    push_conflict(
        &mut conflicts,
        source.fetch_preflight != CliFetchPreflightMode::HeadFirst,
        "--fetch-preflight",
    );
    conflicts
}

fn extend_selection_request_file_conflicts(
    conflicts: &mut Vec<&'static str>,
    selection: &SelectionArgs,
) {
    push_conflict(
        conflicts,
        selection.r#match != CliMatchMode::First,
        "--match",
    );
    push_conflict(conflicts, selection.index.is_some(), "--index");
}

fn extend_extract_request_file_conflicts(
    conflicts: &mut Vec<&'static str>,
    output: &ExtractOutputArgs,
) {
    push_conflict(conflicts, output.value != CliValueMode::Text, "--value");
    push_conflict(conflicts, output.attribute.is_some(), "--attribute");
    push_conflict(
        conflicts,
        output.whitespace != CliWhitespaceMode::Preserve,
        "--whitespace",
    );
    push_conflict(conflicts, output.rewrite_urls, "--rewrite-urls");
    push_conflict(
        conflicts,
        output.preview_chars != DEFAULT_PREVIEW_CHARS,
        "--preview-chars",
    );
    push_conflict(
        conflicts,
        output.include_source_text,
        "--include-source-text",
    );
}

fn extend_inspect_request_file_conflicts(
    conflicts: &mut Vec<&'static str>,
    output: &InspectOutputArgs,
) {
    push_conflict(
        conflicts,
        output.preview_chars != DEFAULT_PREVIEW_CHARS,
        "--preview-chars",
    );
    push_conflict(
        conflicts,
        output.include_source_text,
        "--include-source-text",
    );
}

fn push_conflict(conflicts: &mut Vec<&'static str>, condition: bool, flag: &'static str) {
    if condition {
        conflicts.push(flag);
    }
}

fn reject_request_file_conflicts(conflicts: Vec<&'static str>) -> Result<(), CliError> {
    if conflicts.is_empty() {
        return Ok(());
    }

    Err(usage_error(
        CliErrorCode::RequestFileConflict,
        format!(
            "--request-file owns the extraction definition; remove the inline request flags: {}. If you want to keep the inline form, drop `--request-file` and use `--emit-request-file <PATH>` to save the canonical definition.",
            conflicts.join(", ")
        ),
    ))
}
