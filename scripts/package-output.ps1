param([string]$BuildRoot='')
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$version = (Get-Content -Raw -LiteralPath (Join-Path $projectRoot 'package.json') | ConvertFrom-Json).version
if(-not $BuildRoot){$BuildRoot=Join-Path $projectRoot 'src-tauri/target/release'}
$BuildRoot=[IO.Path]::GetFullPath($BuildRoot)
if(-not $BuildRoot.StartsWith(([IO.Path]::GetFullPath($projectRoot)+[IO.Path]::DirectorySeparatorChar),[StringComparison]::OrdinalIgnoreCase)){throw 'Build root must be inside this project'}
$outputRoot = Join-Path $projectRoot 'outputs'
$null = New-Item -ItemType Directory -Path $outputRoot -Force
Copy-Item -LiteralPath (Join-Path $BuildRoot 'nova-pc-suite.exe') -Destination (Join-Path $outputRoot "Nova-PC-Suite-$version.exe")
Copy-Item -LiteralPath (Join-Path $BuildRoot "bundle/nsis/Nova PC Suite_${version}_x64-setup.exe") -Destination (Join-Path $outputRoot "Nova-PC-Suite-$version-setup.exe")
$lines = @("Nova-PC-Suite-$version.exe","Nova-PC-Suite-$version-setup.exe") | ForEach-Object {
    $stream = [IO.File]::OpenRead((Join-Path $outputRoot $_))
    $sha = [Security.Cryptography.SHA256]::Create()
    try { ([BitConverter]::ToString($sha.ComputeHash($stream))).Replace('-','').ToLowerInvariant() + '  ' + $_ }
    finally { $stream.Dispose(); $sha.Dispose() }
}
[IO.File]::WriteAllLines((Join-Path $outputRoot 'SHA256SUMS.txt'), $lines, (New-Object Text.UTF8Encoding($false)))
Get-ChildItem -LiteralPath $outputRoot | Select-Object Name,Length
