# QA resource boundaries

The stdlib [maintenance process owner](../tools/htmlcut_maintenance/process.py) owns elapsed-time, combined-log and sampled disk limits for installation, fuzzing and mutation. Callers receive a result after cleanup; JSON retains the actual command, status, refusal and resource evidence. Python children use the invoking `sys.executable`.

The owner preserves unreaped POSIX identity through group termination. Missing waitid/WNOWAIT or custom SIGCHLD handling refuses before launch. macOS requires Python 3.13+; Linux/Windows require 3.11+. It preserves raw Windows child statuses and cleanup/accounting/reader failures. It never signals a saved, reaped PID. Separate-group descendants require cooperative cleanup or external OS containment; the owner is not a process sandbox. Unconfirmed cleanup retains affected owned roots and fails.

Disk accounting uses non-following entry metadata and does not descend symlinks or Windows reparse directories. It counts both allocated and logical size conservatively. `--poll-seconds` controls disk-sample cadence, capped at one second; mutation uses that one-second default rather than rescanning every 200 ms. Short-lived peaks may be missed between samples. Cheap cancellation/exit/log/deadline observations have at most a 50 ms idle wait, plus synchronous scan/syscall latency. Initial and final samples remain mandatory; `disk_samples` and `disk_scan_seconds` expose count and scan wall time. This is sampled enforcement, not a quota or a hard bound over blocked metadata calls. Owner Cargo/rustup caches remain outside transient products and are never automatically pruned.

## Tool and dependency reuse

[Tool pins](../tools/contributor-tools.toml) and [compiler pin](../rust-toolchain.toml) are separate authorities. Install optional tools once into a caller-selected fresh root and keep its `bin` on PATH. `install TOOL --root NEW_PATH` uses locked inputs, a 4 GiB sampled compilation allowance, 900 seconds and a 4 MiB log. Bounded version probes allow ten seconds, 4 MiB sampled products and 1 MiB logs. These are refusal allowances, not measured maxima. Cold and restored tools share the actual version/protocol verifier, including Cargo subcommand and Nextest multiline formats. Confirmed stopped compiler products are removed; failures retain bounded evidence and uncertain products.

GitHub jobs use pinned standard cache actions for public crates.io compressed archives/index and exact-input installed tool binaries. Download keys can fall back within the same public OS namespace; missing locked inputs are still fetched. Executable keys include hosted OS/architecture/image and tool/compiler/installation inputs, with no fallback. Product source or Cargo.lock changes do not invalidate unrelated tool binaries. Every exact hit executes the bounded pin probe; a corrupt/wrong/hung hit fails. A missing/unavailable cache takes the cold path.

Only main pushes and current-main manual mutation selections may save; PRs, tags, historical selections and published-native jobs restore only. Save permission defaults false and stays visible at workflow call sites. Credentials/config, unpacked registry sources, arbitrary private/Git registries, project targets, mutated workspaces, test results, release bytes, signatures and evidence are excluded. Cache identity/version checks do not prove the trustworthiness of an arbitrary executable; trusted writers and exact inputs remain the boundary. Local owner caches are not replaced by a new cache framework.

## Mutation execution

Prepare locked dependencies and install the pinned cargo-mutants and cargo-nextest tools before running:

```sh
cargo fetch --locked
python3 -m tools.htmlcut_maintenance qa mutation --output /outside/checkout/mutation-results
```

The default runner is Nextest with a tracked mutation profile: full enabled workspace binary tests, all features, no retries, fail-fast scheduling and core/runtime tests before slower CLI tests. Already-running tests finish normally after a failure. Per-worker `--test-threads` defaults to available CPUs divided by `--jobs`; compiler tasks share one CPU-count jobserver. Recursive test children use one libtest thread. These settings bound scheduled slots, not every OS thread or test-owned descendant.

Baseline and mutant executions use the same workspace detector scope even for a focused mutation selection. A failing unmodified downstream package stops execution before any mutant is tested. Test discovery is retained rather than maintained as a second test inventory; pinned runner summary quirks do not override actual discovered and completed test identities.

Nextest does not run doctests. The wrapper runs an actual same-source locked/offline workspace doctest baseline in an owned source/build root, within the campaign budget. A regular public-example test preserves the current semantic assertion per mutant. Reports explicitly say doctests are baseline-only. Use `--test-tool cargo` for ordinary workspace Cargo tests including doctests on every mutant. Both runners retain the existing ignored/native opt-in scope; delivered-package acceptance runs those controls separately.

Nextest failure status alone is insufficient detection evidence. The wrapper validates pinned structured test events and forced status output. No-tests, execution/setup failures, runner timeouts, leaks, malformed or incomplete evidence are never silently counted as caught. Ambiguous cleanup leaves an incomplete nonzero audit and retained scratch. The raw tool verdict remains available. Fail-fast means stopping additional tests after a genuine detection, not stopping the whole campaign after a survivor or hiding failures with retries.

Inherited `HTMLCUT_*` delivery/test-helper controls, including empty values, and shared Cargo build overrides are refused before launch. Tests cannot accidentally exercise an unmutated delivered binary or skip parent assertions through a child marker. Each worker owns its own reusable build tree. No shared mutant target, unsafe name-only resume or cross-source result cache is used.

The default campaign allowance remains 4 GiB sampled owned output and 1,800 seconds; tool-version probing is separate. Test phases allow 180 seconds and builds 300 seconds. `audit.json` retains runner/pin/profile, actual scope/concurrency, source identity, baseline/outcome completeness and build/test timing summaries. Execution completion never means survivor/timeout review is complete. Nextest can create separate test process groups; tests must clean their owned children, and physical containment remains an operator responsibility.

For a short developer cycle, select the relevant mutation plan explicitly:

```sh
python3 -m tools.htmlcut_maintenance qa mutation --in-diff /outside/checkout/change.diff --jobs 2 --test-threads 2 --output /outside/checkout/focused-mutations
```

`--file`, `--exclude-file` and `--regex` are also available. A focused run only covers its selected plan. A short wall budget that leaves mutants untested is incomplete. Full workspace review remains an explicit release prerequisite, including every survivor and timeout; focused platform success is not a substitute. Source/policy changes create new evidence and do not retroactively amend a running campaign.

## Fuzz and bounded properties

```sh
python3 -m tools.htmlcut_maintenance qa fuzz --runs 250 --require-mutations /outside/checkout/fuzz-results
```

Both manifest targets run through the same owner, sequentially. Framed input is limited to 64 KiB. Exact generated duplicates share one corpus entry with all source aliases retained; authored inputs remain unchanged. Reports distinguish initialization, completed executions and post-initialization work. The 250-run CI job is a short sanitizer/progress preflight. The default 10,000-run smoke is a separate broader check; neither is a full mutation review.

Extraction fuzzing selects either one small-budget parser-admission pass or one semantic parse with extraction/outline/export invariants. It uses finite structural bounds and libFuzzer/owner limits rather than a load-sensitive cooperative deadline that silently skips later operations. Source/lock changes refuse stable-source completion. Small deterministic core properties independently check composed Unicode/escaped values and exact/one-below export bounds without a new testing framework. See [fuzzing](../fuzz/README.md) for framing and replay requirements.

Run maintenance tests with:

```sh
python3 -m unittest discover -s tools/htmlcut_maintenance/tests -t . -v
```

The standard `check` command rejects empty discovery. Acquisition integration needs an explicit built/delivered `HTMLCUT_TEST_BINARY`; ordinary discovery reports its absence as a skip. That delivery override is appropriate for package testing and intentionally forbidden for mutation. Native Windows/macOS/Linux controls and GitHub cache/aggregate execution require their real environments; local fixtures, Actionlint and version queries do not establish those external gates.
