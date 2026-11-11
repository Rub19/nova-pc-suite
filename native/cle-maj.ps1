$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot
$key = Join-Path $HOME ".tauri\nova-pc-suite.key"
if (Test-Path $key) { "La clé existe déjà : $key"; exit 1 }
New-Item -ItemType Directory -Force (Split-Path $key) | Out-Null
cargo tauri signer generate -w $key --ci
if (-not (Test-Path "$key.pub")) { throw "clé non créée" }
[IO.File]::WriteAllText((Join-Path $PSScriptRoot "update.pub"), (Get-Content "$key.pub" -Raw).Trim())
"Clé privée : $key (à garder, ne jamais la partager). Clé publique copiée dans update.pub : recompile Nova et commite update.pub."
