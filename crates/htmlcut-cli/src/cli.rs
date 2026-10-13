// SPDX-License-Identifier: MPL-2.0
//! Command grammar; guide.txt is the complete reader-facing guide.
use clap::{Args, CommandFactory, Parser, Subcommand};
use htmlcut_core::{Error, ErrorCode, Limits};
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
    Schema,
}
#[derive(Args)]
pub(crate) struct Common {
    #[arg(long)]
    pub(crate) base_url: Option<String>,
    #[arg(long)]
    pub(crate) encoding: Option<String>,
    #[arg(short = 'o', long)]
    pub(crate) output: Option<PathBuf>,
    #[arg(long,default_value="10",value_parser=timeout)]
    pub(crate) timeout: Duration,
    #[arg(long, default_value = "2000000")]
    pub(crate) max_nodes: usize,
    #[arg(long, default_value = "67108864")]
    pub(crate) max_input_bytes: usize,
    #[arg(long, default_value = "67108864")]
    pub(crate) max_output_bytes: usize,
}
impl Common {
    pub(crate) fn limits(&self) -> Limits {
        let mut limits = Limits::default();
        limits.max_input_bytes = self.max_input_bytes;
        limits.max_nodes = self.max_nodes;
        limits.max_output_bytes = self.max_output_bytes;
        limits.time_limit = Some(self.timeout);
        limits
    }
}
#[derive(Args)]
pub(crate) struct Outline {
    pub(crate) file: Option<PathBuf>,
    #[arg(long)]
    pub(crate) select: Option<String>,
    #[arg(long, default_value = "6")]
    pub(crate) depth: usize,
    #[arg(long, default_value = "80")]
    pub(crate) max_lines: usize,
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
    #[arg(long, default_value = "text")]
    pub(crate) read: String,
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
    Error::new(
        ErrorCode::InvalidArguments,
        format!(
            "{} Run htmlcut --help for the guide.",
            error.to_string().trim_end()
        ),
    )
}
