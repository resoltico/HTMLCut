// SPDX-License-Identifier: MPL-2.0
use std::io::Write;
#[cfg(unix)]
#[allow(unsafe_code)]
pub(crate) fn fill_pipe(writer: &mut std::io::PipeWriter) {
    use std::os::fd::AsRawFd;
    let fd = writer.as_raw_fd();
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    assert!(flags >= 0);
    assert_eq!(
        unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) },
        0
    );
    let mut capacity = 0;
    loop {
        match writer.write(&[b'x'; 4096]) {
            Ok(written) => capacity += written,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => break,
            Err(error) => panic!("pipe fill failed: {error}"),
        }
    }
    assert!(capacity > 0);
    assert_eq!(unsafe { libc::fcntl(fd, libc::F_SETFL, flags) }, 0);
}
#[cfg(windows)]
#[allow(unsafe_code)]
pub(crate) fn fill_pipe(writer: &mut std::io::PipeWriter) {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::{
        Foundation::ERROR_NO_DATA,
        System::Pipes::{PIPE_NOWAIT, PIPE_WAIT, SetNamedPipeHandleState},
    };
    let mode = PIPE_NOWAIT;
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
        "native nonblocking pipe control is required"
    );
    let mut capacity = 0;
    loop {
        match writer.write(&[b'x'; 4096]) {
            Ok(0) => break,
            Ok(written) => capacity += written,
            Err(error)
                if error.kind() == std::io::ErrorKind::WouldBlock
                    || error.raw_os_error() == Some(ERROR_NO_DATA as i32) =>
            {
                break;
            }
            Err(error) => panic!("native pipe fill failed: {error}"),
        }
    }
    assert!(capacity > 0);
    let mode = PIPE_WAIT;
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
        "restore blocking mode before the executable inherits the writer"
    );
}
