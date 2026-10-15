// SPDX-License-Identifier: MPL-2.0
//! Complete input, a single publication claim, and atomic named output.
use crate::authorization::{Policy, identity, identity_path};
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
const COMMIT_IN_PROGRESS: u8 = 1;
const EXPIRED: u8 = 2;
const SUCCESS: u8 = 3;
const ERROR: u8 = 4;
const REPORT_GRACE: Duration = Duration::from_millis(50);

pub(crate) fn report(error: &Error) {
    let bytes = error.to_json_line();
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let _ = io::stderr().lock().write_all(&bytes);
        let _ = sender.send(());
    });
    let _ = receiver.recv_timeout(REPORT_GRACE);
}
pub(crate) fn execute(command: Command) -> Result<(), Error> {
    // Resolve competing stdin claims before acquisition and before starting work.
    if let Command::Run(args) = &command {
        if args.from_file.is_some() && args.operands.len() > 1 {
            return Err(cli::argument_error(
                "run --from-file accepts at most one FILE operand",
            ));
        }
        if args.from_file.is_none() && args.operands.is_empty() {
            return Err(cli::argument_error(
                "run requires CONTRACT-JSON or --from-file PATH",
            ));
        }
        if args
            .from_file
            .as_deref()
            .is_some_and(|path| path.as_os_str() == "-")
        {
            let file = args.operands.first().map(std::path::PathBuf::from);
            if file.as_deref().is_none_or(obvious_stdin_alias) {
                return Err(cli::argument_error(
                    "contract stdin requires a distinct explicit HTML file; competing standard input claims are refused before reading",
                ));
            }
        }
    }
    let duration = match &command {
        Command::Schema(a) => a.timeout,
        Command::Get(a) => a.common.timeout,
        Command::Run(a) => a.common.timeout,
        Command::Outline(a) => a.common.timeout,
    };
    let watchdog = Watchdog::new(duration);
    watchdog.check();
    let result = execute_work(command, &watchdog);
    watchdog.finish(result)
}
fn execute_work(command: Command, watchdog: &Watchdog) -> Result<(), Error> {
    match command {
        Command::Schema(args) => {
            let mut bytes = serde_json::to_vec(&Contract::schema()).map_err(|e| {
                Error::new(ErrorCode::Internal, format!("Schema encoding failed: {e}."))
            })?;
            bytes.push(b'\n');
            publish_timed(&bytes, args.output.as_deref(), Some(watchdog))
        }
        Command::Get(args) => {
            let mut value = serde_json::json!({"select":args.css,"read":args.read});
            if args.all {
                value["match"] = "all".into();
            }
            let contract = Contract::from_json(&value.to_string())?;
            extract(
                contract,
                args.file.as_deref(),
                args.raw,
                &args.common,
                watchdog,
            )
        }
        Command::Run(args) => {
            if args
                .from_file
                .as_deref()
                .is_some_and(|path| path.as_os_str() == "-")
                && args
                    .operands
                    .first()
                    .is_some_and(|file| stdin_alias(Path::new(file)))
            {
                return Err(cli::argument_error(
                    "contract stdin requires a distinct HTML source; this path aliases descriptor 0",
                ));
            }
            let (program, file) = if let Some(path) = &args.from_file {
                if args.operands.len() > 1 {
                    return Err(cli::argument_error(
                        "run --from-file accepts at most one FILE operand",
                    ));
                }
                let bytes = if path.as_os_str() == "-" {
                    read(&mut io::stdin().lock(), 256 * 1024, path, true)?
                } else {
                    read_file(path, 256 * 1024, true)?
                };
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
            extract(contract, file.as_deref(), args.raw, &args.common, watchdog)
        }
        Command::Outline(args) => {
            let bytes = input(args.file.as_deref(), args.common.max_input_bytes)?;
            let result = (|| {
                let document = prepare(&bytes, &args.common)?;
                let mut options = OutlineOptions::default();
                options.select = args.select;
                options.depth = args.depth;
                options.max_lines = args.max_lines;
                options.attrs = args.attrs;
                Ok(document.outline(&options)?.into_bytes())
            })();
            let bytes = result?;
            publish_timed(&bytes, args.common.output.as_deref(), Some(watchdog))
        }
    }
}
fn extract(
    contract: Contract,
    file: Option<&Path>,
    raw: bool,
    common: &Common,
    watchdog: &Watchdog,
) -> Result<(), Error> {
    if raw && contract.produces_records() {
        return Err(cli::argument_error(
            "--raw requires a value root, not a record root",
        ));
    }
    let bytes = input(file, common.max_input_bytes)?;
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
    let bytes = result?;
    publish_timed(&bytes, common.output.as_deref(), Some(watchdog))
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
            for (index, value) in values.into_iter().enumerate() {
                if let Value::String(value) = value {
                    if value.contains('\n') {
                        let mut error = Error::new(
                            ErrorCode::UnrepresentableOutput,
                            "Raw lists require one LF-free string per record. Use canonical JSON output (omit --raw).",
                        );
                        error.path = Some("$".into());
                        error.row = Some(vec![index + 1]);
                        return Err(error);
                    }
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
        Some(path) if path.as_os_str() == "-" => {
            read(&mut io::stdin().lock(), maximum, path, false)
        }
        Some(path) => read_file(path, maximum, false),
    }
}
fn read_file(path: &Path, maximum: usize, contract: bool) -> Result<Vec<u8>, Error> {
    let mut file = File::open(path).map_err(|e| {
        let mut error=io_error(ErrorCode::InputUnreadable,path,e);
        if path.to_string_lossy().starts_with("https://") || path.to_string_lossy().starts_with("http://") {error.message.push_str(" FILE is a local snapshot path. Fetch successfully into owned storage first, then pass the effective URL with --base-url and declared charset with --encoding; see the acquisition recipe in docs/contract.md.");}
        error
    })?;
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

#[cfg(unix)]
enum DescriptorTarget {
    Current(std::os::fd::RawFd),
    Path,
}

// Foreign process descriptor names are path streams, never atomic file destinations.
fn descriptor_alias(path: &Path) -> bool {
    #[cfg(not(unix))]
    {
        let _ = path;
        false
    }
    #[cfg(unix)]
    {
        descriptor_target(path).is_some()
    }
}

#[cfg(unix)]
fn descriptor_target(path: &Path) -> Option<DescriptorTarget> {
    use std::os::unix::ffi::OsStrExt;
    // Path components discard terminal separators and dots, but the OS must
    // still require a directory at these names rather than duplicate a stream.
    let spelling = path.as_os_str().as_bytes();
    if spelling.ends_with(b"/") || spelling.ends_with(b"/.") {
        return None;
    }
    let parent = path.parent()?;
    let Ok(parent) = std::fs::canonicalize(if parent.as_os_str().is_empty() {
        Path::new(".")
    } else {
        parent
    }) else {
        return None;
    };
    let name = path.file_name().and_then(|s| s.to_str())?;
    if parent == Path::new("/dev") && matches!(name, "stdin" | "stdout" | "stderr") {
        return Some(DescriptorTarget::Current(match name {
            "stdin" => 0,
            "stdout" => 1,
            _ => 2,
        }));
    }
    if !name.bytes().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let current_namespace = parent == Path::new("/dev/fd")
        || ["/proc/self/fd", "/proc/thread-self/fd"]
            .iter()
            .any(|namespace| {
                std::fs::canonicalize(namespace).is_ok_and(|current| parent == current)
            });
    if current_namespace {
        return Some(
            name.parse::<std::os::fd::RawFd>()
                .ok()
                .filter(|fd| fd.to_string() == name)
                .map_or(DescriptorTarget::Path, DescriptorTarget::Current),
        );
    }
    let parts = parent.iter().filter_map(|p| p.to_str()).collect::<Vec<_>>();
    matches!(
        parts.as_slice(),
        ["/", "proc", _, "fd"] | ["/", "proc", _, "task", _, "fd"]
    )
    .then_some(DescriptorTarget::Path)
}

#[cfg(unix)]
fn duplicate_output_descriptor(
    fd: std::os::fd::RawFd,
) -> io::Result<filedescriptor::FileDescriptor> {
    struct DescriptorNumber(std::os::fd::RawFd);
    impl filedescriptor::AsRawFileDescriptor for DescriptorNumber {
        fn as_raw_file_descriptor(&self) -> filedescriptor::RawFileDescriptor {
            self.0
        }
    }
    filedescriptor::FileDescriptor::dup(&DescriptorNumber(fd)).map_err(|error| match error {
        filedescriptor::Error::Dup { source, .. }
        | filedescriptor::Error::Fcntl(source)
        | filedescriptor::Error::Cloexec(source) => source,
        other => io::Error::other(other),
    })
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
    publish_timed(bytes, destination, None)
}
fn publish_timed(
    bytes: &[u8],
    destination: Option<&Path>,
    watchdog: Option<&Watchdog>,
) -> Result<(), Error> {
    let Some(path) = destination.filter(|path| path.as_os_str() != "-") else {
        return write_output(&mut io::stdout().lock(), bytes, Path::new("-"));
    };
    #[cfg(unix)]
    let resolved =
        resolve_output_links(path).map_err(|e| io_error(ErrorCode::OutputUnwritable, path, e))?;
    #[cfg(unix)]
    if let Some(DescriptorTarget::Current(fd)) = descriptor_target(&resolved) {
        let mut stream = duplicate_output_descriptor(fd)
            .map_err(|e| io_error(ErrorCode::OutputUnwritable, path, e))?;
        // Manual link resolution identifies the stream but cannot admit a path
        // beyond the kernel's traversal limit. Lookup without reopening it.
        std::fs::metadata(path).map_err(|e| io_error(ErrorCode::OutputUnwritable, path, e))?;
        return write_output(&mut stream, bytes, path);
    }
    let (resolved, pinned) = match OpenOptions::new().write(true).open(path) {
        Ok(mut file) => {
            // Windows metadata attributes do not distinguish disk files from
            // connected pipes. Inspect the existing handle before any path reopen.
            #[cfg(windows)]
            let requires_filesystem_metadata = winapi_util::file::typ(&file)
                .map_err(|e| io_error(ErrorCode::OutputUnwritable, path, e))?
                .is_disk();
            #[cfg(not(windows))]
            let requires_filesystem_metadata = true;
            if !requires_filesystem_metadata {
                return write_output(&mut file, bytes, path);
            }
            let metadata = file
                .metadata()
                .map_err(|e| io_error(ErrorCode::OutputUnwritable, path, e))?;
            if metadata.is_dir() {
                return Err(io_error(
                    ErrorCode::OutputUnwritable,
                    path,
                    io::Error::new(io::ErrorKind::IsADirectory, "destination is a directory"),
                ));
            }
            if !metadata.is_file() {
                return write_output(&mut file, bytes, path);
            }
            #[cfg(not(unix))]
            let resolved = resolve_output_links(path)
                .map_err(|e| io_error(ErrorCode::OutputUnwritable, path, e))?;
            if descriptor_alias(&resolved) {
                return write_output(&mut file, bytes, path);
            }
            let resolved = std::fs::canonicalize(resolved)
                .map_err(|e| io_error(ErrorCode::OutputUnwritable, path, e))?;
            let policy =
                Policy::read(&file).map_err(|e| io_error(ErrorCode::OutputUnwritable, path, e))?;
            let id = identity(&file).map_err(|e| io_error(ErrorCode::OutputUnwritable, path, e))?;
            (resolved, Some((file, id, policy)))
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            #[cfg(not(unix))]
            let resolved = resolve_output_links(path)
                .map_err(|e| io_error(ErrorCode::OutputUnwritable, path, e))?;
            let spelling = path.as_os_str().to_string_lossy();
            if spelling.ends_with('/')
                || spelling.ends_with("/.")
                || resolved.as_os_str().to_string_lossy().ends_with('/')
                || resolved.as_os_str().to_string_lossy().ends_with("/.")
            {
                return Err(io_error(ErrorCode::OutputUnwritable, path, error));
            }
            if descriptor_alias(&resolved) {
                return Err(io_error(ErrorCode::OutputUnwritable, path, error));
            }
            (resolved, None)
        }
        Err(error) => return Err(io_error(ErrorCode::OutputUnwritable, path, error)),
    };
    if resolved.file_name().is_none() {
        return Err(io_error(
            ErrorCode::OutputUnwritable,
            path,
            io::Error::new(io::ErrorKind::InvalidInput, "output path has no file name"),
        ));
    }
    let parent = resolved
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let parent_handle = crate::authorization::open_parent(parent);
    let parent_handle =
        parent_handle.map_err(|e| io_error(ErrorCode::OutputUnwritable, path, e))?;
    let parent_id =
        identity(&parent_handle).map_err(|e| io_error(ErrorCode::OutputUnwritable, path, e))?;
    // An absent destination follows ordinary inherited/umask policy, captured on an
    // empty policy probe. It never contains extraction bytes.
    let new_policy = if pinned.is_none() {
        let probe = create_stage(parent, watchdog, StageKind::InheritedPolicyProbe)
            .map_err(|e| io_error(ErrorCode::OutputUnwritable, path, e))?;
        Some(
            Policy::read(probe.as_file())
                .map_err(|e| io_error(ErrorCode::OutputUnwritable, path, e))?,
        )
    } else {
        None
    };
    #[cfg(unix)]
    let directory = create_private_directory(parent, watchdog)
        .map_err(|e| io_error(ErrorCode::OutputUnwritable, path, e))?;
    #[cfg(unix)]
    let stage_parent = directory.path();
    #[cfg(windows)]
    let stage_parent = parent;
    let policy = pinned
        .as_ref()
        .map(|(_, _, policy)| policy)
        .unwrap_or_else(|| new_policy.as_ref().unwrap());
    #[cfg(windows)]
    {
        // A retained auto-inherited ACL can predate a move between parents.
        // Native inheritance reconstruction may then add permissions. Verify
        // reproduction on a distinct empty inode before any result bytes;
        // readers retained from this probe can never observe the real result.
        let probe = create_stage(
            stage_parent,
            watchdog,
            StageKind::ReproductionPolicyProbe(policy),
        )
        .map_err(|e| io_error(ErrorCode::OutputUnwritable, path, e))?;
        policy.preflight(probe.as_file()).map_err(|error| {
            io_error(
                ErrorCode::OutputUnwritable,
                path,
                io::Error::new(
                    error.kind(),
                    format!("Cannot reproduce output authorization on empty policy probe: {error}"),
                ),
            )
        })?;
        #[cfg(test)]
        checkpoint("policy-probe", probe.path());
    }
    let mut temporary = create_stage(stage_parent, watchdog, StageKind::Result(policy))
        .map_err(|e| io_error(ErrorCode::OutputUnwritable, path, e))?;
    policy.prepare(temporary.as_file()).map_err(|e| {
        io_error(
            ErrorCode::OutputUnwritable,
            path,
            io::Error::new(
                e.kind(),
                format!("Cannot establish private staging authorization: {e}"),
            ),
        )
    })?;
    #[cfg(test)]
    checkpoint("restricted", temporary.path());
    temporary
        .write_all(bytes)
        .and_then(|()| temporary.flush())
        .map_err(|e| io_error(ErrorCode::OutputUnwritable, path, e))?;
    #[cfg(test)]
    checkpoint("filled", temporary.path());
    policy
        .apply(
            temporary.as_file(),
            #[cfg(test)]
            temporary.path(),
        )
        .map_err(|e| io_error(ErrorCode::OutputUnwritable, path, e))?;
    #[cfg(test)]
    checkpoint("policy", temporary.path());
    // Recheck target identity, handle-derived policy and containing directory before the commit claim.
    let current_parent = crate::authorization::open_parent(parent);
    let current_parent =
        current_parent.map_err(|e| io_error(ErrorCode::OutputUnwritable, path, e))?;
    if identity(&current_parent).map_err(|e| io_error(ErrorCode::OutputUnwritable, path, e))?
        != parent_id
    {
        return Err(io_error(
            ErrorCode::OutputUnwritable,
            path,
            io::Error::other("publication parent identity changed during staging"),
        ));
    }
    match (&pinned, OpenOptions::new().write(true).open(&resolved)) {
        (Some((file, id, policy)), Ok(current)) => {
            if identity(&current).map_err(|e| io_error(ErrorCode::OutputUnwritable, path, e))?
                != *id
                || Policy::read(file).map_err(|e| io_error(ErrorCode::OutputUnwritable, path, e))?
                    != *policy
            {
                return Err(io_error(
                    ErrorCode::OutputUnwritable,
                    path,
                    io::Error::other(
                        "destination identity or authorization changed during staging",
                    ),
                ));
            }
        }
        (None, Err(e)) if e.kind() == io::ErrorKind::NotFound => (),
        _ => {
            return Err(io_error(
                ErrorCode::OutputUnwritable,
                path,
                io::Error::other("destination changed during staging"),
            ));
        }
    }
    let current_resolution =
        resolve_output_links(path).map_err(|e| io_error(ErrorCode::OutputUnwritable, path, e))?;
    let resolution_parent = current_resolution
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let expected_parent = std::fs::canonicalize(parent)
        .map_err(|e| io_error(ErrorCode::OutputUnwritable, path, e))?;
    let observed_parent = std::fs::canonicalize(resolution_parent)
        .map_err(|e| io_error(ErrorCode::OutputUnwritable, path, e))?;
    if observed_parent != expected_parent || current_resolution.file_name() != resolved.file_name()
    {
        return Err(io_error(
            ErrorCode::OutputUnwritable,
            path,
            io::Error::other("output path resolution changed during staging"),
        ));
    }
    if Policy::read(temporary.as_file())
        .map_err(|e| io_error(ErrorCode::OutputUnwritable, path, e))?
        != *policy
    {
        return Err(io_error(
            ErrorCode::OutputUnwritable,
            path,
            io::Error::other("staging authorization changed before commit"),
        ));
    }
    if identity_path(temporary.path())
        .map_err(|e| io_error(ErrorCode::OutputUnwritable, path, e))?
        != identity(temporary.as_file())
            .map_err(|e| io_error(ErrorCode::OutputUnwritable, path, e))?
    {
        return Err(io_error(
            ErrorCode::OutputUnwritable,
            path,
            io::Error::other(
                "staging identity changed before commit; foreign replacement left untouched",
            ),
        ));
    }
    #[cfg(test)]
    checkpoint("precommit", temporary.path());
    if let Some(watchdog) = watchdog {
        watchdog.commit();
    }
    #[cfg(test)]
    checkpoint("commit", temporary.path());
    #[cfg(unix)]
    crate::authorization::commit(
        &parent_handle,
        temporary.path(),
        &resolved,
        &directory.handle,
    )
    .map_err(|e| io_error(ErrorCode::OutputUnwritable, path, e))?;
    #[cfg(windows)]
    std::fs::rename(temporary.path(), &resolved)
        .map_err(|e| io_error(ErrorCode::OutputUnwritable, path, e))?;
    Ok(())
}

struct StageOwnership {
    path: std::path::PathBuf,
    created_identity: std::sync::OnceLock<crate::authorization::Identity>,
    creation_finished: std::sync::atomic::AtomicBool,
    #[cfg(unix)]
    directory: bool,
    #[cfg(windows)]
    deletion_handle: std::sync::Mutex<Option<File>>,
}
impl StageOwnership {
    fn cleanup(&self) {
        let owned = self
            .created_identity
            .get()
            .is_some_and(|id| identity_path(&self.path).is_ok_and(|current| current == *id));
        #[cfg(windows)]
        if let Some(file) = self.deletion_handle.lock().unwrap().take()
            && owned
        {
            let _ = crate::authorization::remove_stage(&file);
        }
        #[cfg(unix)]
        if owned {
            if self.directory {
                let _ = std::fs::remove_dir(&self.path);
            } else {
                let _ = std::fs::remove_file(&self.path);
            }
        }
    }
}
struct CreationHandoff<'a>(&'a StageOwnership);
impl Drop for CreationHandoff<'_> {
    fn drop(&mut self) {
        self.0.creation_finished.store(true, Ordering::SeqCst);
    }
}
struct OwnedStage {
    file: File,
    ownership: Arc<StageOwnership>,
}
impl OwnedStage {
    fn as_file(&self) -> &File {
        &self.file
    }
    fn path(&self) -> &Path {
        &self.ownership.path
    }
}
impl Write for OwnedStage {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.file.write(bytes)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}
impl Drop for OwnedStage {
    fn drop(&mut self) {
        self.ownership.cleanup();
    }
}
#[derive(Clone, Copy)]
enum StageKind<'a> {
    InheritedPolicyProbe,
    #[cfg(windows)]
    ReproductionPolicyProbe(&'a Policy),
    Result(&'a Policy),
}
fn create_stage(
    parent: &Path,
    watchdog: Option<&Watchdog>,
    kind: StageKind<'_>,
) -> io::Result<OwnedStage> {
    let (private, policy) = match kind {
        StageKind::InheritedPolicyProbe => (false, None),
        #[cfg(windows)]
        StageKind::ReproductionPolicyProbe(policy) => (true, Some(policy)),
        StageKind::Result(policy) => (true, Some(policy)),
    };
    #[cfg(not(windows))]
    let _ = policy;
    let mut ownership = None;
    let mut builder = tempfile::Builder::new();
    builder.disable_cleanup(true);
    let temporary = builder.make_in(parent, |candidate| {
        let record = Arc::new(StageOwnership {
            path: candidate.to_path_buf(),
            created_identity: std::sync::OnceLock::new(),
            creation_finished: std::sync::atomic::AtomicBool::new(false),
            #[cfg(unix)]
            directory: false,
            #[cfg(windows)]
            deletion_handle: std::sync::Mutex::new(None),
        });
        let _handoff = CreationHandoff(&record);
        if let Some(watchdog) = watchdog {
            watchdog.reserve(Arc::clone(&record));
            watchdog.check();
        }
        #[cfg(test)]
        if matches!(kind, StageKind::Result(_)) {
            checkpoint("reserved", candidate);
        }
        #[cfg(unix)]
        let file = {
            use std::os::unix::fs::OpenOptionsExt;
            OpenOptions::new()
                .read(true)
                .write(true)
                .create_new(true)
                .mode(if private { 0 } else { 0o666 })
                .open(candidate)
        }?;
        #[cfg(windows)]
        let file = match policy {
            Some(policy) => crate::authorization::create_private_file(candidate, policy),
            None => crate::authorization::stage_options().open(candidate),
        }?;
        let id = identity(&file).map_err(|error| {
            #[cfg(windows)]
            let _ = crate::authorization::remove_stage(&file);
            io::Error::new(
                error.kind(),
                format!(
                    "Cannot confirm created staging identity at {}: {error}; unconfirmed Unix objects are left untouched",
                    candidate.display()
                ),
            )
        })?;
        #[cfg(windows)]
        {
            match file.try_clone() {
                Ok(handle) => *record.deletion_handle.lock().unwrap() = Some(handle),
                Err(error) => {
                    let _ = crate::authorization::remove_stage(&file);
                    return Err(error);
                }
            }
        }
        record.created_identity.set(id).unwrap();
        ownership = Some(Arc::clone(&record));
        #[cfg(test)]
        if matches!(kind, StageKind::Result(_)) {
            checkpoint("created", candidate);
        }
        if let Some(watchdog) = watchdog {
            watchdog.check();
        }
        Ok(file)
    })?;
    let (file, path) = temporary.into_parts();
    drop(path); // Cleanup is identity checked, never pathname-only.
    let stage = OwnedStage {
        file,
        ownership: ownership.unwrap(),
    };
    if private {
        crate::authorization::restrict(stage.as_file())?;
    }
    Ok(stage)
}
#[cfg(unix)]
struct PrivateDirectory {
    handle: File,
    ownership: Arc<StageOwnership>,
}
#[cfg(unix)]
impl PrivateDirectory {
    fn path(&self) -> &Path {
        &self.ownership.path
    }
}
#[cfg(unix)]
impl Drop for PrivateDirectory {
    fn drop(&mut self) {
        self.ownership.cleanup();
    }
}
#[cfg(unix)]
fn create_private_directory(
    parent: &Path,
    watchdog: Option<&Watchdog>,
) -> io::Result<PrivateDirectory> {
    let mut builder = tempfile::Builder::new();
    builder.disable_cleanup(true);
    let directory = builder.make_in(parent, |candidate| {
        let ownership = Arc::new(StageOwnership {
            path: candidate.to_path_buf(),
            created_identity: std::sync::OnceLock::new(),
            creation_finished: std::sync::atomic::AtomicBool::new(false),
            #[cfg(unix)]
            directory: true,
            #[cfg(windows)]
            deletion_handle: std::sync::Mutex::new(None),
        });
        let _handoff = CreationHandoff(&ownership);
        if let Some(watchdog) = watchdog {
            watchdog.reserve(Arc::clone(&ownership));
            watchdog.check();
        }
        std::fs::create_dir(candidate)?;
        let handle = crate::authorization::open_private_directory(candidate).map_err(|error| {
            io::Error::new(error.kind(), format!(
                "Cannot open created staging directory {}: {error}; unconfirmed directory may remain and is not removed by pathname",
                candidate.display()
            ))
        })?;
        ownership.created_identity.set(identity(&handle).map_err(|error| {
            io::Error::new(error.kind(), format!(
                "Cannot confirm created staging directory {}: {error}; unconfirmed directory may remain",
                candidate.display()
            ))
        })?).unwrap();
        let owned = PrivateDirectory {
            handle,
            ownership: Arc::clone(&ownership),
        };
        crate::authorization::private_directory(&owned.handle)?;
        if let Some(watchdog) = watchdog {
            watchdog.check();
        }
        Ok(owned)
    })?;
    let (owned, path) = directory.into_parts();
    drop(path);
    Ok(owned)
}
fn cleanup_owned_stages(records: &[Arc<StageOwnership>], deadline: Instant) {
    loop {
        for owned in records.iter().rev() {
            owned.cleanup();
        }
        let pending = records.iter().any(|owned| {
            owned.created_identity.get().is_none()
                && !owned.creation_finished.load(Ordering::SeqCst)
        });
        #[cfg(test)]
        tests::complete_test_creation_after_pending_snapshot(records);
        if !pending || Instant::now() >= deadline {
            break;
        }
        std::thread::yield_now();
    }
}
fn expire(duration: Duration, staging: &Arc<std::sync::Mutex<Vec<Arc<StageOwnership>>>>) -> ! {
    let diagnostic = staging
        .try_lock()
        .ok()
        .and_then(|records| records.last().cloned());
    let cleanup = Arc::clone(staging);
    let started = Instant::now();
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let records = cleanup.lock().unwrap().clone();
        cleanup_owned_stages(&records, started + REPORT_GRACE);
        let _ = sender.send(());
    });
    let mut error = Error::limit("time", duration.as_secs_f64());
    if let Some(owned) = diagnostic {
        error.detail = Some(format!(
            "Staging candidate {}; extraordinary blocked filesystem calls can leave residue.",
            owned.path.display()
        ));
    }
    report(&error);
    let _ = receiver.recv_timeout(REPORT_GRACE.saturating_sub(started.elapsed()));
    std::process::exit(4);
}
#[cfg(test)]
pub(crate) fn checkpoint(phase: &str, path: &Path) {
    if std::env::var("HTMLCUT_PUBLICATION_PHASE").is_ok_and(|selected| selected == phase) {
        let ready =
            std::path::PathBuf::from(std::env::var_os("HTMLCUT_PUBLICATION_READY").unwrap());
        let mut pending_name = ready.file_name().unwrap().to_os_string();
        pending_name.push(".pending");
        let pending = ready.with_file_name(pending_name);
        std::fs::write(&pending, path.as_os_str().as_encoded_bytes()).unwrap();
        std::fs::rename(&pending, &ready).unwrap();
        if phase == "policy-probe" || std::env::var_os("HTMLCUT_PUBLICATION_RELEASE").is_some() {
            let release =
                std::path::PathBuf::from(std::env::var_os("HTMLCUT_PUBLICATION_RELEASE").unwrap());
            let started = Instant::now();
            while !release.exists() && started.elapsed() < Duration::from_secs(2) {
                std::thread::sleep(Duration::from_millis(1));
            }
            assert!(
                release.exists(),
                "policy probe reader did not release publication"
            );
        } else if phase == "commit" {
            std::thread::sleep(Duration::from_millis(800));
        } else {
            loop {
                std::thread::park();
            }
        }
    }
}

struct Watchdog {
    state: Arc<AtomicU8>,
    started: Instant,
    duration: Duration,
    staging: Arc<std::sync::Mutex<Vec<Arc<StageOwnership>>>>,
}
impl Watchdog {
    fn new(duration: Duration) -> Self {
        let state = Arc::new(AtomicU8::new(ACTIVE));
        let started = Instant::now();
        let timer_state = Arc::clone(&state);
        let staging = Arc::new(std::sync::Mutex::new(Vec::new()));
        let timer_staging = Arc::clone(&staging);
        std::thread::spawn(move || {
            std::thread::sleep(duration);
            if timer_state
                .compare_exchange(ACTIVE, EXPIRED, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                expire(duration, &timer_staging);
            }
        });
        Self {
            state,
            started,
            duration,
            staging,
        }
    }
    fn reserve(&self, ownership: Arc<StageOwnership>) {
        self.staging.lock().unwrap().push(ownership);
    }
    fn check(&self) {
        if self.started.elapsed() >= self.duration
            && self
                .state
                .compare_exchange(ACTIVE, EXPIRED, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
        {
            expire(self.duration, &self.staging);
        }
        if self.state.load(Ordering::SeqCst) == EXPIRED {
            await_watchdog();
        }
    }
    fn commit(&self) {
        self.check();
        if self
            .state
            .compare_exchange(
                ACTIVE,
                COMMIT_IN_PROGRESS,
                Ordering::SeqCst,
                Ordering::SeqCst,
            )
            .is_err()
        {
            await_watchdog();
        }
    }
    fn finish(&self, result: Result<(), Error>) -> Result<(), Error> {
        if result.is_ok() && self.state.load(Ordering::SeqCst) == ACTIVE {
            self.check();
        }
        let terminal = if result.is_ok() { SUCCESS } else { ERROR };
        loop {
            let previous = self.state.load(Ordering::SeqCst);
            if previous == EXPIRED {
                await_watchdog();
            }
            if self
                .state
                .compare_exchange(previous, terminal, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                return result;
            }
        }
    }
}
impl Drop for Watchdog {
    fn drop(&mut self) {
        if self
            .state
            .compare_exchange(ACTIVE, ERROR, Ordering::SeqCst, Ordering::SeqCst)
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
fn obvious_stdin_alias(path: &Path) -> bool {
    if path.as_os_str() == "-" {
        return true;
    }
    #[cfg(unix)]
    {
        let value = path.as_os_str();
        [
            "/dev/stdin",
            "/dev/fd/0",
            "/proc/self/fd/0",
            "/proc/thread-self/fd/0",
        ]
        .iter()
        .any(|known| value == *known)
            || path == Path::new(&format!("/proc/{}/fd/0", std::process::id()))
    }
    #[cfg(not(unix))]
    {
        false
    }
}
fn stdin_alias(path: &Path) -> bool {
    if path.as_os_str() == "-" {
        return true;
    }
    #[cfg(unix)]
    {
        let resolved = resolve_output_links(path).unwrap_or_else(|_| path.to_path_buf());
        matches!(
            descriptor_target(&resolved),
            Some(DescriptorTarget::Current(0))
        )
    }
    #[cfg(not(unix))]
    {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const PENDING_CREATION_CHILD: &str = "HTMLCUT_PENDING_CREATION_CHILD";

    struct TestChild<'a> {
        child: Option<std::process::Child>,
        fixture: Option<&'a mut tempfile::TempDir>,
    }
    impl TestChild<'_> {
        fn try_wait(&mut self) -> io::Result<Option<std::process::ExitStatus>> {
            self.child.as_mut().unwrap().try_wait()
        }
    }
    impl Drop for TestChild<'_> {
        fn drop(&mut self) {
            let Some(mut child) = self.child.take() else {
                return;
            };
            let pid = child.id();
            let kill_error = child.kill().err();
            let deadline = Instant::now() + Duration::from_secs(2);
            loop {
                match child.try_wait() {
                    Ok(Some(_)) => {
                        // These selected test children are leaves. Reaping
                        // closes all writers before their fixture can drop.
                        let output = child.wait_with_output();
                        eprintln!("Test child {pid} stopped and reaped: {output:?}");
                        return;
                    }
                    Ok(None) if Instant::now() < deadline => {
                        std::thread::sleep(Duration::from_millis(1))
                    }
                    _ => break,
                }
            }
            let retained = self.fixture.as_mut().map(|fixture| {
                fixture.disable_cleanup(true);
                fixture.path().to_path_buf()
            });
            eprintln!(
                "Test child {pid} cleanup unconfirmed; kill error {kill_error:?}; retained fixture {retained:?}"
            );
        }
    }
    fn wait_for_test_child(
        mut child: TestChild<'_>,
        description: &str,
        deadline: Instant,
    ) -> std::process::Output {
        loop {
            match child.try_wait() {
                Ok(Some(_)) => return child.child.take().unwrap().wait_with_output().unwrap(),
                Ok(None) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(1))
                }
                outcome => panic!("{description} did not terminate: {outcome:?}"),
            }
        }
    }

    pub(super) fn complete_test_creation_after_pending_snapshot(records: &[Arc<StageOwnership>]) {
        if std::env::var_os(PENDING_CREATION_CHILD).is_none() {
            return;
        }
        let owned = records.last().unwrap();
        if owned.created_identity.get().is_some() {
            return;
        }
        let _handoff = CreationHandoff(owned);
        #[cfg(unix)]
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&owned.path)
            .unwrap();
        #[cfg(windows)]
        let file = crate::authorization::stage_options()
            .open(&owned.path)
            .unwrap();
        owned
            .created_identity
            .set(identity(&file).unwrap())
            .unwrap();
        #[cfg(windows)]
        {
            *owned.deletion_handle.lock().unwrap() = Some(file.try_clone().unwrap());
        }
        assert_eq!(file.metadata().unwrap().len(), 0);
        std::fs::write(owned.path.with_file_name("handoff-confirmed"), b"confirmed").unwrap();
    }

    #[test]
    fn expiry_retries_cleanup_when_reserved_creation_finishes_during_grace() {
        if let Some(root) = std::env::var_os(PENDING_CREATION_CHILD) {
            let owned = Arc::new(StageOwnership {
                path: Path::new(&root).join("pending-stage"),
                created_identity: std::sync::OnceLock::new(),
                creation_finished: std::sync::atomic::AtomicBool::new(false),
                #[cfg(unix)]
                directory: false,
                #[cfg(windows)]
                deletion_handle: std::sync::Mutex::new(None),
            });
            // The first cleanup pass sees an unconfirmed reservation. The
            // test-only handoff completes after its pending snapshot, forcing
            // a second real cleanup pass within the ordinary local grace.
            let records = vec![owned];
            // Exercise retry logic synchronously with a generous test deadline,
            // independent of the hard-exit worker's 50ms scheduling window.
            cleanup_owned_stages(&records, Instant::now() + Duration::from_secs(2));
            assert!(
                !records[0].path.exists(),
                "the retry did not clean the completed handoff"
            );
            // Real expiry still establishes the process exit and diagnostic.
            expire(
                Duration::from_millis(1),
                &Arc::new(std::sync::Mutex::new(records)),
            );
        }
        let mut root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("destination"), b"old").unwrap();
        std::fs::write(root.path().join("foreign"), b"foreign").unwrap();
        let child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "run::tests::expiry_retries_cleanup_when_reserved_creation_finishes_during_grace",
                "--nocapture",
            ])
            .env(PENDING_CREATION_CHILD, root.path())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        let child = TestChild {
            child: Some(child),
            fixture: Some(&mut root),
        };
        let output = wait_for_test_child(
            child,
            "reserved creation expiry",
            Instant::now() + Duration::from_secs(3),
        );
        assert_eq!(output.status.code(), Some(4), "{output:?}");
        let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(error["error"], "limit_exceeded");
        assert_eq!(
            std::fs::read(root.path().join("handoff-confirmed")).unwrap(),
            b"confirmed"
        );
        assert_eq!(
            std::fs::read(root.path().join("destination")).unwrap(),
            b"old"
        );
        assert_eq!(
            std::fs::read(root.path().join("foreign")).unwrap(),
            b"foreign"
        );
        assert!(
            !root.path().join("pending-stage").exists(),
            "confirmed empty stage survived expiry"
        );
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 3);
    }

    #[test]
    fn delayed_timer_preserves_error_priority_and_refuses_late_success() {
        const CHILD: &str = "HTMLCUT_DELAYED_TIMER_CHILD";
        if let Ok(operation) = std::env::var(CHILD) {
            // The timer thread can be descheduled past its deadline. Construct
            // that reachable state directly, so priority does not depend on a
            // scheduler race; invoke the real methods in an expendable child.
            let watchdog = Watchdog {
                state: Arc::new(AtomicU8::new(ACTIVE)),
                started: Instant::now() - Duration::from_secs(1),
                duration: Duration::from_millis(10),
                staging: Arc::new(std::sync::Mutex::new(Vec::new())),
            };
            match operation.as_str() {
                "check" => watchdog.check(),
                "success" => watchdog.finish(Ok(())).unwrap(),
                "error" => {
                    let error = watchdog
                        .finish(Err(Error::new(
                            ErrorCode::OutputUnwritable,
                            "native publication failed before the timer claimed expiry",
                        )))
                        .unwrap_err();
                    report(&error);
                    std::process::exit(error.error.exit_status());
                }
                _ => panic!("unknown delayed timer operation"),
            }
            std::process::exit(0);
        }
        for (operation, expected_status, expected_error) in [
            ("check", 4, "limit_exceeded"),
            ("success", 4, "limit_exceeded"),
            ("error", 5, "output_unwritable"),
        ] {
            let child = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "run::tests::delayed_timer_preserves_error_priority_and_refuses_late_success",
                    "--nocapture",
                ])
                .env(CHILD, operation)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .unwrap();
            let child = TestChild {
                child: Some(child),
                fixture: None,
            };
            let output = wait_for_test_child(
                child,
                "delayed timer",
                Instant::now() + Duration::from_secs(3),
            );
            assert_eq!(output.status.code(), Some(expected_status), "{output:?}");
            let diagnostic: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
            assert_eq!(diagnostic["error"], expected_error);
            assert_eq!(output.stderr.iter().filter(|c| **c == b'\n').count(), 1);
        }
    }

    #[test]
    fn competing_stdin_arguments_keep_priority_over_a_zero_deadline() {
        const CHILD: &str = "HTMLCUT_ZERO_DEADLINE_ALIAS_CHILD";
        if let Ok(source) = std::env::var(CHILD) {
            use clap::Parser;
            let source = if source == "current-process-stdin" {
                format!("/proc/{}/fd/0", std::process::id())
            } else {
                source
            };
            let mut arguments = ["htmlcut", "run", "--from-file", "-", "--timeout", "0"]
                .map(str::to_owned)
                .to_vec();
            arguments.push(source);
            let command = cli::Cli::try_parse_from(arguments).unwrap().command;
            let error = execute(command).unwrap_err();
            report(&error);
            std::process::exit(error.error.exit_status());
        }
        let sources = vec!["-"];
        #[cfg(unix)]
        let sources = {
            let mut sources = sources;
            sources.extend(["/dev/stdin", "/dev/fd/0"]);
            #[cfg(target_os = "linux")]
            sources.extend([
                "/proc/self/fd/0",
                "/proc/thread-self/fd/0",
                "current-process-stdin",
            ]);
            sources
        };
        for source in sources {
            let child = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "run::tests::competing_stdin_arguments_keep_priority_over_a_zero_deadline",
                    "--nocapture",
                ])
                .env(CHILD, source)
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .unwrap();
            let child = TestChild {
                child: Some(child),
                fixture: None,
            };
            let output = wait_for_test_child(
                child,
                "zero-deadline stdin argument refusal",
                Instant::now() + Duration::from_secs(3),
            );
            assert_eq!(output.status.code(), Some(2), "{source}: {output:?}");
            let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
            assert_eq!(error["error"], "invalid_arguments");
        }
    }

    #[test]
    fn publication_revalidation_child() {
        let Some(destination) = std::env::var_os("HTMLCUT_REVALIDATION_DESTINATION") else {
            return;
        };
        if let Err(error) = publish(b"complete-private-result", Some(Path::new(&destination))) {
            report(&error);
            std::process::exit(error.error.exit_status());
        }
    }

    #[cfg(unix)]
    #[test]
    fn existing_unwritable_output_reports_the_native_admission_failure() {
        use std::os::unix::fs::PermissionsExt;
        let root = tempfile::tempdir().unwrap();
        let target = root.path().join("existing");
        std::fs::write(&target, b"preserve").unwrap();
        std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o444)).unwrap();
        let native = OpenOptions::new().write(true).open(&target).unwrap_err();
        assert_eq!(native.kind(), io::ErrorKind::PermissionDenied);
        let error = publish(b"replacement", Some(&target)).unwrap_err();
        assert_eq!(error.error, ErrorCode::OutputUnwritable);
        assert_eq!(error.detail.as_deref(), Some(native.to_string().as_str()));
        assert_eq!(std::fs::read(&target).unwrap(), b"preserve");
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn changed_targets_and_aliases_refuse_before_commit_without_erasing_foreign_work() {
        use std::os::unix::fs::{PermissionsExt, symlink};
        for change in [
            "identity",
            "policy",
            "appeared-readonly",
            "alias-name",
            "alias-parent",
        ] {
            let mut root = tempfile::tempdir().unwrap();
            let parent = root.path().join("parent");
            std::fs::create_dir(&parent).unwrap();
            let target = parent.join("target");
            if change != "appeared-readonly" {
                std::fs::write(&target, b"old").unwrap();
                std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o644)).unwrap();
            }
            let alternate = if change == "alias-parent" {
                let alternate_parent = root.path().join("alternate-parent");
                std::fs::create_dir(&alternate_parent).unwrap();
                alternate_parent.join("target")
            } else {
                parent.join("alternate")
            };
            let alias = root.path().join("output-alias");
            let destination = match change {
                "alias-name" => {
                    std::fs::write(&alternate, b"foreign").unwrap();
                    symlink(&target, &alias).unwrap();
                    alias.clone()
                }
                "alias-parent" => {
                    std::fs::write(&alternate, b"foreign").unwrap();
                    symlink(&parent, &alias).unwrap();
                    alias.join("target")
                }
                _ => target.clone(),
            };
            let ready = root.path().join("ready");
            let release = root.path().join("release");
            let child = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "run::tests::publication_revalidation_child",
                    "--nocapture",
                ])
                .env("HTMLCUT_REVALIDATION_DESTINATION", &destination)
                .env("HTMLCUT_PUBLICATION_PHASE", "policy")
                .env("HTMLCUT_PUBLICATION_READY", &ready)
                .env("HTMLCUT_PUBLICATION_RELEASE", &release)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .unwrap();
            let mut child = TestChild {
                child: Some(child),
                fixture: Some(&mut root),
            };
            let deadline = Instant::now() + Duration::from_secs(3);
            while !ready.exists()
                && matches!(child.try_wait(), Ok(None))
                && Instant::now() < deadline
            {
                std::thread::sleep(Duration::from_millis(1));
            }
            if !ready.exists() {
                panic!("{change} did not reach completed policy");
            }
            let stage = std::path::PathBuf::from(std::fs::read_to_string(&ready).unwrap());
            assert_eq!(std::fs::read(&stage).unwrap(), b"complete-private-result");
            match change {
                "identity" => {
                    std::fs::rename(&target, &alternate).unwrap();
                    std::fs::write(&target, b"foreign").unwrap();
                    std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o644))
                        .unwrap();
                }
                "policy" => {
                    std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o600))
                        .unwrap();
                    assert!(OpenOptions::new().write(true).open(&target).is_ok());
                }
                "appeared-readonly" => {
                    std::fs::write(&target, b"foreign").unwrap();
                    std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o444))
                        .unwrap();
                    assert_eq!(
                        OpenOptions::new()
                            .write(true)
                            .open(&target)
                            .unwrap_err()
                            .kind(),
                        io::ErrorKind::PermissionDenied
                    );
                }
                "alias-name" => {
                    std::fs::remove_file(&alias).unwrap();
                    symlink(&alternate, &alias).unwrap();
                }
                "alias-parent" => {
                    std::fs::remove_file(&alias).unwrap();
                    symlink(alternate.parent().unwrap(), &alias).unwrap();
                }
                _ => unreachable!(),
            }
            std::fs::write(&release, b"continue").unwrap();
            let output = wait_for_test_child(child, "publication revalidation", deadline);
            assert_eq!(output.status.code(), Some(5), "{change}: {output:?}");
            let diagnostic: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
            assert_eq!(diagnostic["error"], "output_unwritable");
            assert_eq!(
                std::fs::read(&target).unwrap(),
                if matches!(change, "identity" | "appeared-readonly") {
                    b"foreign".as_slice()
                } else {
                    b"old".as_slice()
                }
            );
            if change.starts_with("alias-") {
                assert_eq!(std::fs::read(&alternate).unwrap(), b"foreign");
            } else if change == "identity" {
                assert_eq!(std::fs::read(&alternate).unwrap(), b"old");
            }
            if change == "policy" {
                assert_eq!(
                    target.metadata().unwrap().permissions().mode() & 0o777,
                    0o600
                );
            }
            assert!(!stage.exists(), "{change} stranded its owned result stage");
            assert!(
                !stage.parent().unwrap().exists(),
                "{change} stranded its owned private directory"
            );
        }
    }

    #[test]
    fn publication_phase_child() {
        let Some(destination) = std::env::var_os("HTMLCUT_PUBLICATION_DESTINATION") else {
            return;
        };
        let duration = if std::env::var("HTMLCUT_PUBLICATION_PHASE")
            .is_ok_and(|phase| phase == "policy-probe")
        {
            Duration::from_secs(2)
        } else {
            Duration::from_millis(500)
        };
        let watchdog = Watchdog::new(duration);
        let result = publish_timed(
            b"complete-private-result",
            Some(Path::new(&destination)),
            Some(&watchdog),
        );
        let result = watchdog.finish(result);
        if let Err(error) = result {
            report(&error);
            std::process::exit(error.error.exit_status());
        }
    }
    #[cfg(windows)]
    #[test]
    fn reader_retained_from_actual_policy_probe_never_observes_result_bytes() {
        let mut root = tempfile::tempdir().unwrap();
        let destination = root.path().join("result.json");
        let ready = root.path().join("ready");
        let release = root.path().join("release");
        std::fs::write(&destination, b"old").unwrap();
        let child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "run::tests::publication_phase_child",
                "--nocapture",
            ])
            .env("HTMLCUT_PUBLICATION_PHASE", "policy-probe")
            .env("HTMLCUT_PUBLICATION_DESTINATION", &destination)
            .env("HTMLCUT_PUBLICATION_READY", &ready)
            .env("HTMLCUT_PUBLICATION_RELEASE", &release)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        let child = TestChild {
            child: Some(child),
            fixture: Some(&mut root),
        };
        let started = Instant::now();
        while !ready.exists() && started.elapsed() < Duration::from_secs(3) {
            std::thread::sleep(Duration::from_millis(1));
        }
        if !ready.exists() {
            panic!("production policy probe was not reached");
        }
        let probe = std::path::PathBuf::from(std::fs::read_to_string(&ready).unwrap());
        let mut reader = File::open(&probe).unwrap();
        let probe_id = identity(&reader).unwrap();
        assert_eq!(reader.metadata().unwrap().len(), 0);
        std::fs::write(&release, b"continue").unwrap();
        let result = wait_for_test_child(
            child,
            "released policy probe",
            started + Duration::from_secs(3),
        );
        assert!(
            result.status.success() && result.stderr.is_empty(),
            "{result:?}"
        );
        assert_eq!(
            std::fs::read(&destination).unwrap(),
            b"complete-private-result"
        );
        assert_ne!(
            identity(&File::open(&destination).unwrap()).unwrap(),
            probe_id
        );
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).unwrap();
        assert!(
            bytes.is_empty(),
            "actual empty policy probe was reused for result bytes"
        );
        assert_eq!(reader.metadata().unwrap().len(), 0);
        assert!(!probe.exists(), "actual owned policy probe was not removed");
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 3);
    }
    #[test]
    fn local_creation_handoffs_clean_owned_files_and_commit_wins_expiry() {
        for phase in [
            "reserved",
            "created",
            "restricted",
            "filled",
            "metadata",
            "policy",
            "precommit",
            "commit",
        ] {
            let mut root = tempfile::tempdir().unwrap();
            let destination = root.path().join("result.json");
            let ready = root.path().join("ready");
            std::fs::write(&destination, b"old").unwrap();
            let child = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "run::tests::publication_phase_child",
                    "--nocapture",
                ])
                .env("HTMLCUT_PUBLICATION_PHASE", phase)
                .env("HTMLCUT_PUBLICATION_DESTINATION", &destination)
                .env("HTMLCUT_PUBLICATION_READY", &ready)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .unwrap();
            let child = TestChild {
                child: Some(child),
                fixture: Some(&mut root),
            };
            let started = Instant::now();
            while !ready.exists() && started.elapsed() < Duration::from_secs(3) {
                std::thread::sleep(Duration::from_millis(1));
            }
            assert!(ready.exists(), "checkpoint {phase} was not reached");
            let stage = std::path::PathBuf::from(std::fs::read_to_string(&ready).unwrap());
            if matches!(phase, "created" | "restricted" | "filled") {
                assert!(
                    std::fs::read(&stage).is_err(),
                    "{phase} stage is readable before completed policy"
                );
            }
            let result = wait_for_test_child(
                child,
                "publication checkpoint",
                started + Duration::from_secs(3),
            );
            if phase == "commit" {
                assert!(result.status.success(), "{result:?}");
                assert_eq!(
                    std::fs::read(&destination).unwrap(),
                    b"complete-private-result"
                );
                assert!(result.stderr.is_empty());
            } else {
                assert_eq!(result.status.code(), Some(4), "{result:?}");
                assert_eq!(std::fs::read(&destination).unwrap(), b"old");
                assert_eq!(result.stderr.iter().filter(|c| **c == b'\n').count(), 1);
                assert!(!stage.exists());
            }
            assert_eq!(
                std::fs::read_dir(root.path()).unwrap().count(),
                2,
                "{phase} stranded owned staging"
            );
        }
    }
    #[test]
    fn cleanup_never_unlinks_unconfirmed_or_replaced_objects() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("collision");
        std::fs::write(&path, b"foreign").unwrap();
        let unconfirmed = StageOwnership {
            path: path.clone(),
            created_identity: std::sync::OnceLock::new(),
            creation_finished: std::sync::atomic::AtomicBool::new(false),
            #[cfg(unix)]
            directory: false,
            #[cfg(windows)]
            deletion_handle: std::sync::Mutex::new(None),
        };
        unconfirmed.cleanup();
        assert_eq!(std::fs::read(&path).unwrap(), b"foreign");
        let stage = create_stage(root.path(), None, StageKind::InheritedPolicyProbe).unwrap();
        let displaced = root.path().join("displaced");
        std::fs::rename(stage.path(), &displaced).unwrap();
        std::fs::write(stage.path(), b"foreign replacement").unwrap();
        let name = stage.path().to_path_buf();
        drop(stage);
        assert_eq!(std::fs::read(name).unwrap(), b"foreign replacement");
        std::fs::remove_file(displaced).unwrap();
    }
    #[test]
    fn output_under_a_regular_file_refuses_and_preserves_the_parent() {
        let directory = tempfile::tempdir().unwrap();
        let file = directory.path().join("file");
        std::fs::write(&file, b"unchanged").unwrap();
        let child = file.join("child");
        let expected = std::fs::symlink_metadata(&child).unwrap_err();
        if expected.kind() == io::ErrorKind::NotFound {
            // Windows reports PATH_NOT_FOUND here. Resolution does not promise
            // that a candidate's parent is a directory; publication checks that.
            assert_eq!(resolve_output_links(&child).unwrap(), child);
        } else {
            let error = resolve_output_links(&child).unwrap_err();
            assert_eq!(error.kind(), expected.kind());
            assert_eq!(error.raw_os_error(), expected.raw_os_error());
        }
        let error = publish(b"new", Some(&child)).unwrap_err();
        assert_eq!(error.error, ErrorCode::OutputUnwritable);
        assert_eq!(
            error.file.as_deref(),
            Some(child.to_string_lossy().as_ref())
        );
        assert!(error.detail.is_some());
        assert_eq!(std::fs::read(file).unwrap(), b"unchanged");
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
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
                .compare_exchange(
                    ACTIVE,
                    COMMIT_IN_PROGRESS,
                    Ordering::SeqCst,
                    Ordering::SeqCst,
                )
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
