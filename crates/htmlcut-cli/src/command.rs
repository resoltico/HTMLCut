//! Private argument grammar for the single binary product.

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};
use htmlcut_core::{Boundary, ExtractionPlan, Projection, Selection, Strategy, ValueProjection};

use crate::input::{MAX_CONFIG_BYTES, options, read_file};

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
    /// Recompute and verify a self-contained snapshot bundle.
    #[command(about = crate::operation_metadata::RUN_ABOUT)]
    Run(Run),
    /// Count an explicit selector and return bounded samples.
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
}

#[derive(Args)]
pub(crate) struct SourceOptions {
    #[command(flatten)]
    pub(crate) source: SourceChoice,
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
    /// Atomically publish the complete execution receipt before data delivery.
    #[arg(long)]
    pub(crate) receipt: Option<PathBuf>,
}

#[derive(Clone, Copy, ValueEnum)]
#[value(rename_all = "snake_case")]
pub(crate) enum ProjectionArg {
    DomText,
    Markdown,
    InnerHtml,
    OuterHtml,
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
    #[arg(conflicts_with = "receipt")]
    pub(crate) bundle: Option<PathBuf>,
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
        plan.projection = if let Some(name) = &self.attribute {
            if self.projection.is_some() || self.start.is_some() {
                return Err(options(
                    "Attribute selection conflicts with an explicit projection or source boundaries.",
                ));
            }
            Projection::Value(ValueProjection::Attribute { name: name.clone() })
        } else {
            match self.projection.unwrap_or(if self.start.is_some() {
                ProjectionArg::Source
            } else {
                ProjectionArg::DomText
            }) {
                ProjectionArg::DomText => Projection::Value(ValueProjection::DomText {}),
                ProjectionArg::Markdown => Projection::Value(ValueProjection::Markdown {}),
                ProjectionArg::InnerHtml => Projection::Value(ValueProjection::InnerHtml {}),
                ProjectionArg::OuterHtml => Projection::Value(ValueProjection::OuterHtml {}),
                ProjectionArg::Source => Projection::Source {},
            }
        };
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
    /// Selector to count and sample; never guessed automatically.
    #[arg(long)]
    pub(crate) css: String,
    /// Number of samples; complete matching is still required.
    #[arg(long, default_value = "3")]
    pub(crate) samples: u32,
}
