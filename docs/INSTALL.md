# Install, update and uninstall HTMLCut

Choose the user-local installer, Cargo installation or a manual release archive.
The installer manages the CLI and its own PATH entries. HTML snapshots, contracts
and extracted output are outside its scope. No administrator access, background
updater or scheduled task is needed.

## Existing executable without an installer receipt

`Existing executable has no installer receipt` means a file exists at the printed
path, but this installer has no record of managing it. This does not identify how
it was installed. No program or PATH entry was changed by this refusal.

- **Replace it:** use the complete command printed by the installer. It retains
  the selected directory, version pin and choice to leave PATH unchanged. Only
  the named executable is replaced, after the new candidate verifies, and the
  original bytes are kept in a recovery backup.
- **Keep it:** install into a fresh directory with `--no-modify-path` /
  `-NoModifyPath`, then use the printed full executable path. Choose which
  installation should be used before adding another command to PATH.

For the usual default directories and a saved installer, replacement is:

```sh
sh ./install-htmlcut.sh --install-dir "$HOME/.local/bin" --replace-unmanaged
```

```powershell
powershell -NoProfile -ExecutionPolicy ByPass -File .\install-htmlcut.ps1 -InstallDir "$env:LOCALAPPDATA\Programs\HTMLCut" -ReplaceUnmanaged
if ($LASTEXITCODE -ne 0) { throw "Installer failed ($LASTEXITCODE)." }
```

Use the printed command instead if `XDG_BIN_HOME`, a custom directory, a version
pin or disabled PATH changes were selected. If execution through stdin or an
expression prevents identifying a saved script, [save and inspect it](#installer)
first, then apply the printed options to that file. Mirror settings remain in
the environment; recovery commands do not print them.

To keep the existing executable, choose a fresh directory, for example:

```sh
sh ./install-htmlcut.sh --install-dir "$HOME/htmlcut-cli" --no-modify-path
```

```powershell
powershell -NoProfile -ExecutionPolicy ByPass -File .\install-htmlcut.ps1 -InstallDir "$env:LOCALAPPDATA\HTMLCut-CLI" -NoModifyPath
if ($LASTEXITCODE -ne 0) { throw "Installer failed ($LASTEXITCODE)." }
```

Retain any selected version option. After replacement, keep the original backup
at its exact reported path until you have [verified the new executable](#verify).
Uninstall does not remove the backup. To restore it after successful replacement,
first [uninstall](#uninstall) the new managed copy from the same directory. Only
after uninstall succeeds, move the exact reported backup to the original
executable path. Review PATH separately; old entries were not adopted, and new
owned entries may have been removed. If activation or restoration failed,
preserve the reported recovery files and inspect them before retrying.

The replacement flag is install-only. It cannot bypass changed managed files,
corrupt state, symlinks, nonregular targets or access failures. If the error occurs
during **uninstall**, use the method that installed that executable. If its origin
is unknown, preserve and inspect the exact file before deciding on manual removal.
Replacement is not a way to uninstall an untracked executable.

## Installer

The scripts in a reviewed checkout install a selected release:

```sh
sh install/install.sh
```

```powershell
powershell -NoProfile -ExecutionPolicy ByPass -File .\install\install.ps1
if ($LASTEXITCODE -ne 0) { throw "Installer failed ($LASTEXITCODE)." }
```

To use a standalone release installer, save, inspect and run it:

```sh
curl --proto '=https' --proto-redir '=https' -fL -o ./install-htmlcut.sh https://github.com/resoltico/HTMLCut/releases/latest/download/install.sh &&
  less ./install-htmlcut.sh &&
  sh ./install-htmlcut.sh
```

If curl is unavailable, use `wget --https-only -O ./install-htmlcut.sh URL` with
the same URL and `&&` chain. The installer needs standard shell tools, tar and a
SHA-256 tool (`sha256sum`, `shasum` or `openssl`). `less` is only for review.

```powershell
& {
    $ErrorActionPreference = 'Stop'
    Invoke-WebRequest -UseBasicParsing -Uri https://github.com/resoltico/HTMLCut/releases/latest/download/install.ps1 -OutFile .\install-htmlcut.ps1 -ErrorAction Stop
    Get-Content .\install-htmlcut.ps1
    powershell -NoProfile -ExecutionPolicy ByPass -File .\install-htmlcut.ps1
    if ($LASTEXITCODE -ne 0) { throw "Installer failed ($LASTEXITCODE)." }
}
```

To pause for review, stop after download, inspect the file, then run the child
command. The guarded downloads prevent a failed fetch from executing an old
saved installer and preserve installer failure status. A bare `curl | sh` can
hide a failed download behind a successful empty shell.

Windows PowerShell 5.1 and PowerShell 7 are supported; use `pwsh` instead of
`powershell` to select PowerShell 7. `ByPass` applies to the child process only;
it does not change persistent policy or override managed-device restrictions.
Keep the saved installer for offline removal. `installers.sha256` contains hashes
of the two standalone installer scripts; compare a downloaded script with its
entry using the SHA-256 commands under [manual archives](#manual-archives).
`sha256.sum` covers release archives; installer hashes are published separately. Published installers link Help to
their source commit; checkout scripts link to the current `main` guide.

Supported release targets are macOS 12 or later on Apple Silicon and Intel,
Linux x86-64 musl, and Windows x86-64 MSVC. Native CPU selection handles translated
Mac shells. Linux ARM64 and Windows ARM64 are not shipped release targets.
Candidate execution checks the local runtime; it does not establish support for
every older OS version or override execution policy.

The default executable is `$XDG_BIN_HOME/htmlcut` when `XDG_BIN_HOME` is set,
otherwise `~/.local/bin/htmlcut`. Windows uses
`%LOCALAPPDATA%\Programs\HTMLCut\htmlcut.exe`. Choose an absolute directory with
`--install-dir DIR` / `-InstallDir DIR`. `--no-modify-path` / `-NoModifyPath`
leaves persistent and current-session PATH unchanged. Otherwise, open a new
terminal after PATH changes; a child installer cannot change its parent's PATH.

## Verify

Run the full executable path printed by the installer with `--version` and
`--help`. This selects the installation just changed rather than another command
on PATH. For the usual default directories:

```sh
"$HOME/.local/bin/htmlcut" --version
"$HOME/.local/bin/htmlcut" --help
```

```powershell
& "$env:LOCALAPPDATA\Programs\HTMLCut\htmlcut.exe" --version
& "$env:LOCALAPPDATA\Programs\HTMLCut\htmlcut.exe" --help
```

Use the printed path for `XDG_BIN_HOME` or custom directories. After opening a
new terminal, `command -v htmlcut` on Unix or `Get-Command htmlcut` in PowerShell
shows which command PATH selects. Do not remove an unrelated installation merely
because it shadows the new executable.

## Update and choose a release

Repeat installation to update an unchanged installer-managed copy. Latest is
resolved once to a stable `vX.Y.Z` tag; archive and `sha256.sum` come from that
release. A failed download, checksum or candidate test precedes activation.

To choose a stable payload with a reviewed checkout installer, for example:

```sh
sh install/install.sh --version 25.0.0
```

```powershell
powershell -NoProfile -ExecutionPolicy ByPass -File .\install\install.ps1 -Version 25.0.0
if ($LASTEXITCODE -ne 0) { throw "Installer failed ($LASTEXITCODE)." }
```

Download installer and payload from the same chosen release: replace `latest/download` in the download URL with
`download/vX.Y.Z`, and pass the same `X.Y.Z` with `--version` / `-Version`.
`HTMLCUT_VERSION` supplies the version when no option is given. Clear an
environment pin to return to latest:

```sh
unset HTMLCUT_VERSION
```

```powershell
Remove-Item Env:HTMLCUT_VERSION -ErrorAction SilentlyContinue
```

## Ownership and recovery

The installer stores ownership data in `.htmlcut-install` beside the executable.
It verifies the receipt and every remaining managed file before updating or
removing them. A filename, version string or PATH entry is not ownership proof.
It never executes an untracked executable to identify it. Follow the
[replace or keep instructions](#existing-executable-without-an-installer-receipt)
when a regular executable exists without installer state.

The private legal directory retains `LICENSE`, `NOTICE`,
`PROJECT-NOTICE.md`, `LICENSE-COMMERCIAL.md` and `THIRD-PARTY-LICENSES.html`
from the archive. Receipts are data, not shell code.
An intact receipt with only the executable missing permits repair by reinstalling
or cleanup by uninstalling. Remaining owned files must still verify. Never edit
hashes to bypass refusal, copy receipts between directories, or delete a shared
bin directory.

Local conflicts are diagnosed before download. The installer stages privately on
the destination filesystem and rechecks the target under an exclusive lock
before activation. A held lock fails promptly; establish that its installer has
stopped before removing a stale lock. Pre-existing directories and PATH entries
remain unowned. Unrelated files and user edits are preserved.

Activation failure attempts to restore prior executable/state and only this
transaction's attributable PATH additions. Handled interruption can run cleanup;
abrupt termination and power loss cannot. An error reporting incomplete recovery
is not proof that prior state was restored. Preserve the exact reported staging
and recovery paths. Unix startup-file recovery images may contain private shell
configuration; inspect them privately before manual restoration or cleanup.
Windows may retain one receipt-owned running executable; close processes using
it before retrying a refused update or removal.

## Uninstall

Use the saved, reviewed installer and the same custom directory, if any:

```sh
sh ./install-htmlcut.sh --uninstall
```

```powershell
powershell -NoProfile -ExecutionPolicy ByPass -File .\install-htmlcut.ps1 -Uninstall
if ($LASTEXITCODE -ne 0) { throw "Installer failed ($LASTEXITCODE)." }
```

Removal requires no program download. It removes only verified managed files
and attributable PATH entries. `--no-modify-path` / `-NoModifyPath` also disables
PATH cleanup on uninstall. User-edited PATH entries and shared-directory entries
may be retained with an explanation. A failed Windows PATH removal leaves the
executable and receipt for retry. Changed or untracked files remain protected.
Use the originating installation method to remove an untracked executable.

## Options and mirrors

| Unix | Windows | Effect |
|---|---|---|
| `--version X.Y.Z` | `-Version X.Y.Z` | Select a stable payload release; one leading `v` is allowed. |
| `--install-dir DIR` | `-InstallDir DIR` | Select an absolute install directory. |
| `--no-modify-path` | `-NoModifyPath` | Leave PATH unchanged, including on uninstall. |
| `--replace-unmanaged` | `-ReplaceUnmanaged` | Explicitly back up and replace one untracked regular executable; install only. |
| `--uninstall` | `-Uninstall` | Remove verified managed files and owned PATH entries. |
| `--help` | `-Help` | Show installer usage. |

`HTMLCUT_INSTALL_DIR` and `HTMLCUT_NO_MODIFY_PATH=1` supply directory and PATH
choices. `HTMLCUT_DOWNLOAD_BASE` selects an HTTPS directory or absolute local
release directory containing the native archive and `sha256.sum`. Redirects must
stay on HTTPS; mirror URLs may not contain credentials, queries or fragments.
Authenticated fetches can be handled separately into a local directory. A mirror
is trusted as the source; its checksum does not authenticate its publisher.

## Cargo

With Rust and the native linker installed, run `cargo install htmlcut --locked`.
Repeat it to update; remove that installation with `cargo uninstall htmlcut`.
Cargo owns its installation metadata. This installer does not adopt Cargo's
records; use Cargo to manage a Cargo installation, or choose another directory.
See [source-build prerequisites](../README.md#install).

## Manual archives

Download the native archive and `sha256.sum` from the same
[release](https://github.com/resoltico/HTMLCut/releases). Compare its SHA-256 with
that entry using `shasum -a 256 ARCHIVE` on macOS, `sha256sum ARCHIVE` on Linux, or
`Get-FileHash ARCHIVE -Algorithm SHA256` in PowerShell, then extract into a fresh
directory. Archives are `htmlcut-TARGET.tar.gz` on Unix and `htmlcut-TARGET.zip`
on Windows. Keep their license notices. Run the extracted executable using its
full path before adding its directory to PATH. For a manual update, preserve the
old executable and notices before replacing them; remove only the files and PATH
entry you added when uninstalling manually.

## Trust and licenses

Checksums detect damaged downloads, not a compromised release or mirror. The
installer executes the checksum-verified candidate as your user; receipts protect
against accidental replacement, not another hostile process using your account.
It does not bypass Gatekeeper, SmartScreen or managed-device policy. Mac builds
are not Developer ID signed or notarized; Windows builds are unsigned.

Original HTMLCut software, installers, tests and documentation use AGPL-3.0-only;
see [LICENSE](../LICENSE), [licensing and source](../PROJECT-NOTICE.md) and
[NOTICE](../NOTICE). Adapted ISC portions and registry dependencies retain their
own terms. Matching vendored source is provided separately as `SOURCE.tar.gz` on
the same release. The shared installer engine's reviewed source revision is in
`install/engine-source.txt`. Release artifact attestations can be verified separately
with `gh attestation verify ASSET --repo resoltico/HTMLCut`; the installer does not
claim to authenticate them.
