// SPDX-License-Identifier: MPL-2.0
//! Command grammar and rendering policy.
use clap::{Args, Parser, Subcommand};
use htmlcut_core::{ErrorClass, OutlineOptions, ParseOptions, Reading, ResultLimits};
use std::path::PathBuf;
pub(crate) const DEFAULT_TIMEOUT: u64 = 10;
pub(crate) fn status(class: ErrorClass) -> u8 {
    match class {
        ErrorClass::Request => 2,
        ErrorClass::Refusal => 3,
        ErrorClass::Limit => 4,
        ErrorClass::Io => 5,
        ErrorClass::Internal => 6,
        ErrorClass::Cancelled => 130,
    }
}
fn long_help() -> String {
    let exits = [
        ErrorClass::Request,
        ErrorClass::Refusal,
        ErrorClass::Limit,
        ErrorClass::Io,
        ErrorClass::Internal,
        ErrorClass::Cancelled,
    ]
    .into_iter()
    .map(|c| format!("  {} {c:?}", status(c)))
    .collect::<Vec<_>>()
    .join("\n");
    format!(
        "Workflow examples:\n  htmlcut outline page.html --select 'article.product'\n  htmlcut get h1 page.html\n  htmlcut run contract.json page.html\n\n{}\n\nExit statuses:\n  0 Success\n{exits}\n\nThe timeout is cooperative; use timeout 10 htmlcut … for a hard wall-clock bound.",
        htmlcut_core::CONTRACT_SUMMARY
    )
}
#[derive(Parser)]
#[command(name="htmlcut",version,about="Extract what a contract declares from an HTML snapshot, or refuse.",after_long_help=long_help())]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub command: Command,
}
#[derive(Subcommand)]
pub(crate) enum Command {
    /// Inspect the document before authoring a contract.
    Outline(Outline),
    /// Read one selector's value, or all matching values.
    Get(Get),
    /// Run a contract file, or use --json TEXT with an optional HTML file.
    Run(Run),
    /// Print the generated contract authoring schema.
    Schema {
        #[arg(short, long, value_name = "DEST")]
        output: Option<PathBuf>,
    },
}
#[derive(Args)]
pub(crate) struct Common {
    #[arg(long, value_name = "URL")]
    pub base_url: Option<String>,
    #[arg(long, value_name = "LABEL")]
    pub encoding: Option<String>,
    #[arg(short, long, value_name = "DEST")]
    pub output: Option<PathBuf>,
    /// Cooperative timeout in seconds; blocking I/O can exceed it.
    #[arg(long,value_name="SECONDS",default_value_t=DEFAULT_TIMEOUT)]
    pub timeout: u64,
    #[arg(long,value_name="N",default_value_t=ParseOptions::default().max_nodes)]
    pub max_nodes: usize,
    #[arg(long,value_name="N",default_value_t=ParseOptions::default().max_input_bytes)]
    pub max_input_bytes: usize,
    #[arg(long,value_name="N",default_value_t=ResultLimits::default().max_reading_bytes)]
    pub max_reading_bytes: usize,
    #[arg(long,value_name="N",default_value_t=ResultLimits::default().max_output_bytes)]
    pub max_output_bytes: usize,
}
impl Common {
    pub fn parse_options(&self) -> ParseOptions {
        ParseOptions {
            encoding: self.encoding.clone(),
            base_url: self.base_url.clone(),
            max_nodes: self.max_nodes,
            max_input_bytes: self.max_input_bytes,
        }
    }
    pub fn result_limits(&self) -> ResultLimits {
        ResultLimits {
            max_reading_bytes: self.max_reading_bytes,
            max_output_bytes: self.max_output_bytes,
        }
    }
}
#[derive(Args)]
pub(crate) struct Outline {
    #[arg(value_name = "FILE")]
    pub file: Option<PathBuf>,
    #[arg(long, value_name = "CSS")]
    pub select: Option<String>,
    #[arg(long,value_name="N",default_value_t=OutlineOptions::default().depth)]
    pub depth: usize,
    #[arg(long,value_name="N",default_value_t=OutlineOptions::default().max_lines)]
    pub max_lines: usize,
    #[arg(long)]
    pub attrs: bool,
    #[command(flatten)]
    pub common: Common,
}
#[derive(Args)]
pub(crate) struct Get {
    #[arg(value_name = "CSS")]
    pub select: String,
    #[arg(value_name = "FILE")]
    pub file: Option<PathBuf>,
    #[arg(long)]
    pub all: bool,
    #[arg(long,value_name="READING",default_value_t=Reading::default(),help=htmlcut_core::CONTRACT_SUMMARY.lines().last().unwrap())]
    pub read: Reading,
    #[arg(short = 'r', long)]
    pub raw: bool,
    #[command(flatten)]
    pub common: Common,
}
#[derive(Args)]
pub(crate) struct Run {
    /// Contract path (or -); with --json this operand is the HTML file.
    #[arg(value_name = "CONTRACT_OR_FILE")]
    pub first: Option<PathBuf>,
    #[arg(value_name = "FILE")]
    pub second: Option<PathBuf>,
    /// Inline contract JSON. Accepts at most one positional HTML file.
    #[arg(long, value_name = "TEXT", required_unless_present = "first")]
    pub json: Option<String>,
    #[arg(short = 'r', long)]
    pub raw: bool,
    #[command(flatten)]
    pub common: Common,
}
#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;
    #[test]
    fn help_uses_core_defaults_and_derived_member_summary() {
        let mut cmd = Cli::command();
        let help = cmd.render_long_help().to_string();
        assert!(help.contains(htmlcut_core::CONTRACT_SUMMARY));
        let get = cmd
            .find_subcommand_mut("get")
            .unwrap()
            .render_long_help()
            .to_string();
        for n in [
            ParseOptions::default().max_nodes,
            ParseOptions::default().max_input_bytes,
            ResultLimits::default().max_reading_bytes,
            ResultLimits::default().max_output_bytes,
        ] {
            assert!(get.contains(&format!("[default: {n}]")));
        }
        for r in [
            "text",
            "text-content",
            "outer-html",
            "attr:NAME",
            "url:NAME",
        ] {
            assert!(get.contains(r));
        }
    }
}
