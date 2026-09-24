#!/usr/bin/env bash
# Install the pinned official nextest release archive after verifying its SHA-256 digest.

set -euo pipefail

nextest_script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/common.sh
. "${nextest_script_dir}/common.sh"

version="${1:-}"
[[ "${version}" == "0.9.146" && $# == 1 ]] || htmlcut_die \
    "unsupported nextest version; update the pinned platform digests with the tool inventory"

case "$(uname -s):$(uname -m)" in
    Darwin:*)
        platform="universal-apple-darwin"
        expected_sha256="39785160b3c2f6ed9a765049cf4fa79f3b39aa02eb7598a5a0e2a1a0b9ffb9a8"
        binary_name="cargo-nextest"
        ;;
    Linux:x86_64)
        platform="x86_64-unknown-linux-gnu"
        expected_sha256="682c21b777c333e96fd532e114d3a5a894e0729ab88d94c0a9f20f8419695428"
        binary_name="cargo-nextest"
        ;;
    Linux:aarch64)
        platform="aarch64-unknown-linux-gnu"
        expected_sha256="b2e33d7c72de7ade0ff7b3a948ac37516b24f8a836b7a8870c1f634a94be9de9"
        binary_name="cargo-nextest"
        ;;
    MINGW*:*|MSYS*:*|CYGWIN*:*)
        case "$(uname -m)" in
            x86_64)
                platform="x86_64-pc-windows-msvc"
                expected_sha256="809b01f2c94d031ab6cf5f4d4a4f477c8f51b69c96ab3cb2fe4db7ecb222d0d3"
                ;;
            aarch64)
                platform="aarch64-pc-windows-msvc"
                expected_sha256="c94029615d05072af35429d9d66e0f83ba59886bb4e13163a191134e5f0ad47c"
                ;;
            *) htmlcut_die "unsupported Windows architecture for pinned nextest" ;;
        esac
        binary_name="cargo-nextest.exe"
        ;;
    *) htmlcut_die "unsupported platform for pinned nextest" ;;
esac

archive_name="cargo-nextest-${version}-${platform}.tar.gz"
archive_url="https://github.com/nextest-rs/nextest/releases/download/cargo-nextest-${version}/${archive_name}"
scratch="$(mktemp -d "${TMPDIR:-/tmp}/htmlcut-nextest.XXXXXX")"
trap 'rm -rf -- "${scratch}"' EXIT
archive_path="${scratch}/${archive_name}"
curl --fail --location --silent --show-error --retry 3 --max-time 120 \
    --output "${archive_path}" "${archive_url}"

if command -v shasum >/dev/null 2>&1; then
    actual_sha256="$(shasum -a 256 "${archive_path}" | awk '{print $1}')"
else
    actual_sha256="$(sha256sum "${archive_path}" | awk '{print $1}')"
fi
[[ "${actual_sha256}" == "${expected_sha256}" ]] || htmlcut_die \
    "pinned nextest archive SHA-256 mismatch"
[[ "$(tar -tzf "${archive_path}")" == "${binary_name}" ]] || htmlcut_die \
    "pinned nextest archive has an unexpected file inventory"
tar -xzf "${archive_path}" -C "${scratch}" "${binary_name}"
install -d "${CARGO_HOME:?CARGO_HOME must be set}/bin"
install -m 755 "${scratch}/${binary_name}" "${CARGO_HOME}/bin/${binary_name}"
"${CARGO_HOME}/bin/${binary_name}" --version | grep -F "${version}" >/dev/null || htmlcut_die \
    "installed nextest binary does not report the pinned version"
printf 'contributor cargo tool: installed verified %s %s archive\n' "${binary_name}" "${version}"
