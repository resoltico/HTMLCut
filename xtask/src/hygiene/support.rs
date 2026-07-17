//! Internal artifact-inventory helpers for the hygiene workflow.

use super::*;

fn prepare_managed_artifact_root(
    repo_root: &Path,
    artifact_root: &Path,
    kind: ManagedArtifactKind,
    purpose: &str,
) -> DynResult<()> {
    fs::create_dir_all(artifact_root).map_err(|error| {
        format!(
            "failed to create managed hygiene artifact root {}: {error}",
            artifact_root.display()
        )
    })?;
    write_cachedir_tag(artifact_root).map_err(|error| {
        format!(
            "failed to write managed hygiene cache marker {}: {error}",
            artifact_root.display()
        )
    })?;
    let repo_root_string = repo_root.display().to_string();
    let manifest = ArtifactRootManifest {
        schema: ARTIFACT_SCHEMA_NAME,
        kind,
        repo_root: &repo_root_string,
        safe_to_delete: true,
        purpose,
    };
    let manifest_path = artifact_root.join(ARTIFACT_MANIFEST_NAME);
    let manifest_contents = toml::to_string(&manifest)?;
    fs::write(&manifest_path, manifest_contents).map_err(|error| {
        format!(
            "failed to write managed hygiene manifest {}: {error}",
            manifest_path.display()
        )
    })?;
    Ok(())
}

pub(super) fn prepare_managed_artifact_roots<const N: usize>(
    repo_root: &Path,
    roots: [(&Path, ManagedArtifactKind, &str); N],
) -> DynResult<()> {
    for (artifact_root, kind, purpose) in roots {
        prepare_managed_artifact_root(repo_root, artifact_root, kind, purpose)?;
    }
    Ok(())
}

fn write_cachedir_tag(path: &Path) -> DynResult<()> {
    let tag_path = path.join(CACHEDIR_TAG_NAME);
    if tag_path.exists() {
        return Ok(());
    }

    fs::write(tag_path, CACHEDIR_TAG_CONTENTS)?;
    Ok(())
}

fn managed_entry(id: &str, kind: &str, path: &Path, budget_bytes: u64) -> DynResult<HygieneEntry> {
    entry_from_path(id, kind, path, Some(budget_bytes), true, true, Vec::new())
}

pub(super) fn managed_entries<const N: usize>(
    entries: [(&str, &str, &Path, u64); N],
) -> DynResult<Vec<HygieneEntry>> {
    entries
        .into_iter()
        .map(|(id, kind, path, budget_bytes)| managed_entry(id, kind, path, budget_bytes))
        .collect()
}

fn unmanaged_entry(
    id: &str,
    kind: &str,
    path: &Path,
    budget_bytes: u64,
    details: Vec<String>,
) -> DynResult<HygieneEntry> {
    entry_from_path(id, kind, path, Some(budget_bytes), false, true, details)
}

pub(super) fn unmanaged_entries<const N: usize>(
    entries: [(&str, &str, &Path, u64, Vec<String>); N],
) -> DynResult<Vec<HygieneEntry>> {
    entries
        .into_iter()
        .map(|(id, kind, path, budget_bytes, details)| {
            unmanaged_entry(id, kind, path, budget_bytes, details)
        })
        .collect()
}

fn repo_tmp_details(tmp_cargo_roots: &[PathBuf]) -> Vec<String> {
    let mut details = vec![
        "Repository scratch root mandated by AGENTS.md for temporary investigations.".to_owned(),
    ];
    if !tmp_cargo_roots.is_empty() {
        details.push(format!(
            "Excludes {} repo-local Cargo target roots reported separately under repo-tmp-cargo-targets.",
            tmp_cargo_roots.len()
        ));
    }
    details
}

pub(super) fn repo_tmp_entry(
    tmp_root: &Path,
    tmp_cargo_roots: &[PathBuf],
) -> DynResult<HygieneEntry> {
    entry_from_path_excluding_roots(
        EntrySpec {
            id: "repo-tmp",
            kind: "repo-tmp",
            path: tmp_root,
            budget_bytes: Some(REPO_TMP_BUDGET_BYTES),
            managed: false,
            safe_to_delete: true,
            details: repo_tmp_details(tmp_cargo_roots),
        },
        tmp_cargo_roots,
    )
}

pub(super) fn repo_tmp_cargo_entry(
    tmp_root: &Path,
    tmp_cargo_roots: &[PathBuf],
) -> DynResult<HygieneEntry> {
    aggregate_entry(
        "repo-tmp-cargo-targets",
        "repo-tmp-cargo-targets",
        tmp_root,
        tmp_cargo_roots,
        None,
        false,
        true,
    )
}

fn aggregate_entry(
    id: &str,
    kind: &str,
    path: &Path,
    roots: &[PathBuf],
    budget_bytes: Option<u64>,
    managed: bool,
    safe_to_delete: bool,
) -> DynResult<HygieneEntry> {
    let mut bytes = 0u64;
    for root in roots {
        bytes += dir_size_bytes(root).map_err(|error| {
            format!(
                "failed to inspect hygiene aggregate member {}: {error}",
                root.display()
            )
        })?;
    }
    let details = roots
        .iter()
        .map(|root| root.display().to_string())
        .collect::<Vec<_>>();

    Ok(HygieneEntry {
        id: id.to_owned(),
        kind: kind.to_owned(),
        path: path.display().to_string(),
        present: !roots.is_empty(),
        bytes,
        budget_bytes,
        managed,
        safe_to_delete,
        details,
    })
}

fn entry_from_path(
    id: &str,
    kind: &str,
    path: &Path,
    budget_bytes: Option<u64>,
    managed: bool,
    safe_to_delete: bool,
    details: Vec<String>,
) -> DynResult<HygieneEntry> {
    entry_from_path_excluding_roots(
        EntrySpec {
            id,
            kind,
            path,
            budget_bytes,
            managed,
            safe_to_delete,
            details,
        },
        &[],
    )
}

fn entry_from_path_excluding_roots(
    spec: EntrySpec<'_>,
    skipped_roots: &[PathBuf],
) -> DynResult<HygieneEntry> {
    Ok(HygieneEntry {
        id: spec.id.to_owned(),
        kind: spec.kind.to_owned(),
        path: spec.path.display().to_string(),
        present: spec.path.exists(),
        bytes: dir_size_bytes_excluding_roots(spec.path, skipped_roots).map_err(|error| {
            format!(
                "failed to inspect hygiene artifact root {}: {error}",
                spec.path.display()
            )
        })?,
        budget_bytes: spec.budget_bytes,
        managed: spec.managed,
        safe_to_delete: spec.safe_to_delete,
        details: spec.details,
    })
}

pub(super) fn report_violations(entries: &[HygieneEntry]) -> Vec<HygieneViolation> {
    let mut violations = Vec::new();

    for entry in entries {
        if entry.managed && entry.present {
            let missing_markers = missing_managed_markers_for_entry(entry);
            if !missing_markers.is_empty() {
                violations.push(HygieneViolation {
                    id: entry.id.clone(),
                    message: format!(
                        "{} is missing managed-artifact markers: {}",
                        entry.path,
                        missing_markers.join(", ")
                    ),
                });
            }
        }

        if let Some(budget_bytes) = entry.budget_bytes
            && entry.bytes > budget_bytes
        {
            violations.push(HygieneViolation {
                id: entry.id.clone(),
                message: format!(
                    "{} exceeds its {} budget ({})",
                    entry.path,
                    format_bytes(budget_bytes),
                    format_bytes(entry.bytes)
                ),
            });
        }

        if entry.id == "repo-tmp-cargo-targets" && entry.present {
            violations.push(HygieneViolation {
                id: entry.id.clone(),
                message: format!(
                    "repository tmp contains {} cargo target roots; move those builds to the managed artifact roots",
                    entry.details.len()
                ),
            });
        }
    }

    violations
}

pub(super) fn repo_tmp_cargo_roots(repo_root: &Path) -> DynResult<Vec<PathBuf>> {
    let tmp_root = repo_root.join("tmp");
    if !tmp_root.is_dir() {
        return Ok(Vec::new());
    }

    let mut roots = fs::read_dir(&tmp_root)
        .map_err(|error| {
            format!(
                "failed to inspect repository temporary root {}: {error}",
                tmp_root.display()
            )
        })?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .filter(|path| looks_like_cargo_target_dir(path))
        .collect::<Vec<_>>();
    roots.sort();
    Ok(roots)
}

fn looks_like_cargo_target_dir(path: &Path) -> bool {
    [
        ".fingerprint",
        ".rustc_info.json",
        "debug",
        "release",
        "dist",
        "package",
        "CACHEDIR.TAG",
    ]
    .iter()
    .any(|component| path.join(component).exists())
}

pub(super) fn dir_size_bytes(path: &Path) -> DynResult<u64> {
    dir_size_bytes_excluding_roots(path, &[])
}

fn dir_size_bytes_excluding_roots(path: &Path, skipped_roots: &[PathBuf]) -> DynResult<u64> {
    if skipped_roots.iter().any(|root| path == root.as_path()) {
        return Ok(0);
    }
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(error) => {
            return Err(format!(
                "failed to read hygiene metadata {}: {error}",
                path.display()
            )
            .into());
        }
    };
    if metadata.file_type().is_symlink() {
        return Ok(0);
    }
    if metadata.is_file() {
        return Ok(metadata.len());
    }
    if !metadata.is_dir() {
        return Ok(0);
    }

    let entries = fs::read_dir(path).map_err(|error| {
        format!(
            "failed to read hygiene directory {}: {error}",
            path.display()
        )
    })?;
    entries.into_iter().try_fold(0u64, |total, entry| {
        let entry = entry?;
        dir_size_bytes_excluding_roots(&entry.path(), skipped_roots)
            .map(|entry_bytes| total + entry_bytes)
    })
}

pub(super) fn deduplicate_root_set(mut paths: Vec<PathBuf>) -> Vec<PathBuf> {
    paths.sort();
    paths.dedup();
    let mut pruned = Vec::new();

    'candidate: for path in paths {
        for existing in &pruned {
            if path.starts_with(existing) {
                continue 'candidate;
            }
        }
        pruned.retain(|existing: &PathBuf| !existing.starts_with(&path));
        pruned.push(path);
    }

    pruned
}

fn missing_managed_markers(path: &Path) -> Vec<&'static str> {
    let mut missing = Vec::new();
    if !path.join(CACHEDIR_TAG_NAME).is_file() {
        missing.push(CACHEDIR_TAG_NAME);
    }
    if !path.join(ARTIFACT_MANIFEST_NAME).is_file() {
        missing.push(ARTIFACT_MANIFEST_NAME);
    }
    missing
}

fn missing_managed_markers_for_entry(entry: &HygieneEntry) -> Vec<String> {
    let mut missing = missing_managed_markers(Path::new(&entry.path))
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();

    if entry.id == "managed-coverage-target" || entry.id == "managed-coverage-build" {
        let nested_path = Path::new(&entry.path).join("llvm-cov-target");
        missing.extend(
            missing_managed_markers(&nested_path)
                .into_iter()
                .map(|marker| format!("llvm-cov-target/{marker}")),
        );
    }

    missing
}

pub(super) fn format_bytes(bytes: u64) -> String {
    if bytes >= GIB {
        return format!("{:.1} GiB", bytes as f64 / GIB as f64);
    }
    if bytes >= MIB {
        return format!("{:.1} MiB", bytes as f64 / MIB as f64);
    }
    if bytes >= 1024 {
        return format!("{:.1} KiB", bytes as f64 / 1024.0);
    }

    format!("{bytes} B")
}

#[cfg(test)]
pub(crate) fn looks_like_cargo_target_dir_for_tests(path: &Path) -> bool {
    looks_like_cargo_target_dir(path)
}

#[cfg(test)]
pub(crate) fn format_bytes_for_tests(bytes: u64) -> String {
    format_bytes(bytes)
}

#[cfg(all(test, unix))]
pub(crate) fn dir_size_bytes_for_tests(path: &Path) -> u64 {
    dir_size_bytes(path).expect("dir size bytes")
}

#[cfg(all(test, unix))]
pub(crate) fn dir_size_bytes_result_for_tests(path: &Path) -> DynResult<u64> {
    dir_size_bytes(path)
}

#[cfg(all(test, unix))]
pub(crate) fn aggregate_entry_for_tests(path: &Path, roots: &[PathBuf]) -> DynResult<HygieneEntry> {
    aggregate_entry(
        "test-aggregate",
        "test-aggregate",
        path,
        roots,
        Some(0),
        false,
        true,
    )
}
