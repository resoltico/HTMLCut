// SPDX-License-Identifier: MPL-2.0
//! Terminal adaptation and disposable-process supervision.
use crate::cli::{self, Command};
use htmlcut_runtime::{
    Completion, ContractInput, Destination, Error, ErrorCode, ExportFormat, Input, Job, JobControl,
    Limits, Operation, OutlineOptions, QueryCardinality, QuickQuery, Request, Supervision,
};
use std::{
    io::Write,
    path::{Path, PathBuf},
    time::Duration,
};
const REPORT_GRACE: Duration = Duration::from_millis(50);
pub(crate) fn exit_status(code: ErrorCode) -> i32 {
    match code {
        ErrorCode::InvalidRequest
        | ErrorCode::InvalidContract
        | ErrorCode::InvalidSelector
        | ErrorCode::InvalidPattern
        | ErrorCode::InvalidEncoding
        | ErrorCode::InvalidBaseUrl => 2,
        ErrorCode::CountMismatch
        | ErrorCode::ValueMismatch
        | ErrorCode::ExpectationFailed
        | ErrorCode::MissingAttribute
        | ErrorCode::UnresolvableUrl
        | ErrorCode::UnrepresentableOutput => 3,
        ErrorCode::LimitExceeded => 4,
        ErrorCode::InputUnreadable | ErrorCode::UndecodableInput | ErrorCode::OutputUnwritable => 5,
        ErrorCode::Internal => 6,
        ErrorCode::Cancelled => 130,
    }
}
pub(crate) fn report(error: &Error) {
    report_for(error, REPORT_GRACE);
}
fn report_for(error: &Error, grace: Duration) {
    let bytes = error.to_json_line();
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    if std::thread::Builder::new()
        .spawn(move || {
            let _ = std::io::stderr().lock().write_all(&bytes);
            let _ = sender.send(());
        })
        .is_ok()
    {
        let _ = receiver.recv_timeout(grace);
    }
}

fn supervise(control: JobControl) -> Result<(), Error> {
    let Some(deadline) = control.deadline() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .spawn(move || {
            loop {
                std::thread::sleep(deadline.saturating_duration_since(std::time::Instant::now()));
                match control.supervise() {
                    Supervision::Pending => continue,
                    Supervision::CommitInProgress
                    | Supervision::Finished
                    | Supervision::EnforcementInProgress => return,
                    Supervision::Claimed(error) => {
                        let status = exit_status(error.error);
                        let grace_started = std::time::Instant::now();
                        // Enforcement still exits if reporting or cleanup setup panics.
                        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            let (sender, receiver) = std::sync::mpsc::sync_channel(1);
                            let _ = std::thread::Builder::new().spawn(move || {
                                control.cleanup(REPORT_GRACE);
                                let _ = sender.send(());
                            });
                            report_for(
                                &error,
                                REPORT_GRACE.saturating_sub(grace_started.elapsed()),
                            );
                            let _ = receiver
                                .recv_timeout(REPORT_GRACE.saturating_sub(grace_started.elapsed()));
                        }));
                        #[cfg(test)]
                        if let Some(path) = std::env::var_os("HTMLCUT_CLI_ENFORCEMENT_TIMING") {
                            let _ = std::fs::write(
                                path,
                                grace_started.elapsed().as_micros().to_string(),
                            );
                        }
                        std::process::exit(status);
                    }
                }
            }
        })
        .map(|_| ())
        .map_err(|error| {
            Error::new(
                ErrorCode::Internal,
                format!("The process supervisor could not start: {error}."),
            )
        })
}

fn completion<T>(result: Completion<T>) -> Result<T, Error> {
    match result {
        Completion::Finished(result) => result,
        Completion::Supervised(error) => {
            // The supervisor owns reporting. A bounded fallback also covers a
            // failed report thread; libraries never wait for this process exit.
            std::thread::sleep(REPORT_GRACE * 3);
            std::process::exit(exit_status(error.error));
        }
    }
}
fn destination(path: Option<PathBuf>) -> Destination {
    match path {
        Some(path) if path.as_os_str() != "-" => Destination::File(path),
        Some(_) | None => Destination::Stdout,
    }
}
fn input(path: Option<PathBuf>) -> Input {
    match path {
        Some(path) if path.as_os_str() != "-" => Input::File(path),
        Some(_) => Input::Stdin {
            allow_terminal: true,
        },
        None => Input::Stdin {
            allow_terminal: false,
        },
    }
}
pub(crate) fn publish(bytes: &[u8], path: Option<&Path>) -> Result<(), Error> {
    let job = Job::new(Limits::default().time_limit)?;
    supervise(job.control())?;
    completion(job.present(bytes, destination(path.map(Path::to_path_buf)))).map(|delivery| {
        match delivery {
            htmlcut_runtime::Delivery::Delivered | htmlcut_runtime::Delivery::Disconnected => (),
        }
    })
}
pub(crate) fn execute(command: Command) -> Result<(), Error> {
    let request = match command {
        Command::Schema(args) => {
            let mut limits = Limits::default();
            if let Some(timeout) = args.timeout {
                limits.time_limit = Some(timeout);
            }
            if let Some(maximum) = args.max_output_bytes {
                limits.max_output_bytes = maximum;
            }
            Request {
                operation: Operation::Schema,
                input: Input::Bytes(Vec::new()),
                encoding: None,
                base_url: None,
                limits,
                destination: destination(args.output),
            }
        }
        Command::Get(args) => Request {
            operation: Operation::Extract {
                contract: ContractInput::Quick(QuickQuery {
                    select: args.css,
                    read: args
                        .read
                        .unwrap_or_else(|| htmlcut_runtime::DEFAULT_READING.into()),
                    cardinality: if args.all {
                        QueryCardinality::All
                    } else {
                        QueryCardinality::One
                    },
                }),
                format: if args.raw {
                    ExportFormat::Raw
                } else {
                    ExportFormat::Json
                },
            },
            input: input(args.file),
            limits: args.common.limits(),
            encoding: args.common.encoding,
            base_url: args.common.base_url,
            destination: destination(args.common.output),
        },
        Command::Run(args) => {
            let (contract, file) = if let Some(path) = args.from_file {
                if args.operands.len() > 1 {
                    return Err(cli::argument_error(
                        "run --from-file accepts at most one FILE operand",
                    ));
                }
                (
                    if path.as_os_str() == "-" {
                        ContractInput::Stdin
                    } else {
                        ContractInput::File(path)
                    },
                    args.operands.first().map(PathBuf::from),
                )
            } else {
                let text = args
                    .operands
                    .first()
                    .ok_or_else(|| {
                        cli::argument_error("run requires CONTRACT-JSON or --from-file PATH")
                    })?
                    .to_str()
                    .ok_or_else(|| cli::argument_error("CONTRACT-JSON must be UTF-8"))?
                    .to_owned();
                (
                    ContractInput::Json(text),
                    args.operands.get(1).map(PathBuf::from),
                )
            };
            Request {
                operation: Operation::Extract {
                    contract,
                    format: if args.raw {
                        ExportFormat::Raw
                    } else {
                        ExportFormat::Json
                    },
                },
                input: input(file),
                limits: args.common.limits(),
                encoding: args.common.encoding,
                base_url: args.common.base_url,
                destination: destination(args.common.output),
            }
        }
        Command::Outline(args) => {
            let mut options = OutlineOptions::default();
            options.select = args.select;
            if let Some(depth) = args.depth {
                options.depth = depth;
            }
            if let Some(lines) = args.max_lines {
                options.max_lines = lines;
            }
            if args.attrs {
                options.attrs = true;
            }
            Request {
                operation: Operation::Outline(options),
                input: input(args.file),
                limits: args.common.limits(),
                encoding: args.common.encoding,
                base_url: args.common.base_url,
                destination: destination(args.common.output),
            }
        }
    };
    let job = request.job()?;
    supervise(job.control())?;
    completion(job.execute(request))
        .and_then(terminal_delivery)
        .map_err(|mut error| {
            if error.error == ErrorCode::InvalidContract && error.file.is_some() {
                error
                    .message
                    .push_str(" Read a contract file with --from-file.");
            }
            if error.error == ErrorCode::InputUnreadable && error.file.as_deref().is_some_and(|path| path.starts_with("https://") || path.starts_with("http://")) {
                error.message.push_str(" FILE is a local snapshot path. Fetch successfully into owned storage first, then pass the effective URL with --base-url and declared charset with --encoding; see the acquisition recipe in docs/contract.md.");
            }
            error
        })
}

fn terminal_delivery(outcome: htmlcut_runtime::Outcome) -> Result<(), Error> {
    match outcome {
        htmlcut_runtime::Outcome::Delivered(
            htmlcut_runtime::Delivery::Delivered | htmlcut_runtime::Delivery::Disconnected,
        ) => Ok(()),
        htmlcut_runtime::Outcome::Returned(_) => Err(Error::new(
            ErrorCode::InvalidRequest,
            "A terminal command requires a stream or file destination.",
        )),
    }
}

#[cfg(test)]
#[path = "supervision_tests.rs"]
mod tests;
