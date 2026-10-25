$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$targetPath = [IO.Path]::GetFullPath((Join-Path $projectRoot 'work/installer-smoke'))
$allowedRoot = [IO.Path]::GetFullPath((Join-Path $projectRoot 'work')) + [IO.Path]::DirectorySeparatorChar
if (-not $targetPath.StartsWith($allowedRoot, [StringComparison]::OrdinalIgnoreCase)) { throw 'Invalid test directory' }
if (Test-Path -LiteralPath $targetPath) { throw 'Test target already exists; refusing to overwrite it.' }
$existing = @(Get-ItemProperty 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*' -ErrorAction SilentlyContinue | Where-Object { $_.DisplayName -eq 'Nova PC Suite' })
if ($existing.Count -gt 0) { throw 'Nova is already installed; refusing to disturb that installation.' }
$installer = Join-Path $projectRoot 'src-tauri/target/release/bundle/nsis/Nova PC Suite_0.1.0_x64-setup.exe'
$install = Start-Process -FilePath $installer -ArgumentList @('/S', ('/D=' + $targetPath)) -WindowStyle Hidden -PassThru
$install.WaitForExit()
if ($install.ExitCode -ne 0) { throw 'Installer failed' }
$exe = Join-Path $targetPath 'nova-pc-suite.exe'
if (-not (Test-Path -LiteralPath $exe)) { throw 'Installed executable missing' }
$source = Get-FileHash -Algorithm SHA256 -LiteralPath (Join-Path $projectRoot 'src-tauri/target/release/nova-pc-suite.exe')
$installed = Get-FileHash -Algorithm SHA256 -LiteralPath $exe
if ($source.Hash -ne $installed.Hash) { throw 'Installed executable mismatch' }
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
