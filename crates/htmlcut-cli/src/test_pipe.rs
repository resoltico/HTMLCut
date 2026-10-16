// SPDX-License-Identifier: MPL-2.0
use std::io::Write;
#[cfg(unix)]
#[allow(unsafe_code)]
pub(crate) fn set_pipe_blocking(writer: &std::io::PipeWriter, blocking: bool) {
    use std::os::fd::AsRawFd;
    let fd = writer.as_raw_fd();
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    assert!(flags >= 0);
    let flags = if blocking {
        flags & !libc::O_NONBLOCK
    } else {
        flags | libc::O_NONBLOCK
    };
    assert_eq!(unsafe { libc::fcntl(fd, libc::F_SETFL, flags) }, 0);
}
#[cfg(windows)]
#[allow(unsafe_code)]
pub(crate) fn set_pipe_blocking(writer: &std::io::PipeWriter, blocking: bool) {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::System::Pipes::{PIPE_NOWAIT, PIPE_WAIT, SetNamedPipeHandleState};
    let mode = if blocking { PIPE_WAIT } else { PIPE_NOWAIT };
    assert_ne!(
        unsafe {
            SetNamedPipeHandleState(
                writer.as_raw_handle(),
                &mode,
                std::ptr::null(),
                std::ptr::null(),
            )
        },
        0,
        "native pipe blocking control is required"
    );
}

pub(crate) fn fill_pipe(writer: &mut std::io::PipeWriter) -> usize {
    set_pipe_blocking(writer, false);
    let mut capacity = 0;
    loop {
        match writer.write(&[b'x'; 4096]) {
            Ok(0) => break,
            Ok(written) => capacity += written,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => break,
            #[cfg(windows)]
            Err(error)
                if error.raw_os_error()
                    == Some(windows_sys::Win32::Foundation::ERROR_NO_DATA as i32) =>
            {
                break;
            }
            Err(error) => panic!("native pipe fill failed: {error}"),
        }
    }
    assert!(capacity > 0);
    set_pipe_blocking(writer, true);
    capacity
}
