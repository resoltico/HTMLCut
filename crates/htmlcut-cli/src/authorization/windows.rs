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
    System::SystemServices::{
        ACCESS_FILTER_SECURITY_INFORMATION, PROCESS_TRUST_LABEL_SECURITY_INFORMATION,
        SYSTEM_MANDATORY_LABEL_ACE_TYPE,
    },
};
const INFO: u32 = OWNER_SECURITY_INFORMATION
    | GROUP_SECURITY_INFORMATION
    | DACL_SECURITY_INFORMATION
    | LABEL_SECURITY_INFORMATION;
// These SACL components can affect access; audit-only SACLs are not queried.
const INSPECT_INFO: u32 = INFO
    | ATTRIBUTE_SECURITY_INFORMATION
    | SCOPE_SECURITY_INFORMATION
    | PROCESS_TRUST_LABEL_SECURITY_INFORMATION as u32
    | ACCESS_FILTER_SECURITY_INFORMATION as u32;
mod security_privilege;
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
                INSPECT_INFO,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut ptr,
            )
        })
        .map_err(|error| {
            io::Error::new(
                error.kind(),
                format!(
                    "Cannot inspect Windows access-bearing policy with this handle/kernel: {error}; unsupported security queries refuse publication"
                ),
            )
        })?;
        let descriptor = Self(ptr);
        descriptor.require_supported_access_sacl()?;
        Ok(descriptor)
    }
    fn require_supported_access_sacl(&self) -> io::Result<()> {
        let mut acl = std::ptr::null_mut();
        let mut present = 0;
        let mut defaulted = 0;
        // SAFETY: this descriptor is API-validated and owned; output locals live
        // across the call, and the returned ACL borrows the descriptor.
        boolean(unsafe {
            GetSecurityDescriptorSacl(self.0, &mut present, &mut acl, &mut defaulted)
        })?;
        if present == 0 || acl.is_null() {
            return Ok(());
        }
        // SAFETY: nonnull ACL is supplied by the successful security API call.
        for index in 0..u32::from(unsafe { (*acl).AceCount }) {
            let mut ace = std::ptr::null_mut();
            // SAFETY: index is inside the native ACL count; pointer borrows it.
            boolean(unsafe { GetAce(acl, index, &mut ace) })?;
            // SAFETY: successful GetAce returns a valid ACE beginning with this header.
            let kind = unsafe { (*(ace as *const ACE_HEADER)).AceType };
            if u32::from(kind) != SYSTEM_MANDATORY_LABEL_ACE_TYPE {
                return Err(io::Error::new(
                    io::ErrorKind::Unsupported,
                    format!(
                        "access-bearing SACL ACE type {kind} cannot be faithfully reproduced; resource, central, trust and filter policies require refusal"
                    ),
                ));
            }
        }
        Ok(())
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
    fn describe(&self, information: u32) -> String {
        self.text_with(information)
            .unwrap_or_else(|error| format!("unavailable ({error})"))
    }
    fn apply(&self, file: &File, information: u32) -> io::Result<()> {
        let before = Self::read(file)?;
        let mut information = information;
        // Do not request an assignment right or change descriptor controls for
        // a component whose complete native representation already matches.
        for component in [
            OWNER_SECURITY_INFORMATION,
            GROUP_SECURITY_INFORMATION,
            DACL_SECURITY_INFORMATION,
            LABEL_SECURITY_INFORMATION,
        ] {
            if information & component != 0
                && self.text_with(component)? == before.text_with(component)?
            {
                information &= !component;
            }
        }
        let mut control = 0;
        let mut revision = 0;
        // SAFETY: this owned descriptor and both output locals remain live.
        boolean(unsafe { GetSecurityDescriptorControl(self.0, &mut control, &mut revision) })?;
        let protection = if control & SE_DACL_PROTECTED != 0 {
            PROTECTED_DACL_SECURITY_INFORMATION
        } else {
            UNPROTECTED_DACL_SECURITY_INFORMATION
        };
        let mut before_control = 0;
        // SAFETY: this owned readback descriptor and output locals remain live.
        boolean(unsafe {
            GetSecurityDescriptorControl(before.0, &mut before_control, &mut revision)
        })?;
        // Preserve already identical protection without requesting an assignment
        // right again. Differing protection still requires its native right;
        // complete label/control readback is never normalized.
        let label_protection = if control & SE_SACL_PROTECTED == before_control & SE_SACL_PROTECTED
        {
            0
        } else if control & SE_SACL_PROTECTED != 0 {
            PROTECTED_SACL_SECURITY_INFORMATION
        } else {
            UNPROTECTED_SACL_SECURITY_INFORMATION
        };
        let inherited_dacl =
            information & DACL_SECURITY_INFORMATION != 0 && control & SE_DACL_AUTO_INHERITED != 0;
        let mut exact_information = information & !LABEL_SECURITY_INFORMATION;
        if inherited_dacl {
            exact_information &= !DACL_SECURITY_INFORMATION;
        }
        let exact_info = exact_information
            | if exact_information & DACL_SECURITY_INFORMATION != 0 {
                protection
            } else {
                0
            };
        // SetSecurityInfo(SE_FILE_OBJECT) regenerates inherited ACEs when
        // clearing protection, changing an existing explicit ACL. This setter
        // supplies the complete pinned descriptor through the retained regular
        // file handle. Exact readback and native parent-propagation controls are
        // required: no policy differences are normalized away.
        // SAFETY: the descriptor and owned writable file handle are live; only
        // selected owner/group/DACL components are requested.
        if exact_information != 0 {
            boolean(unsafe {
                SetKernelObjectSecurity(file.as_raw_handle() as HANDLE, exact_info, self.0)
            })
            .map_err(|error| {
                io::Error::new(
                    error.kind(),
                    format!(
                        "Windows owner/group/DACL assignment failed (mask {exact_info:#x}): {error}; wanted {}, before {}",
                        self.describe(exact_information), before.describe(exact_information)
                    ),
                )
            })?;
        }
        if inherited_dacl {
            let mut acl = std::ptr::null_mut();
            let mut present = 0;
            let mut defaulted = 0;
            // SAFETY: this descriptor owns the returned ACL and remains live.
            boolean(unsafe {
                GetSecurityDescriptorDacl(self.0, &mut present, &mut acl, &mut defaulted)
            })?;
            let dacl_info = DACL_SECURITY_INFORMATION | protection;
            // An auto-inherited source already uses native parent inheritance.
            // Its native file setter preserves that control model; the full-SD
            // setter clears AUTO_INHERITED while clearing stage protection.
            // Same-parent reconstruction must still pass complete exact
            // readback: a changed parent cannot justify extra or missing ACEs.
            // SAFETY: live file handle and descriptor-owned ACL; owner/group
            // and SACL components are not requested by this call.
            status(unsafe {
                SetSecurityInfo(
                    file.as_raw_handle() as HANDLE,
                    SE_FILE_OBJECT,
                    dacl_info,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    acl,
                    std::ptr::null_mut(),
                )
            })
            .map_err(|error| {
                io::Error::new(
                    error.kind(),
                    format!(
                        "Windows auto-inherited DACL assignment failed (mask {dacl_info:#x}, DACL present {present}, control {control:#x}): {error}; wanted {}, before {}",
                        self.describe(DACL_SECURITY_INFORMATION), before.describe(DACL_SECURITY_INFORMATION)
                    ),
                )
            })?;
        }
        if information & LABEL_SECURITY_INFORMATION != 0 {
            let mut acl = std::ptr::null_mut();
            let mut present = 0;
            let mut defaulted = 0;
            // SAFETY: the ACL borrows this live native descriptor.
            boolean(unsafe {
                GetSecurityDescriptorSacl(self.0, &mut present, &mut acl, &mut defaulted)
            })?;
            let sacl_info = LABEL_SECURITY_INFORMATION | label_protection;
            // Mandatory policy uses the maintained file setter. Its SACL
            // inheritance processing is required for labels authored through
            // that API; the full-descriptor setter loses those controls.
            // SAFETY: live file handle and descriptor-owned ACL; no audit ACE
            // payload or backup policy is queried or copied.
            status(unsafe {
                SetSecurityInfo(
                    file.as_raw_handle() as HANDLE,
                    SE_FILE_OBJECT,
                    sacl_info,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    acl,
                )
            })
            .map_err(|error| {
                io::Error::new(
                    error.kind(),
                    format!(
                        "Windows mandatory-policy assignment failed (mask {sacl_info:#x}, SACL present {present}, control {control:#x}): {error}; wanted {}, before {}",
                        self.describe(LABEL_SECURITY_INFORMATION), before.describe(LABEL_SECURITY_INFORMATION)
                    ),
                )
            })?;
        }
        Ok(())
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
        self.verify(file)
    }
    /// The caller supplies a distinct empty probe in the intended result parent.
    /// Its ACL may acquire new parent grants before a reconstruction mismatch is
    /// detected, so this object must never contain result bytes.
    pub(crate) fn preflight(&self, file: &File) -> io::Result<()> {
        self.prepare(file)?;
        Descriptor::parse(&self.0)?.apply(file, INFO)?;
        self.verify(file)
    }
    fn verify(&self, file: &File) -> io::Result<()> {
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
        .access_mode(0)
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

pub(crate) fn create_private_file(path: &Path, policy: &Policy) -> io::Result<File> {
    use std::os::windows::{ffi::OsStrExt, io::FromRawHandle};
    // Owner rights grant only write/metadata/delete access, never data-read access.
    // The retained writer handle then installs and verifies an empty protected DACL.
    // Establish mandatory policy while the discretionary descriptor grants
    // only writer/metadata/delete access. Creation can omit AUTO_INHERITED on
    // a null SACL; initialize that remaining native control on the still-empty
    // private object and verify exactly before returning it to the writer.
    let wanted = Descriptor::parse(&policy.0)?;
    let label = wanted.text_with(LABEL_SECURITY_INFORMATION)?;
    let mut control = 0;
    let mut revision = 0;
    // SAFETY: live native descriptor and valid output locals.
    boolean(unsafe { GetSecurityDescriptorControl(wanted.0, &mut control, &mut revision) })?;
    let mut sacl = std::ptr::null_mut();
    let mut present = 0;
    let mut defaulted = 0;
    // SAFETY: the descriptor remains live; all outputs are valid locals.
    boolean(unsafe {
        GetSecurityDescriptorSacl(wanted.0, &mut present, &mut sacl, &mut defaulted)
    })?;
    if present != 0 && sacl.is_null() && control & SE_SACL_PROTECTED == 0 {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            format!(
                "Cannot reproduce mandatory policy {label:?}: this implementation cannot faithfully establish unprotected null-SACL inheritance"
            ),
        ));
    }
    let descriptor = Descriptor::parse(&format!("D:P(A;;0x1f0196;;;OW){label}"))?;
    let attributes = SECURITY_ATTRIBUTES {
        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: descriptor.0,
        bInheritHandle: 0,
    };
    let wide_path = path
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let create = || {
        // SAFETY: terminated pathname and descriptor live across CreateFile;
        // CREATE_NEW never opens a collision. No publication callbacks occur.
        let handle = unsafe {
            CreateFileW(
                wide_path.as_ptr(),
                FILE_READ_ATTRIBUTES
                    | FILE_GENERIC_WRITE
                    | READ_CONTROL
                    | WRITE_DAC
                    | WRITE_OWNER
                    | DELETE,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                &attributes,
                CREATE_NEW,
                FILE_ATTRIBUTE_NORMAL,
                std::ptr::null_mut(),
            )
        };
        if handle == windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE {
            // Capture before restoring the thread token can replace LastError.
            let error = io::Error::last_os_error();
            return Err(io::Error::new(
                error.kind(),
                format!(
                    "Cannot create private file with pinned mandatory policy {label:?}: {error}"
                ),
            ));
        }
        // SAFETY: successful CreateFile returns one uniquely owned file handle.
        let file = unsafe { File::from_raw_handle(handle) };
        let initialize = || {
            let created = Descriptor::read(&file)?.text_with(LABEL_SECURITY_INFORMATION)?;
            wanted.apply(&file, LABEL_SECURITY_INFORMATION)?;
            let observed = Descriptor::read(&file)?.text_with(LABEL_SECURITY_INFORMATION)?;
            if observed != label {
                return Err(io::Error::other(format!(
                    "Private file mandatory policy differs before result bytes: wanted {label:?}, created {created:?}, observed {observed:?}"
                )));
            }
            Ok(())
        };
        if let Err(error) = initialize() {
            // Delete only the object owned by this returned DELETE-capable
            // handle, even if its pathname has been replaced in the meantime.
            let cleanup = match remove_stage(&file) {
                Ok(()) => "succeeded".to_owned(),
                Err(cleanup_error) => {
                    format!("failed: {cleanup_error}; owned empty file may remain")
                }
            };
            return Err(io::Error::new(
                error.kind(),
                format!(
                    "Cannot initialize private mandatory policy at {}: {error}; owned empty-file cleanup {cleanup}",
                    path.display()
                ),
            ));
        }
        Ok(file)
    };
    if control & SE_SACL_PROTECTED != 0 {
        // Enable only an assigned capability on a duplicate effective token,
        // solely for this thread's protected-policy creation/initialization.
        security_privilege::with_security_privilege(create)
    } else {
        create()
    }
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
    use windows_sys::Win32::System::SystemServices::ACCESS_SYSTEM_SECURITY;

    fn author_dacl(file: &File, text: &str, protection: u32) -> io::Result<()> {
        let descriptor = Descriptor::parse(text)?;
        let mut acl = std::ptr::null_mut();
        let mut present = 0;
        let mut defaulted = 0;
        // SAFETY: valid fixture descriptor/ACL and opened handle remain live.
        unsafe {
            boolean(GetSecurityDescriptorDacl(
                descriptor.0,
                &mut present,
                &mut acl,
                &mut defaulted,
            ))?;
            status(SetSecurityInfo(
                file.as_raw_handle() as HANDLE,
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION | protection,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                acl,
                std::ptr::null_mut(),
            ))
        }
    }

    #[test]
    fn access_policy_inspection_needs_no_audit_privilege() {
        let mut required = 0;
        // SAFETY: only a mask is computed; the output local is valid.
        unsafe { QuerySecurityAccessMask(INSPECT_INFO, &mut required) };
        assert_eq!(required, READ_CONTROL);
    }

    #[test]
    fn resource_policy_refuses_instead_of_silently_losing_claims() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("classified.json");
        let mut file = stage_options().open(&path).unwrap();
        use std::io::Write;
        file.write_all(b"original").unwrap();
        // Independent native resource-claim fixture, using the documented RA
        // format rather than the publication setter under test.
        let descriptor =
            Descriptor::parse("S:(RA;CI;;;;S-1-1-0;(\"Project\",TS,0,\"Windows\",\"SQL\"))")
                .unwrap();
        assert_eq!(
            descriptor
                .require_supported_access_sacl()
                .unwrap_err()
                .kind(),
            io::ErrorKind::Unsupported
        );
        let mut acl = std::ptr::null_mut();
        let mut present = 0;
        let mut defaulted = 0;
        // SAFETY: the valid fixture descriptor/ACL and file handle remain live.
        unsafe {
            boolean(GetSecurityDescriptorSacl(
                descriptor.0,
                &mut present,
                &mut acl,
                &mut defaulted,
            ))
            .unwrap();
            assert!(present != 0 && !acl.is_null());
            status(SetSecurityInfo(
                file.as_raw_handle() as HANDLE,
                SE_FILE_OBJECT,
                ATTRIBUTE_SECURITY_INFORMATION,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                acl,
            ))
            .unwrap();
        }
        assert_eq!(
            Policy::read(&file).unwrap_err().kind(),
            io::ErrorKind::Unsupported
        );
        assert_eq!(std::fs::read(&path).unwrap(), b"original");
    }

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
        let stage = create_private_file(&directory.path().join("stage"), &policy).unwrap();
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

    #[test]
    fn private_creation_preserves_label_and_null_sacl_controls() {
        for label in ["S:PAI(ML;;NW;;;LW)", "S:PAINO_ACCESS_CONTROL"] {
            let directory = tempfile::tempdir().unwrap();
            let probe = stage_options()
                .open(directory.path().join("source-policy"))
                .unwrap();
            let source = Descriptor::read(&probe).unwrap();
            let discretionary = source
                .text_with(
                    OWNER_SECURITY_INFORMATION
                        | GROUP_SECURITY_INFORMATION
                        | DACL_SECURITY_INFORMATION,
                )
                .unwrap();
            let policy = Policy(format!("{discretionary}{label}"));
            let stage_path = directory.path().join("stage");
            let stage = create_private_file(&stage_path, &policy).unwrap();
            // Independent literal control admission is checked on the actual
            // fully initialized private object, before any result byte.
            assert_eq!(
                Descriptor::read(&stage)
                    .unwrap()
                    .text_with(LABEL_SECURITY_INFORMATION)
                    .unwrap(),
                label
            );
            assert_eq!(
                std::fs::read(&stage_path).unwrap_err().kind(),
                io::ErrorKind::PermissionDenied
            );
            restrict(&stage).unwrap();
            policy.preflight(&stage).unwrap();
            assert_eq!(Policy::read(&stage).unwrap(), policy);
            assert_eq!(stage.metadata().unwrap().len(), 0);
        }
    }

    fn thread_token_identity() -> Option<(u32, i32)> {
        use windows_sys::Win32::System::Threading::{GetCurrentThread, OpenThreadToken};
        let mut handle = std::ptr::null_mut();
        // SAFETY: current thread and valid token output pointer.
        if unsafe { OpenThreadToken(GetCurrentThread(), TOKEN_QUERY, 1, &mut handle) } == 0 {
            assert_eq!(
                io::Error::last_os_error().raw_os_error(),
                Some(windows_sys::Win32::Foundation::ERROR_NO_TOKEN as i32)
            );
            return None;
        }
        use std::os::windows::io::{FromRawHandle, OwnedHandle};
        // SAFETY: successful token open returns one owned handle.
        let token = unsafe { OwnedHandle::from_raw_handle(handle) };
        let mut statistics = std::mem::MaybeUninit::<TOKEN_STATISTICS>::uninit();
        let mut returned = 0;
        // SAFETY: live query token and fixed-size native output.
        boolean(unsafe {
            GetTokenInformation(
                token.as_raw_handle() as HANDLE,
                TokenStatistics,
                statistics.as_mut_ptr().cast(),
                std::mem::size_of::<TOKEN_STATISTICS>() as u32,
                &mut returned,
            )
        })
        .unwrap();
        // SAFETY: successful query initialized the statistics structure.
        let statistics = unsafe { statistics.assume_init() };
        Some((statistics.TokenId.LowPart, statistics.TokenId.HighPart))
    }

    #[test]
    fn unprotected_null_sacl_refuses_before_creation_without_changing_context() {
        let directory = tempfile::tempdir().unwrap();
        let stage = directory.path().join("stage");
        let before = thread_token_identity();
        let error = create_private_file(
            &stage,
            &Policy("D:(A;;FA;;;OW)S:AINO_ACCESS_CONTROL".to_owned()),
        )
        .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::Unsupported);
        assert!(error.to_string().contains("S:AINO_ACCESS_CONTROL"));
        assert!(error.to_string().contains("cannot faithfully establish"));
        assert!(!stage.exists());
        assert_eq!(thread_token_identity(), before);
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
    }

    #[test]
    fn native_unprotected_null_sacl_publication_refuses_without_changing_destination() {
        use std::io::Write;
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("destination");
        security_privilege::with_security_privilege(|| {
            let mut file = OpenOptions::new()
                .read(true)
                .write(true)
                .create_new(true)
                .access_mode(
                    FILE_GENERIC_READ | FILE_GENERIC_WRITE | READ_CONTROL | ACCESS_SYSTEM_SECURITY,
                )
                .open(&path)?;
            file.write_all(b"original")?;
            let descriptor = Descriptor::parse("S:AINO_ACCESS_CONTROL")?;
            // Independent native authoring: kernel SACL assignment retains
            // unprotected NULL and drops AI, as established by native diagnostics.
            // SAFETY: owned fixture file and native descriptor remain live.
            boolean(unsafe {
                SetKernelObjectSecurity(
                    file.as_raw_handle() as HANDLE,
                    SACL_SECURITY_INFORMATION,
                    descriptor.0,
                )
            })
        })
        .unwrap();
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .unwrap();
        let policy = Policy::read(&file).unwrap();
        assert!(policy.0.ends_with("S:NO_ACCESS_CONTROL"), "{}", policy.0);
        let before = thread_token_identity();
        let error = crate::run::publish(b"replacement", Some(&path)).unwrap_err();
        assert!(format!("{error:?}").contains("cannot faithfully establish"));
        assert_eq!(std::fs::read(&path).unwrap(), b"original");
        assert_eq!(Policy::read(&file).unwrap(), policy);
        assert_eq!(thread_token_identity(), before);
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
    }

    #[test]
    fn private_stage_reproduces_native_authored_labels_without_data_read_grants() {
        use std::io::Write;
        for (name, dacl, protection, label, denies_data_read) in [
            (
                "inherited",
                "D:(A;;FA;;;OW)",
                UNPROTECTED_DACL_SECURITY_INFORMATION,
                None,
                false,
            ),
            (
                "low-denied-read",
                "D:P(D;;0x1;;;WD)(A;;FA;;;OW)",
                PROTECTED_DACL_SECURITY_INFORMATION,
                Some("S:(ML;;NW;;;LW)"),
                true,
            ),
            (
                "medium-no-read-up",
                "D:(A;;FA;;;OW)",
                UNPROTECTED_DACL_SECURITY_INFORMATION,
                Some("S:(ML;;NR;;;ME)"),
                false,
            ),
        ] {
            let directory = tempfile::tempdir().unwrap();
            let source_path = directory.path().join(name);
            let mut source = stage_options().open(&source_path).unwrap();
            source.write_all(b"original").unwrap();
            let mut acl = std::ptr::null_mut();
            let mut present = 0;
            let mut defaulted = 0;
            // Independent file-policy authoring uses the standard native API,
            // not the reproduction mechanism exercised below.
            author_dacl(&source, dacl, protection).unwrap();
            if let Some(label) = label {
                let descriptor = Descriptor::parse(label).unwrap();
                // SAFETY: both the fixture descriptor/ACL and source handle are live.
                unsafe {
                    boolean(GetSecurityDescriptorSacl(
                        descriptor.0,
                        &mut present,
                        &mut acl,
                        &mut defaulted,
                    ))
                    .unwrap();
                    status(SetSecurityInfo(
                        source.as_raw_handle() as HANDLE,
                        SE_FILE_OBJECT,
                        LABEL_SECURITY_INFORMATION,
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        acl,
                    ))
                    .unwrap();
                }
            }
            if denies_data_read {
                assert_eq!(
                    std::fs::read(&source_path).unwrap_err().kind(),
                    io::ErrorKind::PermissionDenied
                );
            }
            let policy = Policy::read(&source).unwrap();
            let stage_path = directory.path().join("stage");
            let mut stage = create_private_file(&stage_path, &policy).unwrap();
            restrict(&stage).unwrap();
            policy.prepare(&stage).unwrap();
            assert_eq!(
                std::fs::read(&stage_path).unwrap_err().kind(),
                io::ErrorKind::PermissionDenied,
                "preparation must not grant pathname data reads for {name}"
            );
            stage.write_all(b"complete").unwrap();
            policy.apply(&stage, &stage_path).unwrap();
            assert_eq!(Policy::read(&stage).unwrap(), policy, "{name}");
            if denies_data_read {
                assert_eq!(
                    std::fs::read(&stage_path).unwrap_err().kind(),
                    io::ErrorKind::PermissionDenied
                );
            } else {
                assert_eq!(std::fs::read(&stage_path).unwrap(), b"complete");
            }
        }
    }

    #[test]
    fn inherited_policy_reconstruction_refuses_a_different_parent_policy() {
        use std::io::Write;
        let directory = tempfile::tempdir().unwrap();
        let source_parent = directory.path().join("source-parent");
        std::fs::create_dir(&source_parent).unwrap();
        let source_path = source_parent.join("source");
        let mut source = stage_options().open(&source_path).unwrap();
        source.write_all(b"original").unwrap();
        let parent = OpenOptions::new()
            .read(true)
            .access_mode(READ_CONTROL | WRITE_DAC)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
            .open(&source_parent)
            .unwrap();
        // Author a genuine inherited data-read denial, while keeping the
        // parent and retained source metadata/writer handle usable.
        author_dacl(
            &parent,
            "D:P(D;OIIO;0x1;;;WD)(A;OICI;FA;;;OW)",
            PROTECTED_DACL_SECURITY_INFORMATION,
        )
        .unwrap();
        author_dacl(&source, "D:", UNPROTECTED_DACL_SECURITY_INFORMATION).unwrap();
        let inherited = Descriptor::read(&source).unwrap();
        let mut source_acl = std::ptr::null_mut();
        let mut present = 0;
        let mut defaulted = 0;
        // SAFETY: the native descriptor owns this ACL and its ACE headers.
        unsafe {
            boolean(GetSecurityDescriptorDacl(
                inherited.0,
                &mut present,
                &mut source_acl,
                &mut defaulted,
            ))
            .unwrap();
            assert!(present != 0 && !source_acl.is_null() && (*source_acl).AceCount > 0);
            for index in 0..u32::from((*source_acl).AceCount) {
                let mut ace = std::ptr::null_mut();
                boolean(GetAce(source_acl, index, &mut ace)).unwrap();
                assert!(u32::from((*(ace as *const ACE_HEADER)).AceFlags) & INHERITED_ACE != 0);
            }
        }
        assert_eq!(
            std::fs::read(&source_path).unwrap_err().kind(),
            io::ErrorKind::PermissionDenied
        );
        let policy = Policy::read(&source).unwrap();
        let stage_parent = directory.path().join("stage-parent");
        std::fs::create_dir(&stage_parent).unwrap();
        let stage_path = stage_parent.join("stage");
        let stage = create_private_file(&stage_path, &policy).unwrap();
        restrict(&stage).unwrap();
        policy.prepare(&stage).unwrap();
        // Reproduction is challenged on an empty non-secret probe. A differing
        // parent can introduce real read grants, so no result may be placed in
        // this object even though exact readback detects the mismatch.
        let error = policy.preflight(&stage).unwrap_err();
        assert!(error.to_string().contains("staged authorization differs"));
        assert_eq!(stage.metadata().unwrap().len(), 0);
        assert_eq!(std::fs::read(&stage_path).unwrap(), b"");
        assert_eq!(source.metadata().unwrap().len(), b"original".len() as u64);
    }

    #[test]
    fn access_policy_queries_include_every_access_bearing_family() {
        // SECURITY_INFORMATION: owner1, group2, DACL4, label16,
        // resource32, scope64, process-trust128 and access-filter256.
        assert_eq!(INSPECT_INFO, 0x1f7);
        let scoped = Descriptor::parse("S:(SP;;;;;S-1-17-1)").unwrap();
        assert_eq!(
            scoped.require_supported_access_sacl().unwrap_err().kind(),
            io::ErrorKind::Unsupported
        );
    }

    #[test]
    fn native_invalid_handles_preserve_security_and_metadata_errors() {
        use windows_sys::Win32::Foundation::{ERROR_INVALID_HANDLE, GetLastError};
        let mut descriptor = std::ptr::null_mut();
        // SAFETY: deliberately invalid NULL handle; every output pointer is valid.
        let code = unsafe {
            GetSecurityInfo(
                std::ptr::null_mut(),
                SE_FILE_OBJECT,
                INFO,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut descriptor,
            )
        };
        assert_eq!(code, ERROR_INVALID_HANDLE);
        assert_eq!(
            status(code).unwrap_err().raw_os_error(),
            Some(ERROR_INVALID_HANDLE as i32)
        );
        let mut information = std::mem::MaybeUninit::<BY_HANDLE_FILE_INFORMATION>::uninit();
        // SAFETY: the invalid native handle must fail; storage is writable and never assumed initialized.
        let success =
            unsafe { GetFileInformationByHandle(std::ptr::null_mut(), information.as_mut_ptr()) };
        let error = unsafe { GetLastError() };
        assert_eq!(success, 0);
        assert_eq!(error, ERROR_INVALID_HANDLE);
        // Preserve the captured LastError across assertion machinery.
        unsafe { windows_sys::Win32::Foundation::SetLastError(error) };
        assert_eq!(
            boolean(success).unwrap_err().raw_os_error(),
            Some(ERROR_INVALID_HANDLE as i32)
        );
    }

    #[test]
    fn restriction_revokes_new_writer_handles_but_retains_the_original_writer() {
        use std::io::Write;
        let directory = tempfile::tempdir().unwrap();
        let source = stage_options()
            .open(directory.path().join("source"))
            .unwrap();
        let policy = Policy::read(&source).unwrap();
        let path = directory.path().join("private");
        let mut file = create_private_file(&path, &policy).unwrap();
        // The atomic constructor initially permits writer/metadata access only.
        drop(
            OpenOptions::new()
                .write(true)
                .access_mode(FILE_WRITE_DATA)
                .open(&path)
                .unwrap(),
        );
        assert_eq!(file.metadata().unwrap().len(), 0);
        restrict(&file).unwrap();
        assert_eq!(
            Descriptor::read(&file)
                .unwrap()
                .text_with(DACL_SECURITY_INFORMATION)
                .unwrap(),
            "D:P"
        );
        assert_eq!(
            OpenOptions::new()
                .write(true)
                .access_mode(FILE_WRITE_DATA)
                .open(&path)
                .unwrap_err()
                .raw_os_error(),
            Some(5)
        );
        file.write_all(b"retained-writer").unwrap();
        assert_eq!(file.metadata().unwrap().len(), 15);
        assert_eq!(
            std::fs::read(&path).unwrap_err().kind(),
            io::ErrorKind::PermissionDenied
        );
    }

    #[test]
    fn failed_native_assignment_preserves_actual_policy_diagnostics() {
        use windows_sys::Win32::Foundation::{ERROR_ACCESS_DENIED, GetLastError};
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("policy");
        let file = stage_options().open(&path).unwrap();
        let initial = Descriptor::parse("D:P(A;;FA;;;OW)").unwrap();
        // SAFETY: independent full-descriptor fixture assignment keeps the literal controls.
        assert_ne!(
            unsafe {
                SetKernelObjectSecurity(
                    file.as_raw_handle() as HANDLE,
                    DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
                    initial.0,
                )
            },
            0
        );
        assert_eq!(
            Descriptor::read(&file)
                .unwrap()
                .text_with(DACL_SECURITY_INFORMATION)
                .unwrap(),
            "D:P(A;;FA;;;OW)"
        );
        let readonly = OpenOptions::new()
            .read(true)
            .access_mode(READ_CONTROL)
            .open(&path)
            .unwrap();
        let wanted = Descriptor::parse("D:P(D;;CC;;;WD)(A;;FA;;;OW)").unwrap();
        // SAFETY: a live metadata-only handle must reject this DACL assignment.
        let success = unsafe {
            SetKernelObjectSecurity(
                readonly.as_raw_handle() as HANDLE,
                DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
                wanted.0,
            )
        };
        let code = unsafe { GetLastError() };
        assert_eq!(success, 0);
        assert_eq!(code, ERROR_ACCESS_DENIED);
        let error = wanted
            .apply(&readonly, DACL_SECURITY_INFORMATION)
            .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
        let message = error.to_string();
        assert!(message.contains("(os error 5)"), "{message}");
        assert!(
            message.contains("wanted D:P(D;;CC;;;WD)(A;;FA;;;OW)"),
            "{message}"
        );
        assert!(message.contains("before D:P(A;;FA;;;OW)"), "{message}");
    }

    #[test]
    fn native_descriptor_allocations_are_released() {
        use windows_sys::Win32::{
            Foundation::{ERROR_NO_MORE_ITEMS, GetLastError},
            System::Memory::{GetProcessHeap, HeapLock, HeapUnlock, HeapWalk, PROCESS_HEAP_ENTRY},
            System::SystemServices::PROCESS_HEAP_ENTRY_BUSY,
        };
        const CHILD: &str = "HTMLCUT_TEST_WINDOWS_DESCRIPTOR_HEAP";
        fn busy_allocation(address: usize) -> Result<bool, u32> {
            // SAFETY: GetProcessHeap returns the calling process's managed heap.
            let heap = unsafe { GetProcessHeap() };
            if heap.is_null() || unsafe { HeapLock(heap) } == 0 {
                return Err(unsafe { GetLastError() });
            }
            // No allocation, formatting, assertion or early return while locked.
            let mut entry: PROCESS_HEAP_ENTRY = unsafe { std::mem::zeroed() };
            let mut found = false;
            let error = loop {
                if unsafe { HeapWalk(heap, &mut entry) } == 0 {
                    break unsafe { GetLastError() };
                }
                if u32::from(entry.wFlags) & PROCESS_HEAP_ENTRY_BUSY != 0
                    && entry.lpData as usize == address
                {
                    found = true;
                    break 0;
                }
            };
            // SAFETY: this thread acquired the same live process heap above.
            // A failed unlock cannot safely proceed to allocating diagnostics.
            if unsafe { HeapUnlock(heap) } == 0 {
                std::process::abort();
            }
            if found {
                Ok(true)
            } else if error == ERROR_NO_MORE_ITEMS {
                Ok(false)
            } else {
                Err(error)
            }
        }
        if std::env::var_os(CHILD).is_some() {
            for _ in 0..16 {
                let descriptor = Descriptor::parse("D:P(A;;FA;;;OW)").unwrap();
                let address = descriptor.0 as usize;
                assert!(
                    busy_allocation(address).unwrap(),
                    "live descriptor not observable in the native heap"
                );
                std::mem::forget(descriptor);
                let retained = busy_allocation(address).unwrap();
                // SAFETY: this control still owns the live intentionally retained allocation.
                let failed = unsafe { LocalFree(address as *mut std::ffi::c_void) };
                assert!(failed.is_null());
                assert!(retained, "retained-allocation positive control failed");
            }
            for _ in 0..16 {
                let descriptor = Descriptor::parse("D:P(A;;FA;;;OW)").unwrap();
                let address = descriptor.0 as usize;
                assert!(busy_allocation(address).unwrap());
                drop(descriptor);
                // Only compare this saved numeric address with enumerated BUSY entries;
                // never submit a freed pointer as a native handle or read its memory.
                let retained = busy_allocation(address).unwrap();
                assert!(
                    !retained,
                    "native descriptor allocation remained busy after Drop"
                );
            }
            return;
        }
        struct HeapProbeChild(Option<std::process::Child>);
        impl Drop for HeapProbeChild {
            fn drop(&mut self) {
                let Some(mut child) = self.0.take() else {
                    return;
                };
                let kill_error = child.kill().err();
                let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
                let mut wait_error = None;
                loop {
                    match child.try_wait() {
                        Ok(Some(_)) => return,
                        Err(error) => wait_error = Some(error),
                        Ok(None) => {}
                    }
                    if std::time::Instant::now() >= deadline {
                        eprintln!(
                            "Native heap child {} cleanup unconfirmed: kill {kill_error:?}, wait {wait_error:?}",
                            child.id()
                        );
                        return;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
            }
        }
        let test = format!(
            "{}::native_descriptor_allocations_are_released",
            module_path!().split_once("::").unwrap().1
        );
        let mut child = HeapProbeChild(Some(
            std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", &test, "--nocapture"])
                .env(CHILD, "1")
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .unwrap(),
        ));
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        loop {
            match child.0.as_mut().unwrap().try_wait() {
                Ok(Some(_)) => break,
                Err(error) => panic!("native heap child wait failed: {error}"),
                Ok(None) => {}
            }
            assert!(
                std::time::Instant::now() < deadline,
                "native heap child exceeded two seconds"
            );
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let output = child.0.take().unwrap().wait_with_output().unwrap();
        assert!(output.status.success(), "{output:?}");
        assert!(
            String::from_utf8(output.stdout)
                .unwrap()
                .contains("1 passed; 0 failed; 0 ignored;")
        );
    }

    #[test]
    fn preparation_sets_native_assignable_distinct_owner_and_group_before_bytes() {
        use std::os::windows::io::{FromRawHandle, OwnedHandle};
        use windows_sys::Win32::{
            Foundation::{ERROR_INSUFFICIENT_BUFFER, GetLastError},
            System::{
                SystemServices::{SE_GROUP_ENABLED, SE_GROUP_OWNER},
                Threading::{GetCurrentProcess, OpenProcessToken},
            },
        };
        let mut handle = std::ptr::null_mut();
        // SAFETY: current process and valid token output local.
        assert_ne!(
            unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut handle) },
            0
        );
        // SAFETY: the successful native open returned exactly this owned token handle.
        let _token = unsafe { OwnedHandle::from_raw_handle(handle) };
        fn token_information(handle: HANDLE, class: TOKEN_INFORMATION_CLASS) -> (Vec<u64>, usize) {
            let mut needed = 0;
            // SAFETY: documented size query with no writable output buffer.
            let success =
                unsafe { GetTokenInformation(handle, class, std::ptr::null_mut(), 0, &mut needed) };
            let code = unsafe { GetLastError() };
            assert_eq!(success, 0);
            assert_eq!(code, ERROR_INSUFFICIENT_BUFFER);
            let mut storage = vec![0u64; (needed as usize).div_ceil(8)];
            // SAFETY: u64 storage provides native pointer alignment and at least needed bytes.
            assert_ne!(
                unsafe {
                    GetTokenInformation(
                        handle,
                        class,
                        storage.as_mut_ptr().cast(),
                        needed,
                        &mut needed,
                    )
                },
                0
            );
            assert!(needed > 0 && needed as usize <= storage.len() * 8);
            (storage, needed as usize)
        }
        fn validate_sid(storage: &[u64], returned: usize, sid: PSID) {
            let start = storage.as_ptr() as usize;
            let offset = (sid as usize)
                .checked_sub(start)
                .expect("SID pointer precedes token buffer");
            assert!(offset.checked_add(8).is_some_and(|end| end <= returned));
            // SAFETY: returned bytes lie inside the initialized aligned token buffer.
            let bytes =
                unsafe { std::slice::from_raw_parts(storage.as_ptr().cast::<u8>(), returned) };
            let sid_length = 8 + usize::from(bytes[offset + 1]) * 4;
            assert!(
                offset
                    .checked_add(sid_length)
                    .is_some_and(|end| end <= returned)
            );
        }
        let (user, user_returned) = token_information(handle, TokenUser);
        let (groups, groups_returned) = token_information(handle, TokenGroups);
        assert!(user_returned >= std::mem::size_of::<TOKEN_USER>());
        assert!(groups_returned >= std::mem::offset_of!(TOKEN_GROUPS, Groups));
        // SAFETY: successful native queries initialized these aligned structures and their SID pointers.
        let user_sid = unsafe { (*(user.as_ptr().cast::<TOKEN_USER>())).User.Sid };
        validate_sid(&user, user_returned, user_sid);
        let groups_header = groups.as_ptr().cast::<TOKEN_GROUPS>();
        let count = unsafe { (*groups_header).GroupCount } as usize;
        assert!(
            std::mem::offset_of!(TOKEN_GROUPS, Groups)
                + count * std::mem::size_of::<SID_AND_ATTRIBUTES>()
                <= groups_returned
        );
        let entries =
            unsafe { std::slice::from_raw_parts((*groups_header).Groups.as_ptr(), count) };
        for entry in entries {
            validate_sid(&groups, groups_returned, entry.Sid);
        }
        let owners = entries
            .iter()
            .filter(|entry| {
                entry.Attributes & (SE_GROUP_ENABLED | SE_GROUP_OWNER) as u32
                    == (SE_GROUP_ENABLED | SE_GROUP_OWNER) as u32
            })
            .map(|entry| entry.Sid)
            .collect::<Vec<_>>();
        let directory = tempfile::tempdir().unwrap();
        let source = stage_options()
            .open(directory.path().join("source"))
            .unwrap();
        let original = Descriptor::read(&source).unwrap();
        let mut original_owner = std::ptr::null_mut();
        let mut original_group = std::ptr::null_mut();
        let mut defaulted = 0;
        // SAFETY: native descriptor owns both returned SID pointers.
        assert_ne!(
            unsafe { GetSecurityDescriptorOwner(original.0, &mut original_owner, &mut defaulted) },
            0
        );
        assert_ne!(
            unsafe { GetSecurityDescriptorGroup(original.0, &mut original_group, &mut defaulted) },
            0
        );
        let owner = std::iter::once(user_sid)
            .chain(owners.iter().copied())
            .find(|candidate| unsafe { EqualSid(*candidate, original_owner) } == 0)
            .expect("native ownership control needs a distinct currently assignable owner SID");
        let group = owners
            .iter()
            .copied()
            .find(|candidate| unsafe { EqualSid(*candidate, original_group) } == 0)
            .expect("native group control needs a distinct enabled owner-capable group SID");
        // No privilege assignment/activation: both selected SIDs come from this caller's token.
        let assigned = unsafe {
            SetSecurityInfo(
                source.as_raw_handle() as HANDLE,
                SE_FILE_OBJECT,
                OWNER_SECURITY_INFORMATION | GROUP_SECURITY_INFORMATION,
                owner,
                group,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        assert_eq!(
            assigned, 0,
            "native assigned-token ownership fixture was refused: {assigned}"
        );
        let selected_group_flags = entries
            .iter()
            .find(|entry| entry.Sid == group)
            .unwrap()
            .Attributes;
        eprintln!(
            "native ownership fixture: distinct_owner=true distinct_group=true group_flags={selected_group_flags:#x} owner_is_token_user={} native_assignment={assigned}",
            unsafe { EqualSid(owner, user_sid) } != 0
        );
        let policy = Policy::read(&source).unwrap();
        let path = directory.path().join("stage");
        let stage = create_private_file(&path, &policy).unwrap();
        restrict(&stage).unwrap();
        let before = Descriptor::read(&stage).unwrap();
        assert_ne!(
            before
                .text_with(OWNER_SECURITY_INFORMATION | GROUP_SECURITY_INFORMATION)
                .unwrap(),
            Descriptor::read(&source)
                .unwrap()
                .text_with(OWNER_SECURITY_INFORMATION | GROUP_SECURITY_INFORMATION)
                .unwrap()
        );
        policy.prepare(&stage).unwrap();
        let prepared = Descriptor::read(&stage).unwrap();
        let mut observed_owner = std::ptr::null_mut();
        let mut observed_group = std::ptr::null_mut();
        assert_ne!(
            unsafe { GetSecurityDescriptorOwner(prepared.0, &mut observed_owner, &mut defaulted) },
            0
        );
        assert_ne!(
            unsafe { GetSecurityDescriptorGroup(prepared.0, &mut observed_group, &mut defaulted) },
            0
        );
        assert_ne!(unsafe { EqualSid(owner, observed_owner) }, 0);
        assert_ne!(unsafe { EqualSid(group, observed_group) }, 0);
        assert_eq!(stage.metadata().unwrap().len(), 0);
        assert_eq!(
            Descriptor::read(&stage)
                .unwrap()
                .text_with(DACL_SECURITY_INFORMATION)
                .unwrap(),
            "D:P"
        );
        assert_eq!(
            std::fs::read(path).unwrap_err().kind(),
            io::ErrorKind::PermissionDenied
        );
    }

    #[test]
    fn preparation_changes_a_distinct_label_without_reassigning_matching_protection() {
        let directory = tempfile::tempdir().unwrap();
        let initial = Policy("D:P(A;;FA;;;OW)S:PAI(ML;;NW;;;LW)".into());
        let path = directory.path().join("stage");
        let stage = create_private_file(&path, &initial).unwrap();
        restrict(&stage).unwrap();
        let before = Descriptor::read(&stage).unwrap();
        assert_eq!(
            before.text_with(LABEL_SECURITY_INFORMATION).unwrap(),
            "S:PAI(ML;;NW;;;LW)"
        );
        let ownership = before
            .text_with(
                OWNER_SECURITY_INFORMATION | GROUP_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
            )
            .unwrap();
        let wanted = Policy(format!("{ownership}S:PAI(ML;;NR;;;ME)"));
        wanted.prepare(&stage).unwrap();
        assert_eq!(
            Descriptor::read(&stage)
                .unwrap()
                .text_with(LABEL_SECURITY_INFORMATION)
                .unwrap(),
            "S:PAI(ML;;NR;;;ME)"
        );
        assert_eq!(stage.metadata().unwrap().len(), 0);
        assert_eq!(
            std::fs::read(path).unwrap_err().kind(),
            io::ErrorKind::PermissionDenied
        );
    }
}
