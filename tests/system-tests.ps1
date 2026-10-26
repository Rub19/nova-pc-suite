$ErrorActionPreference = 'Stop'
$scriptPath = Join-Path $PSScriptRoot '../src-tauri/scripts/system.ps1'
$tokens = $null
$parseErrors = $null
$null = [System.Management.Automation.Language.Parser]::ParseFile($scriptPath, [ref]$tokens, [ref]$parseErrors)
if ($parseErrors.Count) { throw 'Backend PowerShell syntax is invalid.' }
$shapes = @{
    snapshot = @('os', 'cpu', 'memory', 'isAdmin')
    drivers = @('items')
    'driver-audit' = @('deviceCount', 'signedDriverCount', 'unsignedDriverCount', 'problemDevices')
    startup = @('items')
    security = @('antivirus', 'firewall')
    storage = @('volumes')
    processes = @('items')
}
foreach ($action in @('snapshot', 'drivers', 'driver-audit', 'startup', 'security', 'storage', 'processes')) {
    $raw = & "$PSHOME/powershell.exe" -NoProfile -NonInteractive -File $scriptPath -Action $action
    $result = ($raw -join "`n") | ConvertFrom-Json
    if ($result.ok -isnot [bool]) { throw "$action : invalid JSON envelope" }
    if (-not $result.ok) { throw "$action : inventory failed (details suppressed)" }
    foreach ($field in $shapes[$action]) {
        if ($result.data.PSObject.Properties.Name -notcontains $field) { throw "$action : missing $field" }
    }
    # Never print inventory, command lines, identities, device IDs or file paths.
    Write-Output "$action : PASS"
}
Write-Output 'Read-only host checks passed; no update search or mutations executed.'
