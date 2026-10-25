[CmdletBinding()]
param([Parameter(Mandatory=$true)][ValidatePattern('^[a-fA-F0-9]{40}$')][string]$CertificateThumbprint,[Parameter(Mandatory=$true)][string]$File)
$ErrorActionPreference='Stop'
$projectRoot=Split-Path -Parent $PSScriptRoot
$target=[IO.Path]::GetFullPath($File)
$allowed=[IO.Path]::GetFullPath((Join-Path $projectRoot 'outputs'))+[IO.Path]::DirectorySeparatorChar
if(-not $target.StartsWith($allowed,[StringComparison]::OrdinalIgnoreCase) -or [IO.Path]::GetExtension($target) -ne '.exe' -or -not [IO.File]::Exists($target)){throw 'Only an existing outputs executable can be signed'}
$store=New-Object Security.Cryptography.X509Certificates.X509Store('My','CurrentUser')
try{
 $store.Open('ReadOnly')
 $certificate=@($store.Certificates|Where-Object{$_.Thumbprint -eq $CertificateThumbprint -and $_.HasPrivateKey -and $_.NotAfter -gt (Get-Date) -and $_.NotBefore -lt (Get-Date)})
 if($certificate.Count -ne 1){throw 'A valid code-signing certificate with private key is required in CurrentUser/My'}
 $codeSigning=$false
 foreach($extension in $certificate[0].Extensions){if($extension -is [Security.Cryptography.X509Certificates.X509EnhancedKeyUsageExtension]){foreach($oid in $extension.EnhancedKeyUsages){if($oid.Value -eq '1.3.6.1.5.5.7.3.3'){$codeSigning=$true}}}}
 if(-not $codeSigning){throw 'Certificate is not authorized for code signing'}
}finally{$store.Close()}
$sdkRoot=Join-Path ${env:ProgramFiles(x86)} 'Windows Kits/10/bin'
$tool=@(Get-ChildItem -LiteralPath $sdkRoot -Directory|Sort-Object Name -Descending|ForEach-Object{Join-Path $_.FullName 'x64/signtool.exe'}|Where-Object{Test-Path -LiteralPath $_ -PathType Leaf}|Select-Object -First 1)
if($tool.Count -ne 1){throw 'Windows SDK signtool.exe is required'}
& $tool[0] sign /sha1 $CertificateThumbprint /fd SHA256 /tr 'http://timestamp.digicert.com' /td SHA256 $target
if($LASTEXITCODE -ne 0){throw 'Signing failed'}
& $tool[0] verify /pa /v $target
if($LASTEXITCODE -ne 0){throw 'Signature verification failed'}
Write-Output 'Signature and trust verification succeeded. Regenerate release hashes after signing.'
