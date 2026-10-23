// SPDX-License-Identifier: MPL-2.0
//! Command grammar; guide.txt is the complete reader-facing guide.
use clap::{Args, CommandFactory, Parser, Subcommand};
use htmlcut_runtime::{Error, Limits};
use std::{ffi::OsString, path::PathBuf, time::Duration};

#[derive(Parser)]
#[command(
    name = "htmlcut",
    disable_help_flag = true,
    disable_version_flag = true,
    disable_help_subcommand = true
)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Command,
}
#[derive(Subcommand)]
pub(crate) enum Command {
    #[command(disable_help_flag = true)]
    Outline(Outline),
    #[command(disable_help_flag = true)]
    Get(Get),
    #[command(disable_help_flag = true)]
    Run(Run),
    #[command(disable_help_flag = true)]
    Schema(Schema),
}
#[derive(Args)]
pub(crate) struct Common {
    #[arg(long)]
    pub(crate) base_url: Option<String>,
    #[arg(long)]
    pub(crate) encoding: Option<String>,
    #[arg(short = 'o', long)]
    pub(crate) output: Option<PathBuf>,
    #[arg(long, value_parser=timeout, help=format!("Time allowance in seconds [default: {}]", Limits::default().time_limit.unwrap().as_secs_f64()))]
    pub(crate) timeout: Option<Duration>,
    #[arg(long, help=format!("Policy ceiling [default: {}]", Limits::default().max_nodes))]
    pub(crate) max_nodes: Option<usize>,
    #[arg(long, help=format!("Policy ceiling [default: {}]", Limits::default().max_input_bytes))]
    pub(crate) max_input_bytes: Option<usize>,
    #[arg(long, help=format!("Policy ceiling [default: {}]", Limits::default().max_reading_bytes))]
    pub(crate) max_reading_bytes: Option<usize>,
    #[arg(long, help=format!("Policy ceiling [default: {}]", Limits::default().max_output_bytes))]
    pub(crate) max_output_bytes: Option<usize>,
}
impl Common {
    pub(crate) fn limits(&self) -> Limits {
        let mut limits = Limits::default();
        if let Some(value) = self.max_input_bytes {
            limits.max_input_bytes = value;
        }
        if let Some(value) = self.max_nodes {
            limits.max_nodes = value;
        }
        if let Some(value) = self.max_output_bytes {
            limits.max_output_bytes = value;
        }
        if let Some(value) = self.max_reading_bytes {
            limits.max_reading_bytes = value;
        }
        if let Some(value) = self.timeout {
            limits.time_limit = Some(value);
        }
        limits
    }
}
#[derive(Args)]
pub(crate) struct Schema {
    #[arg(long, help=format!("Complete schema byte ceiling [default: {}]", Limits::default().max_output_bytes))]
    pub(crate) max_output_bytes: Option<usize>,
    #[arg(short = 'o', long)]
    pub(crate) output: Option<PathBuf>,
    #[arg(long, value_parser=timeout, help=format!("Time allowance in seconds [default: {}]", Limits::default().time_limit.unwrap().as_secs_f64()))]
    pub(crate) timeout: Option<Duration>,
}
#[derive(Args)]
pub(crate) struct Outline {
    pub(crate) file: Option<PathBuf>,
    #[arg(long)]
    pub(crate) select: Option<String>,
    #[arg(long, help=format!("Expanded depth [default: {}]", htmlcut_runtime::OutlineOptions::default().depth))]
    pub(crate) depth: Option<usize>,
    #[arg(long, help=format!("Emitted lines [default: {}]", htmlcut_runtime::OutlineOptions::default().max_lines))]
    pub(crate) max_lines: Option<usize>,
    #[arg(long)]
    pub(crate) attrs: bool,
    #[command(flatten)]
    pub(crate) common: Common,
}
#[derive(Args)]
pub(crate) struct Get {
    pub(crate) css: String,
    pub(crate) file: Option<PathBuf>,
    #[arg(long)]
    pub(crate) all: bool,
    #[arg(long, help=format!("Value reading [default: {}]", htmlcut_runtime::DEFAULT_READING))]
    pub(crate) read: Option<String>,
    #[arg(short = 'r', long)]
    pub(crate) raw: bool,
    #[command(flatten)]
    pub(crate) common: Common,
}
#[derive(Args)]
pub(crate) struct Run {
    #[arg(value_name="CONTRACT-JSON|FILE",num_args=0..=2)]
    pub(crate) operands: Vec<OsString>,
    #[arg(long)]
    pub(crate) from_file: Option<PathBuf>,
    #[arg(short = 'r', long)]
    pub(crate) raw: bool,
    #[command(flatten)]
    pub(crate) common: Common,
}
fn timeout(value: &str) -> Result<Duration, String> {
    let seconds: f64 = value
        .parse()
        .map_err(|_| "timeout must be a nonnegative finite number of seconds")?;
    Duration::try_from_secs_f64(seconds)
        .map_err(|_| "timeout must be a nonnegative finite number of seconds".into())
}
pub(crate) fn argument_error(message: &str) -> Error {
    let error = Cli::command().error(clap::error::ErrorKind::ArgumentConflict, message);
    parser_error(error)
}

pub(crate) fn parser_error(error: clap::Error) -> Error {
    Error::InvalidRequest {
        reason: format!(
            "{} Run htmlcut --help for the guide.",
            error.to_string().trim_end()
        ),
    }
}
