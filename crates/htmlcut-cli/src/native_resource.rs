// SPDX-License-Identifier: MPL-2.0
//! Audited native calls only. No source or plan reaches this module.
use std::{io, process::Child};
#[cfg(unix)]
pub(crate) fn attach(_: &Child) -> io::Result<()> {
    Ok(())
}
#[cfg(unix)]
pub(crate) fn install() -> io::Result<()> {
    let baseline = baseline()?;
    let cap = baseline
        .checked_add(crate::worker::MEMORY_BYTES)
        .ok_or_else(|| io::Error::other("Memory policy overflow"))?;
    let limits = libc::rlimit {
        rlim_cur: cap as libc::rlim_t,
        rlim_max: cap as libc::rlim_t,
    };
    // SAFETY: valid initialized rlimit; affects this fresh worker only.
    if unsafe { libc::setrlimit(libc::RLIMIT_AS, &limits) } != 0 {
        return Err(io::Error::last_os_error());
    }
    probe()?;
    let cpu = libc::rlimit {
        rlim_cur: 10,
        rlim_max: 10,
    };
    // SAFETY: valid native limits, same worker process; independent CPU backstop.
    if unsafe { libc::setrlimit(libc::RLIMIT_CPU, &cpu) } != 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: getppid has no preconditions. Capture before monitoring starts.
    let parent = unsafe { libc::getppid() };
    if parent <= 1 {
        return Err(io::Error::other("Supervisor disappeared"));
    }
    #[cfg(target_os = "linux")]
    // SAFETY: Linux parent-death signal setting, followed by race check below.
    if unsafe { libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL) } != 0 {
        return Err(io::Error::last_os_error());
    }
    if unsafe { libc::getppid() } != parent {
        return Err(io::Error::other("Supervisor disappeared"));
    }
    std::thread::Builder::new()
        .name("supervisor-lifetime".into())
        .stack_size(64 * 1024)
        .spawn(move || {
            loop {
                if unsafe { libc::getppid() } != parent {
                    // SAFETY: terminate only this orphaned worker without running destructors.
                    unsafe {
                        libc::_exit(4);
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
        })?;
    Ok(())
}
#[cfg(target_os = "linux")]
fn baseline() -> io::Result<u64> {
    Ok(0)
}
#[cfg(target_os = "macos")]
#[allow(deprecated)] // libc exposes the stable Mach ABI; no additional wrapper dependency needed.
fn baseline() -> io::Result<u64> {
    let mut info = std::mem::MaybeUninit::<libc::mach_task_basic_info>::zeroed();
    let mut count = libc::MACH_TASK_BASIC_INFO_COUNT;
    // SAFETY: task_info receives exactly the initialized ABI buffer/count for this task.
    let result = unsafe {
        libc::task_info(
            libc::mach_task_self_,
            libc::MACH_TASK_BASIC_INFO,
            info.as_mut_ptr().cast(),
            &mut count,
        )
    };
    if result != libc::KERN_SUCCESS || count != libc::MACH_TASK_BASIC_INFO_COUNT {
        return Err(io::Error::other("Memory baseline unavailable"));
    }
    // SAFETY: successful task_info initialized the complete advertised structure.
    let info = unsafe { info.assume_init() };
    // Refuse unexpected pre-input heap use, rather than absorbing it into the allowance.
    if info.resident_size > 64 * 1024 * 1024 {
        return Err(io::Error::other("Worker baseline exceeds allowance"));
    }
    Ok(info.virtual_size)
}
#[cfg(unix)]
pub(crate) fn probe() -> io::Result<()> {
    let size = crate::worker::MEMORY_BYTES as usize + 64 * 1024 * 1024;
    // SAFETY: anonymous probe reserves address space, never dereferenced. Any successful
    // mapping is released immediately. Failure must be ENOMEM, not an unrelated error.
    let address = unsafe {
        libc::mmap(
            std::ptr::null_mut(),
            size,
            libc::PROT_NONE,
            libc::MAP_PRIVATE | libc::MAP_ANON,
            -1,
            0,
        )
    };
    if address == libc::MAP_FAILED {
        if io::Error::last_os_error().raw_os_error() == Some(libc::ENOMEM) {
            return Ok(());
        }
        return Err(io::Error::last_os_error());
    }
    // SAFETY: address/size identify the successful mapping above.
    unsafe {
        libc::munmap(address, size);
    }
    Err(io::Error::other("Native memory policy was ineffective"))
}
#[cfg(windows)]
mod windows {
    use super::*;
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::{
        Foundation::{CloseHandle, HANDLE},
        System::{JobObjects::*, Memory::*},
    };
    pub(crate) struct Job(HANDLE);
    impl Drop for Job {
        fn drop(&mut self) {
            /* SAFETY: owned job handle closed once. */
            unsafe {
                CloseHandle(self.0);
            }
        }
    }
    pub(crate) fn attach(child: &Child) -> io::Result<Job> {
        // SAFETY: null attributes/name create an unnamed non-inherited job.
        let handle = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
        if handle.is_null() {
            return Err(io::Error::last_os_error());
        }
        let job = Job(handle);
        let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { std::mem::zeroed() };
        limits.BasicLimitInformation.LimitFlags =
            JOB_OBJECT_LIMIT_PROCESS_MEMORY | JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        limits.ProcessMemoryLimit = crate::worker::MEMORY_BYTES as usize;
        // SAFETY: native ABI struct and size; valid child handle. Worker blocks on input
        // until assignment completes and the parent releases its request.
        if unsafe {
            SetInformationJobObject(
                handle,
                JobObjectExtendedLimitInformation,
                (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                std::mem::size_of_val(&limits) as u32,
            )
        } == 0
            || unsafe { AssignProcessToJobObject(handle, child.as_raw_handle().cast()) } == 0
        {
            return Err(io::Error::last_os_error());
        }
        Ok(job)
    }
    pub(crate) fn install() -> io::Result<()> {
        let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { std::mem::zeroed() };
        // SAFETY: null job queries the current process's assigned job; full ABI-sized buffer.
        if unsafe {
            QueryInformationJobObject(
                std::ptr::null_mut(),
                JobObjectExtendedLimitInformation,
                (&mut limits as *mut JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                std::mem::size_of_val(&limits) as u32,
                std::ptr::null_mut(),
            )
        } == 0
            || limits.BasicLimitInformation.LimitFlags & JOB_OBJECT_LIMIT_PROCESS_MEMORY == 0
            || limits.ProcessMemoryLimit == 0
            || limits.BasicLimitInformation.LimitFlags & JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE == 0
            || limits.ProcessMemoryLimit > crate::worker::MEMORY_BYTES as usize
        {
            return Err(io::Error::other("Worker job policy unavailable"));
        }
        probe()
    }
    pub(crate) fn probe() -> io::Result<()> {
        // SAFETY: reserve+commit tests Job Object committed-memory enforcement. No access
        // occurs; successful allocation is immediately released and means policy failure.
        let address = unsafe {
            VirtualAlloc(
                std::ptr::null(),
                crate::worker::MEMORY_BYTES as usize + 64 * 1024 * 1024,
                MEM_RESERVE | MEM_COMMIT,
                PAGE_READWRITE,
            )
        };
        if address.is_null() {
            let code = io::Error::last_os_error().raw_os_error();
            if matches!(code, Some(8 | 14 | 1455 | 1816)) {
                return Ok(());
            }
            return Err(io::Error::last_os_error());
        }
        unsafe {
            VirtualFree(address, 0, MEM_RELEASE);
        }
        Err(io::Error::other("Native memory policy was ineffective"))
    }
}
#[cfg(windows)]
pub(crate) use windows::{attach, install, probe};

/// Touch every page of retained native mappings until the installed policy refuses growth.
/// This is a private delivered-binary evaluation workload, never an extraction operation.
pub(crate) fn pressure() -> io::Result<(u64, u64)> {
    let chunk = 32 * 1024 * 1024;
    // Incompressible page contents prevent macOS compression from turning the RSS
    // measurement into a test of zero-filled/repeated-byte storage optimizations.
    let mut pattern = vec![0_u8; 64 * 1024];
    let mut random = 0x91d7_3fa5_7c62_084b_u64;
    for byte in &mut pattern {
        random ^= random << 13;
        random ^= random >> 7;
        random ^= random << 17;
        *byte = random as u8;
    }
    let mut mappings = Vec::new();
    let mut refused = false;
    for _ in 0..40 {
        #[cfg(unix)]
        let address = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                chunk,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_ANON,
                -1,
                0,
            )
        };
        #[cfg(windows)]
        let address = unsafe {
            windows_sys::Win32::System::Memory::VirtualAlloc(
                std::ptr::null(),
                chunk,
                windows_sys::Win32::System::Memory::MEM_RESERVE
                    | windows_sys::Win32::System::Memory::MEM_COMMIT,
                windows_sys::Win32::System::Memory::PAGE_READWRITE,
            )
        };
        #[cfg(unix)]
        let failed = address == libc::MAP_FAILED;
        #[cfg(windows)]
        let failed = address.is_null();
        if failed {
            #[cfg(unix)]
            {
                refused = io::Error::last_os_error().raw_os_error() == Some(libc::ENOMEM);
            }
            #[cfg(windows)]
            {
                refused = matches!(
                    io::Error::last_os_error().raw_os_error(),
                    Some(8 | 14 | 1455 | 1816)
                );
            }
            break;
        }
        for offset in (0..chunk).step_by(pattern.len()) {
            // SAFETY: source and native destination are disjoint; each fixed slice
            // lies inside the successfully allocated writable mapping.
            unsafe {
                std::ptr::copy_nonoverlapping(
                    pattern.as_ptr(),
                    address.cast::<u8>().add(offset),
                    pattern.len(),
                );
            }
        }
        mappings.push(address);
    }
    let allocated = mappings.len() as u64 * chunk as u64;
    let resident = resident_bytes();
    for address in mappings {
        #[cfg(unix)]
        unsafe {
            libc::munmap(address, chunk);
        }
        #[cfg(windows)]
        unsafe {
            windows_sys::Win32::System::Memory::VirtualFree(
                address,
                0,
                windows_sys::Win32::System::Memory::MEM_RELEASE,
            );
        }
    }
    if !refused {
        return Err(io::Error::other("Touched memory pressure was not refused"));
    }
    Ok((allocated, resident?))
}
#[cfg(target_os = "macos")]
#[allow(deprecated)]
fn resident_bytes() -> io::Result<u64> {
    let mut info = std::mem::MaybeUninit::<libc::mach_task_basic_info>::zeroed();
    let mut count = libc::MACH_TASK_BASIC_INFO_COUNT;
    if unsafe {
        libc::task_info(
            libc::mach_task_self_,
            libc::MACH_TASK_BASIC_INFO,
            info.as_mut_ptr().cast(),
            &mut count,
        )
    } != libc::KERN_SUCCESS
    {
        return Err(io::Error::other("Resident measurement unavailable"));
    }
    Ok(unsafe { info.assume_init() }.resident_size)
}
#[cfg(target_os = "linux")]
fn resident_bytes() -> io::Result<u64> {
    let statm = std::fs::read_to_string("/proc/self/statm")?;
    let pages = statm
        .split_whitespace()
        .nth(1)
        .and_then(|value| value.parse::<u64>().ok())
        .ok_or_else(|| io::Error::other("Resident measurement unavailable"))?;
    let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
    if page_size <= 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(pages * page_size as u64)
}
#[cfg(windows)]
fn resident_bytes() -> io::Result<u64> {
    use windows_sys::Win32::System::{
        ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS},
        Threading::GetCurrentProcess,
    };
    let mut counters: PROCESS_MEMORY_COUNTERS = unsafe { std::mem::zeroed() };
    if unsafe {
        GetProcessMemoryInfo(
            GetCurrentProcess(),
            &mut counters,
            std::mem::size_of_val(&counters) as u32,
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    Ok(counters.WorkingSetSize as u64)
}

/// Physical identity for alias admission; never reads file contents or opens Unix FIFOs.
#[cfg(unix)]
pub(crate) fn file_identity(path: &std::path::Path) -> io::Result<Option<(u64, u64)>> {
    use std::os::unix::fs::MetadataExt;
    let metadata = std::fs::metadata(path)?;
    Ok(Some((metadata.dev(), metadata.ino())))
}
#[cfg(windows)]
pub(crate) fn file_identity(path: &std::path::Path) -> io::Result<Option<(u64, u64)>> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::{
        Foundation::{CloseHandle, INVALID_HANDLE_VALUE},
        Storage::FileSystem::*,
    };
    if !std::fs::metadata(path)?.is_file() {
        return Ok(None);
    }
    let name: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    // SAFETY: NUL-terminated path and valid constant attributes. Attribute access only;
    // reparse-point following is disabled and all standard sharing is permitted.
    let handle = unsafe {
        CreateFileW(
            name.as_ptr(),
            FILE_READ_ATTRIBUTES,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            std::ptr::null(),
            OPEN_EXISTING,
            FILE_FLAG_OPEN_REPARSE_POINT,
            std::ptr::null_mut(),
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }
    let identity = windows_handle_identity(handle);
    // SAFETY: owned handle is closed exactly once, after all queries complete.
    unsafe {
        CloseHandle(handle);
    }
    identity?
        .map(Some)
        .ok_or_else(|| io::Error::other("File kind changed during admission"))
}
#[cfg(windows)]
fn windows_handle_identity(
    handle: windows_sys::Win32::Foundation::HANDLE,
) -> io::Result<Option<(u64, u64)>> {
    use windows_sys::Win32::Storage::FileSystem::*;
    // SAFETY: callers pass a currently owned or borrowed valid handle.
    if unsafe { GetFileType(handle) } != FILE_TYPE_DISK {
        return Ok(None);
    }
    let mut information: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
    // SAFETY: valid handle and full initialized ABI structure; no ownership transfer.
    if unsafe { GetFileInformationByHandle(handle, &mut information) } == 0 {
        return Err(io::Error::last_os_error());
    }
    if information.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Err(io::Error::other("File identity changed during admission"));
    }
    Ok(Some((
        information.dwVolumeSerialNumber as u64,
        ((information.nFileIndexHigh as u64) << 32) | information.nFileIndexLow as u64,
    )))
}

#[cfg(unix)]
pub(crate) fn reader_identity(reader: &os_pipe::PipeReader) -> io::Result<Option<(u64, u64)>> {
    let metadata = rustix::fs::fstat(reader)?;
    if rustix::fs::FileType::from_raw_mode(metadata.st_mode) != rustix::fs::FileType::RegularFile {
        return Ok(None);
    }
    Ok(Some((metadata.st_dev as u64, metadata.st_ino as u64)))
}
#[cfg(windows)]
pub(crate) fn reader_identity(reader: &os_pipe::PipeReader) -> io::Result<Option<(u64, u64)>> {
    use std::os::windows::io::AsRawHandle;
    windows_handle_identity(reader.as_raw_handle().cast())
}
