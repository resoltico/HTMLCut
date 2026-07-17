//! Fail-closed ownership, dependency, and source-shape verification for first-party Rust code.

mod metrics;
mod policy;

use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::command_exec::repo_worktree_files;
use crate::model::DynResult;

use self::metrics::{Metrics, measured_internal_dependencies};
use self::policy::{Policy, Rule};

const POLICY_PATH: &str = "tooling/rust-source-shape-policy.toml";
const SOURCE_ROOTS: [&str; 8] = [
    "crates/htmlcut-core/src",
    "crates/htmlcut-core/tests",
    "crates/htmlcut-cli/src",
    "crates/htmlcut-cli/tests",
    "crates/htmlcut-tempdir/src",
    "xtask/src",
    "xtask/tests",
    "fuzz/fuzz_targets",
];

/// Enforces HTMLCut's repository-owned Rust source-structure contract.
pub fn check_source_structure(repo_root: &Path) -> DynResult<()> {
    let started = Instant::now();
    let policy = load_policy(repo_root)?;
    let findings = collect_findings(repo_root, &policy)?;

    if findings.is_empty() {
        if crate::gate_report::is_active() {
            crate::gate_report::record_internal_check(
                "Rust source-structure contract",
                Ok(()),
                started.elapsed(),
            );
        } else {
            println!("Rust source-structure gate passed.");
        }
        return Ok(());
    }

    if crate::gate_report::is_active() {
        crate::gate_report::record_internal_check(
            "Rust source-structure contract",
            Err(findings.join("\n")),
            started.elapsed(),
        );
    } else {
        eprintln!("Rust source-structure gate failed:");
        for finding in findings {
            eprintln!("- {finding}");
        }
    }
    Err("source-structure contract failed".into())
}

/// Prints measured source shape and resolved ownership for every maintained first-party Rust file.
pub fn report_source_structure(repo_root: &Path) -> DynResult<()> {
    let policy = load_policy(repo_root)?;
    let mut rows = Vec::new();

    for source in maintained_sources(repo_root)? {
        let contents = fs::read_to_string(&source.path)?;
        let metrics = Metrics::from_source(&contents)?;
        let role = policy
            .rule_for(&source.relative_path)
            .map_or("UNOWNED", Rule::role);
        rows.push((source.relative_path, role.to_owned(), metrics));
    }

    for (path, role, metrics) in rows {
        println!(
            "{path}\trole={role}\tlines={}\titems={}\tpublic_items={}\timports={}\tfunctions={}\tdecisions={}\tmatch_arms={}",
            metrics.physical_lines,
            metrics.item_count,
            metrics.public_item_count,
            metrics.import_count,
            metrics.function_count,
            metrics.decision_points,
            metrics.match_arms,
        );
    }
    Ok(())
}

#[derive(Debug)]
struct MaintainedSource {
    path: PathBuf,
    relative_path: String,
}

fn load_policy(repo_root: &Path) -> DynResult<Policy> {
    let path = repo_root.join(POLICY_PATH);
    let source = fs::read_to_string(&path).map_err(|error| {
        format!(
            "cannot read Rust source-shape policy {}: {error}",
            path.display()
        )
    })?;
    Policy::parse(&source)
}

fn collect_findings(repo_root: &Path, policy: &Policy) -> DynResult<Vec<String>> {
    let mut findings = policy.expired_rule_findings()?;
    let mut matched_rule_paths = BTreeSet::new();

    for source in maintained_sources(repo_root)? {
        let contents = fs::read_to_string(&source.path)?;
        let metrics = Metrics::from_source(&contents)?;
        let dependencies = measured_internal_dependencies(&contents)?;
        let Some(rule) = policy.rule_for(&source.relative_path) else {
            findings.push(format!(
                "{}: has no declared ownership rule in {POLICY_PATH}",
                source.relative_path
            ));
            continue;
        };

        matched_rule_paths.insert(rule.path().to_owned());
        findings.extend(rule.budget_findings(&source.relative_path, &metrics));
        findings.extend(rule.dependency_findings(&source.relative_path, &dependencies));
    }

    findings.extend(policy.unmatched_rule_findings(&matched_rule_paths));
    findings.sort();
    Ok(findings)
}

fn maintained_sources(repo_root: &Path) -> DynResult<Vec<MaintainedSource>> {
    let paths = match repo_worktree_files(repo_root)? {
        Some(paths) => paths,
        None => recursively_discover_sources(repo_root)?,
    };
    let mut sources = Vec::new();
    for path in paths {
        if let Some(source) = source_from_path(repo_root, path)? {
            sources.push(source);
        }
    }
    sources.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    sources.dedup_by(|left, right| left.relative_path == right.relative_path);
    Ok(sources)
}

fn recursively_discover_sources(repo_root: &Path) -> DynResult<Vec<PathBuf>> {
    let mut paths = Vec::new();
    for root in SOURCE_ROOTS {
        collect_rust_files(&repo_root.join(root), &mut paths)?;
    }
    Ok(paths)
}

fn collect_rust_files(path: &Path, paths: &mut Vec<PathBuf>) -> DynResult<()> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    if metadata.file_type().is_symlink() {
        return Err(format!(
            "Rust source-structure inventory rejects symlinked source path {}",
            path.display()
        )
        .into());
    }
    if metadata.is_file() {
        if path.extension() == Some(OsStr::new("rs")) {
            paths.push(path.to_owned());
        }
        return Ok(());
    }
    if !metadata.is_dir() {
        return Err(format!(
            "Rust source-structure inventory found a non-file, non-directory path {}",
            path.display()
        )
        .into());
    }

    for entry in fs::read_dir(path)? {
        collect_rust_files(&entry?.path(), paths)?;
    }
    Ok(())
}

fn source_from_path(repo_root: &Path, path: PathBuf) -> DynResult<Option<MaintainedSource>> {
    if path.extension() != Some(OsStr::new("rs")) {
        return Ok(None);
    }
    let relative = path.strip_prefix(repo_root).map_err(|_| {
        format!(
            "Rust source-structure inventory found a path outside the repository root: {}",
            path.display()
        )
    })?;
    let relative_path = relative.to_string_lossy().replace('\\', "/");
    let is_maintained = SOURCE_ROOTS
        .iter()
        .any(|root| relative_path.starts_with(&format!("{root}/")));
    if !is_maintained {
        return Ok(None);
    }
    let metadata = fs::symlink_metadata(&path)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(format!(
            "Rust source-structure inventory requires a regular, non-symlink source file: {}",
            path.display()
        )
        .into());
    }
    let canonical_root = repo_root.canonicalize()?;
    let canonical_path = path.canonicalize()?;
    if !canonical_path.starts_with(&canonical_root) {
        return Err(format!(
            "Rust source-structure inventory rejects source escaping the repository root: {}",
            path.display()
        )
        .into());
    }
    Ok(Some(MaintainedSource {
        path,
        relative_path,
    }))
}

#[cfg(test)]
mod tests;
