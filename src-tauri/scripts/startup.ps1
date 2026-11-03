$RunPath = 'Software\Microsoft\Windows\CurrentVersion\Run'
$ApprovedPath = 'Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved'
$Protected = '(?i)defender|securityhealth|antivirus|anti[- ]?malware|smartscreen|crowdstrike|sentinel|sophos|kaspersky|bitdefender|avast|avg|mcafee|norton|symantec|eset|malwarebytes|trend.?micro|webroot|f-secure'

function Reg-Root([string]$Hive) {
    if ($Sandbox) { return 'HKCU:\Software\NovaSandbox\' + $Hive }
    if ($Hive -eq 'hklm') { return 'HKLM:\' }
    'HKCU:\'
}

$Sources = @(
    @{ id = 'hkcu-run'; hive = 'hkcu'; kind = 'reg'; key = $RunPath; approved = 'Run'; label = 'Registre, cet utilisateur' }
    @{ id = 'hklm-run'; hive = 'hklm'; kind = 'reg'; key = $RunPath; approved = 'Run'; label = 'Registre, tous les utilisateurs' }
    @{ id = 'hklm-run32'; hive = 'hklm'; kind = 'reg'; key = 'Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Run'; approved = 'Run32'; label = 'Registre 32 bits, tous les utilisateurs' }
    @{ id = 'user-folder'; hive = 'hkcu'; kind = 'folder'; folder = (Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs\Startup'); approved = 'StartupFolder'; label = 'Dossier Démarrage, cet utilisateur' }
    @{ id = 'common-folder'; hive = 'hklm'; kind = 'folder'; folder = (Join-Path $env:ProgramData 'Microsoft\Windows\Start Menu\Programs\Startup'); approved = 'StartupFolder'; label = 'Dossier Démarrage, tous les utilisateurs' }
)

function Get-Approved($Source, [string]$Name) {
    $key = Join-Path (Reg-Root $Source.hive) "$ApprovedPath\$($Source.approved)"
    try { $b = (Get-ItemProperty -LiteralPath $key -Name $Name).$Name } catch { return $true }
    if ($b -isnot [byte[]] -or $b.Length -lt 1) { return $true }
    -not ($b[0] -band 1)
}

function Set-Approved($Source, [string]$Name, [bool]$Enabled) {
    $key = Join-Path (Reg-Root $Source.hive) "$ApprovedPath\$($Source.approved)"
    if (-not (Test-Path -LiteralPath $key)) { [void](New-Item -Path $key -Force) }
    $bytes = New-Object byte[] 12
    if ($Enabled) { $bytes[0] = 2 } else {
        $bytes[0] = 3
        [Array]::Copy([BitConverter]::GetBytes([datetime]::UtcNow.ToFileTimeUtc()), 0, $bytes, 4, 8)
    }
    Set-ItemProperty -LiteralPath $key -Name $Name -Value $bytes -Type Binary
}

function Get-ExePath([string]$Command) {
    if (-not $Command) { return $null }
    $c = [Environment]::ExpandEnvironmentVariables($Command).Trim()
    if ($c -match '^"([^"]+)"') { return $Matches[1] }
    if ($c -match '^(.+?\.(exe|cmd|bat|com))(\s|$)') { return $Matches[1] }
    ($c -split '\s')[0]
}

function Describe([string]$Exe, $Procs) {
    $publisher = $null; $found = $false; $ram = 0.0
    if ($Exe) {
        try {
            if ([IO.File]::Exists($Exe)) { $found = $true; $publisher = (Get-Item -LiteralPath $Exe).VersionInfo.CompanyName }
        } catch { }
        $key = $Exe.ToLowerInvariant()
        if ($Procs.ContainsKey($key)) { $ram = $Procs[$key] }
    }
    @{ path = $Exe; publisher = $(if ($publisher) { [string]$publisher.Trim() } else { $null }); exists = $found; ramBytes = $ram; running = ($ram -gt 0) }
}

function Get-Lnk([string]$Path) {
    $shell = New-Object -ComObject WScript.Shell
    try { [string]$shell.CreateShortcut($Path).TargetPath } catch { $null }
}

function Get-Entries {
    $admin = Test-Admin
    $procs = @{}
    foreach ($p in Get-Process) {
        try { if ($p.Path) { $k = $p.Path.ToLowerInvariant(); $procs[$k] = [double]$procs[$k] + $p.WorkingSet64 } } catch { }
    }
    $items = @()
    foreach ($s in $Sources) {
        if ($Sandbox -and $s.hive -eq 'hklm') { continue }
        $entries = @()
        if ($s.kind -eq 'reg') {
            $path = Join-Path (Reg-Root $s.hive) $s.key
            if (Test-Path -LiteralPath $path) {
                $reg = Get-Item -LiteralPath $path
                foreach ($n in $reg.GetValueNames()) { if ($n) { $entries += @{ name = $n; exe = Get-ExePath ([string]$reg.GetValue($n)) } } }
            }
        } elseif (Test-Path -LiteralPath $s.folder) {
            foreach ($f in Get-ChildItem -LiteralPath $s.folder -File) {
                if ($f.Name -ne 'desktop.ini') { $entries += @{ name = $f.Name; exe = $(if ($f.Extension -eq '.lnk') { Get-Lnk $f.FullName } else { $f.FullName }) } }
            }
        }
        foreach ($e in $entries) {
            $d = Describe $e.exe $procs
            $guard = ($e.name + ' ' + $e.exe) -match $Protected
            $items += @{
                source = $s.id; sourceLabel = $s.label; name = [string]$e.name; enabled = Get-Approved $s $e.name
                path = $d.path; publisher = $d.publisher; exists = $d.exists; ramBytes = $d.ramBytes; running = $d.running
                protected = [bool]$guard; manageable = (-not $guard) -and ($s.hive -eq 'hkcu' -or $admin)
            }
        }
    }
    if (-not $Sandbox) {
        try {
            foreach ($t in Get-ScheduledTask | Where-Object { $_.TaskPath -notlike '\Microsoft\*' -and @($_.Triggers | Where-Object { $_.CimClass.CimClassName -eq 'MSFT_TaskLogonTrigger' }).Count }) {
                $exe = Get-ExePath ([string]@($t.Actions)[0].Execute)
                $d = Describe $exe $procs
                $guard = ($t.TaskName + ' ' + $exe) -match $Protected
                $items += @{
                    source = 'task'; sourceLabel = 'Tâche planifiée à l''ouverture de session'; name = [string]($t.TaskPath + $t.TaskName); enabled = ($t.State -ne 'Disabled')
                    path = $d.path; publisher = $d.publisher; exists = $d.exists; ramBytes = $d.ramBytes; running = $d.running
                    protected = [bool]$guard; manageable = (-not $guard) -and $admin
                }
            }
        } catch { }
    }
    @($items | Sort-Object @{ e = { -not $_.enabled } }, @{ e = { -$_.ramBytes } }, name)
}

function Invoke-Module([string]$Action) {
    switch ($Action) {
        'list' {
            @{ items = @(Get-Entries); isAdmin = (Test-Admin) }
        }
        'set' {
            $sourceId = [string]$P.source; $name = [string]$P.name; $enable = [bool]$P.enabled
            $current = @(Get-Entries)
            $entry = @($current | Where-Object { $_.source -ceq $sourceId -and $_.name -ceq $name })
            if ($entry.Count -ne 1) { throw 'Cet élément n''existe plus. Actualisez la liste.' }
            if (-not $entry[0].manageable) {
                if ($entry[0].protected) { throw 'Nova ne désactive pas les logiciels de sécurité.' }
                throw 'Cette action demande les droits administrateur.'
            }
            $before = @($current | Where-Object { $_.enabled }).Count
            if ($sourceId -eq 'task') {
                $path = $name.Substring(0, $name.LastIndexOf('\') + 1); $leaf = $name.Substring($name.LastIndexOf('\') + 1)
                if ($enable) { [void](Enable-ScheduledTask -TaskPath $path -TaskName $leaf) } else { [void](Disable-ScheduledTask -TaskPath $path -TaskName $leaf) }
            } else {
                Set-Approved (@($Sources | Where-Object { $_.id -ceq $sourceId })[0]) $name $enable
            }
            $after = @(Get-Entries)
            $check = @($after | Where-Object { $_.source -ceq $sourceId -and $_.name -ceq $name })
            if (-not $check.Count -or [bool]$check[0].enabled -ne $enable) { throw 'Windows n''a pas appliqué le changement.' }
            @{
                summary = "$name $(if ($enable) { 'réactivé' } else { 'désactivé' }) au démarrage"
                measures = @(@{ label = 'Programmes lancés au démarrage'; before = $before; after = @($after | Where-Object { $_.enabled }).Count; unit = 'count' })
                undo = @{ module = 'startup'; action = 'set'; payload = @{ source = $sourceId; name = $name; enabled = (-not $enable) }; label = $(if ($enable) { 'Désactiver à nouveau' } else { 'Réactiver' }) }
            }
        }
        default { throw 'Action inconnue' }
    }
}
