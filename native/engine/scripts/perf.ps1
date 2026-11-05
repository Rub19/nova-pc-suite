$NoClose = '(?i)^(system|idle|registry|smss|csrss|wininit|winlogon|services|lsass|svchost|fontdrvhost|dwm|explorer|sihost|ctfmon|memory compression|nova-pc-suite|msmpeng|securityhealth.*|searchhost|startmenuexperiencehost|shellexperiencehost|textinputhost|runtimebroker|taskhostw|dllhost|conhost|applicationframehost)$'

function Get-Plans {
    $r = Invoke-Native 'powercfg.exe' @('/list')
    $plans = @()
    foreach ($line in $r.lines) {
        if ($line -match '([0-9a-fA-F]{8}-[0-9a-fA-F-]{27})\s+\((.+?)\)\s*(\*)?') {
            $plans += @{ guid = $Matches[1].ToLowerInvariant(); name = $Matches[2]; active = [bool]$Matches[3] }
        }
    }
    $plans
}

function Get-Volumes {
    $mediaByDisk = @{}
    foreach ($d in Get-PhysicalDisk) { $mediaByDisk[[string]$d.DeviceId] = [string]$d.MediaType }
    $last = @{}
    try {
        foreach ($e in Get-WinEvent -FilterHashtable @{ LogName = 'Application'; ProviderName = 'Microsoft-Windows-Defrag'; Id = 258 } -MaxEvents 40) {
            if ($e.Message -match '\(([A-Za-z]):\)' -and -not $last.ContainsKey($Matches[1].ToUpper())) { $last[$Matches[1].ToUpper()] = $e.TimeCreated.ToUniversalTime().ToString('o') }
        }
    } catch { }
    @(Get-Volume | Where-Object { $_.DriveLetter -and [string]$_.DriveType -eq 'Fixed' -and $_.FileSystem -in @('NTFS', 'ReFS') } | ForEach-Object {
        $letter = [string]$_.DriveLetter
        $media = $null
        try { $media = $mediaByDisk[[string](Get-Partition -DriveLetter $letter).DiskNumber] } catch { }
        @{ letter = $letter; label = [string]$_.FileSystemLabel; sizeBytes = [double]$_.Size; freeBytes = [double]$_.SizeRemaining; media = $media; lastOptimized = $last[$letter.ToUpper()]; system = ("$letter`:" -eq $env:SystemDrive) }
    })
}

function Get-Fragmentation([string]$Letter) {
    try {
        $v = Get-CimInstance Win32_Volume -Filter "DriveLetter='$Letter`:'"
        [double](Invoke-CimMethod -InputObject $v -MethodName DefragAnalysis).DefragAnalysis.FilePercentFragmentation
    } catch { $null }
}

function Get-Hibernation {
    $enabled = [bool](Get-ItemProperty 'HKLM:\SYSTEM\CurrentControlSet\Control\Power').HibernateEnabled
    $size = 0.0
    try { $size = [double](Get-Item -LiteralPath "$env:SystemDrive\hiberfil.sys" -Force).Length } catch { }
    @{ enabled = $enabled; fileBytes = $size }
}

function Ping-Host([string]$Target) {
    $ping = New-Object Net.NetworkInformation.Ping
    $rtts = @(); $lost = 0
    try {
        for ($i = 0; $i -lt 5; $i++) {
            try { $r = $ping.Send($Target, 1000); if ($r.Status -eq 'Success') { $rtts += [double]$r.RoundtripTime } else { $lost++ } } catch { $lost++ }
        }
    } finally { $ping.Dispose() }
    @{ target = $Target; lossPercent = $lost * 20; averageMs = $(if ($rtts.Count) { [math]::Round(($rtts | Measure-Object -Average).Average, 1) } else { $null }); worstMs = $(if ($rtts.Count) { ($rtts | Measure-Object -Maximum).Maximum } else { $null }) }
}

function Get-Gpu {
    $exe = Sys32 'nvidia-smi.exe'
    if (-not (Test-Path -LiteralPath $exe)) { return $null }
    try {
        $line = & $exe --query-gpu=name,temperature.gpu,utilization.gpu,memory.used,memory.total,power.draw,driver_version --format=csv,noheader,nounits
        $f = @($line | Select-Object -First 1) -split ',\s*'
        @{ name = $f[0]; temperature = [double]$f[1]; usePercent = [double]$f[2]; memoryUsedMiB = [double]$f[3]; memoryTotalMiB = [double]$f[4]; powerW = $(if ($f[5] -match '^[\d.]+$') { [double]$f[5] } else { $null }); driver = $f[6] }
    } catch { $null }
}

function Invoke-Module([string]$Action) {
    switch ($Action) {
        'overview' {
            @{ plans = @(Get-Plans); volumes = @(Get-Volumes); hibernation = Get-Hibernation; gpu = Get-Gpu; isAdmin = (Test-Admin) }
        }
        'processes' {
            $first = @{}
            foreach ($p in Get-Process) { try { $first[$p.Id] = $p.TotalProcessorTime.TotalSeconds } catch { } }
            Start-Sleep -Milliseconds 1000
            $logical = [Environment]::ProcessorCount
            $groups = @{}
            foreach ($p in Get-Process) {
                if ($p.Id -in 0, 4) { continue }
                $cpu = 0.0
                try { if ($first.ContainsKey($p.Id)) { $cpu = ($p.TotalProcessorTime.TotalSeconds - $first[$p.Id]) * 100 / $logical } } catch { }
                $n = $p.ProcessName
                if (-not $groups.ContainsKey($n)) { $groups[$n] = @{ name = $n; count = 0; cpuPercent = 0.0; ramBytes = 0.0; closable = $false } }
                $g = $groups[$n]; $g.count++; $g.cpuPercent += $cpu; $g.ramBytes += $p.WorkingSet64
                if ($p.MainWindowHandle -ne [IntPtr]::Zero -and $n -notmatch $NoClose) { $g.closable = $true }
            }
            $ranked = @($groups.Values | Sort-Object { $_.ramBytes } -Descending)
            @{ items = @($ranked | Select-Object -First 25) + @($ranked | Select-Object -Skip 25 | Where-Object { $_.closable }) | ForEach-Object { $_.cpuPercent = [math]::Round($_.cpuPercent, 1); $_ } }
        }
        'process-close' {
            $name = [string]$P.name
            if ($name -match $NoClose) { throw 'Nova ne ferme pas les processus de Windows.' }
            $targets = @(Get-Process -Name $name -ErrorAction SilentlyContinue | Where-Object { $_.MainWindowHandle -ne [IntPtr]::Zero })
            if (-not $targets.Count) { throw 'Ce programme n''a plus de fenêtre à fermer.' }
            $before = [double](Get-CimInstance Win32_OperatingSystem).FreePhysicalMemory * 1024
            foreach ($t in $targets) { [void]$t.CloseMainWindow() }
            Start-Sleep -Seconds 3
            $left = @(Get-Process -Name $name -ErrorAction SilentlyContinue | Where-Object { $_.MainWindowHandle -ne [IntPtr]::Zero }).Count
            $after = [double](Get-CimInstance Win32_OperatingSystem).FreePhysicalMemory * 1024
            @{
                summary = $(if ($left) { "$name attend une réponse dans sa fenêtre" } else { "$name fermé" })
                measures = @(@{ label = 'Mémoire libre'; before = $before; after = $after; unit = 'bytes' })
            }
        }
        'power-apply' {
            $guid = ([string]$P.guid).ToLowerInvariant()
            $plans = @(Get-Plans)
            $target = @($plans | Where-Object { $_.guid -ceq $guid })
            if (-not $target.Count) { throw 'Ce mode d''alimentation n''existe pas sur ce PC.' }
            $was = @($plans | Where-Object { $_.active })
            $r = Invoke-Native 'powercfg.exe' @('/setactive', $guid)
            if ($r.exitCode -ne 0) { throw 'Windows a refusé ce mode d''alimentation.' }
            $now = @(Get-Plans | Where-Object { $_.active })
            if (-not $now.Count -or $now[0].guid -cne $guid) { throw 'Le mode actif n''a pas changé.' }
            @{
                summary = "Mode d'alimentation : $($target[0].name)"
                measures = @(@{ label = 'Mode actif'; before = $(if ($was.Count) { $was[0].name } else { 'inconnu' }); after = $target[0].name; unit = 'text' })
                undo = $(if ($was.Count) { @{ module = 'perf'; action = 'power-apply'; payload = @{ guid = $was[0].guid }; label = "Revenir à $($was[0].name)" } } else { $null })
            }
        }
        'volume-optimize' {
            Require-Admin
            $letter = [string]$P.letter
            $vol = @(Get-Volumes | Where-Object { $_.letter -ceq $letter })
            if (-not $vol.Count) { throw 'Volume non pris en charge' }
            $ssd = $vol[0].media -eq 'SSD'
            $before = if ($ssd) { $null } else { Get-Fragmentation $letter }
            Optimize-Volume -DriveLetter $letter
            $after = if ($ssd) { $null } else { Get-Fragmentation $letter }
            $m = @(@{ label = 'Dernière optimisation'; before = $(if ($vol[0].lastOptimized) { $vol[0].lastOptimized } else { 'inconnue' }); after = [datetime]::UtcNow.ToString('o'); unit = 'date' })
            if ($null -ne $before -and $null -ne $after) { $m += @{ label = 'Fichiers fragmentés'; before = $before; after = $after; unit = 'percent' } }
            @{ summary = "$letter`: $(if ($ssd) { 'SSD optimisé (TRIM)' } else { 'disque défragmenté' })"; measures = $m }
        }
        'hibernate-set' {
            Require-Admin
            $enable = [bool]$P.enabled
            $free = Get-FreeBytes
            $r = Invoke-Native 'powercfg.exe' @('/hibernate', $(if ($enable) { 'on' } else { 'off' }))
            if ($r.exitCode -ne 0) { throw 'Windows a refusé ce réglage.' }
            @{
                summary = "Veille prolongée $(if ($enable) { 'activée' } else { 'désactivée' })"
                measures = @(@{ label = "Espace libre sur $env:SystemDrive"; before = $free; after = (Get-FreeBytes); unit = 'bytes' })
                undo = @{ module = 'perf'; action = 'hibernate-set'; payload = @{ enabled = (-not $enable) }; label = $(if ($enable) { 'Désactiver à nouveau' } else { 'Réactiver' }) }
            }
        }
        'network-test' {
            $route = Get-NetRoute -DestinationPrefix '0.0.0.0/0' -ErrorAction SilentlyContinue | Where-Object { $_.NextHop -ne '0.0.0.0' } | Sort-Object RouteMetric | Select-Object -First 1
            $probes = @()
            if ($route) { $probes += (Ping-Host ([string]$route.NextHop)) + @{ role = 'Box ou routeur' } }
            $probes += (Ping-Host '1.1.1.1') + @{ role = 'Internet' }
            $watch = [Diagnostics.Stopwatch]::StartNew(); $dnsOk = $false
            try { $dnsOk = @(Resolve-DnsName -Name 'example.com' -Type A -DnsOnly -NoHostsFile -ErrorAction Stop).Count -gt 0 } catch { }
            @{ probes = $probes; dnsOk = $dnsOk; dnsMs = $watch.ElapsedMilliseconds }
        }
        'dns-flush' {
            Require-Admin
            $before = @(Get-DnsClientCache).Count
            Clear-DnsClientCache
            @{ summary = 'Cache DNS vidé'; measures = @(@{ label = 'Entrées dans le cache DNS'; before = $before; after = @(Get-DnsClientCache).Count; unit = 'count' }) }
        }
        default { throw 'Action inconnue' }
    }
}
