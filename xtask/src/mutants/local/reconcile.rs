//! Exact, deterministic reconciliation of independently executed local mutation shards.

use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

use serde_json::{Value, json};

use crate::{DynResult, remove_dir_if_exists};

use super::workspace::CompletedMutationWorker;

mod retention;
pub(super) use retention::{
    is_non_actionable_outcome, prune_live_non_actionable_evidence,
    retain_actionable_worker_evidence,
};

const LOCAL_AGGREGATE_SCHEMA: &str = "htmlcut.local_mutation_aggregate@1";

/// Reconciled outcome totals for one local mutation campaign.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct LocalMutationSummary {
    /// Number of mutants reconciled across every local shard.
    pub(crate) total_mutants: u64,
    /// Number of mutants rejected by the test suite.
    pub(crate) caught: u64,
    /// Number of mutants that survived the test suite.
    pub(crate) missed: u64,
    /// Number of mutants whose execution timed out.
    pub(crate) timed_out: u64,
    /// Number of mutants that could not be built or run.
    pub(crate) unviable: u64,
}

impl LocalMutationSummary {
    /// Returns whether the reconciled campaign has no surviving or timed-out mutant.
    pub(crate) const fn is_clean(self) -> bool {
        self.missed == 0 && self.timed_out == 0
    }
}

/// Reconciles completed worker roots into one canonical `mutants.out` evidence tree.
pub(super) fn reconcile_workers(
    output_dir: &Path,
    workers: &[CompletedMutationWorker],
    expected_mutants: &[Value],
) -> DynResult<LocalMutationSummary> {
    reconcile_workers_with(
        output_dir,
        workers,
        expected_mutants,
        persist_aggregate_documents,
    )
}

fn reconcile_workers_with(
    output_dir: &Path,
    workers: &[CompletedMutationWorker],
    expected_mutants: &[Value],
    persist: impl Fn(&Path, Value, Value, Value) -> DynResult<()>,
) -> DynResult<LocalMutationSummary> {
    let canonical_root = output_dir.join("mutants.out");
    remove_dir_if_exists(&canonical_root)?;
    fs::create_dir_all(canonical_root.join("shards"))?;

    let expected_names = ensure_unique_mutant_names(expected_mutants, "planned")?;
    let mut aggregate_outcomes = Vec::new();
    let mut aggregate_mutants = Vec::new();
    let mut aggregate_names = BTreeSet::new();
    let mut counts = OutcomeCounts::default();
    let mut start_times = Vec::new();
    let mut end_times = Vec::new();
    let mut versions = BTreeSet::new();
    let mut lists = MutationOutcomeLists::default();
    let mut shard_manifest = Vec::new();

    for worker in workers {
        let shard_root = worker.output_root.join("mutants.out");
        let document = read_worker_outcome(&shard_root)?;
        ensure_worker_baseline_succeeded(worker, &document, &shard_root)?;
        let shard_summary = parse_worker_summary(&document, &shard_root)?;
        let outcomes = document
            .get("outcomes")
            .and_then(Value::as_array)
            .ok_or("worker outcome has no outcomes array after successful validation")?;
        let shard_mutants = read_worker_mutants(&shard_root)?;
        let shard_names = ensure_unique_mutant_names(&shard_mutants, "worker")?;
        if shard_names != worker.expected_names {
            return Err(format!(
                "worker {} did not produce its exact planned mutation partition",
                worker.selector
            )
            .into());
        }
        if shard_summary.total_mutants != usize_to_u64(shard_mutants.len())? {
            return Err(format!(
                "worker {} outcome count does not match its mutant inventory",
                worker.selector
            )
            .into());
        }
        for name in &shard_names {
            if !aggregate_names.insert(name.clone()) {
                return Err(format!("duplicate mutant across local workers: {name}").into());
            }
        }

        let moved_root = canonical_root.join("shards").join(worker.index.to_string());
        fs::rename(&shard_root, &moved_root)?;
        move_worker_runner_logs(worker, &moved_root)?;
        append_worker_outcomes(&mut aggregate_outcomes, outcomes, worker.index)?;
        aggregate_mutants.extend(shard_mutants);
        lists.extend_from_worker(&moved_root)?;
        retain_actionable_worker_evidence(&moved_root, outcomes)?;
        counts.add(&shard_summary)?;
        start_times.push(shard_summary.start_time);
        end_times.push(shard_summary.end_time);
        versions.insert(shard_summary.cargo_mutants_version);
        shard_manifest.push(json!({
            "index": worker.index,
            "selector": worker.selector,
            "result_root": format!("shards/{}", worker.index),
            "total_mutants": shard_summary.total_mutants,
            "caught": shard_summary.caught,
            "missed": shard_summary.missed,
            "timeout": shard_summary.timed_out,
            "unviable": shard_summary.unviable,
        }));
    }

    if aggregate_names != expected_names {
        return Err(
            "local mutation shards did not cover the exact planned mutant inventory".into(),
        );
    }
    if versions.len() != 1 {
        return Err("local mutation workers used different cargo-mutants versions".into());
    }

    lists.write_to(&canonical_root)?;
    let start_time = start_times
        .into_iter()
        .min()
        .ok_or("local mutation campaign had no worker start time")?;
    let end_time = end_times
        .into_iter()
        .max()
        .ok_or("local mutation campaign had no worker end time")?;
    aggregate_mutants.sort_by(|left, right| mutant_name(left).cmp(&mutant_name(right)));
    let outcomes = json!({
        "cargo_mutants_version": versions.into_iter().next().expect("one version"),
        "caught": counts.caught,
        "end_time": end_time,
        "missed": counts.missed,
        "outcomes": aggregate_outcomes,
        "start_time": start_time,
        "success": counts.is_clean(),
        "timeout": counts.timed_out,
        "total_mutants": counts.total_mutants,
        "unviable": counts.unviable,
    });
    let manifest = json!({
        "schema": LOCAL_AGGREGATE_SCHEMA,
        "expected_mutant_count": expected_mutants.len(),
        "worker_count": workers.len(),
        "shards": shard_manifest,
    });
    persist(
        &canonical_root,
        Value::Array(aggregate_mutants),
        outcomes,
        manifest,
    )?;
    let staging_root = workers
        .first()
        .expect("a non-empty mutation campaign always has one completed partition")
        .output_root
        .parent()
        .expect("a worker output root always has its staging parent");
    remove_dir_if_exists(staging_root)?;

    Ok(LocalMutationSummary {
        total_mutants: counts.total_mutants,
        caught: counts.caught,
        missed: counts.missed,
        timed_out: counts.timed_out,
        unviable: counts.unviable,
    })
}

fn persist_aggregate_documents(
    canonical_root: &Path,
    mutants: Value,
    outcomes: Value,
    manifest: Value,
) -> DynResult<()> {
    write_json(&canonical_root.join("mutants.json"), &mutants)?;
    write_json(&canonical_root.join("outcomes.json"), &outcomes)?;
    write_json(&canonical_root.join("local-aggregate.json"), &manifest)
}

fn ensure_worker_baseline_succeeded(
    worker: &CompletedMutationWorker,
    document: &Value,
    shard_root: &Path,
) -> DynResult<()> {
    let outcomes = document
        .get("outcomes")
        .and_then(Value::as_array)
        .ok_or("worker outcome has no outcomes array")?;
    let baseline = outcomes
        .iter()
        .find(|outcome| outcome.get("scenario").and_then(Value::as_str) == Some("Baseline"))
        .ok_or_else(|| {
            format!(
                "worker {} outcome has no baseline evidence: {}",
                worker.selector,
                shard_root.display()
            )
        })?;

    if baseline.get("summary").and_then(Value::as_str) == Some("Success") {
        return Ok(());
    }

    Err(format!(
        "worker {} baseline failed; no mutation outcomes are valid; inspect {}",
        worker.selector,
        shard_root.join("log/baseline.log").display(),
    )
    .into())
}

fn move_worker_runner_logs(
    worker: &CompletedMutationWorker,
    canonical_root: &Path,
) -> DynResult<()> {
    for name in ["runner.stdout.log", "runner.stderr.log"] {
        let source = worker.output_root.join(name);
        let destination = canonical_root.join(name);
        fs::rename(&source, &destination).map_err(|error| {
            format!(
                "failed to retain worker {} runner log {}: {error}",
                worker.selector,
                source.display()
            )
        })?;
    }
    Ok(())
}

#[derive(Debug)]
struct WorkerSummary {
    cargo_mutants_version: String,
    caught: u64,
    end_time: String,
    missed: u64,
    start_time: String,
    timed_out: u64,
    total_mutants: u64,
    unviable: u64,
}

#[derive(Default)]
struct OutcomeCounts {
    caught: u64,
    missed: u64,
    timed_out: u64,
    total_mutants: u64,
    unviable: u64,
}

impl OutcomeCounts {
    fn add(&mut self, summary: &WorkerSummary) -> DynResult<()> {
        self.caught = checked_count_sum(self.caught, summary.caught)?;
        self.missed = checked_count_sum(self.missed, summary.missed)?;
        self.timed_out = checked_count_sum(self.timed_out, summary.timed_out)?;
        self.total_mutants = checked_count_sum(self.total_mutants, summary.total_mutants)?;
        self.unviable = checked_count_sum(self.unviable, summary.unviable)?;
        Ok(())
    }

    fn is_clean(&self) -> bool {
        (self.missed == 0) & (self.timed_out == 0)
    }
}

fn checked_count_sum(left: u64, right: u64) -> DynResult<u64> {
    left.checked_add(right)
        .ok_or_else(|| "local mutation outcome count overflowed".into())
}

fn read_worker_outcome(shard_root: &Path) -> DynResult<Value> {
    let path = shard_root.join("outcomes.json");
    let bytes = fs::read(&path).map_err(|error| {
        format!(
            "missing completed worker outcome {}: {error}",
            path.display()
        )
    })?;
    serde_json::from_slice(&bytes).map_err(|error| {
        format!(
            "invalid worker mutation outcome {}: {error}",
            path.display()
        )
        .into()
    })
}

fn parse_worker_summary(document: &Value, shard_root: &Path) -> DynResult<WorkerSummary> {
    let total_mutants = outcome_count(document, "total_mutants", shard_root)?;
    let caught = outcome_count(document, "caught", shard_root)?;
    let missed = outcome_count(document, "missed", shard_root)?;
    let timed_out = outcome_count(document, "timeout", shard_root)?;
    let unviable = outcome_count(document, "unviable", shard_root)?;
    let reconciled_total = checked_count_sum(
        checked_count_sum(caught, missed)?,
        checked_count_sum(timed_out, unviable)?,
    )?;
    if total_mutants != reconciled_total {
        return Err(format!(
            "worker outcome totals do not reconcile: {}",
            shard_root.display()
        )
        .into());
    }

    Ok(WorkerSummary {
        cargo_mutants_version: outcome_string(document, "cargo_mutants_version", shard_root)?,
        caught,
        end_time: outcome_string(document, "end_time", shard_root)?,
        missed,
        start_time: outcome_string(document, "start_time", shard_root)?,
        timed_out,
        total_mutants,
        unviable,
    })
}

fn outcome_count(document: &Value, key: &str, shard_root: &Path) -> DynResult<u64> {
    document.get(key).and_then(Value::as_u64).ok_or_else(|| {
        format!(
            "worker outcome has no non-negative integer `{key}`: {}",
            shard_root.display()
        )
        .into()
    })
}

fn outcome_string(document: &Value, key: &str, shard_root: &Path) -> DynResult<String> {
    document
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| {
            format!(
                "worker outcome has no non-empty `{key}`: {}",
                shard_root.display()
            )
            .into()
        })
}

fn read_worker_mutants(shard_root: &Path) -> DynResult<Vec<Value>> {
    let path = shard_root.join("mutants.json");
    let bytes = fs::read(&path).map_err(|error| {
        format!(
            "missing worker mutant inventory {}: {error}",
            path.display()
        )
    })?;
    serde_json::from_slice(&bytes).map_err(|error| {
        format!(
            "invalid worker mutant inventory {}: {error}",
            path.display()
        )
        .into()
    })
}

fn append_worker_outcomes(
    aggregate: &mut Vec<Value>,
    outcomes: &[Value],
    worker_index: usize,
) -> DynResult<()> {
    for outcome in outcomes {
        let mut outcome = outcome.clone();
        let non_actionable = is_non_actionable_outcome(&outcome);
        for key in ["log_path", "diff_path"] {
            if let Some(path) = outcome.get_mut(key) {
                let Some(path_text) = path.as_str() else {
                    if path.is_null() {
                        continue;
                    }
                    return Err(
                        format!("worker outcome `{key}` is neither a string nor null").into(),
                    );
                };
                *path = if non_actionable {
                    Value::Null
                } else {
                    Value::String(format!("shards/{worker_index}/{path_text}"))
                };
            }
        }
        aggregate.push(outcome);
    }
    Ok(())
}

#[derive(Default)]
struct MutationOutcomeLists {
    caught: Vec<String>,
    missed: Vec<String>,
    timeout: Vec<String>,
    unviable: Vec<String>,
}

impl MutationOutcomeLists {
    fn extend_from_worker(&mut self, worker_root: &Path) -> DynResult<()> {
        self.caught
            .extend(read_outcome_list(worker_root, "caught")?);
        self.missed
            .extend(read_outcome_list(worker_root, "missed")?);
        self.timeout
            .extend(read_outcome_list(worker_root, "timeout")?);
        self.unviable
            .extend(read_outcome_list(worker_root, "unviable")?);
        Ok(())
    }

    fn write_to(&mut self, canonical_root: &Path) -> DynResult<()> {
        for (name, lines) in [
            ("caught", &mut self.caught),
            ("missed", &mut self.missed),
            ("timeout", &mut self.timeout),
            ("unviable", &mut self.unviable),
        ] {
            lines.sort();
            lines.dedup();
            let mut file = File::create(canonical_root.join(format!("{name}.txt")))?;
            for line in lines {
                writeln!(file, "{line}")?;
            }
        }
        Ok(())
    }
}

fn read_outcome_list(worker_root: &Path, name: &str) -> DynResult<Vec<String>> {
    let path = worker_root.join(format!("{name}.txt"));
    let contents = fs::read_to_string(&path)
        .map_err(|error| format!("missing worker `{name}` list {}: {error}", path.display()))?;
    Ok(contents.lines().map(ToOwned::to_owned).collect())
}

fn ensure_unique_mutant_names(mutants: &[Value], context: &str) -> DynResult<BTreeSet<String>> {
    let mut names = BTreeSet::new();
    for mutant in mutants {
        let name = mutant_name(mutant).ok_or_else(|| {
            format!("{context} mutant inventory contains an entry without a string name")
        })?;
        if !names.insert(name.to_owned()) {
            return Err(format!("duplicate {context} mutant: {name}").into());
        }
    }
    Ok(names)
}

fn mutant_name(mutant: &Value) -> Option<&str> {
    mutant.get("name").and_then(Value::as_str)
}

fn usize_to_u64(value: usize) -> DynResult<u64> {
    u64::try_from(value).map_err(|_| "mutation count cannot be represented as u64".into())
}

fn write_json(path: &Path, value: &Value) -> DynResult<()> {
    fs::write(path, serde_json::to_vec_pretty(value)?)?;
    Ok(())
}

#[cfg(test)]
mod tests;
