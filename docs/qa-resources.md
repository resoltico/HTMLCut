# QA resource limits

Mutation and fuzz checks use [the resource watchdog](../scripts/run-resource-bounded.py). Within the run's owned output root, it samples the larger of logical and allocated bytes for each file and directory, retains a capped combined log and records sampled disk usage, elapsed time, child exit status and refusal reason. Exceeding time, disk or log budgets fails the check. Results remain available for investigation.

Disk accounting is sampled, so writes can exceed the threshold between samples. A filesystem quota is required for a hard disk limit. Tool installation, Cargo's download cache and installed toolchains are separate prerequisites and remain outside the run's disk accounting and cleanup.

On POSIX the watchdog terminates the command's process group. On Windows it uses `taskkill /T /F` for the active process tree; cleanup of descendants orphaned by an already exited Windows parent is not guaranteed.

## Mutation review

Install `cargo-mutants` at the version in [the tool inventory](../scripts/contributor-rust-tools.sh), fetch locked dependencies, and run:

```sh
cargo fetch --locked
python3 scripts/mutation-audit.py --output /path/outside/checkout/mutation-results
```

The output directory must not exist, its parent must exist, and both must be outside the checkout. Unset shared Cargo target/build directory overrides before running. Builds use locked dependencies offline. The runner requires enough free space for the selected disk budget before starting.

The runner's `--help` describes resource and selection options. Defaults are up to two workers, a 4 GiB sampled disk budget and a 30-minute overall deadline. The shared jobserver limits compiler concurrency to available CPUs. Each worker reuses its own build products; incremental compilation and debug information are disabled. Existing targets and VCS data are not copied. Explicit relative Cargo build paths keep products in the owned worker roots, which are removed after success or failure.

Survivor logs, diffs, resource reports and structured outcomes remain in the output directory. Choose a fresh output directory for each invocation. Use `--jobs 1` to reduce duplicate build products, or select an explicit disk or time budget with `--disk-limit-mib` and `--timeout`. Resource reports record observed usage; a refusal leaves the review incomplete.

Use `--file`, `--regex` or `--in-diff` for focused iteration. Every run retains the unmutated baseline, workspace test selection and locked dependencies. A focused selection does not replace the full release review. Inspect every survivor and timeout before declaring that review complete; `audit.json` records execution and review requirements without inferring completion from an exit code.

[Fuzzing](../fuzz/README.md) documents the two bounded fuzz targets, execution counts, resource allowances and cleanup rules used locally and by CI.
