//! Local mutation-campaign orchestration over disposable worker workspaces.

use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroUsize;
use std::path::Path;

use serde_json::Value;

use crate::{DynResult, capture_command_output};

use super::{mutants_list_command, mutants_package_list_command};

mod batching;
mod capacity;
mod reconcile;
mod workspace;

use batching::run_partition_batches;
pub(crate) use reconcile::LocalMutationSummary;
use reconcile::reconcile_workers;
use workspace::{LocalMutationPartition, LocalMutationPartitionKind};

/// Never stage more mutation workspaces than this in one local campaign.
///
/// Every worker owns a complete source copy and isolated incremental Cargo target. The cap bounds
/// temporary storage while permitting the large packages to be split independently.
const MAX_LOCAL_MUTATION_WORKERS: usize = 16;
/// Small maintained fork packages share one worker because their complete mutation corpus is tiny.
const SMALL_PACKAGE_MUTANT_COUNT: usize = 64;
/// Do not spend a fresh workspace baseline on fewer planned mutants than this when a campaign can
/// instead keep them together in one warm worker.
const MIN_MUTANTS_PER_LOCAL_WORKER: usize = 64;
/// Source copies and incremental Cargo targets are storage-heavy. Each lane retains one verified
/// clean workspace and reuses its Cargo baseline for later partitions, so this bounds both disk
/// use and duplicate cold compilation.
const MAX_SIMULTANEOUS_LOCAL_MUTATION_WORKSPACES: usize = 4;

/// Runs a local campaign in independently disposable workspaces and reconciles exact evidence.
///
/// Each worker owns one copied source tree and runs cargo-mutants in place only within that tree.
/// The workers therefore retain one reusable Cargo target apiece without risking the checkout that
/// launched the campaign. Their completed evidence is moved into one canonical `mutants.out` tree
/// and reconciled against the pre-run inventory before this function succeeds.
pub(crate) fn run_local_mutation_campaign(
    repo_root: &Path,
    output_dir: &Path,
    requested_shard: Option<&str>,
    in_diff: Option<&Path>,
) -> DynResult<LocalMutationSummary> {
    let expected_mutants = list_mutants(repo_root, requested_shard, in_diff)?;
    if let Some(summary) = empty_campaign_summary(&expected_mutants) {
        return Ok(summary);
    }

    let partitions = local_partitions(repo_root, &expected_mutants, requested_shard, in_diff)?;
    let worker_root = output_dir.join("local-shard-runs");
    std::fs::create_dir_all(&worker_root)?;
    let (workers, worker_results) =
        run_partition_batches(repo_root, &worker_root, &partitions, in_diff)?;
    let summary = reconcile_workers(output_dir, &workers, &expected_mutants)?;

    let failures = worker_results
        .iter()
        .filter_map(|result| result.as_ref().ok())
        .filter(|status| !status.success())
        .count();
    ensure_worker_failures_are_reconciled(failures, summary)?;

    Ok(summary)
}

fn list_mutants(
    repo_root: &Path,
    requested_shard: Option<&str>,
    in_diff: Option<&Path>,
) -> DynResult<Vec<Value>> {
    let bytes = capture_command_output(repo_root, &mutants_list_command(requested_shard, in_diff))?;
    parse_mutant_inventory(&bytes)
}

fn list_package_mutants(
    repo_root: &Path,
    packages: &[String],
    shard: &str,
    in_diff: Option<&Path>,
) -> DynResult<Vec<Value>> {
    let bytes = capture_command_output(
        repo_root,
        &mutants_package_list_command(packages, shard, in_diff),
    )?;
    parse_mutant_inventory(&bytes)
}

fn parse_mutant_inventory(bytes: &[u8]) -> DynResult<Vec<Value>> {
    if bytes.is_empty() {
        return Ok(Vec::new());
    }
    let mutants = serde_json::from_slice::<Vec<Value>>(bytes)
        .map_err(|error| format!("cargo-mutants emitted an invalid JSON inventory: {error}"))?;
    let _ = mutant_names(&mutants, "planned inventory")?;
    Ok(mutants)
}

fn local_partitions(
    repo_root: &Path,
    expected_mutants: &[Value],
    requested_shard: Option<&str>,
    in_diff: Option<&Path>,
) -> DynResult<Vec<LocalMutationPartition>> {
    if let Some(selector) = requested_shard {
        return Ok(vec![LocalMutationPartition {
            label: format!("workspace-shard:{selector}"),
            kind: LocalMutationPartitionKind::WorkspaceShard(selector.to_owned()),
            expected_names: mutant_names(expected_mutants, "requested shard")?,
        }]);
    }

    let target_worker_count = local_worker_count(expected_mutants.len()).get();
    let groups = coalesce_package_groups(package_groups(expected_mutants)?, target_worker_count);
    let shards_per_group = allocate_group_workers(&groups, target_worker_count);
    let expected_names = mutant_names(expected_mutants, "complete inventory")?;
    let mut selected_names = BTreeSet::new();
    let mut partitions = Vec::with_capacity(target_worker_count);

    for (group, shard_count) in groups.iter().zip(shards_per_group) {
        for index in 0..shard_count {
            let selector = format!("{index}/{shard_count}");
            let inventory = list_package_mutants(repo_root, &group.packages, &selector, in_diff)?;
            let names = mutant_names(&inventory, "package-local workspace shard")?;
            for name in &names {
                if !selected_names.insert(name.clone()) {
                    return Err(
                        format!("duplicate mutant across local package shards: {name}").into(),
                    );
                }
            }
            partitions.push(LocalMutationPartition {
                label: format!("packages:{}:shard:{selector}", group.packages.join(",")),
                kind: LocalMutationPartitionKind::PackageShard {
                    packages: group.packages.clone(),
                    shard_selector: selector,
                },
                expected_names: names,
            });
        }
    }

    ensure_exact_partition_coverage(&selected_names, &expected_names)?;
    Ok(partitions)
}

fn empty_campaign_summary(mutants: &[Value]) -> Option<LocalMutationSummary> {
    mutants.is_empty().then_some(LocalMutationSummary {
        total_mutants: 0,
        caught: 0,
        missed: 0,
        timed_out: 0,
        unviable: 0,
    })
}

fn ensure_worker_failures_are_reconciled(
    failures: usize,
    summary: LocalMutationSummary,
) -> DynResult<()> {
    if failures > 0 && summary.is_clean() {
        return Err(format!(
            "{failures} local cargo-mutants shard(s) exited unsuccessfully without a reconciled missed or timed-out outcome; inspect retained worker logs"
        )
        .into());
    }
    Ok(())
}

fn ensure_exact_partition_coverage(
    selected_names: &BTreeSet<String>,
    expected_names: &BTreeSet<String>,
) -> DynResult<()> {
    if selected_names != expected_names {
        return Err(
            "local package shards did not cover the exact planned mutation inventory".into(),
        );
    }
    Ok(())
}

fn local_worker_count(mutant_count: usize) -> NonZeroUsize {
    let available = std::thread::available_parallelism()
        .map(std::num::NonZeroUsize::get)
        .unwrap_or(1);
    bounded_local_worker_count(available, mutant_count)
}

fn bounded_local_worker_count(available: usize, mutant_count: usize) -> NonZeroUsize {
    let useful_workers = mutant_count.div_ceil(MIN_MUTANTS_PER_LOCAL_WORKER);
    NonZeroUsize::new(
        available
            .min(MAX_LOCAL_MUTATION_WORKERS)
            .min(useful_workers)
            .max(1),
    )
    .expect("the local mutation worker budget is always at least one")
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PackageGroup {
    packages: Vec<String>,
    mutant_count: usize,
}

fn package_groups(mutants: &[Value]) -> DynResult<Vec<PackageGroup>> {
    let mut counts = BTreeMap::<String, usize>::new();
    for mutant in mutants {
        let package = mutant
            .get("package")
            .and_then(Value::as_str)
            .ok_or("cargo-mutants inventory contains an entry without a package")?;
        *counts.entry(package.to_owned()).or_default() += 1;
    }

    let mut groups = Vec::new();
    let mut small_packages = Vec::new();
    let mut small_count = 0;
    for (package, count) in counts {
        if count <= SMALL_PACKAGE_MUTANT_COUNT {
            small_packages.push(package);
            small_count += count;
        } else {
            groups.push(PackageGroup {
                packages: vec![package],
                mutant_count: count,
            });
        }
    }
    if !small_packages.is_empty() {
        groups.push(PackageGroup {
            packages: small_packages,
            mutant_count: small_count,
        });
    }
    groups.sort_by(|left, right| left.packages.cmp(&right.packages));
    Ok(groups)
}

fn coalesce_package_groups(
    mut groups: Vec<PackageGroup>,
    target_worker_count: usize,
) -> Vec<PackageGroup> {
    while groups.len() > target_worker_count {
        groups.sort_by(|left, right| {
            left.mutant_count
                .cmp(&right.mutant_count)
                .then_with(|| left.packages.cmp(&right.packages))
        });
        let left = groups.remove(0);
        let right = groups.remove(0);
        let mut packages = left.packages;
        packages.extend(right.packages);
        packages.sort();
        groups.push(PackageGroup {
            packages,
            mutant_count: left.mutant_count + right.mutant_count,
        });
    }
    groups.sort_by(|left, right| left.packages.cmp(&right.packages));
    groups
}

fn allocate_group_workers(groups: &[PackageGroup], target_worker_count: usize) -> Vec<usize> {
    assert!(
        !groups.is_empty(),
        "mutation worker allocation requires a group"
    );
    assert!(
        groups.len() <= target_worker_count,
        "mutation worker allocation cannot assign fewer workers than groups"
    );
    let mut allocations = vec![1_usize; groups.len()];
    let extra_workers = target_worker_count
        .checked_sub(groups.len())
        .expect("validated mutation worker allocation underflow");
    for _ in 0..extra_workers {
        let mut best = 0;
        for index in 1..groups.len() {
            let left = (groups[index].mutant_count as u128) * (allocations[best] as u128);
            let right = (groups[best].mutant_count as u128) * (allocations[index] as u128);
            let takes_next = match left.cmp(&right) {
                std::cmp::Ordering::Greater => true,
                std::cmp::Ordering::Less => false,
                std::cmp::Ordering::Equal => matches!(
                    groups[index].packages.cmp(&groups[best].packages),
                    std::cmp::Ordering::Less
                ),
            };
            if takes_next {
                best = index;
            }
        }
        allocations[best] = allocations[best]
            .checked_add(1)
            .expect("bounded mutation worker allocation cannot overflow");
    }
    allocations
}

fn mutant_names(mutants: &[Value], context: &str) -> DynResult<BTreeSet<String>> {
    let mut names = BTreeSet::new();
    for mutant in mutants {
        let name = mutant
            .get("name")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("{context} contains an entry without a string name"))?;
        if !names.insert(name.to_owned()) {
            return Err(format!("duplicate mutant in {context}: {name}").into());
        }
    }
    Ok(names)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inventory(package: &str, count: usize) -> Vec<Value> {
        (0..count)
            .map(|index| serde_json::json!({"name": format!("{package}-{index}"), "package": package}))
            .collect()
    }

    #[test]
    fn package_groups_keep_large_packages_isolated_and_coalesce_tiny_forks() {
        let mut mutants = inventory("htmlcut-core", 100);
        mutants.extend(inventory("htmlcut-cli", 70));
        mutants.extend(inventory("htmlcut-selectors", 9));
        mutants.extend(inventory("htmlcut-scraper", 4));

        assert_eq!(
            package_groups(&mutants).expect("package groups"),
            vec![
                PackageGroup {
                    packages: vec!["htmlcut-cli".to_owned()],
                    mutant_count: 70
                },
                PackageGroup {
                    packages: vec!["htmlcut-core".to_owned()],
                    mutant_count: 100
                },
                PackageGroup {
                    packages: vec!["htmlcut-scraper".to_owned(), "htmlcut-selectors".to_owned()],
                    mutant_count: 13,
                },
            ]
        );
    }

    #[test]
    fn package_groups_leave_the_small_group_absent_when_every_package_is_large() {
        let groups = package_groups(&inventory("large", SMALL_PACKAGE_MUTANT_COUNT + 1))
            .expect("large package group");
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].packages, ["large"]);
    }

    #[test]
    fn worker_allocation_balances_mutant_counts_without_splitting_tiny_forks() {
        let groups = vec![
            PackageGroup {
                packages: vec!["cli".to_owned()],
                mutant_count: 1_104,
            },
            PackageGroup {
                packages: vec!["core".to_owned()],
                mutant_count: 2_690,
            },
            PackageGroup {
                packages: vec!["forks".to_owned()],
                mutant_count: 20,
            },
            PackageGroup {
                packages: vec!["xtask".to_owned()],
                mutant_count: 2_068,
            },
        ];
        assert_eq!(allocate_group_workers(&groups, 10), vec![2, 4, 1, 3]);
    }

    #[test]
    fn worker_allocation_uses_load_then_lexical_tie_breaking() {
        let equal_groups = vec![
            PackageGroup {
                packages: vec!["alpha".to_owned()],
                mutant_count: 10,
            },
            PackageGroup {
                packages: vec!["beta".to_owned()],
                mutant_count: 10,
            },
        ];
        assert_eq!(allocate_group_workers(&equal_groups, 3), vec![2, 1]);

        let reverse_lexical_groups = vec![
            PackageGroup {
                packages: vec!["zeta".to_owned()],
                mutant_count: 10,
            },
            PackageGroup {
                packages: vec!["alpha".to_owned()],
                mutant_count: 10,
            },
        ];
        assert_eq!(
            allocate_group_workers(&reverse_lexical_groups, 3),
            vec![1, 2]
        );

        let uneven_groups = vec![
            PackageGroup {
                packages: vec!["zeta".to_owned()],
                mutant_count: 3,
            },
            PackageGroup {
                packages: vec!["alpha".to_owned()],
                mutant_count: 1,
            },
        ];
        assert_eq!(allocate_group_workers(&uneven_groups, 3), vec![2, 1]);
    }

    #[test]
    fn coalescing_respects_a_small_host_worker_budget() {
        let groups = vec![
            PackageGroup {
                packages: vec!["cli".to_owned()],
                mutant_count: 1_104,
            },
            PackageGroup {
                packages: vec!["core".to_owned()],
                mutant_count: 2_690,
            },
            PackageGroup {
                packages: vec!["forks".to_owned()],
                mutant_count: 20,
            },
            PackageGroup {
                packages: vec!["xtask".to_owned()],
                mutant_count: 2_068,
            },
        ];
        let groups = coalesce_package_groups(groups, 2);
        assert_eq!(groups.len(), 2);
        assert_eq!(
            groups.iter().map(|group| group.mutant_count).sum::<usize>(),
            5_882
        );
    }

    #[test]
    fn worker_count_avoids_cold_workspace_fan_out_for_tiny_selections() {
        assert_eq!(bounded_local_worker_count(10, 1).get(), 1);
        assert_eq!(bounded_local_worker_count(10, 63).get(), 1);
        assert_eq!(bounded_local_worker_count(10, 64).get(), 1);
        assert_eq!(bounded_local_worker_count(10, 65).get(), 2);
        assert_eq!(bounded_local_worker_count(10, 5_940).get(), 10);
    }

    #[test]
    fn empty_inventory_has_no_workers_and_nonempty_inventory_does() {
        let empty = parse_mutant_inventory(b"").expect("empty inventory");
        assert_eq!(
            empty_campaign_summary(&empty),
            Some(LocalMutationSummary {
                total_mutants: 0,
                caught: 0,
                missed: 0,
                timed_out: 0,
                unviable: 0,
            })
        );
        assert!(empty_campaign_summary(&inventory("core", 1)).is_none());
    }

    #[test]
    fn worker_failures_must_have_reconciled_failure_evidence() {
        let clean = LocalMutationSummary {
            total_mutants: 1,
            caught: 1,
            missed: 0,
            timed_out: 0,
            unviable: 0,
        };
        assert!(ensure_worker_failures_are_reconciled(0, clean).is_ok());
        assert!(ensure_worker_failures_are_reconciled(1, clean).is_err());
        assert!(
            ensure_worker_failures_are_reconciled(1, LocalMutationSummary { missed: 1, ..clean },)
                .is_ok()
        );
    }

    #[test]
    fn partition_coverage_rejects_missing_or_extra_mutants() {
        let expected = BTreeSet::from(["core-a".to_owned(), "core-b".to_owned()]);
        assert!(ensure_exact_partition_coverage(&expected, &expected).is_ok());
        assert!(
            ensure_exact_partition_coverage(&BTreeSet::from(["core-a".to_owned()]), &expected,)
                .is_err()
        );
        assert!(
            ensure_exact_partition_coverage(
                &BTreeSet::from([
                    "core-a".to_owned(),
                    "core-b".to_owned(),
                    "core-c".to_owned()
                ]),
                &expected,
            )
            .is_err()
        );
    }

    #[test]
    fn mutant_inventory_requires_unique_nonempty_names() {
        let inventory = inventory("core", 2);
        assert_eq!(
            mutant_names(&inventory, "test inventory").expect("unique names"),
            BTreeSet::from(["core-0".to_owned(), "core-1".to_owned()])
        );
        assert!(
            mutant_names(
                &[
                    serde_json::json!({"name": "duplicate"}),
                    serde_json::json!({"name": "duplicate"})
                ],
                "test inventory",
            )
            .is_err()
        );
    }
}

#[cfg(test)]
#[path = "local/tests.rs"]
mod external_tests;
