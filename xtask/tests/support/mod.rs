use std::path::{Path, PathBuf};
use std::process::Command;

use htmlcut_tempdir::{TempDir, tempdir};

/// Owns one integration test's disposable maintainer-gate artifact root.
pub(crate) struct IsolatedArtifacts {
    _root: TempDir,
    target_dir: PathBuf,
    build_dir: PathBuf,
    gate_report_dir: PathBuf,
}

impl IsolatedArtifacts {
    /// Creates an isolated target, build, and retained-evidence root.
    pub(crate) fn new() -> Self {
        let root = tempdir().expect("create isolated artifact root");
        let target_dir = root.path().join("target");
        let build_dir = root.path().join("build");
        let gate_report_dir = root.path().join("gate-runs");

        Self {
            _root: root,
            target_dir,
            build_dir,
            gate_report_dir,
        }
    }

    /// Builds a command that cannot write maintainer evidence into the checkout's artifact root.
    pub(crate) fn xtask_command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_xtask"));
        command
            .env("CARGO_TARGET_DIR", &self.target_dir)
            .env("CARGO_BUILD_BUILD_DIR", &self.build_dir);
        command
    }

    /// Returns the report root configured for the child command.
    pub(crate) fn gate_report_dir(&self) -> &Path {
        &self.gate_report_dir
    }
}
