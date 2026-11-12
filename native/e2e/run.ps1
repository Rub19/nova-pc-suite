param([string]$Script = "nova.txt", [switch]$PendantJeu)
$ErrorActionPreference = "Stop"
Set-Location (Split-Path $PSScriptRoot)
if (-not $PendantJeu -and (Get-Process "League of Legends","VALORANT-Win64-Shipping" -ErrorAction SilentlyContinue)) { "Un jeu est ouvert : rien n'est lancé."; exit 2 }
$build = Start-Process cargo -ArgumentList "build","-p","nova-native","--features","e2e","-j","3" -NoNewWindow -PassThru -RedirectStandardError "$env:TEMP\nova-e2e-build.txt"
$build.PriorityClass = $(if ($PendantJeu) { "Idle" } else { "BelowNormal" })
$build.WaitForExit()
if (Select-String -Path "$env:TEMP\nova-e2e-build.txt" -Pattern "^error" -Quiet) { Get-Content "$env:TEMP\nova-e2e-build.txt" | Select-String "^error" -Context 0,10 | Select-Object -First 3; exit 1 }
$out = Join-Path $env:TEMP "nova-e2e"
Remove-Item -Recurse -Force $out -ErrorAction SilentlyContinue
New-Item -ItemType Directory "$out\captures" | Out-Null
$env:NOVA_E2E = Join-Path $PSScriptRoot $Script
$env:NOVA_E2E_LOG = "$out\journal.txt"
$env:NOVA_E2E_SNAP = "$out\captures"
New-Item -ItemType Directory "$out\exports" | Out-Null
$env:NOVA_E2E_EXPORT = "$out\exports"
$app = Start-Process "target\debug\nova-native.exe" -PassThru -WindowStyle Hidden
$app.PriorityClass = "BelowNormal"
$app.WaitForExit()
Get-Content "$out\journal.txt" -Encoding utf8
Get-ChildItem "$out\exports" -ErrorAction SilentlyContinue | ForEach-Object { "export : {0} ({1} octets)" -f $_.Name, $_.Length }
"Captures : $out\captures"
