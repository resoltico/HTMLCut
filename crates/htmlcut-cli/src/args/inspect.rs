use std::path::PathBuf;

use clap::{Args, Subcommand};
use htmlcut_core::{DEFAULT_INSPECTION_SAMPLE_LIMIT, DEFAULT_PREVIEW_CHARS};

use crate::help::{
    inspect_long_about, inspect_select_about, inspect_select_after_help, inspect_select_long_about,
    inspect_slice_about, inspect_slice_after_help, inspect_slice_long_about, inspect_source_about,
    inspect_source_after_help, inspect_source_long_about,
};

use super::{
    CliInspectOutputMode, CliPatternMode, CliWhitespaceMode, DefinitionArgs, InspectOutputArgs,
    SelectionArgs, SourceArgs, cli_choice_parser,
};

#[derive(Debug, Args)]
#[command(long_about = inspect_long_about())]
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
        after_help = inspect_source_after_help()
    )]
    Source(InspectSourceArgs),
    #[command(
        name = "select",
        about = inspect_select_about(),
        long_about = inspect_select_long_about(),
        after_help = inspect_select_after_help()
    )]
    Select(InspectSelectArgs),
    #[command(
        name = "slice",
        about = inspect_slice_about(),
        long_about = inspect_slice_long_about(),
        after_help = inspect_slice_after_help()
    )]
    Slice(InspectSliceArgs),
}

#[derive(Debug, Args)]
pub(crate) struct InspectSourceArgs {
    #[command(flatten)]
    pub(crate) source: SourceArgs,

    /// Maximum number of headings, links, tags, and classes to sample in the summary.
    #[arg(long, default_value_t = DEFAULT_INSPECTION_SAMPLE_LIMIT)]
    pub(crate) sample_limit: usize,

    /// Render the inspection as compact text or structured JSON.
    #[arg(long, value_parser = cli_choice_parser::<CliInspectOutputMode>(), default_value_t = CliInspectOutputMode::Json)]
    pub(crate) output: CliInspectOutputMode,

    /// Include the full source text in JSON output and a bounded preview in text output.
    #[arg(long, default_value_t = false)]
    pub(crate) include_source_text: bool,

    /// Maximum length of the source preview shown in text mode when `--include-source-text` is used.
    #[arg(long, default_value_t = DEFAULT_PREVIEW_CHARS)]
    pub(crate) preview_chars: usize,

    /// Write the stdout payload to exactly one file instead of stdout.
    #[arg(long, value_name = "PATH")]
    pub(crate) output_file: Option<PathBuf>,
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

    /// Preserve source whitespace or normalize preview text.
    #[arg(long, value_parser = cli_choice_parser::<CliWhitespaceMode>(), default_value_t = CliWhitespaceMode::Preserve)]
    pub(crate) whitespace: CliWhitespaceMode,

    /// Rewrite relative URLs in preview HTML and attribute data with the effective base URL.
    #[arg(long, default_value_t = false)]
    pub(crate) rewrite_urls: bool,

    #[command(flatten)]
    pub(crate) output: InspectOutputArgs,
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

    /// Include the matched `--from` boundary in the preview fragment.
    #[arg(long, default_value_t = false)]
    pub(crate) include_start: bool,

    /// Include the matched `--to` boundary in the preview fragment.
    #[arg(long, default_value_t = false)]
    pub(crate) include_end: bool,

    #[command(flatten)]
    pub(crate) selection: SelectionArgs,

    /// Preserve source whitespace or normalize preview text.
    #[arg(long, value_parser = cli_choice_parser::<CliWhitespaceMode>(), default_value_t = CliWhitespaceMode::Preserve)]
    pub(crate) whitespace: CliWhitespaceMode,

    /// Rewrite relative URLs in preview HTML and attribute data with the effective base URL.
    #[arg(long, default_value_t = false)]
    pub(crate) rewrite_urls: bool,

    #[command(flatten)]
    pub(crate) output: InspectOutputArgs,
}
