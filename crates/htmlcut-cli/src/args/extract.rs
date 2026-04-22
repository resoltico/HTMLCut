use clap::Args;

use super::{
    CliPatternMode, DefinitionArgs, ExtractOutputArgs, SelectionArgs, SourceArgs, cli_choice_parser,
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

    /// Regex flags for `--pattern regex`. Accepts `i`, `m`, `s`, `U`, `u`, and `x`; `g` is accepted for compatibility and ignored.
    #[arg(long)]
    pub(crate) regex_flags: Option<String>,

    /// Include the matched `--from` boundary in the selected fragment.
    #[arg(long, default_value_t = false)]
    pub(crate) include_start: bool,

    /// Include the matched `--to` boundary in the selected fragment.
    #[arg(long, default_value_t = false)]
    pub(crate) include_end: bool,

    #[command(flatten)]
    pub(crate) selection: SelectionArgs,

    #[command(flatten)]
    pub(crate) output: ExtractOutputArgs,
}
