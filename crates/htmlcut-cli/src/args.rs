use std::ffi::OsStr;
use std::marker::PhantomData;
use std::path::PathBuf;

use clap::builder::{PossibleValue, TypedValueParser};
use clap::{Arg, ArgAction, Args, Command, Parser, Subcommand, error::ErrorKind};
use htmlcut_core::{
    CliChoice, DEFAULT_FETCH_TIMEOUT_MS, DEFAULT_INSPECTION_SAMPLE_LIMIT, DEFAULT_MAX_BYTES,
    DEFAULT_PREVIEW_CHARS,
};

use crate::help::{
    catalog_about, catalog_after_help, catalog_long_about, inspect_about, inspect_long_about,
    inspect_select_about, inspect_select_after_help, inspect_select_long_about,
    inspect_slice_about, inspect_slice_after_help, inspect_slice_long_about, inspect_source_about,
    inspect_source_after_help, inspect_source_long_about, root_after_help, root_long_about,
    schema_about, schema_after_help, schema_long_about, select_about, select_after_help,
    select_long_about, slice_about, slice_after_help, slice_long_about,
};
use crate::metadata::{HTMLCUT_DESCRIPTION, TOOL_NAME};

pub(crate) type CliPatternMode = htmlcut_core::PatternMode;
pub(crate) type CliMatchMode = htmlcut_core::CliSelectionMode;
pub(crate) type CliValueMode = htmlcut_core::ValueType;
pub(crate) type CliOutputMode = htmlcut_core::CliOutputMode;
pub(crate) type CliInspectOutputMode = htmlcut_core::CliOutputMode;
pub(crate) type CliCatalogOutputMode = htmlcut_core::CliOutputMode;
pub(crate) type CliSchemaOutputMode = htmlcut_core::CliOutputMode;
pub(crate) type CliWhitespaceMode = htmlcut_core::WhitespaceMode;
pub(crate) type CliFetchPreflightMode = htmlcut_core::FetchPreflightMode;

pub(crate) const TEXT_JSON_OUTPUT_MODES: &[CliOutputMode] =
    &[CliOutputMode::Text, CliOutputMode::Json];

#[derive(Clone, Copy, Debug)]
pub(crate) struct CliChoiceParser<T: 'static> {
    allowed: &'static [T],
    _marker: PhantomData<T>,
}

pub(crate) fn cli_choice_parser<T>() -> CliChoiceParser<T>
where
    T: CliChoice,
{
    cli_choice_subset_parser(T::variants())
}

pub(crate) fn cli_choice_subset_parser<T>(allowed: &'static [T]) -> CliChoiceParser<T>
where
    T: CliChoice,
{
    CliChoiceParser {
        allowed,
        _marker: PhantomData,
    }
}

impl<T> TypedValueParser for CliChoiceParser<T>
where
    T: CliChoice + Send + Sync,
{
    type Value = T;

    fn parse_ref(
        &self,
        _command: &Command,
        arg: Option<&Arg>,
        value: &OsStr,
    ) -> Result<Self::Value, clap::Error> {
        let Some(raw) = value.to_str() else {
            return Err(clap::Error::raw(
                ErrorKind::InvalidUtf8,
                "value is not valid UTF-8",
            ));
        };

        self.allowed
            .iter()
            .copied()
            .find(|candidate| candidate.as_cli_str() == raw)
            .ok_or_else(|| {
                let mut message = match arg {
                    Some(argument) => {
                        format!("invalid value '{raw}' for {}", argument.get_id().as_str())
                    }
                    None => format!("invalid value '{raw}'"),
                };
                let choices = self
                    .allowed
                    .iter()
                    .map(|candidate| candidate.as_cli_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                message.push_str(&format!(" [possible values: {choices}]"));
                clap::Error::raw(ErrorKind::InvalidValue, message)
            })
    }

    fn possible_values(&self) -> Option<Box<dyn Iterator<Item = PossibleValue> + '_>> {
        Some(Box::new(
            self.allowed
                .iter()
                .map(|candidate| PossibleValue::new(candidate.as_cli_str())),
        ))
    }
}

#[derive(Debug, Parser)]
#[command(
    name = TOOL_NAME,
    about = HTMLCUT_DESCRIPTION,
    long_about = root_long_about(),
    after_help = root_after_help(),
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
    #[command(about = catalog_about(), long_about = catalog_long_about(), after_help = catalog_after_help())]
    Catalog(CatalogArgs),
    #[command(about = schema_about(), long_about = schema_long_about(), after_help = schema_after_help())]
    Schema(SchemaArgs),
    #[command(about = select_about(), long_about = select_long_about(), after_help = select_after_help())]
    Select(SelectArgs),
    #[command(about = slice_about(), long_about = slice_long_about(), after_help = slice_after_help())]
    Slice(SliceArgs),
    #[command(about = inspect_about(), long_about = inspect_long_about())]
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

#[derive(Debug, Args)]
#[command(long_about = inspect_long_about())]
pub(crate) struct InspectArgs {
    #[command(subcommand)]
    pub(crate) command: InspectCommands,
}

#[derive(Debug, Args)]
pub(crate) struct CatalogArgs {
    /// Render the catalog as detailed text or structured JSON.
    #[arg(long, value_parser = cli_choice_subset_parser(TEXT_JSON_OUTPUT_MODES), default_value_t = CliCatalogOutputMode::Text)]
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
    #[arg(long, value_parser = cli_choice_subset_parser(TEXT_JSON_OUTPUT_MODES), default_value_t = CliSchemaOutputMode::Text)]
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
    #[arg(long, value_parser = cli_choice_subset_parser(TEXT_JSON_OUTPUT_MODES), default_value_t = CliInspectOutputMode::Json)]
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
    #[arg(long, value_parser = cli_choice_parser::<CliWhitespaceMode>(), default_value_t = CliWhitespaceMode::Preserve)]
    pub(crate) whitespace: CliWhitespaceMode,

    /// Rewrite relative URLs in preview HTML and attribute data with the effective base URL.
    #[arg(long, default_value_t = false)]
    pub(crate) rewrite_urls: bool,

    #[command(flatten)]
    pub(crate) output: InspectOutputArgs,
}
