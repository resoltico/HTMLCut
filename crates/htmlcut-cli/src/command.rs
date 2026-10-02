//! Private argument grammar for the single binary product.

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};
use htmlcut_core::{Boundary, ExtractionPlan, Projection, Selection, Strategy};

use crate::input::{MAX_CONFIG_BYTES, SourceSpec, options, read_file};

#[derive(Parser)]
#[command(
    name = "htmlcut",
    version,
    about = "Bounded, faithful extraction from immutable HTML snapshots."
)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) operation: Operation,
}

#[derive(Subcommand)]
pub(crate) enum Operation {
    /// Extract exactly the requested values; defaults to single/dom_text and compact JSON.
    #[command(about = crate::operation_metadata::EXTRACT_ABOUT, long_about = crate::operation_metadata::EXTRACT_DETAILS)]
    Extract(Box<Extract>),
    /// Execute a closed saved run with caller-owned source configuration.
    #[command(about = crate::operation_metadata::RUN_ABOUT)]
    Run(Run),
    /// Inspect a saved immutable snapshot with bounded descriptors and cursors.
    #[command(about = crate::operation_metadata::INSPECT_ABOUT)]
    Inspect(Inspect),
    /// Retrieve a compact index or one named operation description.
    #[command(about = crate::operation_metadata::DESCRIBE_ABOUT)]
    Describe { operation: Option<String> },
    /// Retrieve one named contract schema.
    #[command(about = crate::operation_metadata::SCHEMA_ABOUT)]
    Schema { name: String },
}

#[derive(Args)]
#[group(id = "source", required = true, multiple = false)]
pub(crate) struct SourceChoice {
    #[arg(long)]
    pub(crate) file: Option<PathBuf>,
    #[arg(long)]
    pub(crate) stdin: bool,
    #[arg(long)]
    pub(crate) url: Option<String>,
    #[arg(long)]
    pub(crate) url_env: Option<String>,
}

impl SourceChoice {
    pub(crate) fn specification(&self) -> Result<SourceSpec, htmlcut_core::ExtractionError> {
        if let Some(path) = &self.file {
            Ok(SourceSpec::File {
                path: path
                    .to_str()
                    .ok_or_else(|| options("Source file paths must be valid UTF-8."))?
                    .into(),
            })
        } else if self.stdin {
            Ok(SourceSpec::Stdin {})
        } else {
            Ok(SourceSpec::Http {
                url: self.url.clone(),
                url_env: self.url_env.clone(),
            })
        }
    }
}

#[derive(Args)]
pub(crate) struct SourceOptions {
    #[command(flatten)]
    pub(crate) source: SourceChoice,
    /// Explicit strict charset; file/stdin and unlabelled HTTP otherwise require UTF-8.
    #[arg(long)]
    pub(crate) encoding: Option<String>,
    /// Explicit effective base; HTML base elements are never inferred.
    #[arg(long)]
    pub(crate) base_url: Option<String>,
}

#[derive(Args)]
pub(crate) struct Output {
    /// Emit exactly one value without a framing LF.
    #[arg(long)]
    pub(crate) raw: bool,
    /// Atomically publish to a file instead of stdout.
    #[arg(long)]
    pub(crate) output: Option<PathBuf>,
    /// Permit atomic replacement of an existing managed output.
    #[arg(long)]
    pub(crate) overwrite: bool,
    /// Publish bounded field-selected evidence before the successful result.
    #[arg(long, requires = "audit_field")]
    pub(crate) audit: Option<PathBuf>,
    #[arg(long, value_delimiter = ',', requires = "audit")]
    pub(crate) audit_field: Vec<AuditField>,
}

#[derive(Clone, Copy, ValueEnum)]
#[value(rename_all = "snake_case")]
pub(crate) enum AuditField {
    Plan,
    SourceDigest,
    PlanDigest,
    ExtractionDigest,
    Counts,
    Ranges,
    Values,
}

#[derive(Clone, Copy, ValueEnum)]
#[value(rename_all = "snake_case")]
pub(crate) enum ProjectionArg {
    DomText,
    DocumentText,
    InnerHtml,
    OuterHtml,
    Attribute,
    Source,
}

#[derive(Clone, Copy, ValueEnum)]
pub(crate) enum MatchArg {
    Single,
    All,
    Nth,
}

#[derive(Args)]
pub(crate) struct Extract {
    #[command(flatten)]
    pub(crate) source: SourceOptions,
    #[command(flatten)]
    pub(crate) output: Output,
    #[arg(long, conflicts_with_all = ["css", "projection", "attribute", "match_mode", "index", "min", "max", "start", "end", "regex", "regex_flags", "include_start", "include_end"])]
    pub(crate) plan: Option<PathBuf>,
    #[arg(long, conflicts_with = "start")]
    pub(crate) css: Option<String>,
    #[arg(long)]
    pub(crate) projection: Option<ProjectionArg>,
    #[arg(long)]
    pub(crate) attribute: Option<String>,
    #[arg(long = "match", default_value = "single")]
    pub(crate) match_mode: MatchArg,
    #[arg(long)]
    pub(crate) index: Option<u32>,
    #[arg(long)]
    pub(crate) min: Option<u32>,
    #[arg(long)]
    pub(crate) max: Option<u32>,
    #[arg(long, requires = "end")]
    pub(crate) start: Option<String>,
    #[arg(long, requires = "start")]
    pub(crate) end: Option<String>,
    #[arg(long, requires = "start")]
    pub(crate) regex: bool,
    #[arg(long, requires = "regex")]
    pub(crate) regex_flags: Option<String>,
    #[arg(long, requires = "start")]
    pub(crate) include_start: bool,
    #[arg(long, requires = "start")]
    pub(crate) include_end: bool,
    #[arg(long)]
    pub(crate) save_run: Option<PathBuf>,
}

impl Extract {
    pub(crate) fn extraction_plan(&self) -> Result<ExtractionPlan, htmlcut_core::ExtractionError> {
        if let Some(path) = &self.plan {
            return ExtractionPlan::from_json(&read_file(path, MAX_CONFIG_BYTES)?);
        }
        let mut plan = ExtractionPlan::css(self.css.as_deref().unwrap_or("*"))?;
        if let (Some(start), Some(end)) = (&self.start, &self.end) {
            let boundary = |value: &String| {
                if self.regex {
                    Boundary::Regex {
                        pattern: value.clone(),
                        flags: self.regex_flags.clone().unwrap_or_default(),
                    }
                } else {
                    Boundary::Literal {
                        value: value.clone(),
                    }
                }
            };
            plan.strategy = Strategy::Slice {
                start: boundary(start),
                end: boundary(end),
                include_start: self.include_start,
                include_end: self.include_end,
            };
        } else if self.css.is_none() {
            return Err(options(
                "Provide a plan, CSS selector or complete source boundaries.",
            ));
        }
        plan.selection = match self.match_mode {
            MatchArg::Single
                if self.index.is_none() && self.min.is_none() && self.max.is_none() =>
            {
                Selection::Single {}
            }
            MatchArg::Nth if self.min.is_none() && self.max.is_none() => Selection::Nth {
                index: self
                    .index
                    .ok_or_else(|| options("Nth selection requires --index."))?,
            },
            MatchArg::All if self.index.is_none() => Selection::All {
                min: self.min.unwrap_or(1),
                max: self.max,
            },
            _ => {
                return Err(options(
                    "Selection options are incompatible with the requested match kind.",
                ));
            }
        };
        plan.projection = match self.projection.unwrap_or(if self.start.is_some() {
            ProjectionArg::Source
        } else {
            ProjectionArg::DomText
        }) {
            ProjectionArg::DomText => Projection::DomText {},
            ProjectionArg::DocumentText => Projection::DocumentText {},
            ProjectionArg::InnerHtml => Projection::InnerHtml {},
            ProjectionArg::OuterHtml => Projection::OuterHtml {},
            ProjectionArg::Source => Projection::Source {},
            ProjectionArg::Attribute => Projection::Attribute {
                name: self
                    .attribute
                    .clone()
                    .ok_or_else(|| options("Attribute projection requires --attribute."))?,
            },
        };
        if self.attribute.is_some() && !matches!(plan.projection, Projection::Attribute { .. }) {
            return Err(options("--attribute applies only to attribute projection."));
        }
        plan.validate()?;
        Ok(plan)
    }
}

#[derive(Args)]
pub(crate) struct Run {
    pub(crate) file: PathBuf,
    #[command(flatten)]
    pub(crate) output: Output,
}

#[derive(Args)]
pub(crate) struct Inspect {
    #[command(flatten)]
    pub(crate) source: SourceOptions,
    #[arg(long)]
    pub(crate) cursor: Option<String>,
    #[arg(long, default_value = "20")]
    pub(crate) page_size: u32,
    #[arg(long, conflicts_with_all = ["cursor", "page_size"])]
    pub(crate) preview_plan: Option<PathBuf>,
    /// Request a labelled positional selector suggestion; adoption remains caller-owned.
    #[arg(long, conflicts_with_all = ["cursor", "preview_plan"])]
    pub(crate) propose: Option<String>,
}
