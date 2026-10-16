// SPDX-License-Identifier: MPL-2.0
use std::{
    ffi::CString,
    fs::File,
    io,
    os::{
        fd::AsRawFd,
        unix::{ffi::OsStrExt, fs::MetadataExt},
    },
    path::Path,
};

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Identity {
    device: u64,
    inode: u64,
}
pub(crate) fn identity(file: &File) -> io::Result<Identity> {
    let m = file.metadata()?;
    Ok(Identity {
        device: m.dev(),
        inode: m.ino(),
    })
}
pub(crate) fn identity_path(path: &Path) -> io::Result<Identity> {
    let m = std::fs::symlink_metadata(path)?;
    Ok(Identity {
        device: m.dev(),
        inode: m.ino(),
    })
}
pub(crate) fn open_parent(path: &Path) -> io::Result<File> {
    #[cfg(target_os = "linux")]
    {
        use std::os::fd::FromRawFd;
        let path = CString::new(path.as_os_str().as_bytes())?;
        // musl includes O_PATH in O_ACCMODE, which OpenOptions::custom_flags
        // masks out. Request the directory capability directly, without listing.
        loop {
            // SAFETY: NUL-terminated pathname; no creation flags or mode argument.
            let descriptor = unsafe {
                libc::open(
                    path.as_ptr(),
                    libc::O_PATH | libc::O_DIRECTORY | libc::O_CLOEXEC,
                )
            };
            if descriptor >= 0 {
                // SAFETY: exclusive ownership of the newly opened descriptor.
                return Ok(unsafe { File::from_raw_fd(descriptor) });
            }
            let error = io::Error::last_os_error();
            if error.kind() != io::ErrorKind::Interrupted {
                return Err(error);
            }
        }
    }
    #[cfg(target_os = "macos")]
    {
        use std::os::unix::fs::OpenOptionsExt;
        // Pin identity and provide renameat's dirfd without requiring listing.
        // Parent aliases follow kernel resolution, as ordinary publication does.
        std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_SEARCH)
            .open(path)
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    File::open(path)
}
pub(crate) fn open_private_directory(path: &Path) -> io::Result<File> {
    #[cfg(target_os = "macos")]
    {
        use std::os::unix::fs::OpenOptionsExt;
        // Inherited denial of directory listing must not prevent the owner
        // from obtaining the handle needed to remove that inherited ACL.
        std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_SEARCH | libc::O_NOFOLLOW)
            .open(path)
    }
    #[cfg(not(target_os = "macos"))]
    File::open(path)
}
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Policy {
    owner: u32,
    group: u32,
    mode: u32,
    attributes: Vec<(&'static str, Option<Vec<u8>>)>,
    #[cfg(target_os = "macos")]
    acl: String,
}
impl Policy {
    pub(crate) fn read(file: &File) -> io::Result<Self> {
        let m = file.metadata()?;
        #[cfg(target_os = "macos")]
        let names = ["com.apple.macl"];
        #[cfg(not(target_os = "macos"))]
        let names = [
            "system.posix_acl_access",
            "security.selinux",
            "security.SMACK64",
        ];
        let attributes = names
            .into_iter()
            .map(|name| read_attribute(file, name).map(|value| (name, value)))
            .collect::<io::Result<_>>()?;
        Ok(Self {
            owner: m.uid(),
            group: m.gid(),
            mode: m.mode() & 0o777,
            attributes,
            #[cfg(target_os = "macos")]
            acl: Acl::read(file)?.text()?,
        })
    }
    pub(crate) fn prepare(&self, file: &File) -> io::Result<()> {
        let m = file.metadata()?;
        if (m.uid(), m.gid()) != (self.owner, self.group) {
            // SAFETY: owned open descriptor; uid/gid are from the pinned destination.
            syscall(unsafe { libc::fchown(file.as_raw_fd(), self.owner, self.group) })?;
        }
        #[cfg(target_os = "macos")]
        {
            // Metadata inspection needs mode read bits on Darwin. Deny data reads
            // before granting those bits, so no retained data handle can appear.
            Acl::deny_data_read()?.apply(file)?;
            // SAFETY: live descriptor, metadata-only phase under verified deny-read ACL.
            syscall(unsafe { libc::fchmod(file.as_raw_fd(), 0o600) })?;
        }
        for (name, value) in self
            .attributes
            .iter()
            .filter(|(name, _)| *name != "system.posix_acl_access")
        {
            if read_attribute(file, name)? != *value {
                write_attribute(file, name, value.as_deref())?;
            }
            if read_attribute(file, name)? != *value {
                return Err(io::Error::other(
                    "mandatory staging policy differs before result bytes",
                ));
            }
        }
        // SAFETY: mode zero keeps the staged content private while the writer retains its descriptor.
        syscall(unsafe { libc::fchmod(file.as_raw_fd(), 0) })?;
        Ok(())
    }
    pub(crate) fn apply(
        &self,
        file: &File,
        #[cfg(any(test, feature = "instrumented-tests"))] stage: &Path,
    ) -> io::Result<()> {
        // Mandatory policy/ownership are established before writing. Complete
        // content receives its final ACL before mode bits can grant pathname reads.
        #[cfg(target_os = "macos")]
        Acl::from_text(&self.acl)?.apply(file)?;
        #[cfg(not(target_os = "macos"))]
        for (name, value) in self
            .attributes
            .iter()
            .filter(|(name, _)| *name == "system.posix_acl_access")
        {
            if read_attribute(file, name)? != *value {
                write_attribute(file, name, value.as_deref())?;
            }
        }
        // SAFETY: open descriptor; discard special privilege bits and preserve the final ACL mask.
        syscall(unsafe { libc::fchmod(file.as_raw_fd(), self.mode as libc::mode_t) })?;
        #[cfg(any(test, feature = "instrumented-tests"))]
        crate::local_io::checkpoint("metadata", stage);
        if Self::read(file)? != *self {
            return Err(io::Error::other(
                "staged authorization differs from the pinned destination policy",
            ));
        }
        Ok(())
    }
}
pub(crate) fn restrict(file: &File) -> io::Result<()> {
    // Remove inherited grants on an empty file, before any result byte.
    #[cfg(target_os = "macos")]
    Acl::empty()?.apply(file)?;
    #[cfg(not(target_os = "macos"))]
    write_attribute(file, "system.posix_acl_access", None)?;
    // SAFETY: the file stays open for writing, while pathname readers get no rwx grant.
    syscall(unsafe { libc::fchmod(file.as_raw_fd(), 0) })?;
    if file.metadata()?.mode() & 0o777 != 0 {
        return Err(io::Error::other("staging rwx restriction did not hold"));
    }
    #[cfg(target_os = "macos")]
    if Acl::read(file)?.text()? != Acl::empty()?.text()? {
        return Err(io::Error::other("staging ACL restriction did not hold"));
    }
    #[cfg(not(target_os = "macos"))]
    if read_attribute(file, "system.posix_acl_access")?.is_some() {
        return Err(io::Error::other("staging inherited ACL remained"));
    }
    Ok(())
}
fn syscall(result: i32) -> io::Result<()> {
    if result < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}
fn missing(error: &io::Error) -> bool {
    #[cfg(target_os = "macos")]
    {
        error.raw_os_error() == Some(libc::ENOATTR)
    }
    #[cfg(not(target_os = "macos"))]
    {
        error.raw_os_error() == Some(libc::ENODATA)
    }
}
fn read_attribute(file: &File, name: &str) -> io::Result<Option<Vec<u8>>> {
    let name = CString::new(name).unwrap();
    // SAFETY: name is NUL terminated, null buffer explicitly queries size; file remains open.
    let size = unsafe {
        #[cfg(target_os = "macos")]
        {
            libc::fgetxattr(
                file.as_raw_fd(),
                name.as_ptr(),
                std::ptr::null_mut(),
                0,
                0,
                0,
            )
        }
        #[cfg(not(target_os = "macos"))]
        {
            libc::fgetxattr(file.as_raw_fd(), name.as_ptr(), std::ptr::null_mut(), 0)
        }
    };
    if size < 0 {
        let e = io::Error::last_os_error();
        return if missing(&e) || e.raw_os_error() == Some(libc::ENOTSUP) {
            Ok(None)
        } else {
            Err(e)
        };
    }
    let mut value = vec![0; size as usize];
    // SAFETY: the writable buffer has precisely the queried size; concurrent growth returns ERANGE.
    let read = unsafe {
        #[cfg(target_os = "macos")]
        {
            libc::fgetxattr(
                file.as_raw_fd(),
                name.as_ptr(),
                value.as_mut_ptr().cast(),
                value.len(),
                0,
                0,
            )
        }
        #[cfg(not(target_os = "macos"))]
        {
            libc::fgetxattr(
                file.as_raw_fd(),
                name.as_ptr(),
                value.as_mut_ptr().cast(),
                value.len(),
            )
        }
    };
    if read < 0 {
        return Err(io::Error::last_os_error());
    }
    value.truncate(read as usize);
    Ok(Some(value))
}
fn write_attribute(file: &File, name: &str, value: Option<&[u8]>) -> io::Result<()> {
    let name = CString::new(name).unwrap();
    // SAFETY: name and value buffers live across the native call; fd is an owned open file.
    let result = unsafe {
        if let Some(value) = value {
            #[cfg(target_os = "macos")]
            {
                libc::fsetxattr(
                    file.as_raw_fd(),
                    name.as_ptr(),
                    value.as_ptr().cast(),
                    value.len(),
                    0,
                    0,
                )
            }
            #[cfg(not(target_os = "macos"))]
            {
                libc::fsetxattr(
                    file.as_raw_fd(),
                    name.as_ptr(),
                    value.as_ptr().cast(),
                    value.len(),
                    0,
                )
            }
        } else {
            #[cfg(target_os = "macos")]
            {
                libc::fremovexattr(file.as_raw_fd(), name.as_ptr(), 0)
            }
            #[cfg(not(target_os = "macos"))]
            {
                libc::fremovexattr(file.as_raw_fd(), name.as_ptr())
            }
        }
    };
    if result < 0 {
        let e = io::Error::last_os_error();
        if value.is_none() && (missing(&e) || e.raw_os_error() == Some(libc::ENOTSUP)) {
            return Ok(());
        }
        return Err(e);
    }
    Ok(())
}
pub(crate) fn commit(
    parent: &File,
    stage: &Path,
    destination: &Path,
    stage_parent: &File,
) -> io::Result<()> {
    let stage = CString::new(stage.file_name().unwrap().as_bytes())?;
    let destination = CString::new(destination.file_name().unwrap().as_bytes())?;
    // SAFETY: both basenames are NUL terminated; rename is relative to the pinned open directory.
    syscall(unsafe {
        libc::renameat(
            stage_parent.as_raw_fd(),
            stage.as_ptr(),
            parent.as_raw_fd(),
            destination.as_ptr(),
        )
    })
}
#[cfg(target_os = "macos")]
mod acl {
    use super::*;
    type AclPointer = *mut libc::c_void;
    unsafe extern "C" {
        fn acl_get_fd_np(fd: i32, kind: i32) -> AclPointer;
        fn acl_set_fd_np(fd: i32, acl: AclPointer, kind: i32) -> i32;
        fn acl_init(count: i32) -> AclPointer;
        fn acl_to_text(acl: AclPointer, len: *mut libc::ssize_t) -> *mut libc::c_char;
        fn acl_from_text(text: *const libc::c_char) -> AclPointer;
        fn acl_create_entry(acl: *mut AclPointer, entry: *mut AclPointer) -> i32;
        fn acl_set_tag_type(entry: AclPointer, tag: i32) -> i32;
        fn acl_set_qualifier(entry: AclPointer, qualifier: *const libc::c_void) -> i32;
        fn acl_get_permset(entry: AclPointer, permissions: *mut AclPointer) -> i32;
        fn acl_add_perm(permissions: AclPointer, permission: u32) -> i32;
        fn mbr_gid_to_uuid(gid: libc::gid_t, uuid: *mut u8) -> i32;
        fn acl_free(value: *mut libc::c_void) -> i32;
    }
    pub(super) struct Acl(AclPointer);
    impl Acl {
        fn checked(value: AclPointer) -> io::Result<Self> {
            if value.is_null() {
                Err(io::Error::last_os_error())
            } else {
                Ok(Self(value))
            }
        }
        pub(super) fn read(file: &File) -> io::Result<Self> {
            // SAFETY: open fd, Darwin's ACL_TYPE_EXTENDED; successful allocation is owned by Acl.
            let value = unsafe { acl_get_fd_np(file.as_raw_fd(), 0x100) };
            if value.is_null()
                && (missing(&io::Error::last_os_error())
                    || io::Error::last_os_error().raw_os_error() == Some(libc::ENOENT))
            {
                return Self::empty();
            }
            Self::checked(value)
        }
        pub(super) fn empty() -> io::Result<Self> {
            // SAFETY: native allocator with valid zero-entry capacity.
            Self::checked(unsafe { acl_init(0) })
        }
        pub(super) fn deny_data_read() -> io::Result<Self> {
            let mut group = std::mem::MaybeUninit::<libc::group>::uninit();
            let mut found = std::ptr::null_mut();
            let mut buffer = vec![0i8; 8192];
            // SAFETY: reentrant lookup uses live writable buffers and a terminated fixed group name.
            let code = unsafe {
                libc::getgrnam_r(
                    c"everyone".as_ptr(),
                    group.as_mut_ptr(),
                    buffer.as_mut_ptr(),
                    buffer.len(),
                    &mut found,
                )
            };
            if code != 0 {
                return Err(io::Error::from_raw_os_error(code));
            }
            if found.is_null() {
                return Err(io::Error::other(
                    "cannot identify the native everyone group for private staging",
                ));
            }
            // SAFETY: successful lookup initialized group and its pointers remain backed by buffer.
            let group = unsafe { group.assume_init() };
            let mut uuid = [0u8; 16];
            // SAFETY: UUID output is a writable sixteen-byte native UUID buffer.
            let code = unsafe { mbr_gid_to_uuid(group.gr_gid, uuid.as_mut_ptr()) };
            if code != 0 {
                return Err(io::Error::from_raw_os_error(code));
            }
            let mut acl = Self::empty()?;
            let mut entry = std::ptr::null_mut();
            let mut permissions = std::ptr::null_mut();
            // SAFETY: the ACL owns native entries; qualifier/permission buffers live across the calls.
            unsafe {
                syscall(acl_create_entry(&mut acl.0, &mut entry))?;
                syscall(acl_set_tag_type(entry, 2))?; // ACL_EXTENDED_DENY
                syscall(acl_set_qualifier(entry, uuid.as_ptr().cast()))?;
                syscall(acl_get_permset(entry, &mut permissions))?;
                syscall(acl_add_perm(permissions, 2))?; // ACL_READ_DATA
            }
            Ok(acl)
        }
        pub(super) fn from_text(text: &str) -> io::Result<Self> {
            let text = CString::new(text)?;
            // SAFETY: native ACL text is NUL terminated for the duration of parsing.
            Self::checked(unsafe { acl_from_text(text.as_ptr()) })
        }
        pub(super) fn text(&self) -> io::Result<String> {
            // SAFETY: valid owned ACL; returned string is allocated and freed using acl_free.
            let ptr = unsafe { acl_to_text(self.0, std::ptr::null_mut()) };
            if ptr.is_null() {
                return Err(io::Error::last_os_error());
            }
            // SAFETY: native API returns a NUL-terminated allocated string.
            let value = unsafe { std::ffi::CStr::from_ptr(ptr) }
                .to_str()
                .map(str::to_owned)
                .map_err(io::Error::other);
            // SAFETY: pointer was allocated by acl_to_text and has not been freed.
            unsafe { acl_free(ptr.cast()) };
            value
        }
        pub(super) fn apply(&self, file: &File) -> io::Result<()> {
            // SAFETY: fd is open and ACL remains valid throughout the native setter.
            syscall(unsafe { acl_set_fd_np(file.as_raw_fd(), self.0, 0x100) })
        }
    }
    impl Drop for Acl {
        fn drop(&mut self) {
            // SAFETY: this wrapper owns exactly one native ACL allocation.
            unsafe { acl_free(self.0) };
        }
    }
}
#[cfg(target_os = "macos")]
use acl::Acl;

pub(crate) fn private_directory(file: &File) -> io::Result<()> {
    #[cfg(target_os = "macos")]
    Acl::empty()?.apply(file)?;
    #[cfg(not(target_os = "macos"))]
    {
        write_attribute(file, "system.posix_acl_access", None)?;
        write_attribute(file, "system.posix_acl_default", None)?;
    }
    let mode = 0o700 | (file.metadata()?.mode() & 0o2000);
    // SAFETY: this is an opened empty owned directory; preserve only inherited directory setgid.
    syscall(unsafe { libc::fchmod(file.as_raw_fd(), mode as libc::mode_t) })?;
    if file.metadata()?.mode() & 0o777 != 0o700 {
        return Err(io::Error::other("private staging directory mode differs"));
    }
    #[cfg(target_os = "macos")]
    if Acl::read(file)?.text()? != Acl::empty()?.text()? {
        return Err(io::Error::other("private directory inherited ACL remained"));
    }
    #[cfg(not(target_os = "macos"))]
    for name in ["system.posix_acl_access", "system.posix_acl_default"] {
        if read_attribute(file, name)?.is_some() {
            return Err(io::Error::other("private directory inherited ACL remained"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    const ATTRIBUTE: &str = "user.htmlcut.authorization-control";

    // Permission faults must not leave an unreadable fixture behind. Cleanup
    // deliberately bypasses the production policy helpers that mutants alter.
    struct DirectoryPolicyFixture {
        directory: tempfile::TempDir,
        file: File,
        restored: bool,
        #[cfg(target_os = "macos")]
        empty_acl: *mut libc::c_void,
    }

    #[cfg(target_os = "macos")]
    unsafe extern "C" {
        fn acl_init(count: i32) -> *mut libc::c_void;
        fn acl_from_text(text: *const libc::c_char) -> *mut libc::c_void;
        fn acl_set_fd_np(fd: i32, acl: *mut libc::c_void, kind: i32) -> i32;
        fn acl_free(value: *mut libc::c_void) -> i32;
    }

    impl DirectoryPolicyFixture {
        fn new() -> Self {
            let directory = tempfile::tempdir().unwrap();
            let file = open_private_directory(directory.path()).unwrap();
            #[cfg(target_os = "macos")]
            // SAFETY: allocate an independent empty native ACL before applying any grant.
            let empty_acl = unsafe { acl_init(0) };
            #[cfg(target_os = "macos")]
            assert!(!empty_acl.is_null(), "{}", io::Error::last_os_error());
            Self {
                directory,
                file,
                restored: false,
                #[cfg(target_os = "macos")]
                empty_acl,
            }
        }

        #[cfg(target_os = "macos")]
        fn allow_directory_listing_and_search(&self) {
            // Darwin resolves the named group; directory read/execute mean list/search.
            // SAFETY: fixed terminated ACL text; native allocation is freed after assignment.
            let grant = unsafe {
                acl_from_text(c"!#acl 1\ngroup::everyone::allow:read,execute\n".as_ptr())
            };
            assert!(!grant.is_null(), "{}", io::Error::last_os_error());
            // SAFETY: pinned owned directory and live native extended ACL.
            let status = unsafe { acl_set_fd_np(self.file.as_raw_fd(), grant, 0x100) };
            let error = io::Error::last_os_error();
            // SAFETY: grant is the native allocation created above.
            unsafe { acl_free(grant) };
            assert_eq!(status, 0, "{error}");
        }

        fn restore(&mut self) -> io::Result<()> {
            #[cfg(target_os = "macos")]
            // SAFETY: pinned owned directory and the independently allocated empty ACL.
            let acl_error =
                (unsafe { acl_set_fd_np(self.file.as_raw_fd(), self.empty_acl, 0x100) } != 0)
                    .then(io::Error::last_os_error);
            // SAFETY: restore traversable owner-only permissions on the pinned owned directory.
            let mode_error = (unsafe { libc::fchmod(self.file.as_raw_fd(), 0o700) } != 0)
                .then(io::Error::last_os_error);
            #[cfg(target_os = "macos")]
            if let Some(error) = acl_error {
                return Err(error);
            }
            if let Some(error) = mode_error {
                return Err(error);
            }
            self.restored = true;
            Ok(())
        }
    }

    impl Drop for DirectoryPolicyFixture {
        fn drop(&mut self) {
            if !self.restored
                && let Err(error) = self.restore()
            {
                self.directory.disable_cleanup(true);
                eprintln!(
                    "Directory policy fixture restoration failed: {error}; retained {}",
                    self.directory.path().display()
                );
            }
            #[cfg(target_os = "macos")]
            // SAFETY: this guard owns exactly this native empty ACL allocation.
            unsafe {
                acl_free(self.empty_acl)
            };
        }
    }

    #[test]
    fn native_syscall_errors_and_missing_attributes_are_distinct() {
        // SAFETY: invalid fd is rejected by the kernel; no memory is accessed.
        // A real rejected native call supplies errno; successful syscalls return zero.
        let result = unsafe { libc::fchmod(-1, 0) };
        assert_eq!(result, -1);
        assert_eq!(
            syscall(result).unwrap_err().raw_os_error(),
            Some(libc::EBADF)
        );
        syscall(0).unwrap();
        #[cfg(target_os = "macos")]
        let absent = libc::ENOATTR;
        #[cfg(not(target_os = "macos"))]
        let absent = libc::ENODATA;
        assert!(missing(&io::Error::from_raw_os_error(absent)));
        for error in [libc::EACCES, libc::EBADF, libc::ENOTSUP] {
            assert!(!missing(&io::Error::from_raw_os_error(error)));
        }
    }

    #[test]
    fn native_attributes_preserve_empty_and_nonempty_bytes_and_removal() {
        let file = tempfile::tempfile().unwrap();
        assert_eq!(read_attribute(&file, ATTRIBUTE).unwrap(), None);
        for value in [b"".as_slice(), b"\0\x01authorization\xff".as_slice()] {
            write_attribute(&file, ATTRIBUTE, Some(value)).unwrap();
            assert_eq!(
                read_attribute(&file, ATTRIBUTE).unwrap(),
                Some(value.to_vec())
            );
        }
        write_attribute(&file, ATTRIBUTE, None).unwrap();
        assert_eq!(read_attribute(&file, ATTRIBUTE).unwrap(), None);
        write_attribute(&file, ATTRIBUTE, None).unwrap();
        // An invalid attribute name is an error, never an absent attribute or a successful write.
        assert!(read_attribute(&file, "").is_err());
        assert!(write_attribute(&file, "", Some(b"value")).is_err());
        assert!(write_attribute(&file, "", None).is_err());
    }

    #[test]
    fn restriction_removes_native_grants_before_bytes() {
        let file = tempfile::tempfile().unwrap();
        file.set_permissions(std::fs::Permissions::from_mode(0o666))
            .unwrap();
        #[cfg(target_os = "macos")]
        Acl::deny_data_read().unwrap().apply(&file).unwrap();
        restrict(&file).unwrap();
        assert_eq!(file.metadata().unwrap().mode() & 0o777, 0);
        assert_eq!(file.metadata().unwrap().len(), 0);
        #[cfg(target_os = "macos")]
        assert_eq!(
            Acl::read(&file).unwrap().text().unwrap(),
            Acl::empty().unwrap().text().unwrap()
        );
    }

    #[test]
    fn preparation_establishes_attributes_privately_before_final_policy() {
        let destination = tempfile::NamedTempFile::new().unwrap();
        destination
            .as_file()
            .set_permissions(std::fs::Permissions::from_mode(0o640))
            .unwrap();
        let mut policy = Policy::read(destination.as_file()).unwrap();
        // A writable user attribute exercises the same native copy/check path without
        // claiming this host grants authority to synthesize a mandatory security label.
        policy
            .attributes
            .push((ATTRIBUTE, Some(b"pinned-label".to_vec())));
        let stage = tempfile::NamedTempFile::new().unwrap();
        write_attribute(stage.as_file(), ATTRIBUTE, Some(b"other-label")).unwrap();
        policy.prepare(stage.as_file()).unwrap();
        assert_eq!(stage.as_file().metadata().unwrap().mode() & 0o777, 0);
        assert_eq!(stage.as_file().metadata().unwrap().len(), 0);
        stage
            .as_file()
            .set_permissions(std::fs::Permissions::from_mode(0o600))
            .unwrap();
        assert_eq!(
            read_attribute(stage.as_file(), ATTRIBUTE).unwrap(),
            Some(b"pinned-label".to_vec())
        );
        policy.attributes.retain(|(name, _)| *name != ATTRIBUTE);
        policy.apply(stage.as_file(), stage.path()).unwrap();
        assert_eq!(stage.as_file().metadata().unwrap().mode() & 0o777, 0o640);
        assert_eq!(Policy::read(stage.as_file()).unwrap(), policy);
    }

    #[test]
    fn preparing_different_ownership_changes_owner_or_refuses() {
        let stage = tempfile::NamedTempFile::new().unwrap();
        let mut policy = Policy::read(stage.as_file()).unwrap();
        let original_owner = policy.owner;
        policy.owner = original_owner.checked_add(1).unwrap();
        match policy.prepare(stage.as_file()) {
            Ok(()) => assert_eq!(stage.as_file().metadata().unwrap().uid(), policy.owner),
            Err(_) => assert_eq!(stage.as_file().metadata().unwrap().uid(), original_owner),
        }
        assert_eq!(stage.as_file().metadata().unwrap().len(), 0);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn named_posix_acl_is_deferred_until_complete_bytes_and_preserved_on_publication() {
        use std::io::Write;

        let destination = tempfile::NamedTempFile::new().unwrap();
        let owner = destination.as_file().metadata().unwrap().uid();
        let named_user = if owner == 65534 { 65533 } else { 65534 };
        // Linux posix_acl_xattr_header(version=2), followed by five entries:
        // owner rw, a distinct named user r, group none, mask r, other none.
        // Author this independently of Policy and its attribute helpers.
        let mut expected = 2u32.to_le_bytes().to_vec();
        for (tag, permissions, id) in [
            (1u16, 6u16, u32::MAX),
            (2, 4, named_user),
            (4, 0, u32::MAX),
            (16, 4, u32::MAX),
            (32, 0, u32::MAX),
        ] {
            expected.extend(tag.to_le_bytes());
            expected.extend(permissions.to_le_bytes());
            expected.extend(id.to_le_bytes());
        }
        assert_eq!(expected.len(), 44);
        // SAFETY: owned descriptor, fixed terminated name and initialized byte buffer.
        let status = unsafe {
            libc::fsetxattr(
                destination.as_file().as_raw_fd(),
                c"system.posix_acl_access".as_ptr(),
                expected.as_ptr().cast(),
                expected.len(),
                0,
            )
        };
        assert_eq!(
            status,
            0,
            "native Linux POSIX ACL setup: {}",
            io::Error::last_os_error()
        );

        fn native_acl(file: &File) -> Option<Vec<u8>> {
            let mut bytes = [0u8; 44];
            // SAFETY: pinned descriptor and live writable fixed-size byte buffer.
            let count = unsafe {
                libc::fgetxattr(
                    file.as_raw_fd(),
                    c"system.posix_acl_access".as_ptr(),
                    bytes.as_mut_ptr().cast(),
                    bytes.len(),
                )
            };
            if count < 0 {
                assert_eq!(
                    io::Error::last_os_error().raw_os_error(),
                    Some(libc::ENODATA)
                );
                None
            } else {
                assert_eq!(count, 44, "nontrivial named ACL must remain present");
                Some(bytes.to_vec())
            }
        }

        assert_eq!(native_acl(destination.as_file()), Some(expected.clone()));
        assert_eq!(
            destination.as_file().metadata().unwrap().mode() & 0o777,
            0o640
        );
        let policy = Policy::read(destination.as_file()).unwrap();
        let stage = tempfile::NamedTempFile::new().unwrap();
        restrict(stage.as_file()).unwrap();
        policy.prepare(stage.as_file()).unwrap();
        assert_eq!(stage.as_file().metadata().unwrap().mode() & 0o777, 0);
        assert_eq!(native_acl(stage.as_file()), None);
        stage
            .as_file()
            .write_all(b"complete-private-result")
            .unwrap();
        policy.apply(stage.as_file(), stage.path()).unwrap();
        assert_eq!(native_acl(stage.as_file()), Some(expected.clone()));
        assert_eq!(stage.as_file().metadata().unwrap().mode() & 0o777, 0o640);
        assert_eq!(Policy::read(stage.as_file()).unwrap(), policy);

        crate::local_io::publish(b"complete-published-result", Some(destination.path())).unwrap();
        let published = File::open(destination.path()).unwrap();
        assert_eq!(native_acl(&published), Some(expected));
        assert_eq!(published.metadata().unwrap().mode() & 0o777, 0o640);
        assert_eq!(Policy::read(&published).unwrap(), policy);
        assert_eq!(
            std::fs::read(destination.path()).unwrap(),
            b"complete-published-result"
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn parent_capabilities_require_no_listing_and_reject_nondirectories_and_missing_paths() {
        use std::{
            process::{Command, Stdio},
            time::{Duration, Instant},
        };
        const CHILD: &str = "HTMLCUT_TEST_LINUX_PARENT_CAPABILITY";
        if std::env::var_os(CHILD).is_some() {
            let mut fixture = DirectoryPolicyFixture::new();
            // Unlink while still readable: the accounting walker never has an
            // inaccessible pathname, including if the following checks panic.
            std::fs::remove_dir(fixture.directory.path()).unwrap();
            fixture
                .file
                .set_permissions(std::fs::Permissions::from_mode(0o300))
                .unwrap();
            let alias = format!("/proc/self/fd/{}", fixture.file.as_raw_fd());
            let denied = File::open(&alias).expect_err(
                "Linux capability control needs an unprivileged process without DAC bypass",
            );
            assert_eq!(
                denied.raw_os_error(),
                Some(libc::EACCES),
                "Linux capability control needs an unprivileged process without DAC bypass"
            );
            let parent = open_parent(Path::new(&alias)).unwrap();
            assert_eq!(identity(&parent).unwrap(), identity(&fixture.file).unwrap());
            // SAFETY: query the live owned descriptor, without altering flags.
            let flags = unsafe { libc::fcntl(parent.as_raw_fd(), libc::F_GETFD) };
            assert!(flags >= 0);
            assert_ne!(
                flags & libc::FD_CLOEXEC,
                0,
                "parent capability must not leak across exec"
            );
            fixture.restore().unwrap();

            let regular = tempfile::NamedTempFile::new().unwrap();
            let alias = format!("/proc/self/fd/{}", regular.as_file().as_raw_fd());
            assert_eq!(
                open_parent(Path::new(&alias)).unwrap_err().raw_os_error(),
                Some(libc::ENOTDIR)
            );
            let missing = regular.path().with_extension("missing-parent");
            assert_eq!(
                open_parent(&missing).unwrap_err().raw_os_error(),
                Some(libc::ENOENT)
            );
            return;
        }

        let mut owned = tempfile::tempdir().unwrap();
        let test = format!(
            "{}::parent_capabilities_require_no_listing_and_reject_nondirectories_and_missing_paths",
            module_path!().split_once("::").unwrap().1
        );
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", &test, "--nocapture"])
            .env(CHILD, "1")
            .env("TMPDIR", owned.path())
            .env_remove("RUST_BACKTRACE")
            .env_remove("RUST_LIB_BACKTRACE")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(2);
        let result = loop {
            match child.try_wait() {
                Ok(Some(status)) => break Ok(status),
                Ok(None) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(5))
                }
                Err(error) => {
                    owned.disable_cleanup(true);
                    panic!(
                        "native parent capability child identity unconfirmed: {error}; no signals sent; retained {}",
                        owned.path().display()
                    );
                }
                other => {
                    break Err(format!(
                        "native parent capability probe exceeded deadline or wait failed: {other:?}"
                    ));
                }
            }
        };
        if let Err(error) = result {
            let kill_error = child.kill().err();
            let deadline = Instant::now() + Duration::from_secs(2);
            loop {
                match child.try_wait() {
                    Ok(Some(_)) => break,
                    Ok(None) if Instant::now() < deadline => {
                        std::thread::sleep(Duration::from_millis(5))
                    }
                    _ => {
                        owned.disable_cleanup(true);
                        panic!(
                            "{error}; child cleanup unconfirmed ({kill_error:?}); retained {}",
                            owned.path().display()
                        );
                    }
                }
            }
            panic!("{error}; child stopped and reaped");
        }
        let output = child.wait_with_output().unwrap();
        assert!(output.status.success(), "{output:?}");
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert!(
            stdout.contains("running 1 test") && stdout.contains("1 passed"),
            "{stdout}"
        );
        assert_eq!(std::fs::read_dir(owned.path()).unwrap().count(), 0);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn private_directory_handle_refuses_a_symlink() {
        let directory = tempfile::tempdir().unwrap();
        let alias = directory.path().join("alias");
        std::os::unix::fs::symlink(directory.path(), &alias).unwrap();
        assert!(open_private_directory(&alias).is_err());
        assert_eq!(libc::O_SEARCH & libc::O_NOFOLLOW, 0);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn native_acl_read_preserves_present_acl_despite_stale_missing_errno() {
        let file = tempfile::tempfile().unwrap();
        let expected = Acl::deny_data_read().unwrap();
        expected.apply(&file).unwrap();
        let text = expected.text().unwrap();
        assert_ne!(text, Acl::empty().unwrap().text().unwrap());
        // A successful native call need not clear errno. A present ACL must win
        // over a stale missing-attribute error from an earlier unrelated call.
        // SAFETY: Darwin returns this thread's live errno pointer.
        unsafe {
            *libc::__error() = libc::ENOATTR;
        }
        assert_eq!(Acl::read(&file).unwrap().text().unwrap(), text);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn native_acl_allocations_are_released() {
        const MODE: &str = "HTMLCUT_TEST_NATIVE_ACL_ALLOCATION";
        if let Ok(mode) = std::env::var(MODE) {
            assert!(matches!(mode.as_str(), "drop" | "retain"));
            for _ in 0..16 {
                let acl = Acl::empty().unwrap();
                if mode == "retain" {
                    std::mem::forget(acl);
                }
            }
            return;
        }

        fn inspect(mode: &str) -> (std::process::ExitStatus, String) {
            use std::{
                io::{Read, Seek, SeekFrom},
                os::unix::process::CommandExt,
                process::{Command, Stdio},
                time::{Duration, Instant},
            };
            let diagnostics = tempfile::tempdir().unwrap();
            let mut output = tempfile::NamedTempFile::new_in(diagnostics.path()).unwrap();
            let test = format!(
                "{}::native_acl_allocations_are_released",
                module_path!().split_once("::").unwrap().1
            );
            let mut child = Command::new("/usr/bin/leaks")
                .args(["--atExit", "--"])
                .arg(std::env::current_exe().unwrap())
                .args(["--exact", &test, "--nocapture"])
                .env(MODE, mode)
                .env("LC_ALL", "C")
                // malloc(3) documents this directory override; leaks(1) documents
                // lite mode. Do not inherit full on-disk allocation history.
                .env("TMPDIR", diagnostics.path())
                .env("MallocStackLoggingDirectory", diagnostics.path())
                .env("MallocStackLogging", "lite")
                .env_remove("MallocStackLoggingNoCompact")
                .stdout(Stdio::from(output.as_file().try_clone().unwrap()))
                .stderr(Stdio::from(output.as_file().try_clone().unwrap()))
                .process_group(0)
                .spawn()
                .expect("macOS authorization tests require /usr/bin/leaks");
            // Native Intel symbolication can outlast the allocation child.
            // Detector admission and failed-run cleanup have separate budgets.
            let detector_seconds = 20;
            let deadline = Instant::now() + Duration::from_secs(detector_seconds);
            let result = (|| -> io::Result<std::process::ExitStatus> {
                loop {
                    if output.as_file().metadata()?.len() > 1024 * 1024 {
                        return Err(io::Error::other(
                            "native leak detector output exceeded 1 MiB",
                        ));
                    }
                    if let Some(status) = child.try_wait()? {
                        return Ok(status);
                    }
                    if Instant::now() >= deadline {
                        return Err(io::Error::other(format!(
                            "native leak detector exceeded {detector_seconds} seconds"
                        )));
                    }
                    std::thread::sleep(Duration::from_millis(10));
                }
            })();
            let mut reaped = true;
            let mut group_stop_error = None;
            if result.is_err() {
                // SAFETY: the unreaped child owns this fresh process group; kill
                // both the detector and its test process before reaping the child.
                if unsafe { libc::kill(-(child.id() as i32), libc::SIGKILL) } != 0 {
                    group_stop_error = Some(io::Error::last_os_error());
                }
                let _ = child.kill();
                let cleanup_deadline = Instant::now() + Duration::from_secs(2);
                reaped = loop {
                    if matches!(child.try_wait(), Ok(Some(_))) {
                        break true;
                    }
                    if Instant::now() >= cleanup_deadline {
                        break false;
                    }
                    std::thread::sleep(Duration::from_millis(10));
                };
            }
            if !reaped || group_stop_error.is_some() {
                let retained_log = output.keep().map(|(_, path)| path);
                let retained = diagnostics.keep();
                panic!(
                    "native leak detector cleanup unconfirmed (leader reaped: {reaped}, group stop: {group_stop_error:?}); diagnostics retained at {}, log {retained_log:?}: {result:?}",
                    retained.display()
                );
            }
            output.as_file_mut().seek(SeekFrom::Start(0)).unwrap();
            let mut text = String::new();
            output
                .as_file_mut()
                .take(1024 * 1024 + 1)
                .read_to_string(&mut text)
                .unwrap();
            assert!(
                text.len() <= 1024 * 1024,
                "native leak detector output exceeded 1 MiB"
            );
            let status = result
                .unwrap_or_else(|error| panic!("native leak detector failed: {error}: {text}"));
            assert!(
                text.contains("test result: ok. 1 passed; 0 failed; 0 ignored;"),
                "allocation child did not execute exactly one passing test: {text}"
            );
            (status, text)
        }

        // Prove the native detector recognizes intentionally retained allocations
        // before accepting zero leaks from the same child with ordinary drops.
        let (retained, report) = inspect("retain");
        assert_eq!(
            retained.code(),
            Some(1),
            "positive leak control failed: {report}"
        );
        assert!(
            report.contains("acl_init"),
            "positive control did not identify native ACL allocations: {report}"
        );
        assert!(
            report.lines().any(|line| {
                let Some((_, summary)) = line
                    .strip_prefix("Process ")
                    .and_then(|line| line.split_once(": "))
                else {
                    return false;
                };
                let fields: Vec<_> = summary.split_whitespace().collect();
                fields.len() == 7
                    && fields[1..3] == ["leaks", "for"]
                    && fields[4..] == ["total", "leaked", "bytes."]
                    && fields[0].parse::<u64>().is_ok_and(|count| count > 0)
                    && fields[3].parse::<u64>().is_ok_and(|bytes| bytes > 0)
            }),
            "positive leak control has no nonzero leak summary: {report}"
        );
        let (released, report) = inspect("drop");
        assert!(
            released.success(),
            "native ACL allocations leaked: {report}"
        );
        assert!(
            report.contains(": 0 leaks for 0 total leaked bytes."),
            "native detector did not confirm zero leaks: {report}"
        );
    }

    #[test]
    fn private_directory_removes_grants_and_preserves_setgid() {
        let mut fixture = DirectoryPolicyFixture::new();
        fixture
            .file
            .set_permissions(std::fs::Permissions::from_mode(0o2777))
            .unwrap();
        #[cfg(target_os = "macos")]
        {
            fixture.allow_directory_listing_and_search();
            let before = Acl::read(&fixture.file).unwrap().text().unwrap();
            assert!(before.contains(":everyone:"), "{before}");
            assert!(before.contains(":allow:read,execute"), "{before}");
        }
        private_directory(&fixture.file).unwrap();
        let observed_mode = fixture.file.metadata().unwrap().mode() & 0o2777;
        #[cfg(target_os = "macos")]
        let observed_acl = Acl::read(&fixture.file).unwrap().text().unwrap();
        // Snapshot the policy under test before restoring; assertion failures
        // then cannot strand either grants or restrictive mutant permissions.
        fixture.restore().unwrap();
        assert_eq!(observed_mode, 0o2700);
        #[cfg(target_os = "macos")]
        assert_eq!(observed_acl, Acl::empty().unwrap().text().unwrap());
    }
}
