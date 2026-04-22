use std::io::Write;

mod cli_io;
mod commands;
mod outcomes;
mod raw_args;

use clap::{CommandFactory, Parser, error::ErrorKind};

pub(crate) use self::cli_io::{output_file_notice, write_outcome, write_request_definition};
#[cfg(test)]
pub(crate) use self::cli_io::{
    request_definition_parent_dir_for_tests, write_stdout_payload_for_tests,
};
pub(crate) use self::commands::{
    run_catalog, run_inspect_select, run_inspect_slice, run_inspect_source, run_schema, run_select,
    run_slice,
};
pub(crate) use self::outcomes::{error_outcome, execute_extraction, execute_preview};
#[cfg(test)]
pub(crate) use self::outcomes::{human_error_outcome, json_error_outcome};
pub(crate) use self::raw_args::{
    clap_error_message, command_name_from_raw_args, raw_args_prefers_json, raw_args_requests_help,
    raw_args_requests_version,
};
use crate::args::{Cli, Commands, InspectCommands};
use crate::error::usage_error;
use crate::metadata::version_banner;

pub(crate) struct ExecutionOutcome {
    pub(crate) stdout: Option<String>,
    pub(crate) output_file: Option<std::path::PathBuf>,
    pub(crate) post_write_stderr: Vec<String>,
    pub(crate) stderr: Vec<String>,
    pub(crate) exit_code: i32,
}

/// Executes the HTMLCut CLI against one argv stream and writes the rendered result.
pub fn run<I, W1, W2>(args: I, stdout: &mut W1, stderr: &mut W2) -> i32
where
    I: IntoIterator<Item = String>,
    W1: Write,
    W2: Write,
{
    let raw_args: Vec<String> = args.into_iter().collect();
    if raw_args.len() <= 1 {
        let mut command = Cli::command();
        let _ = command.write_long_help(stdout);
        let _ = writeln!(stdout);
        return 0;
    }

    if raw_args_requests_version(&raw_args) && !raw_args_requests_help(&raw_args) {
        let _ = writeln!(stdout, "{}", version_banner());
        return 0;
    }

    let prefers_json_errors = raw_args_prefers_json(&raw_args);
    let parsed = match Cli::try_parse_from(raw_args.clone()) {
        Ok(args) => args,
        Err(error) => {
            if error.kind() == ErrorKind::DisplayHelp {
                let _ = write!(stdout, "{error}");
                return 0;
            }

            let outcome = error_outcome(
                command_name_from_raw_args(&raw_args),
                prefers_json_errors,
                None,
                usage_error("CLI_PARSE_ERROR", clap_error_message(&error)),
            );
            return write_outcome(outcome, stdout, stderr);
        }
    };

    write_outcome(execute(parsed), stdout, stderr)
}

pub(crate) fn execute(cli: Cli) -> ExecutionOutcome {
    let verbose = cli.global.verbose;
    let quiet = cli.global.quiet;
    match cli.command {
        Commands::Catalog(args) => run_catalog(args, verbose, quiet),
        Commands::Schema(args) => run_schema(args, verbose, quiet),
        Commands::Select(args) => run_select(args, verbose, quiet),
        Commands::Slice(args) => run_slice(args, verbose, quiet),
        Commands::Inspect(args) => match args.command {
            InspectCommands::Source(args) => run_inspect_source(args, verbose, quiet),
            InspectCommands::Select(args) => run_inspect_select(args, verbose, quiet),
            InspectCommands::Slice(args) => run_inspect_slice(args, verbose, quiet),
        },
    }
}
