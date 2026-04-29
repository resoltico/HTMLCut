---
afad: "4.0"
version: "6.0.0"
domain: SETUP
updated: "2026-04-29"
route:
  keywords: [getting started, quick start, install, release package, cargo install, first extraction, request file]
  questions: ["how do I install HTMLCut?", "how do I try HTMLCut on a sample page?", "how do I save a reusable extraction request?"]
  related: [../README.md, cli.md, platform-support.md, core.md, interop-v1.md]
---

# Getting Started

This guide gets you from install to a first saved extraction request.

If you want the short storefront view first, start at [../README.md](../README.md).
If you want the full operator model after this guide, continue into [cli.md](cli.md).

## Choose Your Start

- Use a prebuilt release package when you want the `htmlcut` binary quickly.
- Use `cargo install --path ...` from this repository when you already keep Rust tools locally.
- Use a source build when you want the debug binary or a local development loop.

## Install A Prebuilt Release Package

Platform coverage and release-target details live in [platform-support.md](platform-support.md).
Release packages are published on the [HTMLCut releases page](https://github.com/resoltico/HTMLCut/releases).

### macOS Or Linux

```bash
VERSION=6.0.0
TARGET=aarch64-apple-darwin # or x86_64-apple-darwin / x86_64-unknown-linux-musl
curl -fsSLO "https://github.com/resoltico/HTMLCut/releases/download/v${VERSION}/htmlcut-${VERSION}-${TARGET}.tar.gz"
curl -fsSLO "https://github.com/resoltico/HTMLCut/releases/download/v${VERSION}/htmlcut-${VERSION}-checksums.txt"
EXPECTED="$(grep "  htmlcut-${VERSION}-${TARGET}.tar.gz$" "htmlcut-${VERSION}-checksums.txt" | awk '{print $1}')"
if command -v sha256sum >/dev/null 2>&1; then
  ACTUAL="$(sha256sum "htmlcut-${VERSION}-${TARGET}.tar.gz" | awk '{print $1}')"
else
  ACTUAL="$(shasum -a 256 "htmlcut-${VERSION}-${TARGET}.tar.gz" | awk '{print $1}')"
fi
if [ "$ACTUAL" != "$EXPECTED" ]; then
  printf 'checksum mismatch for %s\n' "htmlcut-${VERSION}-${TARGET}.tar.gz" >&2
  exit 1
fi
tar -xzf "htmlcut-${VERSION}-${TARGET}.tar.gz"
mkdir -p "$HOME/.local/bin"
install "htmlcut-${VERSION}-${TARGET}/htmlcut" "$HOME/.local/bin/htmlcut"
export PATH="$HOME/.local/bin:$PATH"
htmlcut --help
```

### Windows PowerShell

```powershell
$Version = "6.0.0"
$Target = "x86_64-pc-windows-msvc"
Invoke-WebRequest "https://github.com/resoltico/HTMLCut/releases/download/v$Version/htmlcut-$Version-$Target.zip" -OutFile "htmlcut-$Version-$Target.zip"
Invoke-WebRequest "https://github.com/resoltico/HTMLCut/releases/download/v$Version/htmlcut-$Version-checksums.txt" -OutFile "htmlcut-$Version-checksums.txt"
$Expected = ((Select-String -Path "htmlcut-$Version-checksums.txt" -Pattern "  htmlcut-$Version-$Target\.zip$").Line -replace ' .*', '').ToLowerInvariant()
$Actual = (Get-FileHash "htmlcut-$Version-$Target.zip" -Algorithm SHA256).Hash.ToLowerInvariant()
if ($Actual -ne $Expected) { throw "checksum mismatch" }
Expand-Archive "htmlcut-$Version-$Target.zip" -DestinationPath .
New-Item -ItemType Directory -Force "$HOME\bin" | Out-Null
Copy-Item "htmlcut-$Version-$Target\htmlcut*" "$HOME\bin"
$env:Path = "$HOME\bin;$env:Path"
htmlcut --help
```

Each prebuilt package contains the platform binary plus `README.md`, `LICENSE`, `NOTICE`, and
`PATENTS.md`.

## Install From This Repository

Build from source with the repo-pinned Rust `1.95.0` toolchain:

```bash
rustup toolchain install 1.95.0 --profile minimal
source "$HOME/.cargo/env"
cargo build --locked -p htmlcut-cli --bin htmlcut
./target/debug/htmlcut --help
```

`rust-toolchain.toml` is the canonical exact repo toolchain pin, and `Cargo.toml`
`[workspace.package] rust-version` mirrors the published compiler requirement.

Install into Cargo's bin directory from this repository root:

```bash
source "$HOME/.cargo/env"
cargo install --path crates/htmlcut-cli --locked
htmlcut --help
```

If you are bootstrapping a fresh maintainer machine, use
[developer-setup.md](developer-setup.md) instead. That guide covers nightly, `cargo-fuzz`,
coverage tooling, and the rest of the contributor toolchain.

## Try It On A Small Page

Create the demo page used by the commands below:

```bash
cat > ./page.html <<'HTML'
<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <title>HTMLCut README Fixture</title>
</head>
<body>
  <main>
    <article>
      <h1>Guide</h1>
      <div class="card">Card alpha</div>
      <div class="card">Card beta</div>
      <p><a class="more" href="../guide.html">Read more</a></p>
      <pre>START::Regex slice payload::END</pre>
    </article>
  </main>
</body>
</html>
HTML
```

Extract readable text from the first article:

```bash
htmlcut select ./page.html --css article
```

Require exactly one match:

```bash
htmlcut select ./page.html --css article --match single
```

Extract the matched node as inner HTML:

```bash
htmlcut select ./page.html --css article --value inner-html
```

Extract every card as outer HTML:

```bash
htmlcut select ./page.html --css '.card' --match all --value outer-html
```

Rewrite a relative link against a base URL:

```bash
htmlcut select ./page.html \
  --css 'article a.more' \
  --value attribute \
  --attribute href \
  --rewrite-urls \
  --base-url https://example.com/docs/start.html
```

Slice raw source between literal boundaries:

```bash
htmlcut slice ./page.html --from '<article>' --to '</article>'
```

Slice raw source between regex boundaries:

```bash
htmlcut slice ./page.html \
  --from 'START::' \
  --to '::END' \
  --pattern regex \
  --match all \
  --output json
```

Inspect a source before choosing selectors:

```bash
htmlcut inspect source ./page.html --output text
```

Preview selector matches before final extraction:

```bash
htmlcut inspect select ./page.html --css '.card' --match all
```

Preview slice matches before final extraction:

```bash
htmlcut inspect slice ./page.html --from '<article>' --to '</article>'
```

## Save A Repeat Request

Write the normalized extraction-definition file while you prototype inline flags:

```bash
htmlcut select ./page.html \
  --css 'article a.more' \
  --value attribute \
  --attribute href \
  --emit-request-file ./article-links.json
```

Run that saved request again:

```bash
htmlcut select --request-file ./article-links.json
```

Write only the stdout payload to one file:

```bash
htmlcut select ./page.html \
  --css article \
  --output-file ./article.txt
```

The deeper output rules, bundle behavior, request-file constraints, and discovery surfaces live in
[cli.md](cli.md).

## Where To Go Next

- [cli.md](cli.md) for the full command model, output rules, and bundle workflow
- [core.md](core.md) for the Rust library entrypoints
- [interop-v1.md](interop-v1.md) for the downstream integration profile
- [schema.md](schema.md) for the maintained public schema registry
- [README.md](../README.md) for the short product-facing overview
