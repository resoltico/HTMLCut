// SPDX-License-Identifier: MPL-2.0
//! Complete input, a single publication claim, and atomic named output.
use crate::cli::{self, Command, Common};
use htmlcut_core::{Contract, Document, Error, ErrorCode, OutlineOptions, Source, Value};
use std::{
    fs::{File, OpenOptions},
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
            let contract = Contract::from_json(&program).map_err(|mut error| {
                if args.from_file.is_none()
                    && serde_json::from_str::<serde_json::Value>(&program).is_err()
                    && Path::new(&program).is_file()
                {
                    error
                        .message
                        .push_str(" Read a contract file with --from-file.");
                }
                error
            })?;
            extract(contract, file.as_deref(), args.raw, &args.common)
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
fn write_output(writer: &mut impl Write, bytes: &[u8], path: &Path) -> Result<(), Error> {
    match writer.write_all(bytes).and_then(|()| writer.flush()) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => Ok(()),
        Err(error) => Err(io_error(ErrorCode::OutputUnwritable, path, error)),
    }
}

// Descriptor names identify an already-open stream, not a pathname to replace.
fn descriptor_alias(path: &Path) -> bool {
    #[cfg(not(unix))]
    {
        let _ = path;
        false
    }
    #[cfg(unix)]
    {
        let Some(parent) = path.parent() else {
            return false;
        };
        let Ok(parent) = std::fs::canonicalize(if parent.as_os_str().is_empty() {
            Path::new(".")
        } else {
            parent
        }) else {
            return false;
        };
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
            return false;
        };
        if parent == Path::new("/dev") && matches!(name, "stdin" | "stdout" | "stderr") {
            return true;
        }
        if !name.bytes().all(|c| c.is_ascii_digit()) {
            return false;
        }
        if parent == Path::new("/dev/fd") {
            return true;
        }
        let parts = parent.iter().filter_map(|p| p.to_str()).collect::<Vec<_>>();
        matches!(
            parts.as_slice(),
            ["/", "proc", _, "fd"] | ["/", "proc", _, "task", _, "fd"]
        )
    }
}

fn resolve_output_links(path: &Path) -> io::Result<std::path::PathBuf> {
    let mut current = path.to_path_buf();
    let mut visited = std::collections::HashSet::new();
    loop {
        if descriptor_alias(&current) {
            return Ok(current);
        }
        match std::fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                // Resolve the parent with the OS rather than collapsing `..` across
                // symlinks. The identity stays stable even if relative links grow the path.
                let parent = current
                    .parent()
                    .filter(|p| !p.as_os_str().is_empty())
                    .unwrap_or_else(|| Path::new("."));
                let identity = (
                    std::fs::canonicalize(parent)?,
                    current.file_name().map(|name| name.to_owned()),
                );
                if !visited.insert(identity) {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "symbolic link cycle",
                    ));
                }
                let target = std::fs::read_link(&current)?;
                current = if target.is_absolute() {
                    target
                } else {
                    current
                        .parent()
                        .unwrap_or_else(|| Path::new("."))
                        .join(target)
                };
            }
            Ok(_) => return Ok(current),
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(current),
            Err(error) => return Err(error),
        }
    }
}

pub(crate) fn publish(bytes: &[u8], destination: Option<&Path>) -> Result<(), Error> {
    let Some(path) = destination.filter(|path| *path != Path::new("-")) else {
        return write_output(&mut io::stdout().lock(), bytes, Path::new("-"));
    };
    let (resolved, permissions) = match OpenOptions::new().write(true).open(path) {
        Ok(mut file) => {
            let metadata = match file.metadata() {
                Ok(metadata) => Some(metadata),
                // Windows device and pipe handles may not support filesystem metadata.
                #[cfg(windows)]
                Err(_) => None,
                #[cfg(not(windows))]
                Err(error) => return Err(io_error(ErrorCode::OutputUnwritable, path, error)),
            };
            if metadata.as_ref().is_some_and(|m| m.is_dir()) {
                return Err(io_error(
                    ErrorCode::OutputUnwritable,
                    path,
                    io::Error::new(io::ErrorKind::IsADirectory, "destination is a directory"),
                ));
            }
            if !metadata.as_ref().is_some_and(|m| m.is_file()) {
                return write_output(&mut file, bytes, path);
            }
            let resolved = resolve_output_links(path)
                .map_err(|e| io_error(ErrorCode::OutputUnwritable, path, e))?;
            if descriptor_alias(&resolved) {
                return write_output(&mut file, bytes, path);
            }
            let resolved = std::fs::canonicalize(resolved)
                .map_err(|e| io_error(ErrorCode::OutputUnwritable, path, e))?;
            (resolved, metadata.map(|m| m.permissions()))
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            let resolved = resolve_output_links(path)
                .map_err(|e| io_error(ErrorCode::OutputUnwritable, path, e))?;
            (resolved, None)
        }
        Err(error) => return Err(io_error(ErrorCode::OutputUnwritable, path, error)),
    };
    let parent = resolved
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    #[cfg(unix)]
    let mut builder = tempfile::Builder::new();
    #[cfg(not(unix))]
    let builder = tempfile::Builder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if permissions.is_none() {
            builder.permissions(std::fs::Permissions::from_mode(0o666));
        }
    }
    let mut temporary = builder
        .tempfile_in(parent)
        .map_err(|e| io_error(ErrorCode::OutputUnwritable, path, e))?;
    temporary
        .write_all(bytes)
        .and_then(|()| temporary.flush())
        .map_err(|e| io_error(ErrorCode::OutputUnwritable, path, e))?;
    #[cfg(unix)]
    if let Some(permissions) = permissions {
        use std::os::unix::fs::PermissionsExt;
        temporary
            .as_file()
            .set_permissions(std::fs::Permissions::from_mode(permissions.mode() & 0o777))
            .map_err(|e| io_error(ErrorCode::OutputUnwritable, path, e))?;
    }
    #[cfg(not(unix))]
    let _ = permissions;
    temporary
        .persist(resolved)
        .map_err(|e| io_error(ErrorCode::OutputUnwritable, path, e.error))?;
    Ok(())
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
    fn link_resolution_propagates_a_non_directory_parent_failure() {
        let directory = tempfile::tempdir().unwrap();
        let file = directory.path().join("file");
        std::fs::write(&file, b"unchanged").unwrap();
        let child = file.join("child");
        let expected = std::fs::symlink_metadata(&child).unwrap_err();
        let error = resolve_output_links(&child).unwrap_err();
        assert_eq!(error.kind(), expected.kind());
        assert_eq!(error.raw_os_error(), expected.raw_os_error());
        assert_eq!(std::fs::read(file).unwrap(), b"unchanged");
    }
    #[test]
    fn output_ignores_only_broken_pipe_from_writing_or_flushing() {
        struct FailingWriter {
            kind: io::ErrorKind,
            on_flush: bool,
        }
        impl Write for FailingWriter {
            fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
                if self.on_flush {
                    Ok(bytes.len())
                } else {
                    Err(io::Error::new(self.kind, "destination failure"))
                }
            }
            fn flush(&mut self) -> io::Result<()> {
                if self.on_flush {
                    Err(io::Error::new(self.kind, "destination failure"))
                } else {
                    Ok(())
                }
            }
        }
        for on_flush in [false, true] {
            let path = Path::new("output.json");
            let mut writer = FailingWriter {
                kind: io::ErrorKind::BrokenPipe,
                on_flush,
            };
            write_output(&mut writer, b"complete", path).unwrap();
            writer.kind = io::ErrorKind::PermissionDenied;
            let error = write_output(&mut writer, b"complete", path).unwrap_err();
            assert_eq!(error.error, ErrorCode::OutputUnwritable);
            assert_eq!(error.file.as_deref(), Some("output.json"));
            assert_eq!(error.detail.as_deref(), Some("destination failure"));
        }
    }

    #[test]
    fn ordinary_output_names_are_not_descriptor_aliases() {
        let directory = tempfile::tempdir().unwrap();
        for name in ["stdout", "stdin", "stderr", "1", "123"] {
            let path = directory.path().join(name);
            std::fs::write(&path, b"ordinary file").unwrap();
            assert!(!descriptor_alias(&path), "{path:?}");
        }
        let ordinary_fd_directory = directory.path().join("fd");
        std::fs::create_dir(&ordinary_fd_directory).unwrap();
        assert!(!descriptor_alias(&ordinary_fd_directory.join("1")));
    }

    #[cfg(unix)]
    #[test]
    fn descriptor_names_require_the_stream_namespace_and_numeric_fd_names() {
        for path in ["/dev/stdin", "/dev/stdout", "/dev/stderr", "/dev/fd/1"] {
            assert!(descriptor_alias(Path::new(path)), "{path}");
        }
        for path in ["/dev/null", "/dev/1", "/dev/fd/not-a-descriptor"] {
            assert!(!descriptor_alias(Path::new(path)), "{path}");
        }
    }

    #[test]
    fn panic_unwinding_cancels_the_watchdog_before_delayed_error_handling() {
        const CHILD: &str = "HTMLCUT_WATCHDOG_PANIC_CHILD";
        if std::env::var_os(CHILD).is_some() {
            std::panic::set_hook(Box::new(|_| {}));
            let error = crate::guarded(|| {
                let _watchdog = Watchdog::new(Duration::from_millis(250));
                panic!("extraction interrupted before completion");
            })
            .unwrap_err();
            // The real timer must remain cancelled while error handling continues.
            std::thread::sleep(Duration::from_millis(750));
            io::stderr()
                .lock()
                .write_all(&error.to_json_line())
                .unwrap();
            std::process::exit(error.error.exit_status());
        }

        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "run::tests::panic_unwinding_cancels_the_watchdog_before_delayed_error_handling",
                "--nocapture",
            ])
            .env(CHILD, "1")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if child.try_wait().unwrap().is_some() {
                break;
            }
            if Instant::now() >= deadline {
                child.kill().unwrap();
                let output = child.wait_with_output().unwrap();
                panic!("panic unwinding did not finish: {output:?}");
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let output = child.wait_with_output().unwrap();
        assert_eq!(output.status.code(), Some(6), "{output:?}");
        assert_eq!(output.stderr.iter().filter(|c| **c == b'\n').count(), 1);
        let diagnostic: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(diagnostic["error"], "internal");
    }

    #[cfg(unix)]
    #[test]
    fn output_link_resolution_checks_the_target_after_forty_links() {
        use std::os::unix::fs::symlink;
        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("target");
        let mut link = target.clone();
        for index in 0..40 {
            let next = directory.path().join(format!("link-{index}"));
            symlink(link.file_name().unwrap(), &next).unwrap();
            link = next;
        }
        assert_eq!(resolve_output_links(&link).unwrap(), target);
        std::fs::write(&target, b"existing").unwrap();
        assert_eq!(resolve_output_links(&link).unwrap(), target);
        assert_eq!(std::fs::read(target).unwrap(), b"existing");
    }

    #[cfg(unix)]
    #[test]
    fn output_link_cycles_with_growing_relative_paths_are_refused() {
        use std::os::unix::fs::symlink;
        let directory = tempfile::tempdir().unwrap();
        std::fs::create_dir(directory.path().join("child")).unwrap();
        let link = directory.path().join("cycle");
        symlink("child/../cycle", &link).unwrap();
        let error = resolve_output_links(&link).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
        assert_eq!(error.to_string(), "symbolic link cycle");
    }

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
