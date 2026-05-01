use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

use crate::error::{CliError, output_error};
use crate::model::CliErrorCode;

/// Command-wide policy for filesystem outputs created by one HTMLCut invocation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FileWriteMode {
    /// Refuse to replace any pre-existing output path.
    CreateFresh,
    /// Allow the command to replace its managed output paths.
    Overwrite,
}

impl FileWriteMode {
    /// Resolves the command-wide write policy from the user-facing overwrite flag.
    pub(crate) const fn from_overwrite_flag(overwrite: bool) -> Self {
        if overwrite {
            Self::Overwrite
        } else {
            Self::CreateFresh
        }
    }

    pub(crate) const fn overwrites_existing(self) -> bool {
        matches!(self, Self::Overwrite)
    }
}

pub(crate) fn validate_output_file_target(
    path: &Path,
    mode: FileWriteMode,
) -> Result<(), CliError> {
    validate_file_target(
        path,
        mode,
        CliErrorCode::OutputFileExists,
        CliErrorCode::OutputFileWriteFailed,
        "output file",
    )
}

pub(crate) fn validate_request_file_target(
    path: &Path,
    mode: FileWriteMode,
) -> Result<(), CliError> {
    validate_file_target(
        path,
        mode,
        CliErrorCode::RequestFileExists,
        CliErrorCode::RequestFileWriteFailed,
        "request file",
    )
}

pub(crate) fn validate_bundle_target(path: &Path, mode: FileWriteMode) -> Result<(), CliError> {
    if let Some(parent) = parent_dir(path)
        && parent.exists()
        && !parent.is_dir()
    {
        return Err(output_error(
            CliErrorCode::BundleDirectoryCreateFailed,
            format!(
                "Could not create bundle directory {}: parent path {} is not a directory.",
                path.display(),
                parent.display(),
            ),
        ));
    }

    if !path.exists() {
        return Ok(());
    }

    if !mode.overwrites_existing() {
        return Err(output_error(
            CliErrorCode::BundlePathExists,
            format!(
                "Refusing to write bundle into existing path {}. Choose a fresh directory or pass --overwrite.",
                path.display(),
            ),
        ));
    }

    if path.is_dir() {
        Ok(())
    } else {
        Err(output_error(
            CliErrorCode::BundleDirectoryCreateFailed,
            format!(
                "Could not create bundle directory {}: target path is not a directory.",
                path.display(),
            ),
        ))
    }
}

pub(crate) fn write_text_file(
    path: &Path,
    contents: &str,
    mode: FileWriteMode,
) -> std::io::Result<()> {
    create_parent_dirs(path)?;

    match mode {
        FileWriteMode::CreateFresh => {
            let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
            file.write_all(contents.as_bytes())
        }
        FileWriteMode::Overwrite => fs::write(path, contents),
    }
}

pub(crate) fn prepare_bundle_directory(path: &Path, mode: FileWriteMode) -> std::io::Result<()> {
    if let Some(parent) = parent_dir(path) {
        fs::create_dir_all(parent)?;
    }

    match mode {
        FileWriteMode::CreateFresh => fs::create_dir(path),
        FileWriteMode::Overwrite => {
            if path.exists() {
                if path.is_dir() {
                    Ok(())
                } else {
                    Err(std::io::Error::new(
                        std::io::ErrorKind::AlreadyExists,
                        "target path is not a directory",
                    ))
                }
            } else {
                fs::create_dir(path)
            }
        }
    }
}

fn validate_file_target(
    path: &Path,
    mode: FileWriteMode,
    exists_code: CliErrorCode,
    write_failed_code: CliErrorCode,
    label: &str,
) -> Result<(), CliError> {
    if let Some(parent) = parent_dir(path)
        && parent.exists()
        && !parent.is_dir()
    {
        return Err(output_error(
            write_failed_code,
            format!(
                "Could not write {label} {}: parent path {} is not a directory.",
                path.display(),
                parent.display(),
            ),
        ));
    }

    if !path.exists() {
        return Ok(());
    }

    if !mode.overwrites_existing() {
        return Err(output_error(
            exists_code,
            format!(
                "Refusing to overwrite existing {label} {}. Remove it, choose a fresh path, or pass --overwrite.",
                path.display(),
            ),
        ));
    }

    if path.is_dir() {
        Err(output_error(
            write_failed_code,
            format!(
                "Could not write {label} {}: target path is a directory.",
                path.display(),
            ),
        ))
    } else {
        Ok(())
    }
}

fn create_parent_dirs(path: &Path) -> std::io::Result<()> {
    if let Some(parent) = parent_dir(path) {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

fn parent_dir(path: &Path) -> Option<&Path> {
    let parent = path.parent()?;
    (!parent.as_os_str().is_empty()).then_some(parent)
}
