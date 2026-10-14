// SPDX-License-Identifier: MPL-2.0
//! Handle-based access policy inspection; only complete staged content receives final access.
#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub(crate) use unix::*;
#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub(crate) use windows::*;
