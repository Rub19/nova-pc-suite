param([switch]$ResumeOwnedTest)
$ErrorActionPreference = 'Stop'
function File-Sha256([string]$path) {
    $content = [IO.File]::ReadAllBytes($path)
    # Tauri intentionally patches UNK -> NSS in the installed NSIS payload.
    # Normalize only this documented marker; compare every remaining byte.
    $ascii = [Text.Encoding]::ASCII.GetString($content)
    $marker = '__TAURI_BUNDLE_TYPE_VAR_'
    $positions = @()
    foreach ($expectedTag in @('UNK','NSS')) {
        $offset = 0
        while (($found = $ascii.IndexOf(($marker + $expectedTag), $offset, [StringComparison]::Ordinal)) -ge 0) {
            $positions += $found
            $offset = $found + $marker.Length + 3
        }
    }
    # One runtime marker and one NSS match-string are present in this Tauri build.
    if ($positions.Count -ne 2) { throw 'Unexpected Tauri bundle marker count' }
    foreach ($position in $positions) {
        [Array]::Copy([Text.Encoding]::ASCII.GetBytes('UNK'), 0, $content, $position + $marker.Length, 3)
    }
    $sha = [Security.Cryptography.SHA256]::Create()
    try { return ([BitConverter]::ToString($sha.ComputeHash($content))).Replace('-','').ToLowerInvariant() }
    finally { $sha.Dispose() }
}
$projectRoot = Split-Path -Parent $PSScriptRoot
$targetPath = [IO.Path]::GetFullPath((Join-Path $projectRoot 'work/installer-smoke'))
$allowedRoot = [IO.Path]::GetFullPath((Join-Path $projectRoot 'work')) + [IO.Path]::DirectorySeparatorChar
if (-not $targetPath.StartsWith($allowedRoot, [StringComparison]::OrdinalIgnoreCase)) { throw 'Invalid test directory' }
if ((Test-Path -LiteralPath $targetPath) -and -not $ResumeOwnedTest) { throw 'Test target already exists; refusing to overwrite it.' }
$existing = @(Get-ItemProperty 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*' -ErrorAction SilentlyContinue | Where-Object { $_.DisplayName -eq 'Nova PC Suite' })
if ($existing.Count -gt 0 -and -not $ResumeOwnedTest) { throw 'Nova is already installed; refusing to disturb that installation.' }
if ($ResumeOwnedTest -and @($existing | Where-Object { $_.InstallLocation -and ([IO.Path]::GetFullPath($_.InstallLocation.Trim('"')).TrimEnd('\') -ne $targetPath.TrimEnd('\')) }).Count -gt 0) { throw 'Unrelated Nova installation found' }
$version = (Get-Content -Raw -LiteralPath (Join-Path $projectRoot 'package.json') | ConvertFrom-Json).version
$installer = Join-Path $projectRoot "src-tauri/target/release/bundle/nsis/Nova PC Suite_${version}_x64-setup.exe"
if (-not $ResumeOwnedTest) {
    $install = Start-Process -FilePath $installer -ArgumentList @('/S', ('/D=' + $targetPath)) -WindowStyle Hidden -PassThru
    $install.WaitForExit()
    if ($install.ExitCode -ne 0) { throw 'Installer failed' }
}
$exe = Join-Path $targetPath 'nova-pc-suite.exe'
if (-not (Test-Path -LiteralPath $exe)) { throw 'Installed executable missing' }
$source = File-Sha256 (Join-Path $projectRoot 'src-tauri/target/release/nova-pc-suite.exe')
$installed = File-Sha256 $exe
if ($source -ne $installed) { throw 'Installed executable mismatch' }
Write-Output 'Silent install and installed executable integrity: PASS'
$uninstallers = @(Get-ChildItem -LiteralPath $targetPath -Filter 'uninstall*.exe' -File)
if ($uninstallers.Count -ne 1 -or $uninstallers[0].DirectoryName -ne $targetPath) { throw 'Uninstaller target is ambiguous' }
$uninstall = Start-Process -FilePath $uninstallers[0].FullName -ArgumentList '/S' -WindowStyle Hidden -PassThru
$uninstall.WaitForExit()
if ($uninstall.ExitCode -ne 0) { throw 'Uninstaller failed' }
$deadline = (Get-Date).AddSeconds(30)
while ((Test-Path -LiteralPath $exe) -and (Get-Date) -lt $deadline) { Start-Sleep -Milliseconds 200 }
if (Test-Path -LiteralPath $exe) { throw 'Uninstaller left the installed executable' }
Write-Output 'Silent uninstall removed only the temporary test installation: PASS'
