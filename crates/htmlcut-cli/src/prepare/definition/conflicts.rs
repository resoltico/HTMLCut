use htmlcut_core::{
    DEFAULT_FETCH_CONNECT_TIMEOUT_MS, DEFAULT_FETCH_TIMEOUT_MS, DEFAULT_MAX_BYTES,
    DEFAULT_PREVIEW_CHARS,
};

use crate::args::{
    CliFetchPreflightMode, CliMatchMode, CliPatternMode, CliSliceValueMode, CliTlsTrustMode,
    CliValueMode, CliWhitespaceMode, ExtractOutputArgs, InspectOutputArgs, InspectSelectArgs,
    InspectSliceArgs, SelectArgs, SelectionArgs, SliceArgs, SliceExtractOutputArgs, SourceArgs,
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
    push_conflict(
        &mut conflicts,
        args.boundary_retention != crate::args::CliBoundaryRetentionMode::ExcludeBoth,
        "--boundary-retention",
    );
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
    push_conflict(
        &mut conflicts,
        args.value != CliValueMode::Structured,
        "--value",
    );
    push_conflict(&mut conflicts, args.attribute.is_some(), "--attribute");
    push_conflict(&mut conflicts, args.rewrite_urls, "--rewrite-urls");
    push_conflict(
        &mut conflicts,
        args.whitespace != CliWhitespaceMode::Rendered,
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
    push_conflict(
        &mut conflicts,
        args.boundary_retention != crate::args::CliBoundaryRetentionMode::ExcludeBoth,
        "--boundary-retention",
    );
    extend_selection_request_file_conflicts(&mut conflicts, &args.selection);
    extend_inspect_request_file_conflicts(&mut conflicts, &args.output);
    push_conflict(
        &mut conflicts,
        args.value != CliSliceValueMode::Structured,
        "--value",
    );
    push_conflict(&mut conflicts, args.attribute.is_some(), "--attribute");
    push_conflict(&mut conflicts, args.rewrite_urls, "--rewrite-urls");
    push_conflict(
        &mut conflicts,
        args.whitespace != CliWhitespaceMode::Rendered,
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
    push_conflict(
        &mut conflicts,
        source.tls_trust != CliTlsTrustMode::WebPki,
        "--tls-trust",
    );
    push_conflict(
        &mut conflicts,
        source.tls_ca_bundle.is_some(),
        "--tls-ca-bundle",
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

trait ExtractOutputConflictArgs {
    fn value_is_non_default(&self) -> bool;
    fn has_attribute(&self) -> bool;
    fn whitespace(&self) -> CliWhitespaceMode;
    fn rewrite_urls(&self) -> bool;
    fn preview_chars(&self) -> usize;
    fn include_source_text(&self) -> bool;
}

impl ExtractOutputConflictArgs for ExtractOutputArgs {
    fn value_is_non_default(&self) -> bool {
        self.value != CliValueMode::Text
    }

    fn has_attribute(&self) -> bool {
        self.attribute.is_some()
    }

    fn whitespace(&self) -> CliWhitespaceMode {
        self.whitespace
    }

    fn rewrite_urls(&self) -> bool {
        self.rewrite_urls
    }

    fn preview_chars(&self) -> usize {
        self.preview_chars
    }

    fn include_source_text(&self) -> bool {
        self.include_source_text
    }
}

impl ExtractOutputConflictArgs for SliceExtractOutputArgs {
    fn value_is_non_default(&self) -> bool {
        self.value != CliSliceValueMode::Text
    }

    fn has_attribute(&self) -> bool {
        self.attribute.is_some()
    }

    fn whitespace(&self) -> CliWhitespaceMode {
        self.whitespace
    }

    fn rewrite_urls(&self) -> bool {
        self.rewrite_urls
    }

    fn preview_chars(&self) -> usize {
        self.preview_chars
    }

    fn include_source_text(&self) -> bool {
        self.include_source_text
    }
}

fn extend_extract_request_file_conflicts<T>(conflicts: &mut Vec<&'static str>, output: &T)
where
    T: ExtractOutputConflictArgs,
{
    push_conflict(conflicts, output.value_is_non_default(), "--value");
    push_conflict(conflicts, output.has_attribute(), "--attribute");
    push_conflict(
        conflicts,
        output.whitespace() != CliWhitespaceMode::Rendered,
        "--whitespace",
    );
    push_conflict(conflicts, output.rewrite_urls(), "--rewrite-urls");
    push_conflict(
        conflicts,
        output.preview_chars() != DEFAULT_PREVIEW_CHARS,
        "--preview-chars",
    );
    push_conflict(
        conflicts,
        output.include_source_text(),
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
