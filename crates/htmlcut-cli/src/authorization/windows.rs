// SPDX-License-Identifier: MPL-2.0
use std::{
    fs::{File, OpenOptions},
    io,
    os::windows::{fs::OpenOptionsExt, io::AsRawHandle},
    path::Path,
};
use windows_sys::Win32::{
    Foundation::{HANDLE, LocalFree},
    Security::Authorization::*,
    Security::*,
    Storage::FileSystem::*,
};
const INFO: u32 = OWNER_SECURITY_INFORMATION
    | GROUP_SECURITY_INFORMATION
    | DACL_SECURITY_INFORMATION
    | LABEL_SECURITY_INFORMATION;
struct Descriptor(PSECURITY_DESCRIPTOR);
impl Drop for Descriptor {
    fn drop(&mut self) {
        // SAFETY: Descriptor exclusively owns memory allocated by the Windows security API.
        unsafe { LocalFree(self.0) };
    }
}
impl Descriptor {
    fn read(file: &File) -> io::Result<Self> {
        let mut ptr = std::ptr::null_mut();
        // SAFETY: open file handle, valid writable pointer; other optional output pointers are null.
        status(unsafe {
            GetSecurityInfo(
                file.as_raw_handle() as HANDLE,
                SE_FILE_OBJECT,
                INFO,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut ptr,
            )
        })?;
        Ok(Self(ptr))
    }
    fn parse(sddl: &str) -> io::Result<Self> {
        let text = sddl.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
        let mut ptr = std::ptr::null_mut();
        // SAFETY: text is a terminated UTF-16 buffer; API allocates the descriptor output.
        boolean(unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                text.as_ptr(),
                1,
                &mut ptr,
                std::ptr::null_mut(),
            )
        })?;
        Ok(Self(ptr))
    }
    fn text(&self) -> io::Result<String> {
        self.text_with(INFO)
    }
    fn text_with(&self, information: u32) -> io::Result<String> {
        let mut ptr = std::ptr::null_mut();
        // SAFETY: valid descriptor and writable output buffers owned in this scope.
        boolean(unsafe {
            ConvertSecurityDescriptorToStringSecurityDescriptorW(
                self.0,
                1,
                information,
                &mut ptr,
                std::ptr::null_mut(),
            )
        })?;
        if ptr.is_null() {
            return Err(io::Error::other("security descriptor string is absent"));
        }
        // Use the documented NUL-terminated string contract; the optional size
        // is not a string length and has differed for partial component reads.
        let mut length = 0;
        // SAFETY: successful conversion returns a valid NUL-terminated allocation.
        while unsafe { *ptr.add(length) } != 0 {
            length += 1;
        }
        // SAFETY: the scan established exactly the initialized string units.
        let result = String::from_utf16(unsafe { std::slice::from_raw_parts(ptr, length) })
            .map_err(io::Error::other);
        // SAFETY: the pointer is the allocation returned by the conversion and is freed once.
        unsafe { LocalFree(ptr.cast()) };
        result
    }
    fn apply(&self, file: &File, information: u32) -> io::Result<()> {
        // Setting LABEL with a null SACL changes absence into a present null
        // SACL. Preserve an already identical mandatory policy without that
        // mutation. A differing policy still requires application and exact
        // readback by the caller; inability to reproduce it is a refusal.
        let information = if information & LABEL_SECURITY_INFORMATION != 0
            && self.text_with(LABEL_SECURITY_INFORMATION)?
                == Self::read(file)?.text_with(LABEL_SECURITY_INFORMATION)?
        {
            information & !LABEL_SECURITY_INFORMATION
        } else {
            information
        };
        let mut owner = std::ptr::null_mut();
        let mut group = std::ptr::null_mut();
        let mut dacl = std::ptr::null_mut();
        let mut sacl = std::ptr::null_mut();
        let mut present = 0;
        let mut defaulted = 0;
        let mut control = 0;
        let mut revision = 0;
        // SAFETY: descriptor is live; all output pointers are valid locals. Returned SID/ACL pointers borrow it.
        unsafe {
            boolean(GetSecurityDescriptorOwner(
                self.0,
                &mut owner,
                &mut defaulted,
            ))?;
            boolean(GetSecurityDescriptorGroup(
                self.0,
                &mut group,
                &mut defaulted,
            ))?;
            boolean(GetSecurityDescriptorDacl(
                self.0,
                &mut present,
                &mut dacl,
                &mut defaulted,
            ))?;
            boolean(GetSecurityDescriptorSacl(
                self.0,
                &mut present,
                &mut sacl,
                &mut defaulted,
            ))?;
            boolean(GetSecurityDescriptorControl(
                self.0,
                &mut control,
                &mut revision,
            ))?;
        }
        let protection = if control & SE_DACL_PROTECTED != 0 {
            PROTECTED_DACL_SECURITY_INFORMATION
        } else {
            UNPROTECTED_DACL_SECURITY_INFORMATION
        };
        // SACL protection can affect mandatory-label inheritance. Request it only
        // when present; insufficient caller rights refuse rather than silently dropping it.
        let label_protection = if control & SE_SACL_PROTECTED != 0 {
            PROTECTED_SACL_SECURITY_INFORMATION
        } else {
            0
        };
        let info = information
            | if information & DACL_SECURITY_INFORMATION != 0 {
                protection
            } else {
                0
            }
            | if information & LABEL_SECURITY_INFORMATION != 0 {
                label_protection
            } else {
                0
            };
        // SAFETY: all SID/ACL pointers originate in the live descriptor; handle is still owned by File.
        status(unsafe {
            SetSecurityInfo(
                file.as_raw_handle() as HANDLE,
                SE_FILE_OBJECT,
                info,
                owner,
                group,
                dacl,
                sacl,
            )
        })
    }
}
fn status(code: u32) -> io::Result<()> {
    if code == 0 {
        Ok(())
    } else {
        Err(io::Error::from_raw_os_error(code as i32))
    }
}
fn boolean(value: i32) -> io::Result<()> {
    if value != 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Policy(String);
impl Policy {
    pub(crate) fn read(file: &File) -> io::Result<Self> {
        Ok(Self(Descriptor::read(file)?.text()?))
    }
    pub(crate) fn prepare(&self, file: &File) -> io::Result<()> {
        let wanted = Descriptor::parse(&self.0)?;
        let information =
            OWNER_SECURITY_INFORMATION | GROUP_SECURITY_INFORMATION | LABEL_SECURITY_INFORMATION;
        wanted.apply(file, information)?;
        let expected = wanted.text_with(information)?;
        let observed = Descriptor::read(file)?.text_with(information)?;
        if expected != observed {
            return Err(io::Error::other(format!(
                "mandatory staging policy/ownership differs before result bytes: expected {expected:?}, observed {observed:?}"
            )));
        }

        Ok(())
    }
    pub(crate) fn apply(&self, file: &File, #[cfg(test)] stage: &Path) -> io::Result<()> {
        Descriptor::parse(&self.0)?.apply(file, INFO)?;
        #[cfg(test)]
        crate::run::checkpoint("metadata", stage);
        let observed = Self::read(file)?;
        if observed != *self {
            return Err(io::Error::other(format!(
                "staged authorization differs from pinned policy: expected {:?}, observed {:?}",
                self.0, observed.0
            )));
        }
        Ok(())
    }
}
pub(crate) fn restrict(file: &File) -> io::Result<()> {
    Descriptor::parse("D:P")?.apply(file, DACL_SECURITY_INFORMATION)?;
    let descriptor = Descriptor::read(file)?;
    let mut dacl = std::ptr::null_mut();
    let mut present = 0;
    let mut defaulted = 0;
    // SAFETY: valid descriptor and locals; ACL pointer remains owned by descriptor.
    unsafe {
        boolean(GetSecurityDescriptorDacl(
            descriptor.0,
            &mut present,
            &mut dacl,
            &mut defaulted,
        ))?;
    }
    // An empty protected DACL denies pathname data reads, including inherited grants.
    // SAFETY: nonnull dacl is borrowed from the live descriptor and points to a valid ACL header.
    if present == 0 || dacl.is_null() || unsafe { (*dacl).AceCount } != 0 {
        return Err(io::Error::other("staging DACL is not empty"));
    }
    Ok(())
}
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Identity {
    volume: u32,
    high: u32,
    low: u32,
}
pub(crate) fn identity(file: &File) -> io::Result<Identity> {
    let mut info = std::mem::MaybeUninit::<BY_HANDLE_FILE_INFORMATION>::uninit();
    // SAFETY: open file handle and writable storage for the documented fixed struct.
    boolean(unsafe {
        GetFileInformationByHandle(file.as_raw_handle() as HANDLE, info.as_mut_ptr())
    })?;
    // SAFETY: successful API call initialized the entire struct.
    let info = unsafe { info.assume_init() };
    Ok(Identity {
        volume: info.dwVolumeSerialNumber,
        high: info.nFileIndexHigh,
        low: info.nFileIndexLow,
    })
}
pub(crate) fn open_parent(path: &Path) -> io::Result<File> {
    OpenOptions::new()
        .read(true)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
        .open(path)
}
pub(crate) fn stage_options() -> OpenOptions {
    let mut options = OpenOptions::new();
    options.read(true).write(true).create_new(true).access_mode(
        FILE_GENERIC_READ | FILE_GENERIC_WRITE | READ_CONTROL | WRITE_DAC | WRITE_OWNER | DELETE,
    );
    options
}

pub(crate) fn identity_path(path: &Path) -> io::Result<Identity> {
    let file = OpenOptions::new()
        .read(true)
        .access_mode(0)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS)
        .open(path)?;
    identity(&file)
}

pub(crate) fn create_private_file(path: &Path) -> io::Result<File> {
    use std::os::windows::{ffi::OsStrExt, io::FromRawHandle};
    // Owner rights grant only write/metadata/delete access, never data-read access.
    // The retained writer handle then installs and verifies an empty protected DACL.
    let descriptor = Descriptor::parse("D:P(A;;0x1f0196;;;OW)")?;
    let mut attributes = SECURITY_ATTRIBUTES {
        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: descriptor.0,
        bInheritHandle: 0,
    };
    let path = path
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    // SAFETY: terminated pathname and valid descriptor live across CreateFile; CREATE_NEW never opens a collision.
    let handle = unsafe {
        CreateFileW(
            path.as_ptr(),
            FILE_READ_ATTRIBUTES
                | FILE_GENERIC_WRITE
                | READ_CONTROL
                | WRITE_DAC
                | WRITE_OWNER
                | DELETE,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            &mut attributes,
            CREATE_NEW,
            FILE_ATTRIBUTE_NORMAL,
            std::ptr::null_mut(),
        )
    };
    if handle == windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: successful CreateFile returns one uniquely owned handle, now transferred to File.
    let file = unsafe { File::from_raw_handle(handle) };
    Ok(file)
}

pub(crate) fn remove_stage(file: &File) -> io::Result<()> {
    let information = FILE_DISPOSITION_INFO_EX {
        Flags: FILE_DISPOSITION_FLAG_DELETE | FILE_DISPOSITION_FLAG_POSIX_SEMANTICS,
    };
    // SAFETY: the handle was created with DELETE and sharing; this fixed-size struct lives across the native call.
    boolean(unsafe {
        SetFileInformationByHandle(
            file.as_raw_handle() as HANDLE,
            FileDispositionInfoEx,
            (&information as *const FILE_DISPOSITION_INFO_EX).cast(),
            std::mem::size_of::<FILE_DISPOSITION_INFO_EX>() as u32,
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptor_roundtrip_preserves_complete_sid_and_acl() {
        let text = "O:S-1-5-21-1000861503-3182281758-1041194900-513G:S-1-5-21-1000861503-3182281758-1041194900-513D:P(A;;FA;;;SY)";
        let descriptor = Descriptor::parse(text).unwrap();
        assert_eq!(descriptor.text().unwrap(), text);
        assert_eq!(
            descriptor.text_with(GROUP_SECURITY_INFORMATION).unwrap(),
            "G:S-1-5-21-1000861503-3182281758-1041194900-513"
        );
        assert_eq!(
            descriptor
                .text_with(
                    OWNER_SECURITY_INFORMATION
                        | GROUP_SECURITY_INFORMATION
                        | LABEL_SECURITY_INFORMATION
                )
                .unwrap(),
            "O:S-1-5-21-1000861503-3182281758-1041194900-513G:S-1-5-21-1000861503-3182281758-1041194900-513"
        );
        let labelled = Descriptor::parse("S:(ML;;NW;;;LW)").unwrap();
        assert_eq!(
            labelled.text_with(LABEL_SECURITY_INFORMATION).unwrap(),
            "S:(ML;;NW;;;LW)"
        );
    }

    #[test]
    fn private_stage_preserves_already_identical_mandatory_policy() {
        let directory = tempfile::tempdir().unwrap();
        let probe = stage_options()
            .open(directory.path().join("probe"))
            .unwrap();
        let policy = Policy::read(&probe).unwrap();
        let stage = create_private_file(&directory.path().join("stage")).unwrap();
        restrict(&stage).unwrap();
        let before = Descriptor::read(&stage)
            .unwrap()
            .text_with(LABEL_SECURITY_INFORMATION)
            .unwrap();
        assert_eq!(
            before,
            Descriptor::parse(&policy.0)
                .unwrap()
                .text_with(LABEL_SECURITY_INFORMATION)
                .unwrap()
        );
        policy.prepare(&stage).unwrap();
        assert_eq!(
            before,
            Descriptor::read(&stage)
                .unwrap()
                .text_with(LABEL_SECURITY_INFORMATION)
                .unwrap()
        );
        policy
            .apply(&stage, &directory.path().join("stage"))
            .unwrap();
        assert_eq!(Policy::read(&stage).unwrap(), policy);
    }
}
