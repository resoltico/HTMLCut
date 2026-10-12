// SPDX-License-Identifier: MPL-2.0
//! Complete input, a single publication claim, and atomic named output.
use crate::cli::{self, Command, Common};
use htmlcut_core::{Contract, Document, Error, ErrorCode, OutlineOptions, Source, Value};
use std::{
    fs::File,
    io::{self, IsTerminal, Read, Write},
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicU8, Ordering},
    },
    time::{Duration, Instant},
};

const ACTIVE: u8 = 0;
const CLAIMED: u8 = 1;
const EXPIRED: u8 = 2;

pub(crate) fn execute(command: Command) -> Result<(), Error> {
    match command {
        Command::Schema => {
            let mut bytes = serde_json::to_vec(&Contract::schema()).map_err(|e| {
                Error::new(ErrorCode::Internal, format!("Schema encoding failed: {e}."))
            })?;
            bytes.push(b'\n');
            publish(&bytes, None)
        }
        Command::Get(args) => {
            let mut value = serde_json::json!({"select":args.css,"read":args.read});
            if args.all {
                value["match"] = "all".into();
            }
            let contract = Contract::from_json(&value.to_string())?;
            extract(contract, args.file.as_deref(), args.raw, &args.common)
        }
        Command::Run(args) => {
            let (program, file) = if let Some(path) = &args.from_file {
                if args.operands.len() > 1 {
                    return Err(cli::argument_error(
                        "run --from-file accepts at most one FILE operand",
                    ));
                }
                let bytes = read_file(path, 256 * 1024, true)?;
                let program = String::from_utf8(bytes).map_err(|e| {
                    Error::new(
                        ErrorCode::InvalidContract,
                        format!("The contract file is not UTF-8: {e}."),
                    )
                })?;
                (program, args.operands.first().map(std::path::PathBuf::from))
            } else {
                let program = args
                    .operands
                    .first()
                    .ok_or_else(|| {
                        cli::argument_error("run requires CONTRACT-JSON or --from-file PATH")
                    })?
                    .to_str()
                    .ok_or_else(|| cli::argument_error("CONTRACT-JSON must be UTF-8"))?
                    .to_owned();
                (program, args.operands.get(1).map(std::path::PathBuf::from))
            };
            extract(
                Contract::from_json(&program)?,
                file.as_deref(),
                args.raw,
                &args.common,
            )
        }
        Command::Outline(args) => {
            let bytes = input(args.file.as_deref(), args.common.max_input_bytes)?;
            let watchdog = Watchdog::new(args.common.timeout);
            let result = (|| {
                let document = prepare(&bytes, &args.common)?;
                let mut options = OutlineOptions::default();
                options.select = args.select;
                options.depth = args.depth;
                options.max_lines = args.max_lines;
                options.attrs = args.attrs;
                Ok(document.outline(&options)?.into_bytes())
            })();
            let bytes = watchdog.complete(result)?;
            publish(&bytes, args.common.output.as_deref())
        }
    }
}
fn extract(
    contract: Contract,
    file: Option<&Path>,
    raw: bool,
    common: &Common,
) -> Result<(), Error> {
    if raw && contract.produces_records() {
        return Err(cli::argument_error(
            "--raw requires a value root, not a record root",
        ));
    }
    let bytes = input(file, common.max_input_bytes)?;
    let watchdog = Watchdog::new(common.timeout);
    let result = (|| {
        let document = prepare(&bytes, common)?;
        let value = document.extract(&contract)?;
        if raw {
            raw_bytes(value)
        } else {
            let mut bytes = value.to_json(common.max_output_bytes)?;
            bytes.push(b'\n');
            Ok(bytes)
        }
    })();
    let bytes = watchdog.complete(result)?;
    publish(&bytes, common.output.as_deref())
}
fn prepare(bytes: &[u8], common: &Common) -> Result<Document, Error> {
    let mut source = Source::new(bytes);
    source.encoding = common.encoding.as_deref();
    source.base_url = common.base_url.as_deref();
    Document::parse(source, &common.limits())
}
fn raw_bytes(value: Value) -> Result<Vec<u8>, Error> {
    let mut bytes = Vec::new();
    let mut add = |value: String| {
        bytes.extend_from_slice(value.as_bytes());
        bytes.push(b'\n');
    };
    match value {
        Value::Null => (),
        Value::String(value) => add(value),
        Value::List(values) => {
            for value in values {
                if let Value::String(value) = value {
                    add(value);
                } else {
                    return Err(Error::new(
                        ErrorCode::Internal,
                        "A raw value list contained a non-string.",
                    ));
                }
            }
        }
        _ => {
            return Err(Error::new(
                ErrorCode::Internal,
                "A non-value reached raw publication.",
            ));
        }
    }
    Ok(bytes)
}
fn input(path: Option<&Path>, maximum: usize) -> Result<Vec<u8>, Error> {
    if path.is_none() && io::stdin().is_terminal() {
        return Err(cli::argument_error(
            "standard input is a terminal; provide FILE or pipe the document",
        ));
    }
    match path {
        None => read(&mut io::stdin().lock(), maximum, Path::new("-"), false),
        Some(path) if path == Path::new("-") => read(&mut io::stdin().lock(), maximum, path, false),
        Some(path) => read_file(path, maximum, false),
    }
}
fn read_file(path: &Path, maximum: usize, contract: bool) -> Result<Vec<u8>, Error> {
    let mut file = File::open(path).map_err(|e| io_error(ErrorCode::InputUnreadable, path, e))?;
    read(&mut file, maximum, path, contract)
}
fn read(
    reader: &mut impl Read,
    maximum: usize,
    path: &Path,
    contract: bool,
) -> Result<Vec<u8>, Error> {
    let mut bytes = Vec::new();
    reader
        .take((maximum as u64).saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|e| io_error(ErrorCode::InputUnreadable, path, e))?;
    if bytes.len() > maximum {
        return Err(if contract {
            Error::new(
                ErrorCode::InvalidContract,
                format!("The contract exceeds the {maximum}-byte grammar bound."),
            )
        } else {
            Error::limit("input_bytes", maximum)
        });
    }
    Ok(bytes)
}
fn io_error(code: ErrorCode, path: &Path, detail: io::Error) -> Error {
    let file = path.to_string_lossy().into_owned();
    let detail = detail.to_string();
    let mut error = Error::new(code, format!("File {file:?}: {detail}."));
    error.file = Some(file);
    error.detail = Some(detail);
    error
}
pub(crate) fn publish(bytes: &[u8], destination: Option<&Path>) -> Result<(), Error> {
    match destination {
        None => io::stdout()
            .lock()
            .write_all(bytes)
            .and_then(|()| io::stdout().flush())
            .map_err(|e| io_error(ErrorCode::OutputUnwritable, Path::new("-"), e)),
        Some(path) => {
            let parent = path
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or_else(|| Path::new("."));
            let mut temporary = tempfile::NamedTempFile::new_in(parent)
                .map_err(|e| io_error(ErrorCode::OutputUnwritable, path, e))?;
            temporary
                .write_all(bytes)
                .and_then(|()| temporary.flush())
                .map_err(|e| io_error(ErrorCode::OutputUnwritable, path, e))?;
            temporary
                .persist(path)
                .map_err(|e| io_error(ErrorCode::OutputUnwritable, path, e.error))?;
            Ok(())
        }
    }
}

struct Watchdog {
    state: Arc<AtomicU8>,
    started: Instant,
    duration: Duration,
}
impl Watchdog {
    fn new(duration: Duration) -> Self {
        let state = Arc::new(AtomicU8::new(ACTIVE));
        let started = Instant::now();
        let timer_state = Arc::clone(&state);
        std::thread::spawn(move || {
            std::thread::sleep(duration);
            if timer_state
                .compare_exchange(ACTIVE, EXPIRED, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                let error = Error::limit("time", duration.as_secs_f64());
                let _ = io::stderr().lock().write_all(&error.to_json_line());
                std::process::exit(4);
            }
        });
        Self {
            state,
            started,
            duration,
        }
    }
    fn complete(&self, result: Result<Vec<u8>, Error>) -> Result<Vec<u8>, Error> {
        if self
            .state
            .compare_exchange(ACTIVE, CLAIMED, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            await_watchdog();
        }
        if self.started.elapsed() >= self.duration {
            return Err(Error::limit("time", self.duration.as_secs_f64()));
        }
        result
    }
}
impl Drop for Watchdog {
    fn drop(&mut self) {
        if self
            .state
            .compare_exchange(ACTIVE, CLAIMED, Ordering::SeqCst, Ordering::SeqCst)
            == Err(EXPIRED)
        {
            await_watchdog();
        }
    }
}
fn await_watchdog() -> ! {
    loop {
        std::thread::park();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_one_publication_or_timeout_claim_wins() {
        for _ in 0..100 {
            let state = Arc::new(AtomicU8::new(ACTIVE));
            let other = Arc::clone(&state);
            let timer = std::thread::spawn(move || {
                other
                    .compare_exchange(ACTIVE, EXPIRED, Ordering::SeqCst, Ordering::SeqCst)
                    .is_ok()
            });
            let main = state
                .compare_exchange(ACTIVE, CLAIMED, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok();
            assert_ne!(main, timer.join().unwrap());
        }
    }
    #[test]
    fn publication_replaces_a_complete_file_and_cleans_failed_staging() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("result.json");
        std::fs::write(&path, b"old").unwrap();
        publish(b"\"complete\"\n", Some(&path)).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"\"complete\"\n");
        let target_directory = directory.path().join("destination");
        std::fs::create_dir(&target_directory).unwrap();
        let error = publish(b"new", Some(&target_directory)).unwrap_err();
        assert_eq!(error.error, ErrorCode::OutputUnwritable);
        assert!(error.detail.is_some());
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 2);
    }
}
