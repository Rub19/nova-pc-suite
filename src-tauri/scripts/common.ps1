$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
$WarningPreference = 'SilentlyContinue'
[Console]::OutputEncoding = New-Object Text.UTF8Encoding($false)
$P = [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String($PayloadBase64)) | ConvertFrom-Json

function Test-Admin {
    $id = [Security.Principal.WindowsIdentity]::GetCurrent()
    try { (New-Object Security.Principal.WindowsPrincipal($id)).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator) }
    finally { $id.Dispose() }
}

function Require-Admin { if (-not (Test-Admin)) { throw 'Cette action demande les droits administrateur.' } }

function Sys32([string]$Name) { Join-Path $env:SystemRoot "System32\$Name" }

function Invoke-Native([string]$Name, [string[]]$Arguments, [switch]$Utf16) {
    $previous = [Console]::OutputEncoding
    if ($Utf16) { [Console]::OutputEncoding = [Text.Encoding]::Unicode }
    $ErrorActionPreference = 'Continue'
    try {
        $text = (& (Sys32 $Name) @Arguments 2>&1 | ForEach-Object { [string]$_ }) -join "`n"
        $code = $LASTEXITCODE
    } finally {
        [Console]::OutputEncoding = $previous
        $ErrorActionPreference = 'Stop'
    }
    $text = [regex]::Replace($text, '[\x00-\x08\x0B\x0C\x0E-\x1F\x7F]', '')
    $lines = @($text -split "\r?\n" | ForEach-Object { $_.Trim() } | Where-Object { $_ -and $_ -notmatch '^\[?=*\s*\d+(\.\d+)?%' })
    @{ exitCode = $code; text = $text; lines = $lines; tail = @($lines | Select-Object -Last 12) }
}

function Get-FreeBytes([string]$Drive = $env:SystemDrive) {
    [double](Get-CimInstance Win32_LogicalDisk -Filter "DeviceID='$Drive'").FreeSpace
}

function Get-DeviceProblems {
    @(Get-CimInstance Win32_PnPEntity -Property Name,Manufacturer,PNPClass,ConfigManagerErrorCode |
        Where-Object { $_.ConfigManagerErrorCode -ne 0 -and $_.ConfigManagerErrorCode -ne 22 })
}

function Ensure-RestorePoint([string]$Description) {
    Require-Admin
    $latest = @(Get-ComputerRestorePoint | Sort-Object SequenceNumber -Descending | Select-Object -First 1)
    if ($latest.Count -and [Management.ManagementDateTimeConverter]::ToDateTime($latest[0].CreationTime) -gt (Get-Date).AddHours(-24)) {
        return @{ created = $false; description = [string]$latest[0].Description; date = [Management.ManagementDateTimeConverter]::ToDateTime($latest[0].CreationTime).ToString('o') }
    }
    try { Checkpoint-Computer -Description $Description -RestorePointType MODIFY_SETTINGS }
    catch { throw "Point de restauration impossible : activez la protection du système dans Réparer, puis réessayez. ($($_.Exception.Message))" }
    $now = @(Get-ComputerRestorePoint | Sort-Object SequenceNumber -Descending | Select-Object -First 1)
    if (-not $now.Count -or $now[0].Description -ne $Description) { throw 'Windows n''a pas créé le point de restauration. Rien n''a été modifié.' }
    @{ created = $true; description = $Description; date = (Get-Date).ToString('o') }
}
