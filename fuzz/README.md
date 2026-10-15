# Fuzzing

`contract_bytes` feeds arbitrary UTF-8 contract text to the closed compiler. `extract`
feeds a length-prefixed contract and document bytes to document parsing, extraction and
outline. It also parses with node admission budgets from zero through 32, independently
of contract validity, and limits transformed readings to exercise ordinary refusals.
These cooperative operator limits do not impose a physical memory cap.

Run from the repository root with Bash, Python 3.11 or newer (3.13 or newer on macOS),
`cargo-fuzz` and the pinned
nightly with `rust-src`. Install the versions declared in
[contributor-rust-tools.sh](../scripts/contributor-rust-tools.sh).

```sh
# Maintainer/release smoke: 10,000 requested executions per target.
./scripts/focused-smoke.sh
# CI preflight: 250 requested executions, with mutation startup required.
./scripts/focused-smoke.sh --runs 250 --require-mutations
# Seed/harness plumbing: may perform no mutations.
./scripts/focused-smoke.sh --runs 100
```

The runner accepts 1 through 10,000 requested executions per target. It generates
starting inputs from conformance and `fuzz/seeds/documents/`, builds both instrumented
harnesses once, and runs them concurrently. AddressSanitizer, compare tracing and debug
assertions remain enabled. Inputs, including seeds, are limited to 4096 bytes; oversized
seeds are refused. Both targets must complete the selected count. Timeout or resource
refusal fails the check. A failed target cancels its sibling rather than waiting for
that target's deadline; both logs and resource reports remain available.

Seed source sizes and the complete framed extraction input are checked before the
instrumented build, with bounded reads even if a source grows. Named seed inputs must be ordinary regular
files; FIFOs and devices refuse the whole run before the instrumented build. Each initial corpus is
limited to 10,000 files, matching the runner's maximum requested execution count;
exceeding this bound refuses the whole run rather than dropping seeds. This also bounds
seed preparation, which precedes the watchdog, to at most 39.1 MiB of logical input per
target. The retained `seeds.json` records every starting input and its size. Duplicate
or empty seeds can change libFuzzer's initialization count, so this inventory does not
replace the actual callback-progress check below.

libFuzzer initializes every seed before mutation. A requested count below the seed count
can therefore complete more executions than requested without entering the mutation
loop. CI requires the final callback count to exceed the recorded `INITED` count. Corpus
reloading is disabled, so seed replay alone cannot satisfy that check. If initialization
consumes the selected budget, the preflight refuses; increase the count explicitly
without discarding seeds. The CI preflight does not replace the 10,000-execution smoke.

To retain results, supply an output directory outside the checkout:

```sh
fuzz_output=$(mktemp -d "${TMPDIR:-/tmp}/htmlcut-fuzz-results.XXXXXX")
./scripts/focused-smoke.sh --runs 100 "$fuzz_output"
```

Each invocation creates a new subdirectory under the supplied output directory and
preserves existing contents. Without an explicit output directory, success removes the
generated results; failure retains them and prints their path. The isolated instrumented
build directory is removed on success and failure when its watchdog confirms process
cleanup, after copying its logs and resource reports to the results directory. A missing,
malformed report, unconfirmed child exit or nonempty `cleanup_errors` after a process starts leaves that build
or result directory intact and prints its path; possibly active writers must be stopped
before removing it. Cleanup uncertainty fails the run and never becomes a passing smoke.
A single Python controller owns each watchdog process object. On POSIX it cancels
only its unreaped child leaders without signaling saved numeric PIDs. On Windows it
keeps the process handle while invoking `taskkill /T /F` for a live watchdog tree;
terminating only that leader would bypass its child cleanup. Forced Windows tree
stops remain explicitly uncertain and retain their roots, since the killed watchdog
cannot confirm its own resource report. Descendants orphaned by an already exited
Windows watchdog are not guaranteed to be stopped. The entry point refuses
ignored or custom `SIGCHLD` dispositions before creating output. Controller teardown
allows 15 seconds for watchdog cleanup, then at most two seconds for forced leader
termination; Windows also allows up to five seconds per live watchdog for `taskkill`.
Unconfirmed roots remain intact.

Preinstalled toolchains and Cargo download caches are
outside this cleanup. CI retains results for three days.

The runner allows 1 GiB for the build, 64 MiB per target for results and a 4 MiB log per
command. The build deadline is 600 seconds and each target's deadline is 180 seconds.
These are sampled watchdog thresholds, not filesystem quotas; writes can exceed a disk
threshold between samples. See [QA resource limits](../docs/qa-resources.md) for accounting
scope and process cleanup limits. A fresh invocation rebuilds the instrumented harnesses;
build and mutable corpus caches do not accumulate across successful local invocations.

The build allowance covers one shared pair of instrumented harnesses and their
dependencies. A measured clean macOS ARM build used 532.2 MiB, so 1 GiB provides roughly
twice that observed space for platform/compiler variation; a smaller allowance near
the observation can refuse a valid cold build. This is an allowance, not a claim that
every supported toolchain requires exactly 1 GiB. Ten thousand 4096-byte inputs occupy
39.1 MiB before filesystem overhead, but each target's retained
starting corpus, newly interesting corpus entries, allocation overhead and capped log
also consume space. The 64 MiB threshold allows this small regression exploration
without reserving another build-sized directory. A worst-case full initial corpus plus
new corpus entries can exceed it; the result is a visible resource refusal, never
discarded input or a passing incomplete campaign. Budget about 1.13 GiB for one build
plus both result thresholds, with extra filesystem headroom for sampling overshoot.
Tool installation/download caches and retained evidence from previous invocations
remain separate; the runner neither deletes nor promises a bound on those owner resources.

Finite fuzz runs provide regression exploration rather than exhaustive assurance.
Use an explicit execution bound and the same resource controls for longer exploration.
An unbounded `cargo fuzz run` is not a maintained QA command. Reproduce crashes with the
pinned toolchain, minimize them, and add regression tests and retained seeds for corrected
defects. Preserve useful failure results and remove only exploratory output you own.
