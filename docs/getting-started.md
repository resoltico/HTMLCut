---
afad: "4.0"
version: "18.0.0"
domain: SETUP
updated: "2026-10-04"
route:
  keywords: [getting started, quick start, install, release package, cargo install, first extraction, snapshot bundle]
  questions: ["how do I install HTMLCut?", "how do I try HTMLCut on a sample page?", "how do I save a reusable extraction run?"]
  related: [../README.md, cli.md, platform-support.md, core.md, schema.md]
---

# Getting started

Use a completed UTF-8 HTML snapshot and an explicit selector. Choose an exact published version from [GitHub Releases](https://github.com/resoltico/HTMLCut/releases). The commands below select 18.0.0. Run them only after that exact version is listed as a published release; a workspace version does not establish asset availability.

On macOS or Linux, download the native package and checksum file, verify the exact asset entry, then extract:

```sh
set -e
VERSION=18.0.0
case "$(uname -s):$(uname -m)" in
  Darwin:arm64) TARGET=aarch64-apple-darwin ;;
  Darwin:x86_64) TARGET=x86_64-apple-darwin ;;
  Linux:x86_64) TARGET=x86_64-unknown-linux-musl ;;
  *) echo 'Use the platform-specific release package' >&2; exit 1 ;;
esac
ASSET="htmlcut-${VERSION}-${TARGET}.tar.gz"
BASE="https://github.com/resoltico/HTMLCut/releases/download/v${VERSION}"
curl -fLO "$BASE/$ASSET"
curl -fLO "$BASE/htmlcut-${VERSION}-checksums.txt"
EXPECTED=$(awk -v asset="$ASSET" '$2 == asset { print $1 }' "htmlcut-${VERSION}-checksums.txt")
[ "${#EXPECTED}" = 64 ] || exit 1
if command -v sha256sum >/dev/null; then
  ACTUAL=$(sha256sum "$ASSET" | awk '{print $1}')
else
  ACTUAL=$(shasum -a 256 "$ASSET" | awk '{print $1}')
fi
[ "$ACTUAL" = "$EXPECTED" ] || exit 1
tar -xzf "$ASSET"
"./htmlcut-${VERSION}-${TARGET}/htmlcut" --version
```

On Windows, use PowerShell after verifying the chosen version is published:

```powershell
$ErrorActionPreference = "Stop"
$Version = "18.0.0"
$Asset = "htmlcut-$Version-x86_64-pc-windows-msvc.zip"
$Base = "https://github.com/resoltico/HTMLCut/releases/download/v$Version"
Invoke-WebRequest "$Base/$Asset" -OutFile $Asset
Invoke-WebRequest "$Base/htmlcut-$Version-checksums.txt" -OutFile "checksums.txt"
$Pattern = '^[a-fA-F0-9]{64}\s+' + [regex]::Escape($Asset) + '$'
$Entries = @(Get-Content "checksums.txt" | Where-Object { $_ -match $Pattern })
if ($Entries.Count -ne 1) { throw "Missing or ambiguous checksum entry" }
$Expected = ($Entries[0] -split '\s+')[0]
if ((Get-FileHash $Asset -Algorithm SHA256).Hash -ne $Expected) { throw "Checksum mismatch" }
Expand-Archive $Asset -DestinationPath "."
& ".\htmlcut-$Version-x86_64-pc-windows-msvc\htmlcut.exe" --version
```

For this candidate checkout, build with its pinned toolchain and use the current grammar:

```sh
cargo install --path crates/htmlcut-cli --locked
htmlcut describe
htmlcut describe extract
htmlcut extract --file page.html --css article --read markdown --raw
htmlcut extract --file page.html --css 'article a' --read attribute:href --match all
htmlcut inspect --file page.html --css article
```

Default output is a JSON array and default selection requires exactly one match. Use repeated `--field NAME CSS READ` for single-valued records; use a plan for advanced cardinality, guards and exclusions. Present empty values are valid; missing required nodes/attributes fail. `--raw` selects one flat string without a final LF.

Save actual source and execution with `--bundle snapshot.htmlcut.tar`, then recompute with `htmlcut run snapshot.htmlcut.tar`. Move the bundle freely; it does not depend on original paths. `--receipt FILE` is the smaller alternative when only evidence is needed. It is mutually exclusive with bundle output. Output replacement requires `--overwrite`.

Callers acquire/render/decode source first. HTMLCut accepts file/stdin UTF-8 and does not fetch URLs, execute JavaScript, use sessions or crawl. The current candidate is 18.0.0; a workspace version is not proof that matching release assets are published. See [CLI](cli.md), [Core](core.md), [Schemas](schema.md), and [Platform Support](platform-support.md).
