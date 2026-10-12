# Platform support

The maintained native targets and runners are derived from [release-targets.sh](../scripts/release-targets.sh): Apple ARM64 and Intel, Linux x64 musl, and Windows x64 MSVC. CI runs the library/CLI tests and the same conformance corpus against each unpacked native package.

Apple executables target macOS 12 and are signed ad hoc. Strict signature verification establishes sealed-code integrity; it supplies no Developer ID or notarization. Current native runners do not establish execution on every older OS version.

Build from a clean checkout with rustup and a native linker: Xcode Command Line Tools on macOS, a C compiler on Linux, or Visual Studio C++ tools on Windows. The development toolchain is pinned in [rust-toolchain.toml](../rust-toolchain.toml); the tested minimum is declared separately in [Cargo.toml](../Cargo.toml).

Packages include the executable, README, license, target dependency attribution and patent notice. Source archives, checksums and GitHub provenance are distinct checks. See [release process](release.md).
