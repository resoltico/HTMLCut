// SPDX-License-Identifier: MPL-2.0
//! Thread-scoped use of the caller's already assigned security privilege.
use std::{
    io,
    marker::PhantomData,
    os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle},
    rc::Rc,
};
use windows_sys::Win32::{
    Foundation::{
        ERROR_NO_TOKEN, ERROR_NOT_ALL_ASSIGNED, GetLastError, HANDLE, LUID, SetLastError,
    },
    Security::*,
    System::Threading::{
        GetCurrentProcess, GetCurrentThread, OpenProcessToken, OpenThreadToken, SetThreadToken,
    },
};

struct Token(OwnedHandle);
impl Token {
    fn raw(&self) -> HANDLE {
        self.0.as_raw_handle()
    }
    fn take(handle: HANDLE) -> Self {
        // SAFETY: successful token-open/duplicate APIs return one owned handle.
        Self(unsafe { OwnedHandle::from_raw_handle(handle) })
    }
}
fn checked(success: i32) -> io::Result<()> {
    if success == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}
fn effective_token() -> io::Result<(Token, bool)> {
    let mut handle = std::ptr::null_mut();
    // SAFETY: current-thread pseudo handle and valid writable output pointer.
    if unsafe {
        OpenThreadToken(
            GetCurrentThread(),
            TOKEN_QUERY | TOKEN_DUPLICATE | TOKEN_IMPERSONATE,
            1,
            &mut handle,
        )
    } != 0
    {
        return Ok((Token::take(handle), true));
    }
    let error = io::Error::last_os_error();
    if error.raw_os_error() != Some(ERROR_NO_TOKEN as i32) {
        return Err(error);
    }
    // Fall back only for an absent thread token, never for a denied token.
    // SAFETY: current-process pseudo handle and valid output pointer.
    checked(unsafe {
        OpenProcessToken(
            GetCurrentProcess(),
            TOKEN_QUERY | TOKEN_DUPLICATE,
            &mut handle,
        )
    })?;
    Ok((Token::take(handle), false))
}
fn impersonation_level(source: &Token) -> io::Result<SECURITY_IMPERSONATION_LEVEL> {
    let mut level = SecurityAnonymous;
    let mut returned = 0;
    // SAFETY: queried live impersonation token and fixed-size enum output.
    checked(unsafe {
        GetTokenInformation(
            source.raw(),
            TokenImpersonationLevel,
            (&mut level as *mut SECURITY_IMPERSONATION_LEVEL).cast(),
            std::mem::size_of::<SECURITY_IMPERSONATION_LEVEL>() as u32,
            &mut returned,
        )
    })?;
    Ok(level)
}
fn duplicate(source: &Token, level: SECURITY_IMPERSONATION_LEVEL) -> io::Result<Token> {
    let mut handle = std::ptr::null_mut();
    // SAFETY: live source token with DUPLICATE; the API owns the returned token.
    checked(unsafe {
        DuplicateTokenEx(
            source.raw(),
            TOKEN_QUERY | TOKEN_IMPERSONATE | TOKEN_ADJUST_PRIVILEGES,
            std::ptr::null(),
            level,
            TokenImpersonation,
            &mut handle,
        )
    })?;
    Ok(Token::take(handle))
}
fn security_luid() -> io::Result<LUID> {
    let name = "SeSecurityPrivilege\0".encode_utf16().collect::<Vec<_>>();
    let mut luid = LUID {
        LowPart: 0,
        HighPart: 0,
    };
    // SAFETY: terminated UTF-16 name and valid output storage.
    checked(unsafe { LookupPrivilegeValueW(std::ptr::null(), name.as_ptr(), &mut luid) })?;
    Ok(luid)
}
fn adjust_security(token: &Token, attributes: u32) -> io::Result<()> {
    let state = TOKEN_PRIVILEGES {
        PrivilegeCount: 1,
        Privileges: [LUID_AND_ATTRIBUTES {
            Luid: security_luid()?,
            Attributes: attributes,
        }],
    };
    // SAFETY: live token with ADJUST_PRIVILEGES; exactly one native entry.
    unsafe { SetLastError(0) };
    let success = unsafe {
        AdjustTokenPrivileges(
            token.raw(),
            0,
            &state,
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    // Capture immediately: success can still mean the privilege was not assigned.
    let error = unsafe { GetLastError() };
    if success == 0 {
        return Err(io::Error::from_raw_os_error(error as i32));
    }
    if error == ERROR_NOT_ALL_ASSIGNED {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "The caller has no assigned SeSecurityPrivilege; protected mandatory policy cannot be created without that capability",
        ));
    }
    if error != 0 {
        return Err(io::Error::from_raw_os_error(error as i32));
    }
    Ok(())
}
struct Scope {
    previous: Option<Token>,
    _installed: Token,
    active: bool,
    // An impersonation scope must restore on the same thread that installed it.
    _thread: PhantomData<Rc<()>>,
}
impl Scope {
    fn install(token: Token, previous: Option<Token>) -> io::Result<Self> {
        // SAFETY: live impersonation token; null thread selects the current thread.
        checked(unsafe { SetThreadToken(std::ptr::null(), token.raw()) })?;
        Ok(Self {
            previous,
            _installed: token,
            active: true,
            _thread: PhantomData,
        })
    }
    fn enable() -> io::Result<Self> {
        let (source, had_thread) = effective_token()?;
        let level = if had_thread {
            impersonation_level(&source)?
        } else {
            SecurityImpersonation
        };
        if level < SecurityImpersonation {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "The effective Windows thread token cannot impersonate for protected file creation",
            ));
        }
        let token = duplicate(&source, level)?;
        adjust_security(&token, SE_PRIVILEGE_ENABLED)?;
        Self::install(token, had_thread.then_some(source))
    }
    fn restore(&mut self) -> io::Result<()> {
        if !self.active {
            return Ok(());
        }
        let previous = self
            .previous
            .as_ref()
            .map_or(std::ptr::null_mut(), Token::raw);
        // SAFETY: retained previous token has IMPERSONATE; null restores absence.
        checked(unsafe { SetThreadToken(std::ptr::null(), previous) }).map_err(|error| {
            io::Error::new(
                error.kind(),
                format!("Cannot restore the caller's Windows thread security context: {error}"),
            )
        })?;
        self.active = false;
        Ok(())
    }
}
impl Drop for Scope {
    fn drop(&mut self) {
        if self.active && self.restore().is_err() {
            // Continuing after losing the caller's security context is unsafe.
            // No result bytes exist in the protected creation operation.
            std::process::abort();
        }
    }
}
pub(super) fn with_security_privilege<T>(
    operation: impl FnOnce() -> io::Result<T>,
) -> io::Result<T> {
    let mut scope = Scope::enable()?;
    let result = operation();
    scope.restore()?;
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enabled(token: &Token) -> bool {
        let mut required = 0;
        // SAFETY: zero-length query only obtains the required native buffer size.
        let queried = unsafe {
            GetTokenInformation(
                token.raw(),
                TokenPrivileges,
                std::ptr::null_mut(),
                0,
                &mut required,
            )
        };
        assert_eq!(queried, 0);
        assert_eq!(
            io::Error::last_os_error().raw_os_error(),
            Some(windows_sys::Win32::Foundation::ERROR_INSUFFICIENT_BUFFER as i32)
        );
        // u64 storage satisfies the native privilege structure's alignment.
        let mut buffer = vec![0u64; (required as usize).div_ceil(std::mem::size_of::<u64>())];
        // SAFETY: aligned writable storage covers the queried byte count.
        checked(unsafe {
            GetTokenInformation(
                token.raw(),
                TokenPrivileges,
                buffer.as_mut_ptr().cast(),
                required,
                &mut required,
            )
        })
        .unwrap();
        // SAFETY: successful query supplies a valid privilege header.
        let count =
            unsafe { (*(buffer.as_ptr() as *const TOKEN_PRIVILEGES)).PrivilegeCount } as usize;
        let offset = std::mem::offset_of!(TOKEN_PRIVILEGES, Privileges);
        assert!(offset + count * std::mem::size_of::<LUID_AND_ATTRIBUTES>() <= required as usize);
        // SAFETY: checked native count/offset stay within aligned returned storage.
        let entries = unsafe {
            std::slice::from_raw_parts(
                buffer
                    .as_ptr()
                    .cast::<u8>()
                    .add(offset)
                    .cast::<LUID_AND_ATTRIBUTES>(),
                count,
            )
        };
        let wanted = security_luid().unwrap();
        entries.iter().any(|entry| {
            entry.Luid.LowPart == wanted.LowPart
                && entry.Luid.HighPart == wanted.HighPart
                && entry.Attributes & SE_PRIVILEGE_ENABLED != 0
        })
    }
    fn context() -> (bool, u32, i32, bool) {
        let (token, thread) = effective_token().unwrap();
        let mut statistics = std::mem::MaybeUninit::<TOKEN_STATISTICS>::uninit();
        let mut returned = 0;
        // SAFETY: native fixed-size token statistics output and queried token.
        checked(unsafe {
            GetTokenInformation(
                token.raw(),
                TokenStatistics,
                statistics.as_mut_ptr().cast(),
                std::mem::size_of::<TOKEN_STATISTICS>() as u32,
                &mut returned,
            )
        })
        .unwrap();
        // SAFETY: successful API initialized this fixed-size structure.
        let statistics = unsafe { statistics.assume_init() };
        (
            thread,
            statistics.TokenId.LowPart,
            statistics.TokenId.HighPart,
            enabled(&token),
        )
    }
    #[test]
    fn security_scope_restores_success_error_and_unwind_without_process_changes() {
        let before = context();
        with_security_privilege(|| {
            assert!(context().0 && context().3);
            Ok(())
        })
        .unwrap();
        assert_eq!(context(), before);
        let error = with_security_privilege(|| Err::<(), _>(io::Error::other("operation refused")))
            .unwrap_err();
        assert_eq!(error.to_string(), "operation refused");
        assert_eq!(context(), before);
        let panic = std::panic::catch_unwind(|| {
            let _ = with_security_privilege::<()>(|| panic!("creation unwind control"));
        });
        assert!(panic.is_err());
        assert_eq!(context(), before);
        let (source, had_thread) = effective_token().unwrap();
        let mut prior = Scope::install(
            duplicate(&source, SecurityDelegation).unwrap(),
            had_thread.then_some(source),
        )
        .unwrap();
        let impersonated = context();
        with_security_privilege(|| {
            assert_eq!(
                impersonation_level(&effective_token().unwrap().0).unwrap(),
                SecurityDelegation
            );
            Ok(())
        })
        .unwrap();
        assert_eq!(
            context(),
            impersonated,
            "an existing impersonation token must be restored"
        );
        prior.restore().unwrap();
        assert_eq!(context(), before);
    }
    #[test]
    fn missing_assigned_security_privilege_refuses_before_operation() {
        let before = context();
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("protected-stage");
        let (source, had_thread) = effective_token().unwrap();
        let token = duplicate(&source, SecurityImpersonation).unwrap();
        adjust_security(&token, SE_PRIVILEGE_REMOVED).unwrap();
        let mut prior = Scope::install(token, had_thread.then_some(source)).unwrap();
        let restricted = context();
        let mut ran = false;
        let error = with_security_privilege(|| {
            ran = true;
            Ok(())
        })
        .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
        assert!(
            error
                .to_string()
                .contains("no assigned SeSecurityPrivilege")
        );
        assert!(!ran);
        assert_eq!(context(), restricted);
        let policy = super::super::Policy("D:P(A;;FA;;;OW)S:PAI(ML;;NW;;;LW)".into());
        let creation = super::super::create_private_file(&path, &policy).unwrap_err();
        assert_eq!(creation.kind(), io::ErrorKind::PermissionDenied);
        assert!(
            creation
                .to_string()
                .contains("no assigned SeSecurityPrivilege")
        );
        assert!(
            !path.exists(),
            "missing capability must refuse before native creation"
        );
        assert_eq!(context(), restricted);
        prior.restore().unwrap();
        assert_eq!(context(), before);
    }
    #[test]
    fn security_scope_does_not_enable_another_thread() {
        let worker = std::thread::spawn(|| {
            let before = context();
            with_security_privilege(|| {
                let observer = std::thread::spawn(context).join().unwrap();
                assert_eq!(
                    observer, before,
                    "another thread must retain process context"
                );
                Ok(())
            })
            .unwrap();
            assert_eq!(context(), before);
        });
        worker.join().unwrap();
    }
    #[test]
    fn identification_context_is_not_upgraded_for_creation() {
        let before = context();
        let (source, had_thread) = effective_token().unwrap();
        let token = duplicate(&source, SecurityIdentification).unwrap();
        let mut prior = Scope::install(token, had_thread.then_some(source)).unwrap();
        let mut ran = false;
        let error = with_security_privilege(|| {
            ran = true;
            Ok(())
        })
        .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
        assert!(!ran);
        assert_eq!(
            impersonation_level(&effective_token().unwrap().0).unwrap(),
            SecurityIdentification
        );
        prior.restore().unwrap();
        assert_eq!(context(), before);
    }
}
