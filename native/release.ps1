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
(Resolve-Path "release\$name").Path
