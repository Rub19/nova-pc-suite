function New-WuSession {
    $session = New-Object -ComObject Microsoft.Update.Session
    $session.ClientApplicationID = 'Nova PC Suite'
    $session
}

function Search-Wu($Session, [string]$Kind) {
    $searcher = $Session.CreateUpdateSearcher()
    $type = if ($Kind -eq 'Driver') { 'Driver' } else { 'Software' }
    $result = $searcher.Search("IsInstalled=0 and IsHidden=0 and Type='$type'")
    if ([int]$result.ResultCode -ne 2) { throw 'Windows Update n''a pas terminé la recherche. Vérifiez la connexion et réessayez.' }
    $result.Updates
}

function Convert-Wu($U) {
    $size = 0.0
    try { $size = [double]$U.MaxDownloadSize } catch { }
    @{
        id = [string]$U.Identity.UpdateID
        title = [string]$U.Title
        description = [string]$U.Description
        kb = @($U.KBArticleIDs | ForEach-Object { "KB$_" })
        sizeBytes = $size
        severity = [string]$U.MsrcSeverity
        eulaAccepted = [bool]$U.EulaAccepted
        eula = $(if (-not $U.EulaAccepted) { [string]$U.EulaText } else { $null })
        rebootLikely = [bool]($U.InstallationBehavior.RebootBehavior -ne 0)
        driverClass = [string]$U.DriverClass
        driverProvider = [string]$U.DriverProvider
        driverModel = [string]$U.DriverModel
        driverDate = $(if ($null -ne $U.DriverVerDate) { ([datetime]$U.DriverVerDate).ToString('yyyy-MM-dd') } else { $null })
    }
}

function Get-DriverMap {
    $map = @{}
    foreach ($d in Get-CimInstance Win32_PnPSignedDriver -Property DeviceID,DeviceName,DriverVersion) {
        if ($d.DeviceID) { $map[[string]$d.DeviceID] = @{ name = [string]$d.DeviceName; version = [string]$d.DriverVersion } }
    }
    $map
}

function Install-Wu([string]$Kind, [string[]]$Ids, [bool]$AcceptEulas) {
    Require-Admin
    if (-not $Ids.Count -or $Ids.Count -gt 200) { throw 'Sélection invalide' }
    foreach ($id in $Ids) { if ($id -notmatch '^[0-9a-fA-F-]{36}$') { throw 'Sélection invalide' } }
    $session = New-WuSession
    $available = Search-Wu $session $Kind
    $selected = New-Object -ComObject Microsoft.Update.UpdateColl
    $titles = @{}
    for ($i = 0; $i -lt $available.Count; $i++) {
        $u = $available.Item($i)
        if ($Ids -contains [string]$u.Identity.UpdateID) {
            if (-not $u.EulaAccepted) {
                if (-not $AcceptEulas) { throw 'Une licence doit être acceptée avant l''installation.' }
                $u.AcceptEula()
            }
            [void]$selected.Add($u)
            $titles[[string]$u.Identity.UpdateID] = [string]$u.Title
        }
    }
    if ($selected.Count -ne @($Ids | Select-Object -Unique).Count) { throw 'Une mise à jour n''est plus proposée. Relancez la recherche.' }

    $restore = Ensure-RestorePoint 'Nova PC Suite : avant mises à jour'
    $pendingBefore = $available.Count
    $problemsBefore = @(Get-DeviceProblems).Count
    $driversBefore = if ($Kind -eq 'Driver') { Get-DriverMap } else { $null }

    $downloader = $session.CreateUpdateDownloader()
    $downloader.Updates = $selected
    [void]$downloader.Download()
    $installer = $session.CreateUpdateInstaller()
    $installer.Updates = $selected
    $installer.AllowSourcePrompts = $false
    $installer.ForceQuiet = $true
    $outcome = $installer.Install()

    $results = @()
    $okCount = 0
    for ($i = 0; $i -lt $selected.Count; $i++) {
        $r = $outcome.GetUpdateResult($i)
        $good = [int]$r.ResultCode -in @(2, 3)
        if ($good) { $okCount++ }
        $results += @{ id = [string]$selected.Item($i).Identity.UpdateID; title = [string]$selected.Item($i).Title; ok = $good; hresult = ('0x{0:X8}' -f [int]$r.HResult); rebootRequired = [bool]$r.RebootRequired }
    }
    $pendingAfter = (Search-Wu (New-WuSession) $Kind).Count
    $measures = @(@{ label = 'Mises à jour en attente'; before = $pendingBefore; after = $pendingAfter; unit = 'count' })
    $changed = @()
    if ($Kind -eq 'Driver') {
        $measures += @{ label = 'Appareils en erreur'; before = $problemsBefore; after = @(Get-DeviceProblems).Count; unit = 'count' }
        $driversAfter = Get-DriverMap
        foreach ($key in $driversAfter.Keys) {
            if ($driversBefore.ContainsKey($key) -and $driversBefore[$key].version -ne $driversAfter[$key].version) {
                $changed += @{ name = $driversAfter[$key].name; before = $driversBefore[$key].version; after = $driversAfter[$key].version }
            }
        }
    }
    @{
        summary = "$okCount sur $($selected.Count) installée(s)"
        allOk = ($okCount -eq $selected.Count)
        results = $results
        changedDrivers = $changed
        rebootRequired = [bool]$outcome.RebootRequired
        restorePoint = $restore
        measures = $measures
    }
}

function Invoke-Winget([string[]]$Arguments) {
    $exe = Join-Path $env:LOCALAPPDATA 'Microsoft\WindowsApps\winget.exe'
    if (-not (Test-Path -LiteralPath $exe)) { throw 'WinGet n''est pas installé sur ce PC.' }
    $ErrorActionPreference = 'Continue'
    try { $out = (& $exe @Arguments 2>&1 | ForEach-Object { [string]$_ }) -join "`n"; $code = $LASTEXITCODE }
    finally { $ErrorActionPreference = 'Stop' }
    @{ exitCode = $code; text = $out }
}

function Get-WingetUpgrades {
    $r = Invoke-Winget @('upgrade', '--accept-source-agreements', '--disable-interactivity')
    $lines = @(($r.text -replace "`r", "`n") -split "`n" | Where-Object { $_.Trim() })
    $sep = -1
    for ($i = 0; $i -lt $lines.Count; $i++) { if ($lines[$i] -match '^-{10,}') { $sep = $i; break } }
    if ($sep -lt 1) { return @() }
    $head = $lines[$sep - 1]
    $idCol = [regex]::Match($head, '(?<=\s)Id\b|(?<=\s)ID\b').Index
    $cols = @([regex]::Matches($head.Substring($idCol), '\S+') | ForEach-Object { $_.Index + $idCol })
    if ($cols.Count -lt 3) { return @() }
    $items = @()
    foreach ($line in $lines[($sep + 1)..($lines.Count - 1)]) {
        if ($line.Length -le $cols[2] -or $line -match '^\d+\s') { continue }
        $id = $line.Substring($cols[0], $cols[1] - $cols[0]).Trim()
        if ($id -notmatch '^[A-Za-z0-9][A-Za-z0-9._+-]+$') { continue }
        $items += @{
            name = $line.Substring(0, $cols[0]).Trim(); id = $id
            version = $line.Substring($cols[1], $cols[2] - $cols[1]).Trim()
            available = $(if ($cols.Count -gt 3 -and $line.Length -gt $cols[3]) { $line.Substring($cols[2], $cols[3] - $cols[2]).Trim() } else { $line.Substring($cols[2]).Trim() })
        }
    }
    $items
}

$OEM = @(
    @{ id = 'nvidia'; name = 'NVIDIA'; pattern = 'NVIDIA'; url = 'https://www.nvidia.com/fr-fr/software/nvidia-app/' }
    @{ id = 'amd'; name = 'AMD'; pattern = '\bAMD\b|Advanced Micro Devices|AuthenticAMD|Radeon|Ryzen'; url = 'https://www.amd.com/fr/support/download/drivers.html' }
    @{ id = 'intel'; name = 'Intel'; pattern = 'Intel'; url = 'https://www.intel.fr/content/www/fr/fr/support/detect.html' }
    @{ id = 'gigabyte'; name = 'GIGABYTE'; pattern = 'Gigabyte|Giga-byte'; url = 'https://www.gigabyte.com/fr/Support' }
    @{ id = 'asus'; name = 'ASUS'; pattern = 'ASUS'; url = 'https://www.asus.com/fr/support/download-center/' }
    @{ id = 'msi'; name = 'MSI'; pattern = '\bMSI\b|Micro-Star'; url = 'https://fr.msi.com/support/download' }
    @{ id = 'dell'; name = 'Dell'; pattern = 'Dell'; url = 'https://www.dell.com/support/home/fr-fr' }
    @{ id = 'lenovo'; name = 'Lenovo'; pattern = 'Lenovo'; url = 'https://pcsupport.lenovo.com/fr/fr' }
    @{ id = 'hp'; name = 'HP'; pattern = '\bHP\b|Hewlett'; url = 'https://support.hp.com/fr-fr/drivers' }
)

function Invoke-Module([string]$Action) {
    switch ($Action) {
        'devices' {
            $map = @(Get-CimInstance Win32_PnPSignedDriver -Property DeviceName,Manufacturer,DriverVersion,DriverDate,DeviceClass,IsSigned | Where-Object { $_.DeviceName } | ForEach-Object {
                @{ name = [string]$_.DeviceName; manufacturer = [string]$_.Manufacturer; version = [string]$_.DriverVersion; deviceClass = [string]$_.DeviceClass; signed = [bool]$_.IsSigned; date = $(if ($_.DriverDate) { $_.DriverDate.ToString('yyyy-MM-dd') } else { $null }) }
            })
            $problems = @(Get-DeviceProblems | ForEach-Object { @{ name = [string]$_.Name; manufacturer = [string]$_.Manufacturer; deviceClass = [string]$_.PNPClass; code = [int]$_.ConfigManagerErrorCode } })
            $seen = (Get-CimInstance Win32_ComputerSystem).Manufacturer, (Get-CimInstance Win32_BaseBoard).Manufacturer, (Get-CimInstance Win32_Processor).Manufacturer + @(Get-CimInstance Win32_VideoController | ForEach-Object { $_.Name }) -join ' '
            $vendors = @($OEM | Where-Object { $seen -match $_.pattern } | ForEach-Object { @{ id = $_.id; name = $_.name } })
            @{ drivers = $map; problems = $problems; vendors = $vendors }
        }
        'open-vendor' {
            $v = @($OEM | Where-Object { $_.id -ceq [string]$P.id })
            if (-not $v.Count) { throw 'Fabricant inconnu' }
            Start-Process $v[0].url
            @{ opened = $true }
        }
        'wu-search' {
            $list = Search-Wu (New-WuSession) ([string]$P.kind)
            $items = @(); for ($i = 0; $i -lt $list.Count; $i++) { $items += Convert-Wu $list.Item($i) }
            @{ items = $items; searchedAt = [datetime]::UtcNow.ToString('o') }
        }
        'wu-install' { Install-Wu ([string]$P.kind) @($P.ids | ForEach-Object { [string]$_ }) ([bool]$P.acceptEulas) }
        'wu-history' {
            $searcher = (New-WuSession).CreateUpdateSearcher()
            $total = $searcher.GetTotalHistoryCount()
            $items = @()
            if ($total -gt 0) {
                $h = $searcher.QueryHistory(0, [math]::Min(15, $total))
                for ($i = 0; $i -lt $h.Count; $i++) {
                    $e = $h.Item($i)
                    if ($e.Title) { $items += @{ title = [string]$e.Title; date = ([datetime]$e.Date).ToUniversalTime().ToString('o'); ok = [int]$e.ResultCode -in @(2, 3) } }
                }
            }
            @{ items = $items }
        }
        'winget-list' { @{ items = @(Get-WingetUpgrades) } }
        'winget-upgrade' {
            $id = [string]$P.id
            if ($id -notmatch '^[A-Za-z0-9][A-Za-z0-9._+-]{1,127}$') { throw 'Identifiant invalide' }
            $before = @(Get-WingetUpgrades | Where-Object { $_.id -ceq $id })
            if (-not $before.Count) { throw 'Ce logiciel n''a plus de mise à jour proposée.' }
            $r = Invoke-Winget @('upgrade', '--id', $id, '--exact', '--silent', '--accept-package-agreements', '--accept-source-agreements', '--disable-interactivity')
            $still = @(Get-WingetUpgrades | Where-Object { $_.id -ceq $id })
            if ($r.exitCode -ne 0 -or $still.Count) { throw "$($before[0].name) n'a pas été mis à jour (code $($r.exitCode)). L'installeur l'a peut-être refusé." }
            @{ summary = "$($before[0].name) mis à jour"; measures = @(@{ label = $before[0].name; before = $before[0].version; after = $before[0].available; unit = 'text' }) }
        }
        'reboot' {
            Invoke-Native 'shutdown.exe' @('/r', '/t', '60', '/c', 'Redémarrage demandé depuis Nova PC Suite') | Out-Null
            @{ summary = 'Redémarrage dans 60 secondes'; undo = @{ module = 'updates'; action = 'reboot-cancel'; label = 'Annuler le redémarrage' } }
        }
        'reboot-cancel' {
            Invoke-Native 'shutdown.exe' @('/a') | Out-Null
            @{ summary = 'Redémarrage annulé' }
        }
        default { throw 'Action inconnue' }
    }
}
