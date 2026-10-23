// SPDX-License-Identifier: MPL-2.0
//! Complete output bytes, staged beside ordinary destination files.
use htmlcut_core::Error;
use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::Path,
};
fn direct(writer: &mut impl Write, bytes: &[u8]) -> io::Result<()> {
    match writer.write_all(bytes).and_then(|()| writer.flush()) {
        Err(e) if e.kind() == io::ErrorKind::BrokenPipe => Ok(()),
        result => result,
    }
}
fn device(path: &Path) -> bool {
    #[cfg(unix)]
    {
        path.as_os_str().as_encoded_bytes().starts_with(b"/dev/")
    }
    #[cfg(windows)]
    {
        let text = path.to_string_lossy();
        if text.starts_with(r"\\.\") {
            return true;
        }
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        let stem = name.split('.').next().unwrap_or("").to_ascii_uppercase();
        matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
            || ["COM", "LPT"].iter().any(|prefix| {
                stem.strip_prefix(prefix)
                    .is_some_and(|n| n.len() == 1 && matches!(n.as_bytes()[0], b'1'..=b'9'))
            })
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = path;
        false
    }
}
fn file(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if device(path) {
        return direct(&mut OpenOptions::new().write(true).open(path)?, bytes);
    }
    let metadata = match fs::metadata(path) {
        Ok(m) => Some(m),
        Err(e) if e.kind() == io::ErrorKind::NotFound => None,
        Err(e) => return Err(e),
    };
    if let Some(meta) = &metadata {
        if meta.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::IsADirectory,
                "destination is a directory",
            ));
        }
        if !meta.is_file() {
            return direct(&mut OpenOptions::new().write(true).open(path)?, bytes);
        }
    } else {
        match fs::symlink_metadata(path) {
            Ok(m) if m.file_type().is_symlink() => {
                return Err(io::Error::other("dangling symbolic link"));
            }
            Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(e),
            _ => (),
        }
    }
    let target = if metadata.is_some() {
        fs::canonicalize(path)?
    } else {
        path.to_owned()
    };
    let parent = target
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut builder = tempfile::Builder::new();
    builder.prefix(".htmlcut-");
    #[cfg(unix)]
    if metadata.is_none() {
        use std::os::unix::fs::PermissionsExt;
        builder.permissions(fs::Permissions::from_mode(0o666));
    }
    let mut staged = builder.tempfile_in(parent)?;
    staged.write_all(bytes)?;
    if let Some(metadata) = metadata {
        staged.as_file().set_permissions(metadata.permissions())?;
    }
    staged.persist(&target).map_err(|e| e.error)?;
    Ok(())
}
pub(crate) fn write(bytes: &[u8], path: Option<&Path>) -> Result<(), Error> {
    match path {
        None => direct(&mut io::stdout().lock(), bytes),
        Some(path) if path.as_os_str() == "-" => direct(&mut io::stdout().lock(), bytes),
        Some(path) => file(path, bytes),
    }
    .map_err(|e| Error::OutputUnwritable {
        file: path
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|| "-".into()),
        detail: e.to_string(),
    })
}
