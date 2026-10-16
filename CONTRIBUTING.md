# Contributing

Install rustup and the native linker prerequisites in [platform support](docs/platform-support.md). From a clean checkout, `cargo test --locked --workspace --all-features` builds and tests the product. Ordinary Cargo tests do not need Python. The compiler pin is [rust-toolchain.toml](rust-toolchain.toml), the minimum Rust version is [Cargo.toml](Cargo.toml), and dependency resolution is [Cargo.lock](Cargo.lock).

Maintenance uses Python 3.13+ on macOS and 3.11+ on Linux/Windows; invoke a suitable `python3`. CI's exact interpreter is declared in [.python-version](.python-version). The package is stdlib-only. Install the required dependency-policy tool into a fresh directory outside the checkout, add its `bin` directory to PATH, and run:

```sh
python3 -m tools.htmlcut_maintenance install cargo-deny --root /path/outside/checkout/cargo-deny
python3 -m tools.htmlcut_maintenance check
```

The check command runs standard Cargo fmt, warning-denied Clippy, locked workspace tests, warning-denied rustdoc, nonempty maintenance unittest discovery, and cargo-deny. It checks prerequisites before building and does not install optional tools. Its metadata guard enforces the CLI's production dependency allowlist and product dependency direction, with a prohibited-dependency negative control. That guard does not prove the absence of copied business logic; corpus parity and code review cover that risk.

macOS native library tests require `/usr/bin/leaks`, with an intentionally retained allocation as a detection control. Linux library authorization tests require mounted `/proc`, POSIX ACL-capable temporary storage and a regular account without DAC bypass. Delivered-binary native acceptance additionally requires `setfacl` and `sudo -n -u nobody` on Linux, native PowerShell and the required authorization capabilities on Windows, and codesign/ACL tools on macOS. Missing required native capabilities fail acceptance.

Windows native QA requires an account with assigned `SeSecurityPrivilege`, token identities permitting owner/group assignments distinct from file defaults, and native file/directory symbolic-link creation. See [Windows symlink requirements](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-createsymboliclinkw). Missing capabilities fail the relevant controls; these are QA prerequisites, not a promise that every caller needs that authority.

The literal conformance corpus is specification input. Review semantic fixture changes with their intended behavior; there is no byte-freezing gate or append-only policy. Accepted documentation examples must have executable dialect cases. Product integration tests accept `HTMLCUT_TEST_BINARY` so the same tests can exercise an unpacked executable, including explicitly ignored native/performance controls.

[tools/contributor-tools.toml](tools/contributor-tools.toml) pins optional QA tools and nightly. Install `cargo-about`, `cargo-fuzz`, `cargo-mutants` and, for optimized mutation testing, `cargo-nextest` with the same bounded install command as needed. Attribution enables cargo-about's `cli` feature. Keep each installed tool's `bin` directory on PATH; local tools and Cargo downloads persist between QA commands. The cold installer requires a fresh root and never replaces an existing owner installation.

CI reuses public crates.io archives/index and exact-input tool binaries through pinned GitHub cache actions. Every restored executable undergoes the same bounded version probe as a cold install. Only trusted main pushes and current-main mutation dispatches may save; PRs, tags and historical selections restore only. Project targets, credentials, private sources, release artifacts and QA evidence are excluded. A cache miss runs the ordinary bounded installer; owner Cargo/rustup caches are never pruned.

```sh
python3 -m tools.htmlcut_maintenance qa fuzz --runs 250 --require-mutations /path/outside/checkout/fuzz-results
cargo fetch --locked
python3 -m tools.htmlcut_maintenance qa mutation --output /path/outside/checkout/mutation-results
```

Fuzzing requires the pinned nightly with `rust-src`; its default full smoke is 10,000 runs per target. Mutation builds are locked/offline after preparation. Both retain bounded evidence and reject incomplete outcomes; source identity must stay stable. Full workspace mutation survivor/timeout review remains a pre-publication operator requirement. Focused platform campaigns do not establish full review. See [QA resource limits](docs/qa-resources.md), [fuzzing](fuzz/README.md) and [release operations](docs/release.md).
