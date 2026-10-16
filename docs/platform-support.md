# Platform support

The maintained native targets and runners are derived from
[release-targets.toml](../tools/release-targets.toml): Apple ARM64 and Intel, Linux x64
musl, and Windows x64 MSVC. CI runs core/runtime/CLI tests on those targets. Package
acceptance runs the CLI conformance, lifetime, authorization and performance controls
against each unpacked executable.

Apple executables target macOS 12 and are signed ad hoc. Strict signature verification establishes sealed-code integrity; it supplies no Developer ID or notarization. Current native runners do not establish execution on every older OS version.

Build from a clean checkout with rustup and a native linker: Xcode Command Line Tools
on macOS, a C compiler on Linux, or Visual Studio C++ tools on Windows. The development
toolchain is pinned in [rust-toolchain.toml](../rust-toolchain.toml); the supported
minimum Rust version is declared in [Cargo.toml](../Cargo.toml).

Bounded contributor QA requires Python 3.13 or newer on macOS for `waitid`/`WNOWAIT`;
Linux and Windows require Python 3.11 or newer. POSIX runs refuse nondefault `SIGCHLD`
handlers so the owned child cannot be reaped before process-group cleanup. These
prerequisites do not apply to running a packaged CLI.

Native QA also requires platform authorization, pipe, signature and allocation-checking
capabilities, including `/usr/bin/leaks` on macOS. See
[contributor prerequisites](../CONTRIBUTING.md) for the required capabilities on each
host. Missing required capabilities fail the relevant controls; these test prerequisites
do not apply to running a packaged CLI.

Packages include the executable, README, license, target dependency attribution and patent notice. Source archives, checksums and GitHub provenance are distinct checks. See [release process](release.md).
