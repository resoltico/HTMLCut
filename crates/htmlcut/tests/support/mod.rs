// SPDX-License-Identifier: MPL-2.0
use std::{
    path::PathBuf,
    process::{Command, Stdio},
};
pub fn binary() -> PathBuf {
    let path = std::env::var_os("HTMLCUT_TEST_BINARY")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_BIN_EXE_htmlcut")));
    if path.is_absolute() {
        path
    } else {
        std::env::current_dir().unwrap().join(path)
    }
}
pub fn command() -> Command {
    let mut command = Command::new(binary());
    command.stdin(Stdio::null());
    command
}
