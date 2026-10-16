# QA resource boundaries

The stdlib [maintenance process owner](../tools/htmlcut_maintenance/process.py) supplies finite elapsed-time, combined-log and sampled owned-disk bounds for tool installation, fuzz and mutation commands. Callers invoke its API directly and receive a concrete result after cleanup; JSON is retained as external evidence. Python children use the invoking `sys.executable`.

The owner preserves unreaped POSIX process identity through group termination; unavailable waitid/WNOWAIT or custom SIGCHLD handling refuses before launch. macOS requires Python 3.13+; Linux/Windows require 3.11+. It records actual raw child status, refusal, accounting/reader errors and cleanup uncertainty. Wide nonzero Windows statuses never become successful wrapper exits. An exited/reaped identity is never signalled by a saved PID. Cooperative separate-group cleanup uses readiness controls and a post-cleanup descendant release gate; deliberately omitted cleanup must be detected.

Disk sampling is not a quota and may overshoot between samples. Owner Cargo registry/git downloads, rustup toolchains and existing caches are outside owned temporary product accounting and are never automatically deleted. Hard kills can prevent cleanup. Unconfirmed child termination retains affected owned roots and fails; confirmed stopped build products can be removed while logs/reports remain available.

[Tool pins](../tools/contributor-tools.toml) and [compiler pin](../rust-toolchain.toml) are separate authorities. `install TOOL --root NEW_PATH` installs one locked pinned tool into a fresh root, enabling cargo-about's required `cli` feature. Cold compilation has a 4 GiB sampled allowance, 900 seconds and a 4 MiB log cap; exact version probes have ten seconds and a 1 MiB log cap. These are conservative refusal allowances, not measured maxima. Failures preserve evidence; custom binary cache generations, trust/restore/save policy and shell path bridges are removed. Native CI job budgets include cold setup plus tests.

```sh
python3 -m tools.htmlcut_maintenance qa fuzz --runs 250 --require-mutations /outside/checkout/fuzz-results
cargo fetch --locked
python3 -m tools.htmlcut_maintenance qa mutation --output /outside/checkout/mutation-results
```

Fuzz builds and both manifest targets run through the same owner, sequentially. Seed copies are exact and bounded to 64 KiB per framed input, including the shared stress cases. Oversized, nonregular, empty/incomplete seed discovery and incomplete iteration/mutation progress refuse. The default full smoke is 10,000 runs per target and requires post-initialization mutation; shorter runs are explicitly incomplete full-smoke evidence.

Mutation uses locked offline prepared builds and owned outputs, records exact source identity before/after, rejects empty/duplicate/incomplete plans or outcomes and keeps survivors/timeouts visible. Default campaign allowance is 4 GiB sampled owned output and 1,800 seconds; version probing is separate. Individual test phases allow 180 seconds and builds 300 seconds so the unmodified workspace, including native lifecycle proofs, can finish before mutant outcomes are assessed. `--jobs`, `--file`, `--exclude-file`, `--regex` and `--in-diff` support focused iteration. Exclusions and staged diff identity remain in evidence. Product scopes include core, runtime and CLI; platform selections cover active runtime authorization modules. Full-review status is never inferred from mere campaign completion or focused native success.

Run maintenance tests with:

```sh
python3 -m unittest discover -s tools/htmlcut_maintenance/tests -t . -v
```

The `check` command refuses empty discovery. Acquisition integration receives an explicit built/delivered binary through `HTMLCUT_TEST_BINARY`; direct unittest discovery reports a skip when none was supplied. Native Windows/macOS controls require their actual hosts; local mocks and version queries do not establish these real integration gates.
