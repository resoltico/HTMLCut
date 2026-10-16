// SPDX-License-Identifier: MPL-2.0
//! Handle-based access policy inspection; only complete staged content receives final access.
#[cfg(any(target_os = "macos", target_os = "linux"))]
mod unix;
#[cfg(any(target_os = "macos", target_os = "linux"))]
pub(crate) use unix::*;
#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub(crate) use windows::*;
