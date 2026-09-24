//! Capacity-aware concurrency sizing for disposable local mutation workspaces.

use std::num::NonZeroUsize;
use std::path::Path;
#[cfg(unix)]
use std::process::Command;

use crate::DynResult;

const GIB: u64 = 1024 * 1024 * 1024;
/// Conservative per-lane budget for one reusable source copy, its incremental Cargo state, and
/// compact actionable mutation evidence. Caught and unviable per-mutant logs are deliberately not
/// retained, so they do not inflate this operational budget.
const WORKSPACE_BUDGET_BYTES: u64 = 10 * GIB;
/// Capacity never allocated to staged source workspaces. The independent runtime guard below keeps
/// two GiB free before every batch, so this reserve protects preparation and ordinary host work.
const FREE_SPACE_RESERVE_BYTES: u64 = 12 * GIB;
/// Minimum free space required immediately before launching another mutation batch in an already
/// staged workspace. The larger admission budget above accounts for materializing source and
/// Cargo state; this guard prevents an unrelated writer from turning a warm campaign into a
/// filesystem-exhaustion failure between partitions.
const RUNTIME_BATCH_HEADROOM_BYTES: u64 = 2 * GIB;

/// Returns the maximum safely concurrent source-workspace lanes for this host and request.
pub(super) fn available_workspace_lanes(
    requested_lanes: usize,
    temporary_root: &Path,
) -> DynResult<NonZeroUsize> {
    let available = available_bytes(temporary_root)?;
    let lanes = workspace_lane_count_from_available_bytes(requested_lanes, available)?;
    Ok(NonZeroUsize::new(lanes).expect("workspace lane capacity is always nonzero"))
}

/// Refuses to launch another batch when the temporary volume has lost its operational headroom.
///
/// This is deliberately separate from lane admission: an already staged lane should not pretend
/// that it needs to be materialized again, but it must leave room for Cargo, cargo-mutants, and
/// the host while it evaluates its next partition.
pub(super) fn ensure_runtime_batch_headroom(temporary_root: &Path) -> DynResult<()> {
    ensure_runtime_batch_headroom_from_available_bytes(available_bytes(temporary_root)?)
}

fn available_bytes(path: &Path) -> DynResult<u64> {
    #[cfg(unix)]
    {
        let output = Command::new("df")
            .args(["-Pk", &path.display().to_string()])
            .output()?;
        parse_df_available_bytes(
            path,
            output.status.success(),
            &output.stdout,
            &output.status.to_string(),
        )
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Err("safe local mutation workspaces require Unix free-space inspection".into())
    }
}

#[cfg(any(unix, test))]
fn parse_df_available_bytes(
    path: &Path,
    succeeded: bool,
    stdout: &[u8],
    status: &str,
) -> DynResult<u64> {
    if !succeeded {
        return Err(format!(
            "could not inspect free space for local mutation workspaces at {}: df exited {}",
            path.display(),
            status
        )
        .into());
    }
    let stdout = String::from_utf8(stdout.to_vec())
        .map_err(|error| format!("df emitted non-UTF-8 free-space output: {error}"))?;
    parse_df_available_blocks(&stdout).and_then(|blocks| {
        blocks.checked_mul(1024).ok_or_else(|| {
            "df free-space block count overflowed bytes for local mutation workspaces".into()
        })
    })
}

#[cfg(any(unix, test))]
fn parse_df_available_blocks(output: &str) -> DynResult<u64> {
    let row = output
        .lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .ok_or("df produced no filesystem row for local mutation workspaces")?;
    let fields = row.split_whitespace().collect::<Vec<_>>();
    let capacity_index = fields
        .iter()
        .position(|field| field.ends_with('%'))
        .ok_or("df filesystem row has no capacity column for local mutation workspaces")?;
    let available = fields
        .get(
            capacity_index
                .checked_sub(1)
                .ok_or("df filesystem row has no available-block column")?,
        )
        .ok_or("df filesystem row has no available-block column")?;
    available.parse::<u64>().map_err(|error| {
        format!("df available-block value {available:?} is not an unsigned integer: {error}").into()
    })
}

fn workspace_lane_count_from_available_bytes(
    requested_lanes: usize,
    available_bytes: u64,
) -> DynResult<usize> {
    if requested_lanes == 0 {
        return Err("local mutation campaign requested zero source-workspace lanes".into());
    }
    let usable = available_bytes.saturating_sub(FREE_SPACE_RESERVE_BYTES);
    let capacity = usize::try_from(usable / WORKSPACE_BUDGET_BYTES)
        .map_err(|_| "local mutation workspace capacity does not fit usize")?;
    let lanes = requested_lanes.min(capacity);
    if lanes == 0 {
        return Err(format!(
            "local mutation campaign requires at least {} GiB free at {} ({} GiB reserved plus {} GiB per reusable workspace)",
            (FREE_SPACE_RESERVE_BYTES + WORKSPACE_BUDGET_BYTES) / GIB,
            std::env::temp_dir().display(),
            FREE_SPACE_RESERVE_BYTES / GIB,
            WORKSPACE_BUDGET_BYTES / GIB,
        )
        .into());
    }
    Ok(lanes)
}

fn ensure_runtime_batch_headroom_from_available_bytes(available_bytes: u64) -> DynResult<()> {
    if available_bytes < RUNTIME_BATCH_HEADROOM_BYTES {
        return Err(format!(
            "local mutation campaign stopped before another partition because only {} MiB remains on the temporary volume; at least {} GiB runtime headroom is required",
            available_bytes / (1024 * 1024),
            RUNTIME_BATCH_HEADROOM_BYTES / GIB,
        )
        .into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn df_parser_reads_the_available_column_without_assuming_mountpoint_shape() {
        assert_eq!(
            parse_df_available_blocks(
                "Filesystem 1024-blocks Used Available Capacity Mounted on\n/dev/disk3s5 100 20 80 20% /System/Volumes/Data\n"
            )
            .expect("parse conventional df"),
            80
        );
        assert_eq!(
            parse_df_available_blocks(
                "Filesystem 1024-blocks Used Available Capacity Mounted on\n/dev/disk3s5 100 20 80 20% /Volumes/with space\n"
            )
            .expect("parse spaced mountpoint"),
            80
        );
    }

    #[test]
    fn df_capacity_reader_rejects_failed_malformed_or_unrepresentable_output() {
        let path = Path::new("/temporary-workspace");
        assert!(parse_df_available_bytes(path, false, b"", "exit status: 9").is_err());
        assert!(parse_df_available_bytes(path, true, &[0xff], "success").is_err());
        assert!(parse_df_available_blocks("Filesystem\n").is_err());
        assert!(parse_df_available_blocks("Filesystem\n/dev 100 20 80 /tmp\n").is_err());
        assert!(parse_df_available_blocks("Filesystem\n/dev 100 20 80 20% /tmp\n").is_ok());
        assert!(parse_df_available_blocks("Filesystem\n/dev 100 20 value 20% /tmp\n").is_err());
        assert!(
            parse_df_available_bytes(
                path,
                true,
                b"Filesystem\n/dev 1 1 18446744073709551615 1% /tmp\n",
                "success",
            )
            .is_err()
        );
    }

    #[test]
    fn lane_capacity_reserves_space_before_admitting_reusable_workspaces() {
        const BYTES_PER_GIB: u64 = 1_073_741_824;
        let one_lane = 22 * BYTES_PER_GIB;
        assert_eq!(
            workspace_lane_count_from_available_bytes(4, one_lane).expect("one lane"),
            1
        );
        assert_eq!(
            workspace_lane_count_from_available_bytes(4, one_lane + 30 * BYTES_PER_GIB)
                .expect("four lanes"),
            4
        );
        assert!(workspace_lane_count_from_available_bytes(1, one_lane - 1).is_err());
        assert!(workspace_lane_count_from_available_bytes(0, one_lane).is_err());
    }

    #[test]
    fn runtime_batch_headroom_refuses_another_partition_before_the_volume_is_exhausted() {
        let documented_runtime_reserve = 2_u64 * 1024 * 1024 * 1024;
        assert_eq!(RUNTIME_BATCH_HEADROOM_BYTES, documented_runtime_reserve);
        assert!(
            ensure_runtime_batch_headroom_from_available_bytes(documented_runtime_reserve).is_ok()
        );
        assert!(
            ensure_runtime_batch_headroom_from_available_bytes(documented_runtime_reserve - 1)
                .is_err()
        );
    }
}
