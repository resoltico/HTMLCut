param(
    [Parameter(Mandatory=$true)][string]$Binary,
    [Parameter(Mandatory=$true)][string]$Evidence
)
$ErrorActionPreference = 'Stop'
if ($env:OS -ne 'Windows_NT') { throw 'Native Windows stdio proof requires Windows' }
$Binary = (Resolve-Path -LiteralPath $Binary).Path
$before = (Get-FileHash -LiteralPath $Binary -Algorithm SHA256).Hash.ToLowerInvariant()
Add-Type -Path (Join-Path $PSScriptRoot 'windows-console-process.cs')
$root = Join-Path ([System.IO.Path]::GetTempPath()) ('htmlcut-console-' + [Guid]::NewGuid().ToString('N'))
[System.IO.Directory]::CreateDirectory($root) | Out-Null
$source = Join-Path $root 'source.html'
$sink = Join-Path $root 'readonly.txt'
$unicode = "Caf$([char]0xe9) $([char]0x20ac) $([char]0x6771)$([char]0x4eac)"
$utf8 = New-Object System.Text.UTF8Encoding($false, $true)
[System.IO.File]::WriteAllText($source, ('<p>' + $unicode + '</p>'), $utf8)
[System.IO.File]::WriteAllText($sink, 'KEEP', $utf8)
$rows = @()
function RunConsoleCase([string]$name, [string]$mode, [bool]$consoleInput, [bool]$invalidSelector) {
    $arguments = @('extract')
    if ($consoleInput) { $arguments += '--stdin' } else { $arguments += @('--file', $source) }
    $arguments += @('--css', $(if ($invalidSelector) { '[' } else { 'p' }), '--raw')
    $inputText = '<p>' + $unicode + '</p>' + [char]13 + [char]26
    $start = [DateTime]::UtcNow
    $row = [ordered]@{ case=$name; mode=$mode; console_input=$consoleInput; passed=$false }
    try {
        $result = [WindowsConsoleProcess]::Run($Binary, $arguments, $mode, $consoleInput, $inputText, $sink)
        $stdout = $utf8.GetString([Convert]::FromBase64String($result.StdoutBase64))
        $stderr = $utf8.GetString([Convert]::FromBase64String($result.StderrBase64))
        $row.exit_code = $result.ExitCode; $row.stdout = $stdout; $row.stderr = $stderr
        $row.screen = $result.Screen; $row.output_code_page = $result.CodePage
        if ($invalidSelector) {
            $diagnostic = $(if ($mode -eq 'stderr-console') { $result.Screen } else { $stderr }) | ConvertFrom-Json
            $unused = $(if ($mode -eq 'stderr-console') { $stderr } else { $result.Screen })
            $row.passed = $result.ExitCode -eq 2 -and $stdout -eq '' -and $unused -eq '' -and $diagnostic.code -eq 'invalid_selector'
        } elseif ($mode -eq 'readonly' -or $mode -eq 'invalid') {
            $diagnostic = $stderr | ConvertFrom-Json
            $row.passed = $result.ExitCode -eq 5 -and $stdout -eq '' -and $result.Screen -eq '' -and $diagnostic.code -eq 'publication'
        } elseif ($mode -eq 'stdout-console') {
            $row.passed = $result.ExitCode -eq 0 -and $stdout -eq '' -and $stderr -eq '' -and $result.Screen -ceq $unicode
        } else {
            $row.passed = $result.ExitCode -eq 0 -and $stdout -ceq $unicode -and $stderr -eq '' -and $result.Screen -eq ''
        }
        if ([System.IO.File]::ReadAllText($sink) -ne 'KEEP') { $row.passed = $false }
        if ($result.CodePage -ne 437) { $row.passed = $false }
    } catch { $row.harness_error = $_.Exception.ToString() }
    $row.seconds = ([DateTime]::UtcNow - $start).TotalSeconds
    return [pscustomobject]$row
}
try {
    [WindowsConsoleProcess]::Begin()
    try {
        $rows += RunConsoleCase 'console-stdout-unicode' 'stdout-console' $false $false
        $rows += RunConsoleCase 'console-stdin-unicode' 'stdout-console' $true $false
        $rows += RunConsoleCase 'pipe-stdout-console-stderr' 'stderr-console' $false $false
        $rows += RunConsoleCase 'error-pipe-stderr-console-stdout' 'stdout-console' $false $true
        $rows += RunConsoleCase 'error-console-stderr-pipe-stdout' 'stderr-console' $false $true
        $rows += RunConsoleCase 'readonly-stdout' 'readonly' $false $false
        $rows += RunConsoleCase 'invalid-stdout' 'invalid' $false $false
    } finally { [WindowsConsoleProcess]::End() }
} finally {
    $after = (Get-FileHash -LiteralPath $Binary -Algorithm SHA256).Hash.ToLowerInvariant()
    $passed = $rows.Count -eq 7 -and @($rows | Where-Object { -not $_.passed }).Count -eq 0 -and $before -eq $after
    $proof = [ordered]@{ schema='htmlcut.native-windows-stdio'; version=1; binary_sha256=$before; binary_unchanged=($before -eq $after); runner_os=[Environment]::OSVersion.ToString(); rows=$rows; passed=$passed }
    [System.IO.File]::WriteAllText($Evidence, ($proof | ConvertTo-Json -Depth 12), $utf8)
    Remove-Item -LiteralPath $root -Recurse -Force
}
if (-not $passed) { throw "Native Windows stdio cases failed; evidence: $Evidence" }
