// SPDX-License-Identifier: MPL-2.0
//! Private argument grammar for the single binary product.

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};
use htmlcut_core::{
    Boundary, ExtractionPlan, FieldSelection, Guard, GuardRead, GuardScope, Predicate, Projection,
    RecordField, Selection, Strategy, Transform, ValueProjection,
};

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
    /// Survey repeated HTML siblings without inferring field meaning.
    #[command(about = crate::operation_metadata::OUTLINE_ABOUT)]
    Outline(Outline),
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
    #[arg(long, conflicts_with_all = ["css", "read", "fields", "field_excludes", "exclude", "expect_text", "following_siblings", "match_mode", "index", "min", "max", "start", "end", "regex", "regex_flags", "include_start", "include_end"])]
    pub(crate) plan: Option<PathBuf>,
    #[arg(long, conflicts_with = "start")]
    pub(crate) css: Option<String>,
    /// dom_text, normalized_text, markdown, resolved_markdown, inner_html,
    /// outer_html, attribute:NAME, resolved_attribute:NAME or source.
    #[arg(long, conflicts_with = "fields")]
    pub(crate) read: Option<Reading>,
    /// Exclude matching descendants from a flat DOM reading, repeatable.
    #[arg(long, action = clap::ArgAction::Append, conflicts_with_all = ["start", "fields"])]
    pub(crate) exclude: Vec<String>,
    /// Require exactly one original-DOM match with exact text: CSS TEXT, repeatable.
    #[arg(long = "expect-text", num_args = 2, value_names = ["CSS", "TEXT"], action = clap::ArgAction::Append, conflicts_with = "start")]
    pub(crate) expect_text: Vec<String>,
    /// Named fields: NAME CSS READ; suffix NAME with ? for zero-or-one matches.
    #[arg(long = "field", num_args = 3, value_names = ["NAME", "CSS", "READ"], action = clap::ArgAction::Append, conflicts_with_all = ["start", "read"])]
    pub(crate) fields: Vec<String>,
    /// Exclude matching descendants from one declared field: NAME CSS, repeatable.
    #[arg(long = "field-exclude", num_args = 2, value_names = ["NAME", "CSS"], action = clap::ArgAction::Append, requires = "fields")]
    pub(crate) field_excludes: Vec<String>,
    /// Include exactly this many following element-sibling subtrees per record.
    #[arg(long, requires = "fields")]
    pub(crate) following_siblings: Option<u32>,
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
        plan.exclude = self.exclude.clone();
        plan.guards = self
            .expect_text
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| Guard {
                scope: GuardScope::Document,
                selector: pair[0].clone(),
                min: 1,
                max: Some(1),
                read: GuardRead::DomText {},
                predicate: Some(Predicate::Exact {
                    value: pair[1].clone(),
                }),
            })
            .collect();
        if self.fields.is_empty() {
            if let Some(reading) = &self.read {
                plan.projection = reading.projection.clone();
                plan.transforms = reading.transforms.clone();
            } else if self.start.is_some() {
                plan.projection = Projection::Source {};
            }
        } else {
            if self.fields.len() > 3 * htmlcut_core::MAX_RECORD_FIELDS {
                return Err(options("The field declaration count exceeds its limit."));
            }
            let mut fields = self
                .fields
                .as_chunks::<3>()
                .0
                .iter()
                .map(|field| {
                    let reading = Reading::parse(&field[2])?;
                    let Projection::Value(projection) = reading.projection else {
                        return Err(options("Record fields require a DOM reading."));
                    };
                    let (name, selection) = match field[0].strip_suffix('?') {
                        Some(name) => (name.to_owned(), FieldSelection::Optional {}),
                        None => (field[0].clone(), FieldSelection::Single {}),
                    };
                    Ok(RecordField {
                        name,
                        selector: field[1].clone(),
                        projection,
                        selection,
                        exclude: vec![],
                        transforms: reading.transforms,
                    })
                })
                .collect::<Result<Vec<_>, htmlcut_core::ExtractionError>>()?;
            for exclusion in self.field_excludes.as_chunks::<2>().0 {
                let field = fields
                    .iter_mut()
                    .find(|field| field.name == exclusion[0])
                    .ok_or_else(|| {
                        options("Field exclusion names must refer to declared fields.")
                    })?;
                field.exclude.push(exclusion[1].clone());
            }
            plan.projection = Projection::Records {
                fields,
                following_siblings: self.following_siblings.unwrap_or(0),
            };
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
    /// Selector to count and sample; never guessed automatically.
    #[arg(long)]
    pub(crate) css: String,
    /// Number of samples; complete matching is still required.
    #[arg(long, default_value = "3")]
    pub(crate) samples: u32,
    /// Show bounded id/class values and structural text for selector authoring.
    #[arg(long)]
    pub(crate) identifiers: bool,
}

#[derive(Args)]
pub(crate) struct Outline {
    #[command(flatten)]
    pub(crate) source: SourceOptions,
    /// Restrict the survey to exactly one selected original-DOM element.
    #[arg(long)]
    pub(crate) within: Option<String>,
    /// Largest groups to return; the complete group count is still established.
    #[arg(long, default_value = "4")]
    pub(crate) limit: u32,
}

#[derive(Clone)]
pub(crate) struct Reading {
    pub(crate) projection: Projection,
    pub(crate) transforms: Vec<Transform>,
}

impl std::str::FromStr for Reading {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value).map_err(|_| "Reading is outside the declared vocabulary.".into())
    }
}

impl Reading {
    pub(crate) fn parse(value: &str) -> Result<Self, htmlcut_core::ExtractionError> {
        let (projection, transform) = match value {
            "source" => {
                return Ok(Self {
                    projection: Projection::Source {},
                    transforms: vec![],
                });
            }
            "dom_text" => (ValueProjection::DomText {}, None),
            "normalized_text" => (
                ValueProjection::DomText {},
                Some(Transform::NormalizeWhitespace {}),
            ),
            "markdown" => (ValueProjection::Markdown {}, None),
            "resolved_markdown" => (
                ValueProjection::Markdown {},
                Some(Transform::ResolveUrls {}),
            ),
            "inner_html" => (ValueProjection::InnerHtml {}, None),
            "outer_html" => (ValueProjection::OuterHtml {}, None),
            _ => {
                if let Some(name) = value.strip_prefix("attribute:") {
                    (ValueProjection::Attribute { name: name.into() }, None)
                } else if let Some(name) = value.strip_prefix("resolved_attribute:") {
                    (
                        ValueProjection::Attribute { name: name.into() },
                        Some(Transform::ResolveUrls {}),
                    )
                } else {
                    return Err(options("Reading is outside the declared vocabulary."));
                }
            }
        };
        Ok(Self {
            projection: Projection::Value(projection),
            transforms: transform.into_iter().collect(),
        })
    }
}
