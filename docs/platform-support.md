---
afad: "3.5"
version: "4.2.0"
domain: PLATFORM
updated: "2026-04-20"
route:
  keywords: [platform support, release targets, standalone binaries, deployment floors, target matrix]
  questions: ["which standalone targets does HTMLCut release?", "what platforms are maintained for HTMLCut?", "where is the release target policy defined?"]
---

# Platform Support

This document defines HTMLCut's maintained build and release target policy.

## Local Development

Local maintainer builds are host-native.

On the current maintained maintainer machine shape, that means:

- `aarch64-apple-darwin`

Local development is not expected to produce every public release artifact. Cross-platform public
artifact production belongs to GitHub release automation.

## Public Standalone Release Targets

HTMLCut publishes standalone binaries and SHA-256 checksum files for:

- `aarch64-apple-darwin`
- `x86_64-apple-darwin`
- `x86_64-unknown-linux-musl`
- `x86_64-pc-windows-msvc`

Release source archives are also published as:

- `htmlcut-X.Y.Z.zip`
- `htmlcut-X.Y.Z.tar.gz`

## Deployment Floors

Apple targets are pinned to:

- macOS 12.0 Monterey

That floor applies to:

- `aarch64-apple-darwin`
- `x86_64-apple-darwin`

Windows standalone artifacts target:

- Windows x64 through the `x86_64-pc-windows-msvc` toolchain

Linux standalone artifacts target:

- Linux x64 through `x86_64-unknown-linux-musl`

## Why These Targets

- `aarch64-apple-darwin` is the primary maintainer and operator path.
- `x86_64-apple-darwin` keeps Intel macOS users first-class instead of silently dropping them.
- `x86_64-unknown-linux-musl` produces a more portable standalone Linux binary than a glibc-bound release.
- `x86_64-pc-windows-msvc` is the mainstream Windows release target and avoids 32-bit or GNU-toolchain drift.

## Workflow Boundary

GitHub release builds currently run on:

- `macos-15` for `aarch64-apple-darwin`
- `macos-15-intel` for `x86_64-apple-darwin`
- `ubuntu-24.04` for `x86_64-unknown-linux-musl`
- `windows-2022` for `x86_64-pc-windows-msvc`

GitHub CI also runs release-target smoke on that same target matrix before the aggregate required
check reports success.

## Source Of Truth

The target policy is implemented in:

- `scripts/release-targets.sh`
- `.github/workflows/ci.yml`
- `.github/workflows/release.yml`
- `scripts/build-release-artifact.sh`
- `scripts/publish-github-release.sh`
- `scripts/verify-github-release.sh`

Do not change the public target matrix in only one of those places. The release system must stay
derived from the same target policy.
