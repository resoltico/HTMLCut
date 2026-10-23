// SPDX-License-Identifier: MPL-2.0
#![deny(unsafe_code)]
mod cli;
mod run;
use clap::{CommandFactory, Parser};
use htmlcut_runtime::Error;
use std::process::ExitCode;

fn main() -> ExitCode {
    std::panic::set_hook(Box::new(|_| {}));
    let result = guarded(entry);
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            run::report(&error);
            ExitCode::from(run::exit_status(error.code()) as u8)
        }
    }
}
fn guarded(
    action: impl FnOnce() -> Result<(), Error> + std::panic::UnwindSafe,
) -> Result<(), Error> {
    std::panic::catch_unwind(action).unwrap_or_else(|_| {
        Err(Error::Internal {
            reason: ("An internal error interrupted the command.").into(),
        })
    })
}
fn entry() -> Result<(), Error> {
    let args = std::env::args_os().collect::<Vec<_>>();
    if args
        .iter()
        .skip(1)
        .take_while(|arg| *arg != "--")
        .any(|arg| arg == "--help" || arg == "-h")
        || (args.len() == 2 && args[1] == "help")
    {
        if let Some(name) = args.get(1).and_then(|a| a.to_str()) {
            let mut command = cli::Cli::command();
            if let Some(subcommand) = command.find_subcommand_mut(name) {
                let mut help = subcommand.render_long_help().to_string();
                if name == "schema" {
                    help.push_str("\n\nSchema is written to stdout by default; -o PATH writes to a file and -o - selects stdout.");
                } else {
                    help.push_str("\n\nFILE is a local snapshot; '-' is standard input.");
                }
                help.push_str(" See htmlcut --help for workflow examples.\n");
                return run::publish(help.as_bytes(), None);
            }
        }
        return run::publish(include_bytes!("guide.txt"), None);
    }
    if args.len() == 2 && (args[1] == "--version" || args[1] == "-V") {
        return run::publish(
            concat!("htmlcut ", env!("CARGO_PKG_VERSION"), "\n").as_bytes(),
            None,
        );
    }
    let args = cli::Cli::try_parse_from(args).map_err(cli::parser_error)?;
    run::execute(args.command)
}

#[cfg(test)]
mod tests {
    #[test]
    fn panic_boundary_produces_an_internal_failure() {
        let error = super::guarded(|| panic!("boundary witness")).unwrap_err();
        assert_eq!(error.code(), htmlcut_runtime::ErrorCode::Internal);
        assert_eq!(crate::run::exit_status(error.code()), 6);
        assert_eq!(
            error.to_json_line().iter().filter(|c| **c == b'\n').count(),
            1
        );
    }
    #[test]
    fn cancellation_has_its_distinct_terminal_status_without_resource_facts() {
        let error = htmlcut_runtime::Error::Cancelled;
        assert_eq!(crate::run::exit_status(error.code()), 130);
        assert!(serde_json::to_value(&error).unwrap().get("limit").is_none());
        assert!(
            serde_json::to_value(&error)
                .unwrap()
                .get("bound")
                .cloned()
                .is_none()
        );
    }
}
