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
    pub(crate) fn apply(&self, file: &File, #[cfg(test)] stage: &Path) -> io::Result<()> {
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
        #[cfg(test)]
        crate::run::checkpoint("metadata", stage);
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
