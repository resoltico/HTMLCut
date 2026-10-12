// SPDX-License-Identifier: MPL-2.0
#![forbid(unsafe_code)]
mod cli;
mod run;
use clap::Parser;
use htmlcut_core::{Error, ErrorCode};
use std::{io::Write, process::ExitCode};

fn main() -> ExitCode {
    std::panic::set_hook(Box::new(|_| {}));
    let result = guarded(entry);
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            let _ = std::io::stderr().lock().write_all(&error.to_json_line());
            ExitCode::from(error.error.exit_status() as u8)
        }
    }
}
fn guarded(
    action: impl FnOnce() -> Result<(), Error> + std::panic::UnwindSafe,
) -> Result<(), Error> {
    std::panic::catch_unwind(action).unwrap_or_else(|_| {
        Err(Error::new(
            ErrorCode::Internal,
            "An internal error interrupted the command.",
        ))
    })
}
fn entry() -> Result<(), Error> {
    let args = std::env::args_os().collect::<Vec<_>>();
    if args
        .iter()
        .skip(1)
        .take_while(|arg| *arg != "--")
        .any(|arg| arg == "--help")
    {
        return run::publish(include_bytes!("guide.txt"), None);
    }
    if args.len() == 2 && args[1] == "--version" {
        return run::publish(
            concat!("htmlcut ", env!("CARGO_PKG_VERSION"), "\n").as_bytes(),
            None,
        );
    }
    let args = cli::Cli::try_parse_from(args)
        .map_err(|error| Error::new(ErrorCode::InvalidArguments, error.to_string()))?;
    run::execute(args.command)
}

#[cfg(test)]
mod tests {
    #[test]
    fn panic_boundary_produces_an_internal_failure() {
        let error = super::guarded(|| panic!("boundary witness")).unwrap_err();
        assert_eq!(error.error, htmlcut_core::ErrorCode::Internal);
        assert_eq!(error.error.exit_status(), 6);
        assert_eq!(
            error.to_json_line().iter().filter(|c| **c == b'\n').count(),
            1
        );
    }
}
