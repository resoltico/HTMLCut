# Contributing

Install rustup and the native linker for your OS. Cargo reads `rust-toolchain.toml`.
Python 3.11+ is needed for release checks. Install the tools listed by
`source scripts/contributor-rust-tools.sh; htmlcut_contributor_cargo_tool_inventory`
with `cargo install NAME --version VERSION --locked`. Add `--features cli` when installing `cargo-about`; its binary is feature-gated.

```sh
./check.sh
```

The script runs direct formatting, clippy, semantic tests, explicitly selected large
resource acceptance, rustdoc, focused release tests and one advisory/license check.
The required Linux CI path also runs `./scripts/focused-smoke.sh`, the same bounded
parser and relational smoke available locally. Install cargo-fuzz from the inventory
and the pinned nightly toolchain first; see [the harness instructions](fuzz/README.md).
No coverage score, full mutation campaign or full GA fuzz campaign is required. For optional full evaluation scripts install `evaluation/requirements.txt` in an isolated environment.
Missing tools produce their normal command errors; install the named pinned tool.

Nightly is needed only for libFuzzer instrumentation. Ordinary tests, clippy,
rustdoc and builds use the pinned stable compiler; there is no coverage-score build.
The removed dependency forks no longer require their Miri verification lanes.
Review the narrow native worker OS wrappers separately and run their real resource
controls on each native target; Miri cannot prove kernel resource enforcement.

Check the public core against the published baseline:

```sh
source scripts/contributor-rust-tools.sh
cargo semver-checks check-release -p htmlcut-core --baseline-rev "$HTMLCUT_PUBLISHED_BASELINE_SOURCE"
```

Cargo materializes the immutable source outside the checkout; do not commit baseline
copies. Let package versions determine allowed change level; never force major to
hide a patch deletion. Native CI extracts packages and executes file/pipe/error, worker containment and
Unicode controls on all four supported targets. Keep the required `Check` status.
See [release protocol](docs/release-protocol.md) for external rollout.
