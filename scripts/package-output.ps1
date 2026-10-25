$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$outputRoot = Join-Path $projectRoot 'outputs'
$null = New-Item -ItemType Directory -Path $outputRoot -Force
Copy-Item -LiteralPath (Join-Path $projectRoot 'src-tauri/target/release/nova-pc-suite.exe') -Destination (Join-Path $outputRoot 'Nova-PC-Suite-0.1.0.exe')
Copy-Item -LiteralPath (Join-Path $projectRoot 'src-tauri/target/release/bundle/nsis/Nova PC Suite_0.1.0_x64-setup.exe') -Destination (Join-Path $outputRoot 'Nova-PC-Suite-0.1.0-setup.exe')
$lines = @('Nova-PC-Suite-0.1.0.exe','Nova-PC-Suite-0.1.0-setup.exe') | ForEach-Object {
    $stream = [IO.File]::OpenRead((Join-Path $outputRoot $_))
    $sha = [Security.Cryptography.SHA256]::Create()
    try { ([BitConverter]::ToString($sha.ComputeHash($stream))).Replace('-','').ToLowerInvariant() + '  ' + $_ }
    finally { $stream.Dispose(); $sha.Dispose() }
}
[IO.File]::WriteAllLines((Join-Path $outputRoot 'SHA256SUMS.txt'), $lines, (New-Object Text.UTF8Encoding($false)))
Get-ChildItem -LiteralPath $outputRoot | Select-Object Name,Length
