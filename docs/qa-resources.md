# QA resource limits

Mutation and fuzz checks use [the resource watchdog](../scripts/run-resource-bounded.py). It samples the larger of logical and allocated file bytes, plus directory allocation where reported, inside the task-owned output root, retains a capped combined log and records the sampled peak, elapsed time, child exit status and refusal reason. Exceeding time, disk or log budgets fails the check. On POSIX it kills the command's process group; on Windows it uses `taskkill /T /F` for the active process tree. Cleanup of descendants orphaned by an already exited Windows parent is not guaranteed by this wrapper; a Windows Job Object would be required for that guarantee. Results remain available for investigation.

Disk accounting is a sampled watchdog, **not a hard filesystem quota**. A child can write past the requested budget between samples. It cannot honestly establish an exact maximum disk footprint; a size-limited filesystem or filesystem quota is required for that property. Tool installation, Cargo's existing download cache and installed toolchains are separate prerequisites. Mutation builds are offline, so the audit does not fetch into the owner's download cache. No script cleans owner caches or other audits.

## Mutation review

Fetch locked dependencies once, install the tools from [the inventory](../scripts/contributor-rust-tools.sh), and run:

```sh
cargo fetch --locked
python3 scripts/mutation-audit.py --output /path/outside/checkout/new-audit
```

The output directory must not exist. [The runner's command help](../scripts/mutation-audit.py) declares the concurrency, disk and time defaults. Two concurrent workers amortize test waiting without creating a build root for every mutant, while cargo-mutants' shared jobserver limits total compiler concurrency to available CPUs. Each worker reuses its own build products during the audit. Incremental compilation and debug information are disabled, existing targets/VCS data are not copied, and worker build roots are removed after success or failure. Explicit relative Cargo target/build arguments keep copied Cargo configuration from redirecting products into owner caches. Survivor logs, diffs and structured outcomes remain; repeatedly running an audit requires a new owned directory rather than rotating previous evidence.

The default 4 GiB disk budget is a conservative starting refusal threshold for two worker builds plus source copies and evidence, not a required minimum or a proven maximum. The pre-existing native debug tree on the audited host occupied 1.3 GiB; the runner strips debug information and disables incremental state rather than copying that tree. This measurement does not establish the size of a fresh build on another target or with changed dependencies. Choose less with `--disk-limit-mib` when recorded fresh-run peaks support it, or use one worker with `--jobs 1` to reduce duplicate build products. Resource reports provide the actual evidence needed to set a target-specific budget; shrinking a budget must produce a visible refusal rather than silently omit work. Filesystem/toolchain caches have their own owner-defined limits.

Use `--file`, `--regex` or `--in-diff` for focused iteration. A tiny selected run verifies the runner and a particular negative control; it does not replace a full release review. The mandatory unmutated baseline, complete workspace test selection, locked dependencies and original mutant expectations remain enabled. The overall wall budget prevents a campaign monopolizing the machine; an interrupted or budget-refused run is incomplete. Raising the budget for a separately authorized full review must be explicit. Inspect every survivor and timeout before claiming the review complete; `audit.json` deliberately does not infer review completion from a process exit code.

There is no scheduled mutation campaign or mutation build-cache artifact in current GitHub CI. The release review uses the same local runner; adding schedules, sharding administration or a new required PR campaign is unnecessary for resource remediation. Current GitHub fuzz CI invokes the same bounded smoke script as local runs and retains evidence separately from rebuildable build products.
