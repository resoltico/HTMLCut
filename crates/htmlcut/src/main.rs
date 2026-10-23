// SPDX-License-Identifier: MPL-2.0
//! Local snapshot command, with one diagnostic and cooperative compute cancellation.
mod cli;
mod input;
mod output;
use clap::Parser;
use htmlcut_core::{
    Contract, Document, Error, ExportFormat, Match, NodeObject, NodeSpec, OutlineOptions, Stop,
};
use std::{io::Write, path::Path, time::Duration};
static PANIC: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);
fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        let message = info
            .payload()
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| info.payload().downcast_ref::<String>().map(String::as_str))
            .unwrap_or("unknown panic");
        let location = info
            .location()
            .map(ToString::to_string)
            .unwrap_or_else(|| "unknown location".into());
        *PANIC.lock().unwrap_or_else(|e| e.into_inner()) = Some(format!("{message} at {location}"));
    }));
}
fn catch_command(
    operation: impl FnOnce() -> Result<(), Error> + std::panic::UnwindSafe,
) -> Result<(), Error> {
    std::panic::catch_unwind(operation).unwrap_or_else(|_| {
        Err(Error::Internal {
            reason: PANIC
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .take()
                .unwrap_or_else(|| "Command panicked.".into()),
        })
    })
}
fn invalid(reason: &str) -> Error {
    Error::InvalidRequest {
        reason: reason.into(),
    }
}
fn extract(
    contract: Contract,
    file: Option<&Path>,
    raw: bool,
    common: &cli::Common,
    stop: &Stop,
) -> Result<(), Error> {
    let format = if raw {
        ExportFormat::Raw
    } else {
        ExportFormat::Json
    };
    contract.supports(format)?;
    let bytes = input::read(file, common.max_input_bytes, false, stop)?;
    let document = Document::parse(&bytes, &common.parse_options(), stop)?;
    let value = document.extract(&contract, format, &common.result_limits(), stop)?;
    let bytes = value.export(format, common.max_output_bytes, stop)?;
    output::write(&bytes, common.output.as_deref())
}
fn entry() -> Result<(), Error> {
    let command = match cli::Cli::try_parse() {
        Ok(args) => args.command,
        Err(error) => {
            if matches!(
                error.kind(),
                clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion
            ) {
                return output::write(error.to_string().as_bytes(), None);
            }
            return Err(invalid(&format!("{error}Run htmlcut --help.")));
        }
    };
    match command {
        cli::Command::Schema { output } => {
            let mut bytes = Contract::schema().to_string().into_bytes();
            bytes.push(b'\n');
            output::write(&bytes, output.as_deref())
        }
        cli::Command::Get(args) => {
            let stop = Stop::after(Duration::from_secs(args.common.timeout))?;
            let spec = NodeSpec::Node(Box::new(NodeObject {
                select: Some(args.select),
                read: Some(args.read),
                r#match: Some(if args.all { Match::All } else { Match::One }),
                ..Default::default()
            }));
            extract(
                Contract::compile(&spec, &stop)?,
                args.file.as_deref(),
                args.raw,
                &args.common,
                &stop,
            )
        }
        cli::Command::Run(args) => {
            let stop = Stop::after(Duration::from_secs(args.common.timeout))?;
            let (contract, file) = if let Some(text) = args.json {
                if args.second.is_some() {
                    return Err(invalid("With --json, supply at most one FILE operand."));
                }
                (Contract::from_json(&text, &stop)?, args.first.as_deref())
            } else {
                let path = args
                    .first
                    .as_deref()
                    .ok_or_else(|| invalid("Supply CONTRACT or --json TEXT."))?;
                if path.as_os_str() == "-"
                    && args.second.as_deref().is_none_or(|p| p.as_os_str() == "-")
                {
                    return Err(invalid("Contract and HTML cannot both use standard input."));
                }
                (input::contract(path, &stop)?, args.second.as_deref())
            };
            extract(contract, file, args.raw, &args.common, &stop)
        }
        cli::Command::Outline(args) => {
            let stop = Stop::after(Duration::from_secs(args.common.timeout))?;
            let mut options = OutlineOptions::default();
            options.select = args.select;
            options.depth = args.depth;
            options.max_lines = args.max_lines;
            options.attrs = args.attrs;
            options.validate(&stop)?;
            let bytes = input::read(
                args.file.as_deref(),
                args.common.max_input_bytes,
                false,
                &stop,
            )?;
            let document = Document::parse(&bytes, &args.common.parse_options(), &stop)?;
            let report = document.outline(&options, &args.common.result_limits(), &stop)?;
            let text = report.render(args.common.max_output_bytes, &stop)?;
            output::write(text.as_bytes(), args.common.output.as_deref())
        }
    }
}
fn main() -> std::process::ExitCode {
    install_panic_hook();
    match catch_command(entry) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            let _ = std::io::stderr().lock().write_all(&error.to_json_line());
            std::process::ExitCode::from(cli::status(error.code().class()))
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn panic_hook_retains_message_and_location_in_one_json_line() {
        let previous = std::panic::take_hook();
        install_panic_hook();
        let result = catch_command(|| panic!("diagnostic panic probe"));
        std::panic::set_hook(previous);
        let error = result.unwrap_err();
        assert_eq!(cli::status(error.code().class()), 6);
        let bytes = error.to_json_line();
        assert_eq!(bytes.iter().filter(|b| **b == b'\n').count(), 1);
        assert!(error.to_string().contains("diagnostic panic probe"));
        assert!(error.to_string().contains("main.rs:"));
    }
}
