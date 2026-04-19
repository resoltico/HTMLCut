use std::fmt;
use std::path::PathBuf;

use clap::{ArgAction, Args, Parser, Subcommand, ValueEnum};
use htmlcut_core::{
    DEFAULT_FETCH_TIMEOUT_MS, DEFAULT_INSPECTION_SAMPLE_LIMIT, DEFAULT_MAX_BYTES,
    DEFAULT_PREVIEW_CHARS,
};

use crate::help::{
    CATALOG_LONG_ABOUT, INSPECT_LONG_ABOUT, INSPECT_SELECT_LONG_ABOUT, INSPECT_SLICE_LONG_ABOUT,
    INSPECT_SOURCE_LONG_ABOUT, ROOT_AFTER_HELP, ROOT_LONG_ABOUT, SCHEMA_LONG_ABOUT,
    SELECT_LONG_ABOUT, SLICE_LONG_ABOUT, catalog_after_help, inspect_select_after_help,
    inspect_slice_after_help, inspect_source_after_help, schema_after_help, select_after_help,
    slice_after_help,
};
use crate::metadata::{HTMLCUT_DESCRIPTION, TOOL_NAME};

#[derive(Clone, Copy, Debug, ValueEnum, PartialEq, Eq)]
pub(crate) enum CliPatternMode {
    Literal,
    Regex,
}

#[derive(Clone, Copy, Debug, ValueEnum, PartialEq, Eq)]
pub(crate) enum CliMatchMode {
    Single,
    First,
    Nth,
    All,
}

#[derive(Clone, Copy, Debug, ValueEnum, PartialEq, Eq)]
pub(crate) enum CliValueMode {
    Text,
    InnerHtml,
    OuterHtml,
    Attribute,
    Structured,
}

#[derive(Clone, Copy, Debug, ValueEnum, PartialEq, Eq)]
pub(crate) enum CliOutputMode {
    Text,
    Html,
    Json,
    None,
}

#[derive(Clone, Copy, Debug, ValueEnum, PartialEq, Eq)]
pub(crate) enum CliInspectOutputMode {
    Text,
    Json,
}

#[derive(Clone, Copy, Debug, ValueEnum, PartialEq, Eq)]
pub(crate) enum CliCatalogOutputMode {
    Text,
    Json,
}

#[derive(Clone, Copy, Debug, ValueEnum, PartialEq, Eq)]
pub(crate) enum CliSchemaOutputMode {
    Text,
    Json,
}

#[derive(Clone, Copy, Debug, ValueEnum, PartialEq, Eq)]
pub(crate) enum CliWhitespaceMode {
    Preserve,
    Normalize,
}

#[derive(Clone, Copy, Debug, ValueEnum, PartialEq, Eq)]
pub(crate) enum CliFetchPreflightMode {
    HeadFirst,
    GetOnly,
}

macro_rules! impl_value_enum_display {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl fmt::Display for $ty {
                fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                    let possible_value = self
                        .to_possible_value()
                        .expect("CLI value-enum variant should always render");
                    formatter.write_str(possible_value.get_name())
                }
            }
        )+
    };
}

impl_value_enum_display!(
    CliPatternMode,
    CliMatchMode,
    CliValueMode,
    CliOutputMode,
    CliInspectOutputMode,
    CliCatalogOutputMode,
    CliSchemaOutputMode,
    CliWhitespaceMode,
    CliFetchPreflightMode,
);

#[derive(Debug, Parser)]
#[command(
    name = TOOL_NAME,
    about = HTMLCUT_DESCRIPTION,
    long_about = ROOT_LONG_ABOUT,
    after_help = ROOT_AFTER_HELP,
    disable_help_subcommand = true,
    disable_version_flag = true,
    subcommand_required = true
)]
pub(crate) struct Cli {
    #[command(flatten)]
    pub(crate) global: GlobalArgs,

    #[command(subcommand)]
    pub(crate) command: Commands,
}

#[derive(Debug, Args)]
pub(crate) struct GlobalArgs {
    /// Emit progress and warning lines to stderr. Repeat for more detail.
    #[arg(short, long, global = true, action = ArgAction::Count, conflicts_with = "quiet")]
    pub(crate) verbose: u8,

    /// Suppress non-fatal stderr diagnostics and verbose progress output.
    #[arg(short, long, global = true, action = ArgAction::SetTrue, conflicts_with = "verbose")]
    pub(crate) quiet: bool,

    /// Print the canonical HTMLCut version, engine identity, schema profile, and support metadata.
    #[arg(short = 'V', long, global = true, action = ArgAction::SetTrue)]
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

#[derive(Debug, Subcommand)]
pub(crate) enum Commands {
    #[command(long_about = CATALOG_LONG_ABOUT, after_help = catalog_after_help())]
    Catalog(CatalogArgs),
    #[command(long_about = SCHEMA_LONG_ABOUT, after_help = schema_after_help())]
    Schema(SchemaArgs),
    #[command(long_about = SELECT_LONG_ABOUT, after_help = select_after_help())]
    Select(SelectArgs),
    #[command(long_about = SLICE_LONG_ABOUT, after_help = slice_after_help())]
    Slice(SliceArgs),
    #[command(long_about = INSPECT_LONG_ABOUT, visible_alias = "analyze")]
    Inspect(InspectArgs),
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
    #[arg(long, default_value_t = CliFetchPreflightMode::HeadFirst)]
    pub(crate) fetch_preflight: CliFetchPreflightMode,
}

#[derive(Debug, Args)]
#[command(next_help_heading = "Selection")]
pub(crate) struct SelectionArgs {
    /// Require exactly one match, keep the first match, keep one 1-based match, or keep every
    /// match.
    #[arg(long, default_value_t = CliMatchMode::First)]
    pub(crate) r#match: CliMatchMode,

    /// The 1-based match index when `--match nth` is used.
    #[arg(short = 'n', long)]
    pub(crate) index: Option<usize>,
}

#[derive(Debug, Args)]
#[command(next_help_heading = "Extraction")]
pub(crate) struct ExtractOutputArgs {
    /// What each selected match should produce before stdout formatting is applied.
    #[arg(long, default_value_t = CliValueMode::Text)]
    pub(crate) value: CliValueMode,

    /// Attribute name to extract when `--value attribute` is used.
    #[arg(long)]
    pub(crate) attribute: Option<String>,

    /// Preserve source whitespace or normalize it for text-like values.
    #[arg(long, default_value_t = CliWhitespaceMode::Preserve)]
    pub(crate) whitespace: CliWhitespaceMode,

    /// Rewrite relative URLs in extracted HTML and attributes with the effective base URL.
    #[arg(long, default_value_t = false)]
    pub(crate) rewrite_urls: bool,

    /// How stdout should be rendered after extraction.
    #[arg(long)]
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
    #[arg(long, default_value_t = CliInspectOutputMode::Json)]
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

#[derive(Debug, Args)]
#[command(long_about = INSPECT_LONG_ABOUT)]
pub(crate) struct InspectArgs {
    #[command(subcommand)]
    pub(crate) command: InspectCommands,
}

#[derive(Debug, Args)]
pub(crate) struct CatalogArgs {
    /// Render the catalog as detailed text or structured JSON.
    #[arg(long, default_value_t = CliCatalogOutputMode::Text)]
    pub(crate) output: CliCatalogOutputMode,

    /// Write the stdout payload to exactly one file instead of stdout.
    #[arg(long, value_name = "PATH")]
    pub(crate) output_file: Option<PathBuf>,

    /// Filter the catalog to one stable operation ID.
    #[arg(long, value_name = "OPERATION_ID")]
    pub(crate) operation: Option<String>,
}

#[derive(Debug, Args)]
pub(crate) struct SchemaArgs {
    /// Render the schema registry as compact text or structured JSON.
    #[arg(long, default_value_t = CliSchemaOutputMode::Text)]
    pub(crate) output: CliSchemaOutputMode,

    /// Write the stdout payload to exactly one file instead of stdout.
    #[arg(long, value_name = "PATH")]
    pub(crate) output_file: Option<PathBuf>,

    /// Filter the registry to one stable schema name.
    #[arg(long, value_name = "SCHEMA_NAME")]
    pub(crate) name: Option<String>,

    /// Filter the registry to one schema version. Requires `--name`.
    #[arg(long = "schema-version", value_name = "SCHEMA_VERSION")]
    pub(crate) schema_version: Option<u32>,
}

#[derive(Debug, Subcommand)]
pub(crate) enum InspectCommands {
    #[command(
        name = "source",
        long_about = INSPECT_SOURCE_LONG_ABOUT,
        after_help = inspect_source_after_help()
    )]
    Source(InspectSourceArgs),
    #[command(
        name = "select",
        long_about = INSPECT_SELECT_LONG_ABOUT,
        after_help = inspect_select_after_help()
    )]
    Select(InspectSelectArgs),
    #[command(
        name = "slice",
        long_about = INSPECT_SLICE_LONG_ABOUT,
        after_help = inspect_slice_after_help()
    )]
    Slice(InspectSliceArgs),
}

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
    #[arg(long, default_value_t = CliPatternMode::Literal)]
    pub(crate) pattern: CliPatternMode,

    /// Regex flags for `--pattern regex`. Accepts `i`, `m`, `s`, `u`, and `x`.
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

#[derive(Debug, Args)]
pub(crate) struct InspectSourceArgs {
    #[command(flatten)]
    pub(crate) source: SourceArgs,

    /// Maximum number of headings, links, tags, and classes to sample in the summary.
    #[arg(long, default_value_t = DEFAULT_INSPECTION_SAMPLE_LIMIT)]
    pub(crate) sample_limit: usize,

    /// Render the inspection as compact text or structured JSON.
    #[arg(long, default_value_t = CliInspectOutputMode::Json)]
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
    #[arg(long, default_value_t = CliWhitespaceMode::Preserve)]
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
    #[arg(long, default_value_t = CliPatternMode::Literal)]
    pub(crate) pattern: CliPatternMode,

    /// Regex flags for `--pattern regex`. Accepts `i`, `m`, `s`, `u`, and `x`.
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
    #[arg(long, default_value_t = CliWhitespaceMode::Preserve)]
    pub(crate) whitespace: CliWhitespaceMode,

    /// Rewrite relative URLs in preview HTML and attribute data with the effective base URL.
    #[arg(long, default_value_t = false)]
    pub(crate) rewrite_urls: bool,

    #[command(flatten)]
    pub(crate) output: InspectOutputArgs,
}
