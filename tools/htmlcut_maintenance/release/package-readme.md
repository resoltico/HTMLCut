# HTMLCut {version}

This package contains the standalone HTMLCut CLI for `{target_triple}`.

## Package Contents

- `{binary_name}` — the packaged HTMLCut executable
- `README.md` — package-specific install and verification guide
- `LICENSE`, `NOTICE`, and `PATENTS.md` — legal notices for this package

## Run It

Run the binary from this directory or move it onto your PATH.

```{example_shell}
{binary_command} --version
```

## Quick Smoke

Create a tiny fixture and extract one link:

```{example_shell}
{fixture_command}
{binary_command} get 'article a.more' ./page.html --read attr:href --raw
```

## Source availability and licensing

Original HTMLCut source is MPL-2.0. Upstream dependencies retain their licenses;
NOTICE contains the complete locked native dependency attribution.

Matching source archives:
- https://github.com/resoltico/HTMLCut/releases/download/v{version}/htmlcut-source-{version}.tar.gz
- https://github.com/resoltico/HTMLCut/releases/download/v{version}/htmlcut-source-{version}.zip

Exact source commit: https://github.com/resoltico/HTMLCut/tree/{source_commit}
Registry dependency source is available at the exact name/version links in NOTICE.

## More

- CLI guide: https://github.com/resoltico/HTMLCut/blob/{source_commit}/crates/htmlcut-cli/src/guide.txt
- Getting started: https://github.com/resoltico/HTMLCut/blob/{source_commit}/README.md
- Embedding and API guarantees: https://github.com/resoltico/HTMLCut/blob/{source_commit}/docs/design.md
