param([string]$Notes = "")
$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot
$build = Start-Process -FilePath "cargo" -ArgumentList "build", "--release", "-p", "nova-native", "-p", "nova-setup", "-j", "3" -NoNewWindow -Wait -PassThru
if ($build.ExitCode -ne 0) { throw "compilation" }
New-Item -ItemType Directory -Force "release" | Out-Null
$version = (Select-String -Path app\Cargo.toml -Pattern '^version = "(.*)"').Matches[0].Groups[1].Value
$name = "Nova-PC-Suite_${version}_x64-setup.exe"
$pack = Start-Process -FilePath "target\release\nova-setup.exe" -ArgumentList "--pack", "target\release\nova-native.exe", "release\$name" -Wait -PassThru
if ($pack.ExitCode -ne 0) { throw "paquet de l'installeur" }
"{0} : {1:N1} Mo" -f "nova-native.exe", ((Get-Item target\release\nova-native.exe).Length / 1MB)
"{0} : {1:N1} Mo" -f $name, ((Get-Item "release\$name").Length / 1MB)
$out = (Resolve-Path "release\$name").Path
$key = if ($env:NOVA_SIGN_KEY) { $env:NOVA_SIGN_KEY } else { Join-Path $HOME ".tauri\nova-pc-suite.key" }
if (-not (Test-Path $key) -or -not (Get-Content update.pub -Raw -ErrorAction SilentlyContinue)) {
    "Pas de clé de signature ($key) ou update.pub vide : installeur non signé, latest.json non écrit."
    $out
    exit 0
}
$updates = Join-Path $PSScriptRoot "release\updates"
New-Item -ItemType Directory -Force $updates | Out-Null
$target = Join-Path $updates $name
Copy-Item $out $target -Force
Remove-Item "$target.sig" -ErrorAction SilentlyContinue
cmd /c "cargo tauri signer sign -f `"$key`" -p `"`" `"$target`" >nul 2>&1"
if ($LASTEXITCODE -ne 0 -or -not (Test-Path "$target.sig")) { throw "signature de l'installeur" }
$latest = [ordered]@{
    version = $version
    notes = $Notes
    pub_date = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ss.fffZ")
    platforms = [ordered]@{
        "windows-x86_64" = [ordered]@{
            signature = (Get-Content "$target.sig" -Raw).Trim()
            url = "https://github.com/Rub19/nova-pc-suite/releases/download/v$version/$name"
        }
    }
}
[IO.File]::WriteAllText((Join-Path $updates "latest.json"), ($latest | ConvertTo-Json -Depth 5), (New-Object Text.UTF8Encoding $false))
"Signé : release\updates\$name et latest.json. À joindre à une version GitHub v$version publiée comme version normale (pas en préversion). Rien n'est publié."
$out
