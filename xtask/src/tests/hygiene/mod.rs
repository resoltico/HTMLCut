use super::*;

const TEST_LEGACY_REPO_TARGET_BYTES: u64 = 512 * 1024 * 1024 + 1;

fn with_test_artifact_overrides<T>(repo_root: &Path, operation: impl FnOnce() -> T) -> T {
    crate::plan::with_cargo_artifact_dir_overrides_for_tests(
        repo_root.join(".managed-artifacts/target"),
        repo_root.join(".managed-artifacts/build"),
        operation,
    )
}

mod artifacts;
mod failures_and_sizes;
mod reports;
