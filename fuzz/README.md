# Fuzzing

`contract_bytes` feeds arbitrary UTF-8 contract text to the closed compiler. `extract`
feeds a length-prefixed contract and document bytes to document parsing, extraction and
outline. It also parses with node admission budgets from zero through 32, independently
of contract validity, and limits transformed readings to exercise ordinary refusals.
These operator budgets exercise maintained public APIs; they are cooperative admission
limits, not a universal physical memory cap.

Pull-request CI uses 250 requested executions per target as a short preflight: replay
all starting seeds, enter the random mutation loop, and complete both AddressSanitizer
harnesses. It explicitly requires the final callback count to exceed the independently
recorded `INITED` count. Corpus reloading is disabled because each target owns its corpus,
so replay alone cannot satisfy that mutation check. If seed initialization consumes the
entire budget, the preflight refuses; increase the explicitly selected count after review,
without discarding seeds. This preflight provides less mutation coverage than the
separately authorized 10,000-execution maintainer/release smoke, which remains the local
default. A short preflight does not establish completion of that longer check.

Both modes generate starting inputs from conformance and `fuzz/seeds/documents/`,
build the instrumented harnesses once, then run them concurrently. Sixteen codegen
units permit parallel code generation; AddressSanitizer, compare tracing and debug
assertions remain enabled. Both targets must finish the selected execution count;
timeout or resource refusal fails the check. libFuzzer initializes every seed before
mutation, so a short requested count below the seed count can complete more executions
than requested, without entering the mutation loop.

```sh
# Separately authorized maintainer/release smoke:
./scripts/focused-smoke.sh
# The short CI preflight, including real mutation startup:
./scripts/focused-smoke.sh --runs 250 --require-mutations
# Seed/harness plumbing only; may perform no mutations:
./scripts/focused-smoke.sh --runs 100
```

Python 3.11 or newer must be on PATH. Install the tool versions and nightly declared in
[contributor-rust-tools.sh](../scripts/contributor-rust-tools.sh), including its rust-src
component. To keep evidence, supply a directory outside the checkout:

```sh
fuzz_output=$(mktemp -d "${TMPDIR:-/tmp}/htmlcut-exploration.XXXXXX")
./scripts/focused-smoke.sh --runs 100 "$fuzz_output"
```

Every invocation creates its own evidence subdirectory when an output directory is
supplied; previous contents are never overwritten. Without an explicit output directory,
success removes generated evidence and failure retains it and prints its path. The
instrumented build directory is task-owned, isolated from owner caches, and removed
on both success and failure; build logs and resource reports are copied to evidence.
A fresh invocation recompiles its instrumented dependencies; this trades hot-cache
latency for no cumulative build-cache growth. The measured cold baseline build took
106 seconds, so parallel execution is not a claim of universal overall acceleration.

See [QA resource limits](../docs/qa-resources.md) for the shared watchdog and mutation
runner. The fuzz runner allows 1 GiB for the build and 64 MiB per target for evidence,
with a 4 MiB
log ceiling per command, a 600-second build deadline and a 180-second target deadline.
The sampled disk watchdog refuses excess; it is **not a filesystem quota**, and a writer
can exceed the budget between samples. These are operational allowances, not a claim
that Rust toolchains or arbitrary filesystem metadata have an exact portable maximum.
An actual filesystem quota is needed for a hard aggregate disk guarantee. Preinstalled
Rust/tool caches are outside these allowances and are never cleaned by the runner.

Each target can save at most one 4096-byte mutation per execution: 10,000 executions
therefore account for at most 39.1 MiB of generated input payload per target, plus its
initial seeds, one failure artifact, logs and filesystem allocation/metadata. The seed
size check refuses inputs above 4096 bytes rather than silently trimming coverage.
A 64 MiB evidence allowance leaves room for those seeds and allocation overhead while
remaining below twice the worst-case generated payload. The 1 GiB build allowance
allows roughly twice the measured 532 MiB peak of a fresh instrumented baseline build
for compiler/linker intermediates and host differences. Reports record the actual
candidate peak so this allowance can be challenged on each supported host. Failure to fit is a visible unavailable smoke, not a
reason to silently grow the budget. No cumulative build or mutable corpus cache grows
across successful local invocations. CI retains bounded evidence for three days.

The 10,000-run maintainer/release smoke is reproducible regression exploration, not
exhaustive fuzz assurance. PR preflight reduces latency and mutation opportunities;
it does not claim equivalent coverage. Parallel execution removes serial target latency
while preserving the selected execution count. Longer/random campaigns need an
explicit finite run bound and the same disk/time/log controls; an unbounded `cargo fuzz run` is not a maintained
QA command. Inspect and reproduce crashes with the pinned toolchain, minimize them,
and add regression tests and retained seeds for corrected defects. Keep useful failure
evidence and remove only exploratory output you own when finished.
