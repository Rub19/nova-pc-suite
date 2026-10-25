[CmdletBinding()]
param(
    [ValidateSet('apps-list','app-upgrades','app-upgrade','startup-list','startup-disable','startup-enable')]
    [string]$Action = 'apps-list',
    [string]$PayloadBase64 = '',
    # Trusted Rust argument; never taken from the JSON payload.
    [string]$DataRoot
)

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
[Console]::OutputEncoding = New-Object Text.UTF8Encoding($false)

function Read-AppsPayload([string]$Operation, [string]$Encoded) {
    if ($Encoded.Length -gt 8192) { throw 'PayloadTooLarge' }
    try {
        $text = '{}'
        if ($Encoded) {
            $bytes = [Convert]::FromBase64String($Encoded)
            if ($bytes.Length -gt 4096) { throw 'InvalidPayload' }
            $text = (New-Object Text.UTF8Encoding($false, $true)).GetString($bytes)
        }
        if (-not $text.Trim().StartsWith('{')) { throw 'InvalidPayload' }
        $payload = ConvertFrom-Json -InputObject $text
        if ($null -eq $payload -or $payload -isnot [pscustomobject]) { throw 'InvalidPayload' }
    } catch { throw 'InvalidPayload' }
    $needsId = $Operation -in @('app-upgrade','startup-disable','startup-enable')
    $names = @($payload.PSObject.Properties | ForEach-Object { $_.Name })
    if ($needsId) {
        if ($names.Count -ne 1 -or $names[0] -cne 'id') { throw 'InvalidPayload' }
        $pattern = if ($Operation -eq 'app-upgrade') { '^[A-Za-z0-9][A-Za-z0-9._-]{1,127}$' } else { '^[a-f0-9]{64}$' }
        if ($payload.id -isnot [string] -or $payload.id -cnotmatch $pattern) { throw 'InvalidId' }
    } elseif ($names.Count) { throw 'InvalidPayload' }
    return $payload
}

function Convert-SafeText($Value) {
    $s = [string]$Value
    $s = [regex]::Replace($s, '\x1B\][^\x07\x1B]*(?:\x07|\x1B\\)', '')
    $s = [regex]::Replace($s, '\x1B\[[0-?]*[ -/]*[@-~]', '')
    $s = [regex]::Replace($s, '[\x00-\x08\x0B\x0C\x0E-\x1F\x7F\u202A-\u202E\u2066-\u2069]', '')
    return $s.Substring(0, [Math]::Min($s.Length, 65536))
}

function Get-InstalledApps {
    foreach ($hive in @([Microsoft.Win32.RegistryHive]::LocalMachine, [Microsoft.Win32.RegistryHive]::CurrentUser)) {
        foreach ($view in @([Microsoft.Win32.RegistryView]::Registry64, [Microsoft.Win32.RegistryView]::Registry32)) {
            $base = $null; $uninstall = $null
            try {
                $base = [Microsoft.Win32.RegistryKey]::OpenBaseKey($hive, $view)
                $uninstall = $base.OpenSubKey('Software\Microsoft\Windows\CurrentVersion\Uninstall', $false)
                if ($null -eq $uninstall) { continue }
                foreach ($name in $uninstall.GetSubKeyNames()) {
                    $key = $null
                    try {
                        $key = $uninstall.OpenSubKey($name, $false)
                        if ($null -eq $key) { continue }
                        $display = $key.GetValue('DisplayName', '', [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
                        if ($display -is [string] -and $display.Trim()) {
                            @{ name = Convert-SafeText $display
                               version = Convert-SafeText ($key.GetValue('DisplayVersion', '', 1))
                               publisher = Convert-SafeText ($key.GetValue('Publisher', '', 1)) }
                        }
                    } finally { if ($key) { $key.Dispose() } }
                }
            } finally { if ($uninstall) { $uninstall.Dispose() }; if ($base) { $base.Dispose() } }
        }
    }
}

function Invoke-Winget([string[]]$Arguments, [switch]$Mutating) {
    # Fixed per-user App Installer alias; do not search PATH/current directory.
    $exe = Join-Path ([Environment]::GetFolderPath('LocalApplicationData')) 'Microsoft\WindowsApps\winget.exe'
    if (-not [IO.File]::Exists($exe)) { return @{ available=$false; output=''; exitCode=$null; truncated=$false } }
    foreach ($arg in $Arguments) {
        if ($arg -cnotmatch '^[A-Za-z0-9._-]+$') { throw 'InvalidWingetArgument' }
    }
    $info = New-Object Diagnostics.ProcessStartInfo
    $info.FileName = $exe
    $info.Arguments = $Arguments -join ' '
    $info.UseShellExecute = $false
    $info.CreateNoWindow = $true
    $info.RedirectStandardOutput = $true
    $info.RedirectStandardError = $true
    $info.RedirectStandardInput = $true
    $process = New-Object Diagnostics.Process
    $process.StartInfo = $info
    try {
        if (-not $process.Start()) { throw 'WingetStartFailed' }
        $process.StandardInput.Close()
        # Drain both streams concurrently but retain at most 64 KiB per stream.
        # ReadToEndAsync would allow a hostile installer to exhaust backend memory.
        $readers = @($process.StandardOutput, $process.StandardError)
        $buffers = @((New-Object char[] 4096), (New-Object char[] 4096))
        $texts = @((New-Object Text.StringBuilder), (New-Object Text.StringBuilder))
        $pending = @($readers[0].ReadAsync($buffers[0], 0, 4096), $readers[1].ReadAsync($buffers[1], 0, 4096))
        $done = @($false, $false)
        $truncated = $false
        $clock = [Diagnostics.Stopwatch]::StartNew()
        while (-not ($done[0] -and $done[1] -and $process.HasExited)) {
            # A silent upgrade can outlive a killed winget controller. Keep this
            # controller and the caller's mutex alive until mutation completes.
            # Only read-only queries may be terminated on a time limit.
            if (-not $Mutating -and $clock.ElapsedMilliseconds -gt 120000) {
                if (-not $process.HasExited) { $process.Kill() }
                throw 'WingetTimeout'
            }
            for ($i = 0; $i -lt 2; $i++) {
                if (-not $done[$i] -and $pending[$i].IsCompleted) {
                    $count = $pending[$i].GetAwaiter().GetResult()
                    if ($count -eq 0) { $done[$i] = $true; continue }
                    $keep = [Math]::Min($count, 65536 - $texts[$i].Length)
                    if ($keep -lt $count) { $truncated = $true }
                    if ($keep -gt 0) { $null = $texts[$i].Append($buffers[$i], 0, $keep) }
                    $pending[$i] = $readers[$i].ReadAsync($buffers[$i], 0, 4096)
                }
            }
            Start-Sleep -Milliseconds 20
        }
        $output = $texts[0].ToString() + "`n" + $texts[1].ToString()
        return @{ available=$true; output=Convert-SafeText $output; exitCode=$process.ExitCode; truncated=($truncated -or $output.Length -gt 65536) }
    } finally { $process.Dispose() }
}

function Invoke-AppUpgrade([string]$Id) {
    if ($Id -cnotmatch '^[A-Za-z0-9][A-Za-z0-9._-]{1,127}$') { throw 'InvalidId' }
    $installed = Invoke-Winget @('list','--id',$Id,'--exact','--disable-interactivity')
    if (-not $installed.available) { return $installed }
    if ($installed.exitCode -ne 0) { throw 'PackageNotInstalledOrQueryFailed' }
    return Invoke-Winget -Arguments @('upgrade','--id',$Id,'--exact','--silent','--disable-interactivity') -Mutating
}

function Get-StartupId([string]$Name) {
    $hash = [Security.Cryptography.SHA256]::Create()
    try { return ([BitConverter]::ToString($hash.ComputeHash([Text.Encoding]::UTF8.GetBytes($Name)))).Replace('-', '').ToLowerInvariant() }
    finally { $hash.Dispose() }
}

function Test-ProtectedStartup([string]$Name, [string]$Value) {
    return ($Name + ' ' + $Value) -match '(?i)defender|securityhealth|antivirus|anti[- ]?malware|msmpeng|sense|wdav|smartscreen|crowdstrike|sentinel|sophos|kaspersky|bitdefender|avast|avg|mcafee|norton|symantec|eset|malwarebytes|trend.?micro|cylance|carbon.?black|forti|webroot|f-secure|endpoint.?protect'
}

function Open-UserRun([bool]$Writable) {
    return [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey('Software\Microsoft\Windows\CurrentVersion\Run', $Writable)
}

function Assert-SnapshotRecord($Record, [string]$Id) {
    if ($null -eq $Record -or $Record.version -ne 1 -or $Record.id -cne $Id -or
        $Record.name -isnot [string] -or $Record.name.Length -lt 1 -or $Record.name.Length -gt 16383 -or
        $Record.name.Contains([char]0) -or (Get-StartupId $Record.name) -cne $Id -or
        $Record.kind -cnotin @('String','ExpandString') -or $Record.value -isnot [string] -or
        $Record.value.Length -gt 32767 -or (Test-ProtectedStartup $Record.name $Record.value)) { throw 'InvalidBackup' }
}

function Assert-NoReparse([string]$Path) {
    $item = Get-Item -LiteralPath $Path -Force
    while ($null -ne $item) {
        if (($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { throw 'UnsafeBackupPath' }
        if ($item -is [IO.FileInfo]) { $item = $item.Directory } else { $item = $item.Parent }
    }
}

function Get-BackupDirectory([string]$Root) {
    if ([string]::IsNullOrWhiteSpace($Root) -or -not [IO.Path]::IsPathRooted($Root) -or $Root.StartsWith('\\')) { throw 'InvalidDataRoot' }
    $full = [IO.Path]::GetFullPath($Root)
    # Require the Rust-created root; never create or widen an arbitrary parent.
    if (-not [IO.Directory]::Exists($full)) { throw 'InvalidDataRoot' }
    Assert-NoReparse $full
    $dir = Join-Path $full 'startup-backups'
    if (-not [IO.Directory]::Exists($dir)) { $null = [IO.Directory]::CreateDirectory($dir) }
    Assert-NoReparse $dir
    $sid = [Security.Principal.WindowsIdentity]::GetCurrent().User
    $acl = New-Object Security.AccessControl.DirectorySecurity
    $acl.SetAccessRuleProtection($true, $false)
    $acl.SetOwner($sid)
    $rule = New-Object Security.AccessControl.FileSystemAccessRule($sid, 'FullControl', 'ContainerInherit,ObjectInherit', 'None', 'Allow')
    $acl.AddAccessRule($rule)
    Set-Acl -LiteralPath $dir -AclObject $acl
    return $dir
}

function Read-StartupSnapshot([string]$Root, [string]$Id) {
    if ($Id -cnotmatch '^[a-f0-9]{64}$') { throw 'InvalidId' }
    $dir = Get-BackupDirectory $Root
    $path = Join-Path $dir ($Id + '.json')
    if (-not [IO.File]::Exists($path)) { throw 'BackupNotFound' }
    Assert-NoReparse $path
    if ((Get-Item -LiteralPath $path).Length -gt 262144) { throw 'InvalidBackup' }
    try {
        Add-Type -AssemblyName System.Security
        $envelope = ConvertFrom-Json -InputObject ([IO.File]::ReadAllText($path))
        if ($envelope.version -ne 1 -or $envelope.protectedData -isnot [string]) { throw 'InvalidBackup' }
        $cipher = [Convert]::FromBase64String($envelope.protectedData)
        $plain = [Security.Cryptography.ProtectedData]::Unprotect($cipher, [Text.Encoding]::UTF8.GetBytes('Nova.Startup.v1'), [Security.Cryptography.DataProtectionScope]::CurrentUser)
        $record = ConvertFrom-Json -InputObject ([Text.Encoding]::UTF8.GetString($plain))
    } catch { throw 'InvalidBackup' }
    Assert-SnapshotRecord $record $Id
    return $record
}

function Write-StartupSnapshot([string]$Root, [string]$Id, $Record) {
    Assert-SnapshotRecord $Record $Id
    $dir = Get-BackupDirectory $Root
    Add-Type -AssemblyName System.Security
    $plain = [Text.Encoding]::UTF8.GetBytes(($Record | ConvertTo-Json -Compress))
    $cipher = [Security.Cryptography.ProtectedData]::Protect($plain, [Text.Encoding]::UTF8.GetBytes('Nova.Startup.v1'), [Security.Cryptography.DataProtectionScope]::CurrentUser)
    $envelope = @{version=1;protectedData=[Convert]::ToBase64String($cipher)} | ConvertTo-Json -Compress
    $bytes = [Text.Encoding]::UTF8.GetBytes($envelope)
    # CreateNew is atomic and never follows/overwrites an existing snapshot.
    $stream = [IO.File]::Open((Join-Path $dir ($Id + '.json')), [IO.FileMode]::CreateNew, [IO.FileAccess]::Write, [IO.FileShare]::None)
    try { $stream.Write($bytes, 0, $bytes.Length); $stream.Flush($true) } finally { $stream.Dispose() }
}

function Set-StartupEntry([string]$Root, [string]$Id, [bool]$Enable) {
    if ($Id -cnotmatch '^[a-f0-9]{64}$') { throw 'InvalidId' }
    $key = Open-UserRun $true
    if ($null -eq $key) { throw 'StartupKeyUnavailable' }
    try {
        if ($Enable) {
            $record = Read-StartupSnapshot $Root $Id
            Assert-SnapshotRecord $record $Id
            # The backend mutex serializes our writers. Windows provides no atomic
            # create-if-absent for values; unrelated registry writers can still race.
            if (@($key.GetValueNames()) -contains $record.name) { throw 'StartupConflict' }
            $key.SetValue($record.name, $record.value, [Microsoft.Win32.RegistryValueKind]::$($record.kind))
        } else {
            $names = @($key.GetValueNames() | Where-Object { (Get-StartupId $_) -ceq $Id })
            if ($names.Count -ne 1) { throw 'StartupNotFound' }
            $name = $names[0]
            $value = $key.GetValue($name, $null, [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
            $kind = [string]$key.GetValueKind($name)
            $record = [pscustomobject]@{version=1;id=$Id;name=$name;value=$value;kind=$kind}
            Assert-SnapshotRecord $record $Id
            # Immutable snapshots remain after restore; only identical subsequent disables are allowed.
            try { Write-StartupSnapshot $Root $Id $record }
            catch [IO.IOException] {
                $saved = Read-StartupSnapshot $Root $Id
                if ($saved.name -cne $name -or $saved.value -cne $value -or $saved.kind -cne $kind) { throw 'StartupBackupConflict' }
            }
            if ($key.GetValue($name, $null, 1) -cne $value -or [string]$key.GetValueKind($name) -cne $kind) { throw 'StartupChanged' }
            $key.DeleteValue($name, $true)
        }
        return @{ id=$Id; enabled=$Enable; source='HKCU.Run' }
    } finally { $key.Dispose() }
}

function Get-StartupEntries([string]$Root) {
    $items = @(); $active = @{}
    $key = Open-UserRun $false
    try {
        if ($key) {
            foreach ($name in $key.GetValueNames()) {
                $value = $key.GetValue($name, $null, 1)
                $id = Get-StartupId $name
                $active[$id] = $true
                $supported = $value -is [string] -and [string]$key.GetValueKind($name) -in @('String','ExpandString') -and -not (Test-ProtectedStartup $name ([string]$value))
                $items += @{id=$id;name=Convert-SafeText $name;source='HKCU.Run';enabled=$true;supported=[bool]$supported;manageable=[bool]$supported}
            }
        }
    } finally { if ($key) { $key.Dispose() } }
    if ($Root) {
        $dir = Get-BackupDirectory $Root
        foreach ($file in Get-ChildItem -LiteralPath $dir -File) {
            if ($file.Name -cmatch '^([a-f0-9]{64})\.json$') {
                $id = $Matches[1]
                $saved = Read-StartupSnapshot $Root $id
                if (-not $active.ContainsKey($id)) {
                    $items += @{id=$id;name=Convert-SafeText $saved.name;source='HKCU.Run';enabled=$false;supported=$true;manageable=$true}
                }
            }
        }
    }
    $warnings = @()
    try {
        foreach ($entry in Get-CimInstance -ClassName Win32_StartupCommand -ErrorAction Stop) {
            # CIM includes Run entries; report other locations without exposing command strings.
            if ($entry.Location -match '(?i)^HKU\\[^\\]+\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run$' -and
                $entry.UserSID -eq [Security.Principal.WindowsIdentity]::GetCurrent().User.Value) { continue }
            $items += @{id=$null;name=Convert-SafeText $entry.Name;source=Convert-SafeText $entry.Location;enabled=$null;supported=$false;manageable=$false}
        }
    } catch { $warnings += 'OtherStartupSourcesUnavailable' }
    return @{items=@($items);warnings=$warnings;scope='HKCU.Run only; startup folders and other sources are read-only; StartupApproved is not modified'}
}

$mutex = $null; $locked = $false
$data = $null; $ok = $false; $errorCode = $null
try {
    $payload = Read-AppsPayload $Action $PayloadBase64
    if ($Action -in @('app-upgrade','startup-disable','startup-enable','startup-list')) {
        $sid = [Security.Principal.WindowsIdentity]::GetCurrent().User.Value
        # Global namespace coordinates this user's processes across sessions. Mutexes need no administrator privilege.
        try {
            $mutex = New-Object Threading.Mutex($false, ('Global\NovaPCSuite.Apps.' + $sid))
            try { $locked = $mutex.WaitOne(0) }
            catch [Threading.AbandonedMutexException] { $locked = $true; throw 'PreviousMaintenanceInterrupted' }
        } catch [UnauthorizedAccessException] { throw 'OperationLockUnavailable' }
        if (-not $locked) { throw 'OperationInProgress' }
    }
    switch ($Action) {
        'apps-list' { $data = @{items=@(Get-InstalledApps | Sort-Object name,version,publisher -Unique)} }
        'app-upgrades' { $data = Invoke-Winget @('upgrade','--disable-interactivity') }
        'app-upgrade' { $data = Invoke-AppUpgrade $payload.id }
        'startup-list' { $data = Get-StartupEntries $DataRoot }
        'startup-disable' { $data = Set-StartupEntry $DataRoot $payload.id $false }
        'startup-enable' { $data = Set-StartupEntry $DataRoot $payload.id $true }
    }
    $ok = $true
    if ($Action -in @('app-upgrades','app-upgrade')) {
        if (-not $data.available) { $ok=$false; $errorCode='WingetUnavailable' }
        elseif ($data.exitCode -ne 0) { $ok=$false; $errorCode='WingetOperationFailed' }
    }
} catch {
    $message = $_.Exception.Message
    $allowed = @('PayloadTooLarge','InvalidPayload','InvalidId','InvalidWingetArgument','WingetStartFailed','WingetTimeout','PackageNotInstalledOrQueryFailed','InvalidBackup','UnsafeBackupPath','InvalidDataRoot','BackupNotFound','StartupKeyUnavailable','StartupConflict','StartupNotFound','StartupBackupConflict','StartupChanged','PreviousMaintenanceInterrupted','OperationLockUnavailable','OperationInProgress')
    $errorCode = if ($message -cin $allowed) { $message } else { 'AppsOperationFailed' }
} finally {
    if ($mutex) { if ($locked) { $mutex.ReleaseMutex() }; $mutex.Dispose() }
}
@{ok=$ok;data=$data;error=$errorCode} | ConvertTo-Json -Depth 8 -Compress
