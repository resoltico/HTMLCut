param([Parameter(Mandatory=$true)][string]$Binary, [Parameter(Mandatory=$true)][string]$Evidence)
$ErrorActionPreference = 'Stop'
if ($env:OS -ne 'Windows_NT') { throw 'Named-pipe proof requires Windows' }
$Binary = (Resolve-Path -LiteralPath $Binary).Path
$rows = @()
foreach ($role in @('source', 'plan')) {
    $name = 'htmlcut-input-' + [Guid]::NewGuid().ToString('N')
    $pipe = New-Object System.IO.Pipes.NamedPipeServerStream($name, [System.IO.Pipes.PipeDirection]::InOut)
    $path = '\\.\pipe\' + $name
    $arguments = $(switch ($role) {
        'source' { @('extract', '--file', $path, '--select', 'p') }
        'plan' { @('extract', '--stdin', '--plan', $path) }
    })
    $row = [ordered]@{role=$role; passed=$false; server_waited_for_connection=$false}
    $process = New-Object System.Diagnostics.Process
    try {
        $process.StartInfo.FileName = $Binary
        $process.StartInfo.Arguments = ($arguments | ForEach-Object { '"' + $_ + '"' }) -join ' '
        $process.StartInfo.UseShellExecute = $false
        $process.StartInfo.RedirectStandardOutput = $true
        $process.StartInfo.RedirectStandardError = $true
        $process.StartInfo.RedirectStandardInput = $true
        if (-not $process.Start()) { throw 'Failed to start pipe input candidate' }
        $process.StandardInput.Close()
        $stdout = $process.StandardOutput.ReadToEndAsync()
        $stderr = $process.StandardError.ReadToEndAsync()
        if (-not $process.WaitForExit(3000)) { $process.Kill(); $process.WaitForExit(); throw 'Named-pipe file acquisition blocked' }
        $row.exit_code = $process.ExitCode
        $row.stdout = $stdout.GetAwaiter().GetResult()
        $row.stderr = $stderr.GetAwaiter().GetResult()
        $errorValue = $row.stderr | ConvertFrom-Json
        $row.passed = $row.exit_code -eq 5 -and $row.stdout -eq '' -and $errorValue.code -eq 'acquisition' -and $errorValue.cause.kind -eq 'io' -and $errorValue.cause.operation -eq 'input'
    } catch { $row.harness_error = $_.Exception.ToString() }
    finally { $process.Dispose(); $pipe.Dispose() }
    $rows += [pscustomobject]$row
}
$passed = $rows.Count -eq 3 -and @($rows | Where-Object { -not $_.passed }).Count -eq 0
$proof = [ordered]@{schema='htmlcut.native-windows-pipe-input'; version=1; binary_sha256=(Get-FileHash -LiteralPath $Binary -Algorithm SHA256).Hash.ToLowerInvariant(); rows=$rows; passed=$passed}
[System.IO.File]::WriteAllText($Evidence, ($proof | ConvertTo-Json -Depth 10), (New-Object System.Text.UTF8Encoding($false)))
if (-not $passed) { throw "Native pipe file-role refusals failed; evidence: $Evidence" }
