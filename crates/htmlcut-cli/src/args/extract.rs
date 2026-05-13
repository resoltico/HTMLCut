use clap::Args;

use super::{
    CliBoundaryRetentionMode, CliPatternMode, DefinitionArgs, ExtractOutputArgs, FileWriteArgs,
    SelectionArgs, SliceExtractOutputArgs, SourceArgs, cli_choice_parser,
};

#[derive(Debug, Args)]
pub(crate) struct SelectArgs {
    #[command(flatten)]
    pub(crate) definition: DefinitionArgs,

    #[command(flatten)]
    pub(crate) source: SourceArgs,

    /// CSS selector that chooses the candidate nodes to extract.
    #[arg(long = "css", required_unless_present = "request_file")]
    pub(crate) css: Option<String>,

    #[command(flatten)]
    pub(crate) selection: SelectionArgs,

    #[command(flatten)]
    pub(crate) output: ExtractOutputArgs,

    #[command(flatten)]
    pub(crate) file_write: FileWriteArgs,
}

#[derive(Debug, Args)]
pub(crate) struct SliceArgs {
    #[command(flatten)]
    pub(crate) definition: DefinitionArgs,

    #[command(flatten)]
    pub(crate) source: SourceArgs,

    /// Start boundary used to locate each candidate slice.
    #[arg(long, required_unless_present = "request_file")]
    pub(crate) from: Option<String>,

    /// End boundary used to locate each candidate slice.
    #[arg(long, required_unless_present = "request_file")]
    pub(crate) to: Option<String>,

    /// Interpret `--from` and `--to` as literal text or regex patterns.
    #[arg(long, value_parser = cli_choice_parser::<CliPatternMode>(), default_value_t = CliPatternMode::Literal)]
    pub(crate) pattern: CliPatternMode,

    /// Regex flags for `--pattern regex`. Accepts `i`, `m`, `s`, `U`, and `x`.
    #[arg(long)]
    pub(crate) regex_flags: Option<String>,

    /// Which matched boundaries become part of the selected fragment.
    #[arg(long, value_parser = cli_choice_parser::<CliBoundaryRetentionMode>(), default_value_t = CliBoundaryRetentionMode::ExcludeBoth)]
    pub(crate) boundary_retention: CliBoundaryRetentionMode,

    #[command(flatten)]
    pub(crate) selection: SelectionArgs,

    #[command(flatten)]
    pub(crate) output: SliceExtractOutputArgs,

    #[command(flatten)]
    pub(crate) file_write: FileWriteArgs,
}
