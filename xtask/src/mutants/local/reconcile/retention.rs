//! Retention rules for reconciled and live mutation evidence.

use std::fs;
use std::path::{Component, Path};

use serde_json::Value;

use crate::DynResult;

/// Caught and unviable mutants have compact aggregate evidence; their per-mutant files are noise.
pub(in crate::mutants::local) fn is_non_actionable_outcome(outcome: &Value) -> bool {
    matches!(
        outcome.get("summary").and_then(Value::as_str),
        Some("CaughtMutant" | "Unviable")
    )
}

/// Removes completed non-actionable evidence during live worker supervision.
pub(in crate::mutants::local) fn prune_live_non_actionable_evidence(
    worker_root: &Path,
) -> DynResult<()> {
    let path = worker_root.join("outcomes.json");
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(format!(
                "failed to read live worker outcome {}: {error}",
                path.display()
            )
            .into());
        }
    };
    let document = match serde_json::from_slice::<Value>(&bytes) {
        Ok(document) => document,
        Err(_) => return Ok(()),
    };
    let Some(outcomes) = document.get("outcomes").and_then(Value::as_array) else {
        return Ok(());
    };
    prune_non_actionable_evidence(worker_root, outcomes, true)
}

/// Retains only actionable per-mutant evidence after strict completed-worker reconciliation.
pub(in crate::mutants::local) fn retain_actionable_worker_evidence(
    worker_root: &Path,
    outcomes: &[Value],
) -> DynResult<()> {
    prune_non_actionable_evidence(worker_root, outcomes, true)?;
    for name in [
        "outcomes.json",
        "mutants.json",
        "caught.txt",
        "missed.txt",
        "timeout.txt",
        "unviable.txt",
    ] {
        let path = worker_root.join(name);
        match fs::remove_file(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(format!(
                    "failed to discard duplicated worker evidence {}: {error}",
                    path.display()
                )
                .into());
            }
        }
    }
    Ok(())
}

fn prune_non_actionable_evidence(
    worker_root: &Path,
    outcomes: &[Value],
    ignore_missing: bool,
) -> DynResult<()> {
    for outcome in outcomes {
        if !is_non_actionable_outcome(outcome) {
            continue;
        }
        for key in ["log_path", "diff_path"] {
            let Some(value) = outcome.get(key) else {
                continue;
            };
            let Some(path_text) = value.as_str() else {
                if value.is_null() {
                    continue;
                }
                return Err(format!("worker outcome `{key}` is neither a string nor null").into());
            };
            let relative = Path::new(path_text);
            if relative.is_absolute()
                || relative
                    .components()
                    .any(|component| !matches!(component, Component::Normal(_)))
            {
                return Err(
                    format!("worker outcome `{key}` is not a safe relative evidence path").into(),
                );
            }
            let path = worker_root.join(relative);
            match fs::remove_file(&path) {
                Ok(()) => {}
                Err(error) if ignore_missing && error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => {
                    return Err(format!(
                        "failed to discard non-actionable worker evidence {}: {error}",
                        path.display()
                    )
                    .into());
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use htmlcut_tempdir::tempdir;
    use serde_json::json;

    use super::*;

    #[test]
    fn strict_pruning_refuses_missing_non_actionable_evidence() {
        let root = tempdir().expect("worker root");
        let outcomes = vec![json!({"summary":"CaughtMutant", "log_path":"log/missing.log"})];
        assert!(prune_non_actionable_evidence(root.path(), &outcomes, false).is_err());
    }
}
