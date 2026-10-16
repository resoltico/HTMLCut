// SPDX-License-Identifier: MPL-2.0
//! Local acquisition and policy-preserving publication.
use crate::authorization::{Policy, identity, identity_path};
use crate::{Error, ErrorCode};
use std::{
    fs::{File, OpenOptions},
    io::{self, IsTerminal, Read, Write},
    path::Path,
    sync::{Arc, atomic::Ordering},
    time::{Duration, Instant},
};
fn invalid_request(message: &str) -> Error {
    Error::new(ErrorCode::InvalidRequest, message)
}
pub(crate) fn input(
    allow_terminal: bool,
    maximum: usize,
    context: &htmlcut_core::ExecutionContext,
) -> Result<Vec<u8>, Error> {
    if !allow_terminal && io::stdin().is_terminal() {
        return Err(invalid_request(
            "standard input is a terminal; provide a file or explicitly select standard input",
        ));
    }
    read(
        &mut io::stdin().lock(),
        maximum,
        Path::new("-"),
        false,
        context,
    )
}
pub(crate) fn read_file(
    path: &Path,
    maximum: usize,
    contract: bool,
    context: &htmlcut_core::ExecutionContext,
) -> Result<Vec<u8>, Error> {
    context.check()?;
    let mut file =
        File::open(path).map_err(|error| io_error(ErrorCode::InputUnreadable, path, error))?;
    read(&mut file, maximum, path, contract, context)
}
pub(crate) fn read(
    reader: &mut impl Read,
    maximum: usize,
    path: &Path,
    contract: bool,
    context: &htmlcut_core::ExecutionContext,
) -> Result<Vec<u8>, Error> {
    let mut bytes = Vec::new();
    let mut buffer = [0; 8192];
    loop {
        context.check()?;
        let remaining = maximum.saturating_sub(bytes.len());
        let length = remaining.saturating_add(1).min(buffer.len());
        let count = match reader.read(&mut buffer[..length]) {
            Ok(count) => count,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(io_error(ErrorCode::InputUnreadable, path, error)),
        };
        if count == 0 {
            break;
        }
        if count > remaining {
            return Err(if contract {
                Error::new(
                    ErrorCode::InvalidContract,
                    format!("The contract exceeds the {maximum}-byte grammar bound."),
                )
            } else {
                Error::limit("input_bytes", maximum)
            });
        }
        bytes.extend_from_slice(&buffer[..count]);
    }
    context.check()?;
    Ok(bytes)
}

fn io_error(code: ErrorCode, path: &Path, detail: io::Error) -> Error {
    if let Some(error) = detail.get_ref().and_then(|e| e.downcast_ref::<Error>()) {
        return error.clone();
    }
    let file = path.to_string_lossy().into_owned();
    let detail = detail.to_string();
    let mut error = Error::new(code, format!("File {file:?}: {detail}."));
    error.file = Some(file);
    error.detail = Some(detail);
    error
}
fn write_all_timed(
    writer: &mut impl Write,
    mut bytes: &[u8],
    job: Option<&crate::Job>,
) -> io::Result<()> {
    while !bytes.is_empty() {
        if let Some(job) = job {
            job.check().map_err(io::Error::other)?;
        }
        match writer.write(bytes) {
            Ok(0) => return Err(io::ErrorKind::WriteZero.into()),
            Ok(written) => bytes = &bytes[written..],
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        }
    }
    if let Some(job) = job {
        job.check().map_err(io::Error::other)?;
    }
    writer.flush()
}
fn write_output(
    writer: &mut impl Write,
    bytes: &[u8],
    path: &Path,
    job: Option<&crate::Job>,
) -> Result<crate::Delivery, Error> {
    match write_all_timed(writer, bytes, job) {
        Ok(()) => Ok(crate::Delivery::Delivered),
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => {
            Ok(crate::Delivery::Disconnected)
        }
        Err(error) => Err(io_error(ErrorCode::OutputUnwritable, path, error)),
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
enum DescriptorTarget {
    Current(std::os::fd::RawFd),
    Path,
}

// Foreign process descriptor names are path streams, never atomic file destinations.
fn descriptor_alias(path: &Path) -> bool {
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        let _ = path;
        false
    }
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        descriptor_target(path).is_some()
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
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

#[cfg(any(target_os = "macos", target_os = "linux"))]
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

fn link_metadata(path: &Path) -> io::Result<std::fs::Metadata> {
    #[cfg(test)]
    LINK_LOOKUP_PROBE.with(|probe| {
        if let Some(probe) = probe.borrow_mut().as_mut() {
            probe(path)?;
        }
        Ok::<(), io::Error>(())
    })?;
    std::fs::symlink_metadata(path)
}
#[cfg(test)]
type LinkLookupProbe = Box<dyn FnMut(&Path) -> io::Result<()>>;
#[cfg(test)]
thread_local! { static LINK_LOOKUP_PROBE: std::cell::RefCell<Option<LinkLookupProbe>> = std::cell::RefCell::new(None); }
#[cfg(test)]
pub(crate) fn with_link_lookup_probe<T>(
    probe: impl FnMut(&Path) -> io::Result<()> + 'static,
    action: impl FnOnce() -> T,
) -> T {
    struct Restore(Option<LinkLookupProbe>);
    impl Drop for Restore {
        fn drop(&mut self) {
            LINK_LOOKUP_PROBE.with(|probe| *probe.borrow_mut() = self.0.take());
        }
    }
    let _restore = Restore(LINK_LOOKUP_PROBE.with(|slot| slot.replace(Some(Box::new(probe)))));
    action()
}

fn resolve_output_links(
    path: &Path,
    context: &htmlcut_core::ExecutionContext,
) -> io::Result<std::path::PathBuf> {
    let mut current = path.to_path_buf();
    let mut visited = std::collections::HashSet::new();
    loop {
        context.check().map_err(io::Error::other)?;
        if descriptor_alias(&current) {
            return Ok(current);
        }
        match link_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                // Resolve the parent with the OS rather than collapsing `..` across
                // symlinks. The identity stays stable even if relative links grow the path.
                let parent = current
                    .parent()
                    .filter(|p| !p.as_os_str().is_empty())
                    .unwrap_or_else(|| Path::new("."));
                context.check().map_err(io::Error::other)?;
                let parent_identity = std::fs::canonicalize(parent)?;
                context.check().map_err(io::Error::other)?;
                let identity = (
                    parent_identity,
                    current.file_name().map(|name| name.to_owned()),
                );
                if !visited.insert(identity) {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "symbolic link cycle",
                    ));
                }
                context.check().map_err(io::Error::other)?;
                let target = std::fs::read_link(&current)?;
                context.check().map_err(io::Error::other)?;
                current = if target.is_absolute() {
                    target
                } else {
                    current
                        .parent()
                        .unwrap_or_else(|| Path::new("."))
                        .join(target)
                };
            }
            Ok(_) => {
                context.check().map_err(io::Error::other)?;
                return Ok(current);
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                context.check().map_err(io::Error::other)?;
                return Ok(current);
            }
            Err(error) => return Err(error),
        }
    }
}

#[cfg(test)]
pub(crate) fn publish(bytes: &[u8], destination: Option<&Path>) -> Result<crate::Delivery, Error> {
    publish_timed(bytes, destination, None)
}
pub(crate) fn publish_timed(
    bytes: &[u8],
    destination: Option<&Path>,
    job: Option<&crate::Job>,
) -> Result<crate::Delivery, Error> {
    let Some(path) = destination else {
        return write_output(&mut io::stdout().lock(), bytes, Path::new("-"), job);
    };
    let context = job
        .map(crate::Job::context)
        .unwrap_or_else(|| htmlcut_core::ExecutionContext::new(None).unwrap());
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    let resolved = resolve_output_links(path, &context)
        .map_err(|e| io_error(ErrorCode::OutputUnwritable, path, e))?;
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    if let Some(DescriptorTarget::Current(fd)) = descriptor_target(&resolved) {
        let mut stream = duplicate_output_descriptor(fd)
            .map_err(|e| io_error(ErrorCode::OutputUnwritable, path, e))?;
        // Manual link resolution identifies the stream but cannot admit a path
        // beyond the kernel's traversal limit. Lookup without reopening it.
        std::fs::metadata(path).map_err(|e| io_error(ErrorCode::OutputUnwritable, path, e))?;
        return write_output(&mut stream, bytes, path, job);
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
                return write_output(&mut file, bytes, path, job);
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
                return write_output(&mut file, bytes, path, job);
            }
            #[cfg(not(any(target_os = "macos", target_os = "linux")))]
            let resolved = resolve_output_links(path, &context)
                .map_err(|e| io_error(ErrorCode::OutputUnwritable, path, e))?;
            if descriptor_alias(&resolved) {
                return write_output(&mut file, bytes, path, job);
            }
            let resolved = std::fs::canonicalize(resolved)
                .map_err(|e| io_error(ErrorCode::OutputUnwritable, path, e))?;
            let policy =
                Policy::read(&file).map_err(|e| io_error(ErrorCode::OutputUnwritable, path, e))?;
            let id = identity(&file).map_err(|e| io_error(ErrorCode::OutputUnwritable, path, e))?;
            (resolved, Some((file, id, policy)))
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            #[cfg(not(any(target_os = "macos", target_os = "linux")))]
            let resolved = resolve_output_links(path, &context)
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
        let probe = create_stage(parent, job, StageKind::InheritedPolicyProbe)
            .map_err(|e| io_error(ErrorCode::OutputUnwritable, path, e))?;
        Some(
            Policy::read(probe.as_file())
                .map_err(|e| io_error(ErrorCode::OutputUnwritable, path, e))?,
        )
    } else {
        None
    };
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    let directory = create_private_directory(parent, job)
        .map_err(|e| io_error(ErrorCode::OutputUnwritable, path, e))?;
    #[cfg(any(target_os = "macos", target_os = "linux"))]
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
            job,
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
        #[cfg(any(test, feature = "instrumented-tests"))]
        checkpoint("policy-probe", probe.path());
    }
    let mut temporary = create_stage(stage_parent, job, StageKind::Result(policy))
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
    #[cfg(feature = "instrumented-tests")]
    if std::env::var_os("HTMLCUT_TEST_EARLY_COMPLETION").is_some()
        && let Some(job) = job
    {
        let _ = job.finish(Ok(()));
    }
    #[cfg(any(test, feature = "instrumented-tests"))]
    checkpoint("restricted", temporary.path());
    write_all_timed(&mut temporary, bytes, job)
        .map_err(|e| io_error(ErrorCode::OutputUnwritable, path, e))?;
    #[cfg(any(test, feature = "instrumented-tests"))]
    checkpoint("filled", temporary.path());
    policy
        .apply(
            temporary.as_file(),
            #[cfg(any(test, feature = "instrumented-tests"))]
            temporary.path(),
        )
        .map_err(|e| io_error(ErrorCode::OutputUnwritable, path, e))?;
    #[cfg(any(test, feature = "instrumented-tests"))]
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
    let current_resolution = resolve_output_links(path, &context)
        .map_err(|e| io_error(ErrorCode::OutputUnwritable, path, e))?;
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
    #[cfg(any(test, feature = "instrumented-tests"))]
    checkpoint("precommit", temporary.path());
    if let Some(job) = job {
        job.commit()?;
    }
    #[cfg(any(test, feature = "instrumented-tests"))]
    checkpoint("commit", temporary.path());
    #[cfg(any(target_os = "macos", target_os = "linux"))]
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
    Ok(crate::Delivery::Delivered)
}

pub(crate) struct StageOwnership {
    path: std::path::PathBuf,
    created_identity: std::sync::OnceLock<crate::authorization::Identity>,
    creation_finished: std::sync::atomic::AtomicBool,
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    directory: bool,
    #[cfg(windows)]
    deletion_handle: std::sync::Mutex<Option<File>>,
}
impl StageOwnership {
    pub(crate) fn candidate_path(&self) -> &Path {
        &self.path
    }
    fn cleanup(&self) {
        #[cfg(any(test, feature = "instrumented-tests"))]
        checkpoint("cleanup", &self.path);
        let owned = self
            .created_identity
            .get()
            .is_some_and(|id| identity_path(&self.path).is_ok_and(|current| current == *id));
        #[cfg(windows)]
        let deletion_handle = self.deletion_handle.lock().unwrap().take();
        #[cfg(windows)]
        if let Some(file) = deletion_handle
            && owned
        {
            let _ = crate::authorization::remove_stage(&file);
        }
        #[cfg(any(target_os = "macos", target_os = "linux"))]
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
    job: Option<&crate::Job>,
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
            #[cfg(any(target_os = "macos", target_os = "linux"))]
            directory: false,
            #[cfg(windows)]
            deletion_handle: std::sync::Mutex::new(None),
        });
        let _handoff = CreationHandoff(&record);
        if let Some(job) = job {
            job.reserve(Arc::clone(&record));
            job.check().map_err(io::Error::other)?;
        }
        #[cfg(any(test, feature = "instrumented-tests"))]
        if matches!(kind, StageKind::Result(_)) {
            checkpoint("reserved", candidate);
        }
        #[cfg(any(target_os = "macos", target_os = "linux"))]
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
        #[cfg(any(test, feature = "instrumented-tests"))]
        if matches!(kind, StageKind::Result(_)) {
            checkpoint("created", candidate);
        }
        if let Some(job) = job {
            job.check().map_err(io::Error::other)?;
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
#[cfg(any(target_os = "macos", target_os = "linux"))]
struct PrivateDirectory {
    handle: File,
    ownership: Arc<StageOwnership>,
}
#[cfg(any(target_os = "macos", target_os = "linux"))]
impl PrivateDirectory {
    fn path(&self) -> &Path {
        &self.ownership.path
    }
}
#[cfg(any(target_os = "macos", target_os = "linux"))]
impl Drop for PrivateDirectory {
    fn drop(&mut self) {
        self.ownership.cleanup();
    }
}
#[cfg(any(target_os = "macos", target_os = "linux"))]
fn create_private_directory(
    parent: &Path,
    job: Option<&crate::Job>,
) -> io::Result<PrivateDirectory> {
    let mut builder = tempfile::Builder::new();
    builder.disable_cleanup(true);
    let directory = builder.make_in(parent, |candidate| {
        let ownership = Arc::new(StageOwnership {
            path: candidate.to_path_buf(),
            created_identity: std::sync::OnceLock::new(),
            creation_finished: std::sync::atomic::AtomicBool::new(false),
            #[cfg(any(target_os = "macos", target_os = "linux"))]
            directory: true,
            #[cfg(windows)]
            deletion_handle: std::sync::Mutex::new(None),
        });
        let _handoff = CreationHandoff(&ownership);
        if let Some(job) = job {
            job.reserve(Arc::clone(&ownership));
            job.check().map_err(io::Error::other)?;
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
        if let Some(job) = job {
            job.check().map_err(io::Error::other)?;
        }
        Ok(owned)
    })?;
    let (owned, path) = directory.into_parts();
    drop(path);
    Ok(owned)
}
pub(crate) fn cleanup_owned_stages(
    records: &[Arc<StageOwnership>],
    started: Instant,
    grace: Duration,
) {
    let deadline = started + grace;
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
#[cfg(any(test, feature = "instrumented-tests"))]
pub(crate) fn checkpoint(phase: &str, path: &Path) {
    if std::env::var("HTMLCUT_PUBLICATION_PANIC_AT").is_ok_and(|selected| selected == phase) {
        panic!("controlled panic before owned native cleanup");
    }
    if std::env::var("HTMLCUT_PUBLICATION_PHASE").is_ok_and(|selected| selected == phase) {
        let ready =
            std::path::PathBuf::from(std::env::var_os("HTMLCUT_PUBLICATION_READY").unwrap());
        if !ready.exists() {
            let mut pending = tempfile::NamedTempFile::new_in(ready.parent().unwrap()).unwrap();
            pending
                .write_all(path.as_os_str().as_encoded_bytes())
                .unwrap();
            let _ = pending.persist_noclobber(&ready);
        }
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
            let hold = std::env::var("HTMLCUT_PUBLICATION_COMMIT_HOLD_MS")
                .ok()
                .map(|value| value.parse::<u64>().unwrap())
                .unwrap_or(800);
            std::thread::sleep(Duration::from_millis(hold.min(4000)));
        } else {
            let started = Instant::now();
            while started.elapsed() < Duration::from_secs(4) {
                std::thread::sleep(Duration::from_millis(1));
            }
        }
    }
}

pub(crate) fn obvious_stdin_alias(path: &Path) -> bool {
    #[cfg(any(target_os = "macos", target_os = "linux"))]
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
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        let _ = path;
        false
    }
}
pub(crate) fn stdin_alias(
    path: &Path,
    context: &htmlcut_core::ExecutionContext,
) -> Result<bool, Error> {
    context.check()?;
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        let resolved = resolve_output_links(path, context)
            .map_err(|error| io_error(ErrorCode::InputUnreadable, path, error))?;
        context.check()?;
        Ok(matches!(
            descriptor_target(&resolved),
            Some(DescriptorTarget::Current(0))
        ))
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        let _ = path;
        Ok(false)
    }
}

#[cfg(test)]
#[path = "publication_tests.rs"]
mod tests;
