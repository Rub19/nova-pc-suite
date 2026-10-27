[CmdletBinding()]
param(
    [ValidateSet('snapshot','drivers','driver-audit','updates','install-drivers','startup','security','storage','processes','restore','repair','trim','energy-balanced','energy-performance')]
    [string]$Action = 'snapshot',
    [string[]]$ApprovedIds = @(),
    [switch]$AcceptEulas
)

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
$WarningPreference = 'SilentlyContinue'
$InformationPreference = 'SilentlyContinue'
[Console]::OutputEncoding = New-Object System.Text.UTF8Encoding($false)

function Test-Administrator {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    try {
        $principal = New-Object Security.Principal.WindowsPrincipal($identity)
        return $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
    } finally { $identity.Dispose() }
}

function Get-DriverUpdates {
    $session = New-Object -ComObject Microsoft.Update.Session
    $session.ClientApplicationID = 'Nova PC Suite'
    $searcher = $session.CreateUpdateSearcher()
    $search = $searcher.Search("IsInstalled=0 and IsHidden=0 and Type='Driver'")
    if ([int]$search.ResultCode -ne 2) { throw 'DriverSearchIncomplete' }
    return ,$search.Updates
}

function Convert-Update($Update) {
    return [ordered]@{
        id = [string]$Update.Identity.UpdateID
        title = [string]$Update.Title
        eulaAccepted = [bool]$Update.EulaAccepted
        eulaText = $(if (-not $Update.EulaAccepted) { [string]$Update.EulaText } else { $null })
        downloaded = [bool]$Update.IsDownloaded
        rebootRequired = [bool]$Update.RebootRequired
    }
}

$data = $null
$ok = $true
$errorCode = $null
$maintenanceMutex = $null
$ownsMaintenanceMutex = $false
try {
    if ($Action -eq 'install-drivers') {
        if ($null -eq $ApprovedIds -or $ApprovedIds.Count -eq 0) { throw 'ApprovedIdsRequired' }
        if ($ApprovedIds.Count -gt 200) { throw 'TooManyApprovedIds' }
        foreach ($approvedId in $ApprovedIds) {
            if ($approvedId -notmatch '^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$') {
                throw 'InvalidApprovedId'
            }
        }
    }
    $mutations = @('install-drivers','restore','repair','trim','energy-balanced','energy-performance')
    if ($mutations -contains $Action) {
        if (-not (Test-Administrator)) { throw 'AdministratorRequired' }
        # The lock belongs to PowerShell, so it survives the Nova window closing
        # and remains held while DISM/SFC or Windows Update finish their work.
        $maintenanceMutex = New-Object System.Threading.Mutex($false, 'Global\NovaPCSuite.Maintenance')
        try { $ownsMaintenanceMutex = $maintenanceMutex.WaitOne(0) }
        catch [System.Threading.AbandonedMutexException] {
            $ownsMaintenanceMutex = $true
            throw 'PreviousMaintenanceInterrupted'
        }
        if (-not $ownsMaintenanceMutex) { throw 'MaintenanceAlreadyRunning' }
    }
    switch ($Action) {
        'snapshot' {
            $os = Get-CimInstance Win32_OperatingSystem
            $cpus = @(Get-CimInstance Win32_Processor)
            $hardware = Get-CimInstance Win32_ComputerSystem -Property Manufacturer,Model
            $uptimeSeconds = $null
            if ($null -ne $os.LastBootUpTime) {
                $uptimeSeconds = [math]::Max(0, [math]::Floor(((Get-Date) - $os.LastBootUpTime).TotalSeconds))
            }
            $pendingReboot = (Test-Path -LiteralPath 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Component Based Servicing\RebootPending') -or
                (Test-Path -LiteralPath 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\WindowsUpdate\Auto Update\RebootRequired')
            $sessionManager = Get-ItemProperty -LiteralPath 'HKLM:\SYSTEM\CurrentControlSet\Control\Session Manager' -Name PendingFileRenameOperations -ErrorAction SilentlyContinue
            if ($null -ne $sessionManager -and @($sessionManager.PendingFileRenameOperations).Count -gt 0) { $pendingReboot = $true }
            $data = [ordered]@{
                os = @{ caption = $os.Caption; version = $os.Version; build = $os.BuildNumber; architecture = $os.OSArchitecture }
                cpu = @($cpus | ForEach-Object { @{ name = $_.Name; cores = $_.NumberOfCores; logicalProcessors = $_.NumberOfLogicalProcessors; loadPercent = $_.LoadPercentage } })
                memory = @{ totalBytes = ([double]$os.TotalVisibleMemorySize * 1024); freeBytes = ([double]$os.FreePhysicalMemory * 1024) }
                isAdmin = Test-Administrator
                uptimeSeconds = $uptimeSeconds
                pendingReboot = [bool]$pendingReboot
                hardware = @{ vendor = $hardware.Manufacturer; model = $hardware.Model }
            }
        }
        'drivers' {
            $data = @{ items = @(Get-CimInstance Win32_PnPSignedDriver | ForEach-Object {
                @{ name = $_.DeviceName; manufacturer = $_.Manufacturer; version = $_.DriverVersion; signed = $_.IsSigned; date = $(if ($null -ne $_.DriverDate) { $_.DriverDate.ToString('yyyy-MM-dd') } else { $null }) }
            }) }
        }
        'driver-audit' {
            # Read-only inventory. ConfigManagerErrorCode is documented by Microsoft as
            # the PnP Configuration Manager status; hardware IDs are deliberately omitted.
            # https://learn.microsoft.com/windows/win32/cimwin32prov/win32-pnpentity
            $devices = @(Get-CimInstance Win32_PnPEntity -Property Name,Manufacturer,PNPClass,Status,ConfigManagerErrorCode | ForEach-Object {
                @{ name = $_.Name; manufacturer = $_.Manufacturer; pnpClass = $_.PNPClass; status = $_.Status; errorCode = [int]$_.ConfigManagerErrorCode }
            })
            # Win32_PnPSignedDriver exposes signature metadata. A missing signature is
            # reported for review only; Nova never blocks or replaces a driver from it.
            # https://learn.microsoft.com/windows/win32/cimwin32prov/win32-pnpsigneddriver
            $signed = @(Get-CimInstance Win32_PnPSignedDriver -Property IsSigned | Where-Object { $_.IsSigned -eq $true }).Count
            $unsigned = @(Get-CimInstance Win32_PnPSignedDriver -Property IsSigned | Where-Object { $_.IsSigned -eq $false }).Count
            $problems = @($devices | Where-Object { $_.errorCode -ne 0 } | Select-Object -First 50)
            $missing = @($devices | Where-Object { $_.errorCode -eq 28 } | Select-Object -First 50)
            $categories = @($devices | Group-Object { switch ($_.pnpClass) {
                'Display' { 'Graphique'; break }
                'Monitor' { 'Graphique'; break }
                'Net' { 'Réseau'; break }
                'Bluetooth' { 'Réseau'; break }
                'Media' { 'Audio'; break }
                'AudioEndpoint' { 'Audio'; break }
                'DiskDrive' { 'Stockage'; break }
                'SCSIAdapter' { 'Stockage'; break }
                'HIDClass' { 'Périphérique'; break }
                'System' { 'Chipset'; break }
                default { 'Périphérique' }
            }} | ForEach-Object { @{ category = $_.Name; count = $_.Count } })
            $critical = @($problems | Where-Object { $_.pnpClass -in @('Display','Net','DiskDrive','SCSIAdapter','System') } | Select-Object -First 25)
            $data = @{
                deviceCount = $devices.Count
                signedDriverCount = $signed
                unsignedDriverCount = $unsigned
                problemDevices = $problems
                missingDriverDevices = $missing
                criticalProblemDevices = $critical
                categories = $categories
                hasProblems = [bool]($problems.Count -gt 0)
            }
        }
        'startup' {
            # Commands, locations and user identities are deliberately omitted.
            $data = @{ items = @(Get-CimInstance Win32_StartupCommand | ForEach-Object { @{ name = $_.Name } }) }
        }
        'security' {
            $antivirus = @()
            $firewall = @()
            $issues = @()
            try {
                $antivirus = @(Get-CimInstance -Namespace root/SecurityCenter2 -ClassName AntiVirusProduct | ForEach-Object {
                    @{ name = $_.displayName; productState = $_.productState }
                })
            } catch { $issues += 'AntivirusInventoryUnavailable' }
            try {
                $firewall = @(Get-CimInstance -Namespace root/StandardCimv2 -ClassName MSFT_NetFirewallProfile | ForEach-Object {
                    @{ name = $_.Name; enabled = $_.Enabled }
                })
            } catch { $issues += 'FirewallInventoryUnavailable' }
            $data = @{ antivirus = $antivirus; firewall = $firewall; issues = $issues }
        }
        'storage' {
            $data = @{ volumes = @(Get-CimInstance Win32_LogicalDisk -Filter 'DriveType=3' | ForEach-Object {
                @{ drive = $_.DeviceID; fileSystem = $_.FileSystem; sizeBytes = $_.Size; freeBytes = $_.FreeSpace }
            }) }
        }
        'processes' {
            $data = @{ items = @(Get-CimInstance Win32_Process | ForEach-Object {
                @{ name = $_.Name; pid = $_.ProcessId; workingSetBytes = $_.WorkingSetSize; threads = $_.ThreadCount }
            }) }
        }
        'updates' {
            $updates = Get-DriverUpdates
            $items = @()
            for ($i = 0; $i -lt $updates.Count; $i++) { $items += Convert-Update $updates.Item($i) }
            $data = @{ items = $items; count = $updates.Count }
        }
        'install-drivers' {
            $available = Get-DriverUpdates
            $updates = New-Object -ComObject Microsoft.Update.UpdateColl
            foreach ($approvedId in @($ApprovedIds | Select-Object -Unique)) {
                $match = $null
                for ($i = 0; $i -lt $available.Count; $i++) {
                    if ([string]$available.Item($i).Identity.UpdateID -eq $approvedId) {
                        $match = $available.Item($i)
                        break
                    }
                }
                if ($null -eq $match) { throw 'ApprovedDriverUnavailable' }
                $null = $updates.Add($match)
            }
            $items = @()
            $unaccepted = $false
            for ($i = 0; $i -lt $updates.Count; $i++) {
                $items += Convert-Update $updates.Item($i)
                if (-not $updates.Item($i).EulaAccepted) { $unaccepted = $true }
            }
            $data = @{ items = $items; results = @(); rebootRequired = $false }
            if ($unaccepted -and -not $AcceptEulas) { throw 'UnacceptedDriverEula' }
            if ($AcceptEulas) {
                # This collection contains only the explicitly approved IDs.
                for ($i = 0; $i -lt $updates.Count; $i++) {
                    if (-not $updates.Item($i).EulaAccepted) { $updates.Item($i).AcceptEula() }
                }
            }
            if ($updates.Count -eq 0) { break }
            $session = New-Object -ComObject Microsoft.Update.Session
            $session.ClientApplicationID = 'Nova PC Suite'
            $downloader = $session.CreateUpdateDownloader()
            $downloader.Updates = $updates
            $downloader.Priority = 2
            $download = $downloader.Download()
            $downloadResults = @()
            $allDownloaded = $true
            for ($i = 0; $i -lt $updates.Count; $i++) {
                $itemResult = $download.GetUpdateResult($i)
                $downloadResults += @{ id = $updates.Item($i).Identity.UpdateID; resultCode = [int]$itemResult.ResultCode; hresult = [int]$itemResult.HResult }
                if (-not $updates.Item($i).IsDownloaded) { $allDownloaded = $false }
            }
            $data.downloadResults = $downloadResults
            $data.downloadResultCode = [int]$download.ResultCode
            if (-not $allDownloaded) { throw 'DriverDownloadFailed' }
            $installer = $session.CreateUpdateInstaller()
            $installer.Updates = $updates
            $installer.AllowSourcePrompts = $false
            $installer.ForceQuiet = $true
            if ($installer.RebootRequiredBeforeInstallation) { throw 'RebootRequiredBeforeInstallation' }
            $installed = $installer.Install()
            $results = @()
            for ($i = 0; $i -lt $updates.Count; $i++) {
                $itemResult = $installed.GetUpdateResult($i)
                $results += @{ id = $updates.Item($i).Identity.UpdateID; resultCode = [int]$itemResult.ResultCode; hresult = [int]$itemResult.HResult; rebootRequired = [bool]$itemResult.RebootRequired }
            }
            $data.results = $results
            $data.resultCode = [int]$installed.ResultCode
            $data.rebootRequired = [bool]$installed.RebootRequired
            if ([int]$installed.ResultCode -ne 2) { throw 'DriverInstallationIncomplete' }
        }
        'restore' {
            $description = 'Nova PC Suite restore point'
            $previousMax = -1L
            foreach ($point in @(Get-ComputerRestorePoint -ErrorAction Stop)) {
                if ([long]$point.SequenceNumber -gt $previousMax) { $previousMax = [long]$point.SequenceNumber }
            }
            Checkpoint-Computer -Description $description -RestorePointType MODIFY_SETTINGS -ErrorAction Stop
            $created = @(Get-ComputerRestorePoint -ErrorAction Stop | Where-Object {
                [long]$_.SequenceNumber -gt $previousMax -and $_.Description -eq $description
            } | Sort-Object SequenceNumber -Descending)
            if ($created.Count -eq 0) { throw 'RestorePointNotCreated' }
            $data = @{ requested = $true; created = $true; sequenceNumber = $created[0].SequenceNumber }
        }
        'repair' {
            # Capture native output; only structured exit codes cross the boundary.
            $null = & "$env:SystemRoot/System32/Dism.exe" /Online /Cleanup-Image /RestoreHealth /NoRestart 2>&1
            $dismExit = $LASTEXITCODE
            $null = & "$env:SystemRoot/System32/sfc.exe" /scannow 2>&1
            $sfcExit = $LASTEXITCODE
            $data = @{ dismExitCode = $dismExit; sfcExitCode = $sfcExit; rebootRequired = ($dismExit -eq 3010); sfcVerificationRequired = $true }
            # SFC's exit code alone does not prove that all corruption was repaired.
            if (($dismExit -notin @(0,3010)) -or $sfcExit -ne 0) { throw 'RepairCommandFailed' }
        }
        'trim' {
            $os = Get-CimInstance Win32_OperatingSystem
            $drive = [string]$os.SystemDrive
            if ($drive -notmatch '^[A-Za-z]:$') { throw 'UnsupportedSystemVolume' }
            $volume = Get-CimInstance Win32_LogicalDisk -Filter "DeviceID='$drive'"
            if ($null -eq $volume -or $volume.DriveType -ne 3 -or $volume.FileSystem -notin @('NTFS','ReFS')) { throw 'UnsupportedSystemVolume' }
            # Optimize-Volume rejects unsupported storage; never fall back to defrag.
            $null = Optimize-Volume -DriveLetter $drive.Substring(0,1) -ReTrim -ErrorAction Stop
            $data = @{ drive = $drive; retrimCompleted = $true }
        }
        { $_ -in @('energy-balanced','energy-performance') } {
            $guid = '381b4222-f694-41f0-9685-ff5bb260df2e'
            if ($Action -eq 'energy-performance') { $guid = '8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c' }
            $null = & "$env:SystemRoot/System32/powercfg.exe" /setactive $guid 2>&1
            $exitCode = $LASTEXITCODE
            $data = @{ schemeGuid = $guid; exitCode = $exitCode }
            if ($exitCode -ne 0) { throw 'PowerSchemeUnavailable' }
        }
    }
} catch {
    $ok = $false
    # Avoid leaking account names, paths or localized diagnostic text.
    $known = @('ApprovedIdsRequired','TooManyApprovedIds','InvalidApprovedId','ApprovedDriverUnavailable','RestorePointNotCreated','AdministratorRequired','DriverSearchIncomplete','UnacceptedDriverEula','DriverDownloadFailed','RebootRequiredBeforeInstallation','DriverInstallationIncomplete','RepairCommandFailed','UnsupportedSystemVolume','PowerSchemeUnavailable')
    $errorCode = 'ActionFailed'
    if ($known -contains $_.Exception.Message -or $_.Exception.Message -in @('PreviousMaintenanceInterrupted','MaintenanceAlreadyRunning')) { $errorCode = $_.Exception.Message }
} finally {
    if ($ownsMaintenanceMutex) { $maintenanceMutex.ReleaseMutex() }
    if ($null -ne $maintenanceMutex) { $maintenanceMutex.Dispose() }
}
$response = [ordered]@{ ok = $ok; data = $data }
if (-not $ok) { $response.error = $errorCode }
$response | ConvertTo-Json -Depth 10 -Compress
