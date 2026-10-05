// SPDX-License-Identifier: MPL-2.0
//! Native command options compiled through the closed core query contract.

use crate::input::{MAX_CONFIG_BYTES, options, read_file};
use clap::{Args, Parser, Subcommand};
use htmlcut_core::{ExtractionPlan, FieldMatch, Guard, GuardScope, Match, Reading, RecordField};
use std::{collections::BTreeMap, io::Read, path::PathBuf};

#[derive(Parser)]
#[command(
    name = "htmlcut",
    version,
    about = "Bounded queries over caller-owned UTF-8 HTML snapshots."
)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) operation: Operation,
}
#[derive(Subcommand)]
pub(crate) enum Operation {
    /// Select, check and read complete strings or related records.
    #[command(after_help = EXTRACT_HELP)]
    Extract(Box<Extract>),
    /// Recompute and verify a self-contained USTAR snapshot bundle.
    Replay(Replay),
    /// Survey repeated sibling/table groups, or count/sample an explicit selector.
    Inspect(Inspect),
    /// Retrieve one exhaustive named contract schema (optional for ordinary use).
    Schema { name: String },
}
const EXTRACT_HELP: &str = r#"Examples:
  htmlcut extract --stdin --select h1
  htmlcut extract --file books.html --select article.product_pod --all --field title 'h3 a' attr:title --field price .price_color text
  htmlcut extract --file books.html --select article.product_pod --nth 1 --read outer-html --raw
  htmlcut extract --file page.html --select 'script[type="application/ld+json"]' --read literal --raw
  htmlcut extract --file quotes.html --plan-json '{"version":6,"select":".quote","match":"all","fields":{"text":{"select":".text"},"author":{"select":".author","match":"optional"},"tags":{"select":".tag","match":"all","min":0}}}'

One requires one node; all defaults min=1; nth is one-based and counts every candidate.
Optional fields return null only for absence; many defaults min=1. Missing attributes fail.
Use JSON for min=0/bounded/nth fields. Names are ASCII identifiers, at most 64 bytes.
Text is static: structural boundaries, Unicode whitespace collapse, parsed pre retained,
inert payloads omitted, hidden/noscript retained. Literal/HTML are not byte-exact cutting.
Unknown fields: inspect --file FILE, then extract one row's outer-html as above.
Default data is bare canonical JSON plus LF; raw is one scalar without LF."#;

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
    /// Explicit HTTP(S) base metadata without userinfo; HTML base elements are not inferred.
    #[arg(long)]
    pub(crate) base_url: Option<String>,
}
#[derive(Args)]
pub(crate) struct Output {
    /// Emit exactly one scalar string without a framing LF.
    #[arg(long)]
    pub(crate) raw: bool,
    /// Atomically publish to a file instead of stdout.
    #[arg(long)]
    pub(crate) output: Option<PathBuf>,
    /// Permit atomic replacement of an existing output.
    #[arg(long)]
    pub(crate) overwrite: bool,
    /// Publish an optional receipt, using the execution's remaining work.
    #[arg(long)]
    pub(crate) receipt: Option<PathBuf>,
}
#[derive(Args)]
#[group(id = "query", required = true, multiple = false, args = ["select", "plan", "plan_json"])]
pub(crate) struct Extract {
    #[command(flatten)]
    pub(crate) source: SourceOptions,
    #[command(flatten)]
    pub(crate) output: Output,
    /// Current query JSON from a regular file; '-' intentionally uses query stdin.
    #[arg(long, conflicts_with_all = ["all", "nth", "min", "max", "read", "fields", "optional_fields", "many_fields", "field_excludes", "exclude", "expect_text", "following_siblings"])]
    pub(crate) plan: Option<PathBuf>,
    /// Current closed query JSON supplied directly.
    #[arg(long, conflicts_with_all = ["all", "nth", "min", "max", "read", "fields", "optional_fields", "many_fields", "field_excludes", "exclude", "expect_text", "following_siblings"])]
    pub(crate) plan_json: Option<String>,
    /// Required inline CSS selector.
    #[arg(long)]
    pub(crate) select: Option<String>,
    /// Select all candidates; minimum one unless explicitly changed.
    #[arg(long, conflicts_with = "nth")]
    pub(crate) all: bool,
    /// Select one positive one-based position; complete candidates are counted.
    #[arg(long, conflicts_with_all = ["min", "max"])]
    pub(crate) nth: Option<u32>,
    #[arg(long, requires = "all")]
    pub(crate) min: Option<u32>,
    #[arg(long, requires = "all")]
    pub(crate) max: Option<u32>,
    /// text (default), literal, markdown, resolved-markdown, inner-html, outer-html, attr:NAME, url:NAME.
    #[arg(long, conflicts_with_all = ["fields", "optional_fields", "many_fields"])]
    pub(crate) read: Option<Reading>,
    /// Descendant exclusion, repeatable; scalar mode only.
    #[arg(long, action = clap::ArgAction::Append, conflicts_with_all = ["fields", "optional_fields", "many_fields"])]
    pub(crate) exclude: Vec<String>,
    /// Exact structural text at exactly one document node: SELECT TEXT, repeatable.
    #[arg(long, num_args = 2, value_names = ["SELECT", "TEXT"], action = clap::ArgAction::Append)]
    pub(crate) expect_text: Vec<String>,
    /// Required field: NAME SELECT READ, repeatable.
    #[arg(long = "field", num_args = 3, value_names = ["NAME", "SELECT", "READ"], action = clap::ArgAction::Append)]
    pub(crate) fields: Vec<String>,
    /// Zero-or-one field: NAME SELECT READ, repeatable.
    #[arg(long = "optional-field", num_args = 3, value_names = ["NAME", "SELECT", "READ"], action = clap::ArgAction::Append)]
    pub(crate) optional_fields: Vec<String>,
    /// All/min=1 field: NAME SELECT READ, repeatable; use JSON for min=0.
    #[arg(long = "many-field", num_args = 3, value_names = ["NAME", "SELECT", "READ"], action = clap::ArgAction::Append)]
    pub(crate) many_fields: Vec<String>,
    /// Field descendant exclusion: NAME SELECT, repeatable.
    #[arg(long = "field-exclude", num_args = 2, value_names = ["NAME", "SELECT"], action = clap::ArgAction::Append)]
    pub(crate) field_excludes: Vec<String>,
    /// Exactly this many following element-sibling subtrees per record (0–63).
    #[arg(long)]
    pub(crate) following_siblings: Option<u32>,
    /// Publish a closed USTAR replay bundle; mutually exclusive with receipt.
    #[arg(long, conflicts_with = "receipt")]
    pub(crate) bundle: Option<PathBuf>,
}
impl Extract {
    pub(crate) fn extraction_plan(
        &self,
        stdin: &mut dyn Read,
    ) -> Result<ExtractionPlan, htmlcut_core::ExtractionError> {
        if let Some(path) = &self.plan {
            if path.as_os_str() == "-" {
                if self.source.source.stdin {
                    return Err(options("Source and query cannot both use stdin."));
                }
                return ExtractionPlan::from_json(&crate::input::read_bounded(
                    stdin,
                    MAX_CONFIG_BYTES,
                )?);
            }
            return ExtractionPlan::from_json(&read_file(path, MAX_CONFIG_BYTES)?);
        }
        if let Some(json) = &self.plan_json {
            return ExtractionPlan::from_json(json.as_bytes());
        }
        let mut plan = ExtractionPlan::css(self.select.as_deref().expect("required query source"))?;
        plan.match_mode = if self.all {
            Match::All
        } else if self.nth.is_some() {
            Match::Nth
        } else {
            Match::One
        };
        plan.min = self.min;
        plan.max = self.max;
        plan.index = self.nth;
        plan.read = self.read.clone();
        if !self.exclude.is_empty() {
            plan.exclude = Some(self.exclude.clone());
        }
        plan.expect = self
            .expect_text
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| Guard {
                select: pair[0].clone(),
                scope: GuardScope::Document,
                min: 1,
                max: 1,
                read: Some(Reading::Text),
                equals: Some(pair[1].clone()),
                pattern: None,
            })
            .collect();
        let mut fields = BTreeMap::new();
        let mut declarations = [
            (&self.fields, FieldMatch::One),
            (&self.optional_fields, FieldMatch::Optional),
            (&self.many_fields, FieldMatch::All),
        ]
        .into_iter()
        .flat_map(|(values, mode)| {
            values
                .as_chunks::<3>()
                .0
                .iter()
                .map(move |triple| (triple, mode))
        })
        .collect::<Vec<_>>();
        declarations.sort_by(|(left, _), (right, _)| left[0].cmp(&right[0]));
        for (triple, mode) in declarations {
            RecordField::validate_name(&triple[0])?;
            if fields.len() == htmlcut_core::MAX_RECORD_FIELDS {
                return Err(options("Field count exceeds 64."));
            }
            let mut field = RecordField::css(&triple[1]);
            field.match_mode = mode;
            field.read =
                triple[2]
                    .parse()
                    .map_err(|mut error: htmlcut_core::ExtractionError| {
                        error.field_name = Some(triple[0].clone());
                        error.plan_path = Some(format!("fields.{}.read", triple[0]));
                        error
                    })?;
            if fields.insert(triple[0].clone(), field).is_some() {
                return Err(options("Duplicate field names are not permitted."));
            }
        }
        for pair in self.field_excludes.as_chunks::<2>().0 {
            fields
                .get_mut(&pair[0])
                .ok_or_else(|| options("Field exclusions require a declared field name."))?
                .exclude
                .push(pair[1].clone());
        }
        if !fields.is_empty() {
            plan.fields = Some(fields);
        }
        plan.following_siblings = self.following_siblings;
        plan.validate()?;
        Ok(plan)
    }
}
#[derive(Args)]
pub(crate) struct Replay {
    pub(crate) file: PathBuf,
    #[command(flatten)]
    pub(crate) output: Output,
}
#[derive(Args)]
pub(crate) struct Inspect {
    #[command(flatten)]
    pub(crate) source: SourceOptions,
    /// Explicit selector enables count/sample mode; omitted means repeated-group survey.
    #[arg(long, conflicts_with_all = ["within", "limit"])]
    pub(crate) select: Option<String>,
    /// Targeted samples, 1–10 (default 3); requires select.
    #[arg(long, requires = "select")]
    pub(crate) samples: Option<u32>,
    /// Survey scope: exactly one selected original-DOM element.
    #[arg(long)]
    pub(crate) within: Option<String>,
    /// Survey groups, 1–16 (default 4).
    #[arg(long)]
    pub(crate) limit: Option<u32>,
}
