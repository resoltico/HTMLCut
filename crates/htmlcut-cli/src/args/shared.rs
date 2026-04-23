use std::path::PathBuf;

use clap::Args;
use htmlcut_core::{DEFAULT_FETCH_TIMEOUT_MS, DEFAULT_MAX_BYTES, DEFAULT_PREVIEW_CHARS};

use super::{
    CliFetchPreflightMode, CliInspectOutputMode, CliMatchMode, CliOutputMode, CliValueMode,
    CliWhitespaceMode, TEXT_JSON_OUTPUT_MODES, cli_choice_parser, cli_choice_subset_parser,
};

#[derive(Debug, Args)]
pub(crate) struct GlobalArgs {
    /// Emit progress and warning lines to stderr. Repeat for more detail.
    #[arg(short, long, global = true, action = clap::ArgAction::Count, conflicts_with = "quiet")]
    pub(crate) verbose: u8,

    /// Suppress non-fatal stderr diagnostics and verbose progress output.
    #[arg(short, long, global = true, action = clap::ArgAction::SetTrue, conflicts_with = "verbose")]
    pub(crate) quiet: bool,

    /// Print the HTMLCut identity banner, engine identity, schema profile, and repository.
    #[arg(short = 'V', long, action = clap::ArgAction::SetTrue)]
    pub(crate) version: bool,
}

#[derive(Debug, Args)]
#[command(next_help_heading = "Definition")]
pub(crate) struct DefinitionArgs {
    /// Load a reusable extraction definition from a JSON file that matches HTMLCut's extraction-definition schema.
    #[arg(long, value_name = "PATH")]
    pub(crate) request_file: Option<PathBuf>,

    /// Write the normalized extraction definition used for this run to a JSON file.
    #[arg(long, value_name = "PATH")]
    pub(crate) emit_request_file: Option<PathBuf>,
}

#[derive(Debug, Args)]
#[command(next_help_heading = "Source")]
pub(crate) struct SourceArgs {
    /// HTML input source: a local file path, an http(s) URL, or `-` for stdin.
    #[arg(value_name = "INPUT")]
    pub(crate) input: Option<String>,

    /// Override the input base URL used for relative-link resolution.
    ///
    /// When the document contains `<base href>`, that value is resolved against this URL and
    /// becomes the effective base for `--rewrite-urls`.
    #[arg(short = 'b', long, value_name = "URL")]
    pub(crate) base_url: Option<String>,

    /// Refuse sources larger than this limit. Accepts raw bytes or `KB`, `MB`, and `GB`.
    #[arg(long, default_value_t = DEFAULT_MAX_BYTES.to_string(), value_name = "SIZE")]
    pub(crate) max_bytes: String,

    /// HTTP fetch timeout in milliseconds for URL inputs.
    #[arg(long, default_value_t = DEFAULT_FETCH_TIMEOUT_MS, value_name = "MILLISECONDS")]
    pub(crate) fetch_timeout_ms: u64,

    /// Probe remote URLs with HEAD before GET, automatically falling back when HEAD is rejected
    /// or broken, or skip the HEAD preflight entirely.
    #[arg(long, value_parser = cli_choice_parser::<CliFetchPreflightMode>(), default_value_t = CliFetchPreflightMode::HeadFirst)]
    pub(crate) fetch_preflight: CliFetchPreflightMode,
}

#[derive(Debug, Args)]
#[command(next_help_heading = "Selection")]
pub(crate) struct SelectionArgs {
    /// Require exactly one match, keep the first match, keep one 1-based match, or keep every
    /// match.
    #[arg(long, value_parser = cli_choice_parser::<CliMatchMode>(), default_value_t = CliMatchMode::First)]
    pub(crate) r#match: CliMatchMode,

    /// The 1-based match index when `--match nth` is used.
    #[arg(short = 'n', long)]
    pub(crate) index: Option<usize>,
}

#[derive(Debug, Args)]
#[command(next_help_heading = "Extraction")]
pub(crate) struct ExtractOutputArgs {
    /// What each selected match should produce before stdout formatting is applied.
    #[arg(long, value_parser = cli_choice_parser::<CliValueMode>(), default_value_t = CliValueMode::Text)]
    pub(crate) value: CliValueMode,

    /// Attribute name to extract when `--value attribute` is used.
    #[arg(long)]
    pub(crate) attribute: Option<String>,

    /// Preserve source whitespace or normalize it for text-like values.
    #[arg(long, value_parser = cli_choice_parser::<CliWhitespaceMode>(), default_value_t = CliWhitespaceMode::Preserve)]
    pub(crate) whitespace: CliWhitespaceMode,

    /// Rewrite relative URLs in extracted HTML and attributes with the effective base URL.
    #[arg(long, default_value_t = false)]
    pub(crate) rewrite_urls: bool,

    /// How stdout should be rendered after extraction.
    #[arg(long, value_parser = cli_choice_parser::<CliOutputMode>())]
    pub(crate) output: Option<CliOutputMode>,

    /// Write `report.json`, `selection.html`, and `selection.txt` to this directory.
    #[arg(long)]
    pub(crate) bundle: Option<PathBuf>,

    /// Write the stdout payload to exactly one file instead of stdout.
    #[arg(long, value_name = "PATH")]
    pub(crate) output_file: Option<PathBuf>,

    /// Maximum preview length stored in structured reports.
    #[arg(long, default_value_t = DEFAULT_PREVIEW_CHARS)]
    pub(crate) preview_chars: usize,

    /// Include the full source text inside structured reports and bundles.
    #[arg(long, default_value_t = false)]
    pub(crate) include_source_text: bool,
}

#[derive(Debug, Args)]
#[command(next_help_heading = "Inspection Output")]
pub(crate) struct InspectOutputArgs {
    /// Render the inspection as compact text or structured JSON.
    #[arg(long, value_parser = cli_choice_subset_parser(TEXT_JSON_OUTPUT_MODES), default_value_t = CliInspectOutputMode::Json)]
    pub(crate) output: CliInspectOutputMode,

    /// Maximum preview length stored in structured preview reports.
    #[arg(long, default_value_t = DEFAULT_PREVIEW_CHARS)]
    pub(crate) preview_chars: usize,

    /// Include the full source text inside structured inspection reports.
    #[arg(long, default_value_t = false)]
    pub(crate) include_source_text: bool,

    /// Write the stdout payload to exactly one file instead of stdout.
    #[arg(long, value_name = "PATH")]
    pub(crate) output_file: Option<PathBuf>,
}
