use std::path::PathBuf;

use clap::{Args, Subcommand};
use htmlcut_core::{DEFAULT_INSPECTION_SAMPLE_LIMIT, DEFAULT_PREVIEW_CHARS};

use crate::help::{
    inspect_select_about, inspect_select_after_help, inspect_select_long_about,
    inspect_slice_about, inspect_slice_after_help, inspect_slice_long_about, inspect_source_about,
    inspect_source_after_help, inspect_source_long_about,
};

use super::{
    CliBoundaryRetentionMode, CliInspectOutputMode, CliPatternMode, CliSliceValueMode,
    CliValueMode, CliWhitespaceMode, DefinitionArgs, InspectOutputArgs, OutputFileWriteArgs,
    PreviewFileWriteArgs, SelectionArgs, SourceArgs, cli_choice_parser,
};

#[derive(Debug, Args)]
pub(crate) struct InspectArgs {
    #[command(subcommand)]
    pub(crate) command: InspectCommands,
}

#[derive(Debug, Subcommand)]
pub(crate) enum InspectCommands {
    #[command(
        name = "source",
        about = inspect_source_about(),
        long_about = inspect_source_long_about(),
        after_long_help = inspect_source_after_help()
    )]
    Source(InspectSourceArgs),
    #[command(
        name = "select",
        about = inspect_select_about(),
        long_about = inspect_select_long_about(),
        after_long_help = inspect_select_after_help()
    )]
    Select(InspectSelectArgs),
    #[command(
        name = "slice",
        about = inspect_slice_about(),
        long_about = inspect_slice_long_about(),
        after_long_help = inspect_slice_after_help()
    )]
    Slice(InspectSliceArgs),
    #[command(
        name = "elements",
        about = "Explore bounded static document elements and selector proposals."
    )]
    Elements(InspectElementsArgs),
    #[command(
        name = "propose",
        about = "Resolve one complete target hint and emit same-snapshot selector proposals."
    )]
    Propose(InspectProposeArgs),
}

#[derive(Debug, Args)]
pub(crate) struct InspectElementsArgs {
    #[command(flatten)]
    pub(crate) source: SourceArgs,

    /// Exact JSON next_cursor object from a prior page of the same source and options.
    #[arg(long, value_name = "JSON")]
    pub(crate) cursor: Option<String>,

    /// Maximum element descriptors returned in one deterministic page.
    #[arg(long, default_value_t = 100)]
    pub(crate) max_elements: u32,

    /// Shared traversal and selector-proof work budget for the page.
    #[arg(long, default_value_t = 100_000)]
    pub(crate) max_work_units: u32,

    /// Maximum fully proved selector proposals returned for one element.
    #[arg(long, default_value_t = 4)]
    pub(crate) max_proposals_per_element: u32,

    /// Maximum UTF-8 bytes retained in one text preview.
    #[arg(long, default_value_t = 256)]
    pub(crate) preview_bytes: u32,

    /// Write the exploration-result JSON to one file instead of stdout.
    #[arg(long, value_name = "PATH")]
    pub(crate) output_file: Option<PathBuf>,

    #[command(flatten)]
    pub(crate) file_write: OutputFileWriteArgs,
}

#[derive(Debug, Args)]
pub(crate) struct InspectProposeArgs {
    #[command(flatten)]
    pub(crate) source: SourceArgs,

    /// Complete `htmlcut.dom_path.v1` path for the browser target.
    #[arg(long)]
    pub(crate) path: String,

    /// Target namespace: html, svg, or mathml.
    #[arg(long)]
    pub(crate) namespace: String,

    /// Parser-preserved target local name.
    #[arg(long)]
    pub(crate) local_name: String,

    /// Lowercase SHA-256 of `htmlcut.dom_text.v1\0` plus UTF-8 descendant DOM text after CRLF and CR normalization.
    #[arg(long)]
    pub(crate) text_digest_sha256: String,

    /// Optional exact semantic attribute in NAME=VALUE form. Repeat to require several attributes.
    #[arg(long = "attribute")]
    pub(crate) attributes: Vec<String>,

    /// Shared path, fingerprint, and selector-proof work budget.
    #[arg(long, default_value_t = 100_000)]
    pub(crate) max_work_units: u32,

    /// Maximum fully proved selector proposals returned for the target.
    #[arg(long, default_value_t = 4)]
    pub(crate) max_proposals: u32,

    /// Write the target-resolution-result JSON to one file instead of stdout.
    #[arg(long, value_name = "PATH")]
    pub(crate) output_file: Option<PathBuf>,

    #[command(flatten)]
    pub(crate) file_write: OutputFileWriteArgs,
}

#[derive(Debug, Args)]
pub(crate) struct InspectSourceArgs {
    #[command(flatten)]
    pub(crate) source: SourceArgs,

    /// Maximum number of content candidates, headings, links, tags, and classes to sample in the summary.
    #[arg(long, default_value_t = DEFAULT_INSPECTION_SAMPLE_LIMIT)]
    pub(crate) sample_limit: usize,

    /// Render the inspection as compact text or structured JSON.
    ///
    /// JSON mode writes either a success or failure document to stdout, and the process exit status
    /// remains authoritative.
    #[arg(long, value_parser = cli_choice_parser::<CliInspectOutputMode>(), default_value_t = CliInspectOutputMode::Text)]
    pub(crate) output: CliInspectOutputMode,

    /// Include the full source text in JSON output and a bounded preview in text output.
    #[arg(long, default_value_t = false)]
    pub(crate) include_source_text: bool,

    /// Maximum length of the source preview shown in text mode when `--include-source-text` is used.
    #[arg(long, default_value_t = DEFAULT_PREVIEW_CHARS)]
    pub(crate) preview_chars: usize,

    /// Write the stdout payload to exactly one file instead of stdout.
    ///
    /// Parent directories are created automatically. Existing files require `--overwrite`.
    #[arg(long, value_name = "PATH")]
    pub(crate) output_file: Option<PathBuf>,

    #[command(flatten)]
    pub(crate) file_write: OutputFileWriteArgs,
}

#[derive(Debug, Args)]
pub(crate) struct InspectSelectArgs {
    #[command(flatten)]
    pub(crate) definition: DefinitionArgs,

    #[command(flatten)]
    pub(crate) source: SourceArgs,

    /// CSS selector that chooses the candidate nodes to preview.
    #[arg(long = "css", required_unless_present = "request_file")]
    pub(crate) css: Option<String>,

    #[command(flatten)]
    pub(crate) selection: SelectionArgs,

    /// What each previewed match should produce before the preview report is rendered.
    #[arg(long, value_parser = cli_choice_parser::<CliValueMode>(), default_value_t = CliValueMode::Structured)]
    pub(crate) value: CliValueMode,

    /// Attribute name to preview when `--value attribute` is used.
    #[arg(long)]
    pub(crate) attribute: Option<String>,

    /// Preserve semantic rendered layout, or normalize whitespace within that HTML-aware rendering.
    ///
    /// Normalization does not flatten headings, lists, tables, or link annotations.
    #[arg(long, value_parser = cli_choice_parser::<CliWhitespaceMode>(), default_value_t = CliWhitespaceMode::Rendered)]
    pub(crate) whitespace: CliWhitespaceMode,

    /// Rewrite relative URLs in preview HTML and attribute data with the effective base URL.
    #[arg(long, default_value_t = false)]
    pub(crate) rewrite_urls: bool,

    #[command(flatten)]
    pub(crate) output: InspectOutputArgs,

    #[command(flatten)]
    pub(crate) file_write: PreviewFileWriteArgs,
}

#[derive(Debug, Args)]
pub(crate) struct InspectSliceArgs {
    #[command(flatten)]
    pub(crate) definition: DefinitionArgs,

    #[command(flatten)]
    pub(crate) source: SourceArgs,

    /// Start boundary used to locate each candidate slice preview.
    #[arg(long, required_unless_present = "request_file")]
    pub(crate) from: Option<String>,

    /// End boundary used to locate each candidate slice preview.
    #[arg(long, required_unless_present = "request_file")]
    pub(crate) to: Option<String>,

    /// Interpret `--from` and `--to` as literal text or regex patterns.
    #[arg(long, value_parser = cli_choice_parser::<CliPatternMode>(), default_value_t = CliPatternMode::Literal)]
    pub(crate) pattern: CliPatternMode,

    /// Regex flags for `--pattern regex`. Accepts `i`, `m`, `s`, `U`, and `x`.
    #[arg(long)]
    pub(crate) regex_flags: Option<String>,

    /// Which matched boundaries become part of the preview fragment.
    #[arg(long, value_parser = cli_choice_parser::<CliBoundaryRetentionMode>(), default_value_t = CliBoundaryRetentionMode::ExcludeBoth)]
    pub(crate) boundary_retention: CliBoundaryRetentionMode,

    #[command(flatten)]
    pub(crate) selection: SelectionArgs,

    /// What each previewed slice should produce before the preview report is rendered.
    #[arg(long, value_parser = cli_choice_parser::<CliSliceValueMode>(), default_value_t = CliSliceValueMode::Structured)]
    pub(crate) value: CliSliceValueMode,

    /// Attribute name to preview when `--value attribute` is used.
    #[arg(long)]
    pub(crate) attribute: Option<String>,

    /// Preserve semantic rendered layout, or normalize whitespace within that HTML-aware rendering.
    ///
    /// Normalization does not flatten headings, lists, tables, or link annotations.
    #[arg(long, value_parser = cli_choice_parser::<CliWhitespaceMode>(), default_value_t = CliWhitespaceMode::Rendered)]
    pub(crate) whitespace: CliWhitespaceMode,

    /// Rewrite relative URLs in preview HTML and attribute data with the effective base URL.
    #[arg(long, default_value_t = false)]
    pub(crate) rewrite_urls: bool,

    #[command(flatten)]
    pub(crate) output: InspectOutputArgs,

    #[command(flatten)]
    pub(crate) file_write: PreviewFileWriteArgs,
}
