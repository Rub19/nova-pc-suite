function Get-Boots([int]$Count = 8) {
    try {
        $items = @(Get-WinEvent -FilterHashtable @{ LogName = 'Microsoft-Windows-Diagnostics-Performance/Operational'; Id = 100 } -MaxEvents $Count | ForEach-Object {
            $xml = [xml]$_.ToXml()
            $ms = ($xml.Event.EventData.Data | Where-Object { $_.Name -eq 'BootTime' }).'#text'
            @{ date = $_.TimeCreated.ToUniversalTime().ToString('o'); ms = [double]$ms }
        })
        @{ available = $true; items = $items }
    } catch { @{ available = $false; items = @() } }
}

function Count-Events([string]$Log, [hashtable]$Filter) {
    try { @(Get-WinEvent -FilterHashtable (@{ LogName = $Log; StartTime = (Get-Date).AddDays(-30) } + $Filter) -MaxEvents 500) }
    catch { @() }
}

function Get-Stability {
    $power = Count-Events 'System' @{ ProviderName = 'Microsoft-Windows-Kernel-Power'; Id = 41 }
    $bug = Count-Events 'System' @{ ProviderName = 'Microsoft-Windows-WER-SystemErrorReporting'; Id = 1001 }
    $wer = Count-Events 'Application' @{ ProviderName = 'Windows Error Reporting'; Id = 1001 }
    $gpuCodes = @('117', '141', '142', '193', '1a1', '1a8', '1b8')
    $seen = @{}
    $gpu = @($wer | Where-Object {
        $v = @($_.Properties | ForEach-Object { [string]$_.Value })
        if ($v -notcontains 'LiveKernelEvent') { return $false }
        $code = ($v[$v.IndexOf('LiveKernelEvent') + 3]).ToLowerInvariant()
        if ($gpuCodes -notcontains $code) { return $false }
        $key = $code + $_.TimeCreated.ToString('yyyyMMddHHmm')
        if ($seen.ContainsKey($key)) { return $false }
        $seen[$key] = $code
        $true
    })
    $codes = @($seen.Values | Group-Object | Sort-Object Count -Descending | ForEach-Object { @{ code = $_.Name; count = $_.Count } })
    $apps = Count-Events 'Application' @{ ProviderName = 'Application Error'; Id = 1000 }
    $top = @($apps | ForEach-Object { [string]$_.Properties[0].Value } | Group-Object | Sort-Object Count -Descending | Select-Object -First 5 | ForEach-Object { @{ name = $_.Name; count = $_.Count } })
    $last = @($power + $bug + $gpu | Sort-Object TimeCreated -Descending | Select-Object -First 1)
    @{
        unexpectedShutdowns = $power.Count
        blueScreens = $bug.Count
        gpuHangs = $gpu.Count
        gpuCodes = $codes
        appCrashes = $apps.Count
        topCrashes = $top
        lastIncident = $(if ($last.Count) { $last[0].TimeCreated.ToUniversalTime().ToString('o') } else { $null })
    }
}

function Get-Disks {
    try {
        @(Get-PhysicalDisk | ForEach-Object {
            $r = $null
            try { $r = $_ | Get-StorageReliabilityCounter } catch { }
            @{
                name = [string]$_.FriendlyName; media = [string]$_.MediaType; bus = [string]$_.BusType
                sizeBytes = [double]$_.Size; health = [string]$_.HealthStatus
                temperature = $(if ($r) { $r.Temperature } else { $null })
                wear = $(if ($r) { $r.Wear } else { $null })
                errors = $(if ($r) { [double]$r.ReadErrorsUncorrected + [double]$r.WriteErrorsUncorrected } else { $null })
            }
        })
    } catch { @() }
}

function Get-Security {
    $defender = $null
    try {
        $s = Get-MpComputerStatus
        $defender = @{ running = [bool]$s.AMServiceEnabled; realtime = [bool]$s.RealTimeProtectionEnabled; signatureAgeDays = [int]$s.AntivirusSignatureAge }
    } catch { }
    $others = @()
    try { $others = @(Get-CimInstance -Namespace root/SecurityCenter2 -ClassName AntiVirusProduct | Where-Object { $_.displayName -notmatch 'Defender' } | ForEach-Object { [string]$_.displayName }) } catch { }
    $firewallOff = $null
    try { $firewallOff = @(Get-NetFirewallProfile | Where-Object { -not $_.Enabled } | ForEach-Object { [string]$_.Name }) } catch { }
    @{ defender = $defender; otherAntivirus = $others; firewallOff = $firewallOff }
}

function Get-LastUpdate {
    try {
        $date = (New-Object -ComObject Microsoft.Update.AutoUpdate).Results.LastInstallationSuccessDate
        if ($date) { return ([datetime]$date).ToUniversalTime().ToString('o') }
    } catch { }
    $null
}

function Invoke-Module([string]$Action) {
    switch ($Action) {
        'scan' {
            $os = Get-CimInstance Win32_OperatingSystem
            $cs = Get-CimInstance Win32_ComputerSystem
            $cpu = @(Get-CimInstance Win32_Processor)[0]
            $ram = @(Get-CimInstance Win32_PhysicalMemory)
            $bios = @(Get-CimInstance Win32_BIOS)[0]
            $board = @(Get-CimInstance Win32_BaseBoard)[0]
            $cv = Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion'
            $pending = (Test-Path 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Component Based Servicing\RebootPending') -or
                (Test-Path 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\WindowsUpdate\Auto Update\RebootRequired')
            @{
                isAdmin = Test-Admin
                timestamp = [datetime]::UtcNow.ToString('o')
                os = @{ name = [string]$os.Caption; release = [string]$cv.DisplayVersion; build = [string]$os.BuildNumber }
                computer = @{ manufacturer = [string]$cs.Manufacturer; model = [string]$cs.Model; board = "$($board.Manufacturer) $($board.Product)".Trim(); bios = [string]$bios.SMBIOSBIOSVersion }
                cpu = @{ name = ([string]$cpu.Name).Trim(); cores = [int]$cpu.NumberOfCores; threads = [int]$cpu.NumberOfLogicalProcessors }
                memory = @{ totalBytes = [double]$os.TotalVisibleMemorySize * 1024; freeBytes = [double]$os.FreePhysicalMemory * 1024; modules = $ram.Count; speedMHz = $(if ($ram.Count) { [int]$ram[0].ConfiguredClockSpeed } else { $null }) }
                gpus = @(Get-CimInstance Win32_VideoController | ForEach-Object { @{ name = [string]$_.Name; driverVersion = [string]$_.DriverVersion; driverDate = $(if ($_.DriverDate) { $_.DriverDate.ToString('yyyy-MM-dd') } else { $null }) } })
                uptimeSeconds = [math]::Floor(((Get-Date) - $os.LastBootUpTime).TotalSeconds)
                pendingReboot = [bool]$pending
                systemDrive = [string]$env:SystemDrive
                volumes = @(Get-CimInstance Win32_LogicalDisk -Filter 'DriveType=3' | Where-Object { $_.FileSystem -in @('NTFS', 'ReFS') } | ForEach-Object { @{ drive = [string]$_.DeviceID; label = [string]$_.VolumeName; sizeBytes = [double]$_.Size; freeBytes = [double]$_.FreeSpace } })
                disks = @(Get-Disks)
                deviceProblems = @(Get-DeviceProblems).Count
                security = Get-Security
                stability = Get-Stability
                lastUpdate = Get-LastUpdate
                boots = Get-Boots 1
            }
        }
        'boots' { Get-Boots 8 }
        default { throw 'Action inconnue' }
    }
}
