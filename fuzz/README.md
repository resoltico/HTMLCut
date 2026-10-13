# Fuzzing

`contract_bytes` feeds arbitrary UTF-8 contract text to the closed compiler. `extract`
feeds a length-prefixed contract and document bytes to document parsing, extraction and
outline. Bounded operators exercise the maintained public library APIs.

The pull-request smoke is a fixed-seed regression run of 10,000 executions per target.
It generates starting inputs from conformance and `fuzz/seeds/documents/`, and discards
its default generated corpus after success. A failure retains evidence and prints its
path. An explicitly supplied output directory is always kept.

```sh
./scripts/focused-smoke.sh
```

Install the tool versions and nightly declared in
[contributor-rust-tools.sh](../scripts/contributor-rust-tools.sh). For an exploratory,
time-boxed run with a random seed and an output directory outside the checkout:

```sh
source scripts/contributor-rust-tools.sh
fuzz_output=$(mktemp -d "${TMPDIR:-/tmp}/htmlcut-exploration.XXXXXX")
mkdir -p "$fuzz_output/corpus" "$fuzz_output/crashes"
cargo +"$HTMLCUT_CONTRIBUTOR_RUST_NIGHTLY_TOOLCHAIN" fuzz run extract "$fuzz_output/corpus" -- -max_total_time=200 -seed="$RANDOM" -max_len=4096 -timeout=10 -rss_limit_mb=2048 -artifact_prefix="$fuzz_output/crashes/"
```

A smoke result does not establish exhaustive fuzz assurance. Inspect and reproduce any
crash with the pinned toolchain, minimize it, and add a regression test and a seed when
the defect is corrected. Keep useful failure evidence; remove exploratory output when
finished. A known upstream parser panic is recorded in the changelog; its seed is added
when the maintained dependency upgrade fixes it.
