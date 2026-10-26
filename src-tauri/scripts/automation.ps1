[CmdletBinding()]
param(
    [ValidateSet('schedule-status','schedule-daily','schedule-weekly','schedule-disable','check-release','open-release','open-source','detect-sources','notifications-on','notifications-off')][string]$Action,
    [string]$PayloadBase64 = '', [string]$DataRoot, [string]$Executable
)
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
[Console]::OutputEncoding = New-Object Text.UTF8Encoding($false)
$reply = @{ ok=$false; data=$null }
try {
    $payload = @{}
    if ($PayloadBase64) { $payload = [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String($PayloadBase64)) | ConvertFrom-Json }
    $sid = [Security.Principal.WindowsIdentity]::GetCurrent().User.Value
    $taskName = 'Nova PC Suite Audit ' + $sid
    $ownerDescription = 'Nova PC Suite read-only audit v1'
    $sources = @{
        intel='https://www.intel.com/content/www/us/en/support/detect.html'
        nvidia='https://www.nvidia.com/en-us/software/nvidia-app/'
        amd='https://www.amd.com/en/support/download/drivers.html'
        dell='https://www.dell.com/support/home'
        lenovo='https://pcsupport.lenovo.com/'
        hp='https://support.hp.com/drivers'
        asus='https://www.asus.com/support/download-center/'
        msi='https://www.msi.com/support'
        gigabyte='https://www.gigabyte.com/Support'
    }
    switch ($Action) {
        'detect-sources' {
            $components=@();$issues=@()
            foreach($query in @(
                @{class='Win32_ComputerSystem';manufacturer='Manufacturer';device='Model'},
                @{class='Win32_BaseBoard';manufacturer='Manufacturer';device='Product'},
                @{class='Win32_Processor';manufacturer='Manufacturer';device='Name'},
                @{class='Win32_VideoController';manufacturer='AdapterCompatibility';device='Name'},
                @{class='Win32_PnPSignedDriver';manufacturer='Manufacturer';device='DeviceName'}
            )) {
                try {
                    $components+=@(Get-CimInstance -ClassName $query.class -Property @($query.manufacturer,$query.device) -ErrorAction Stop|Select-Object -First 2048|ForEach-Object{
                        [pscustomobject]@{manufacturer=[string]$_.$($query.manufacturer);device=[string]$_.$($query.device)}
                    })
                }catch{$issues+=$query.class}
            }
            $patterns=@{intel='Intel';nvidia='NVIDIA';amd='\bAMD\b|Advanced Micro Devices|AuthenticAMD';dell='Dell';lenovo='Lenovo';hp='\bHP\b|Hewlett';asus='ASUS|ASUSTeK';msi='\bMSI\b|Micro-Star|Micro Star';gigabyte='Gigabyte|Giga-byte'}
            $names=@{intel='Intel';nvidia='NVIDIA';amd='AMD';dell='Dell';lenovo='Lenovo';hp='HP';asus='ASUS';msi='MSI';gigabyte='GIGABYTE'}
            $detected=@();$unmatched=@()
            foreach($key in @($patterns.Keys|Sort-Object)){
                $devices=@($components|Where-Object{($_.manufacturer+' '+$_.device) -match $patterns[$key]}|ForEach-Object{$_.device}|Where-Object{$_}|Sort-Object -Unique|Select-Object -First 12)
                if($devices.Count){$detected+=@{id=$key;name=$names[$key];devices=$devices;officialUrl=$sources[$key]}}
            }
            foreach($group in @($components|Where-Object{$_.manufacturer -and $_.manufacturer -notmatch 'Microsoft|^\(|^Standard|^Generic|^Unknown'}|Group-Object manufacturer|Select-Object -First 40)){
                $matched=$false;foreach($pattern in $patterns.Values){if($group.Name -match $pattern){$matched=$true;break}}
                if(-not $matched){$unmatched+=@{manufacturer=$group.Name;devices=@($group.Group|ForEach-Object{$_.device}|Where-Object{$_}|Sort-Object -Unique|Select-Object -First 8)}}
            }
            $reply.data=@{providers=$detected;otherManufacturers=$unmatched;unavailable=$issues;readOnly=$true;automaticInstall=$false;interpretation='Fabricants identifies localement. Windows Update recherche les pilotes applicables ; les supports OEM peuvent proposer des versions supplementaires. Aucun site constructeur contacte pendant cette detection.'}
        }
        'schedule-status' {
            $task = Get-ScheduledTask -TaskName $taskName -ErrorAction SilentlyContinue
            $owned = $null -ne $task -and $task.Description -eq $ownerDescription
            $info = $null
            if ($owned) { $info = Get-ScheduledTaskInfo -TaskName $taskName }
            $report = $null
            $reportPath = Join-Path $DataRoot 'scheduled-report.json'
            if (Test-Path -LiteralPath $reportPath) { $report = Get-Content -LiteralPath $reportPath -Raw | ConvertFrom-Json }
            $notifications = Test-Path -LiteralPath (Join-Path $DataRoot 'notifications-enabled')
            $reply.data = @{ enabled=$owned; state=$(if($owned){[string]$task.State}else{'Disabled'}); nextRun=$(if($info){$info.NextRunTime.ToString('o')}else{$null}); lastResult=$(if($info){$info.LastTaskResult}else{$null}); lastReport=$report; notifications=$notifications; scope='UserLoggedOnOnly'; readOnly=$true }
        }
        { $_ -in @('schedule-daily','schedule-weekly') } {
            if (-not [IO.Path]::IsPathRooted($Executable) -or -not (Test-Path -LiteralPath $Executable -PathType Leaf) -or [IO.Path]::GetExtension($Executable) -ne '.exe') { throw 'InvalidExecutable' }
            $existing = Get-ScheduledTask -TaskName $taskName -ErrorAction SilentlyContinue
            if ($existing -and $existing.Description -ne $ownerDescription) { throw 'ScheduleOwnershipConflict' }
            $taskAction = New-ScheduledTaskAction -Execute $Executable -Argument '--scheduled-scan'
            $trigger = if ($Action -eq 'schedule-daily') { New-ScheduledTaskTrigger -Daily -At '09:00' } else { New-ScheduledTaskTrigger -Weekly -DaysOfWeek Monday -At '09:00' }
            $principal = New-ScheduledTaskPrincipal -UserId $sid -LogonType Interactive -RunLevel Limited
            $settings = New-ScheduledTaskSettingsSet -StartWhenAvailable -MultipleInstances IgnoreNew -ExecutionTimeLimit (New-TimeSpan -Minutes 5)
            $null = Register-ScheduledTask -TaskName $taskName -Description $ownerDescription -Action $taskAction -Trigger $trigger -Principal $principal -Settings $settings -Force
            $reply.data = @{ enabled=$true; frequency=$(if($Action -eq 'schedule-daily'){'daily'}else{'weekly'}); readOnly=$true; scope='UserLoggedOnOnly'; hour='09:00'; executableMustRemainInstalled=$true }
        }
        'schedule-disable' {
            $task = Get-ScheduledTask -TaskName $taskName -ErrorAction SilentlyContinue
            if ($task -and $task.Description -ne $ownerDescription) { throw 'ScheduleOwnershipConflict' }
            if ($task) { Unregister-ScheduledTask -TaskName $taskName -Confirm:$false }
            $reply.data = @{ enabled=$false }
        }
        'check-release' {
            $releases = Invoke-RestMethod -Uri 'https://api.github.com/repos/Rub19/nova-pc-suite/releases?per_page=10' -Headers @{'User-Agent'='Nova-PC-Suite';'Accept'='application/vnd.github+json'} -TimeoutSec 30 -MaximumRedirection 0
            $release = @($releases | Where-Object { -not $_.draft -and $_.tag_name -match '^v[0-9]+\.[0-9]+\.[0-9]+(-preview)?$' } | Select-Object -First 1)
            if ($release.Count -eq 0) { throw 'NoPublishedRelease' }
            $reply.data = @{ tag=$release[0].tag_name; publishedAt=$release[0].published_at; prerelease=$release[0].prerelease; automaticInstall=$false; signatureRequiredForAutoUpdate=$true }
        }
        'open-release' { Start-Process 'https://github.com/Rub19/nova-pc-suite/releases'; $reply.data=@{opened=$true} }
        'open-source' {
            $id=[string]$payload.id
            if (-not $sources.ContainsKey($id)) { throw 'UnknownProvider' }
            Start-Process -FilePath $sources[$id]
            $reply.data=@{opened=$true;provider=$id;automaticDriverSearch=$false}
        }
        'notifications-on' {
            $null = New-Item -Path $DataRoot -ItemType Directory -Force
            [IO.File]::WriteAllText((Join-Path $DataRoot 'notifications-enabled'), 'enabled', (New-Object Text.UTF8Encoding($false)))
            $reply.data=@{enabled=$true;delivery='BestEffortWindowsToast';reportsAlwaysAvailable=$true}
        }
        'notifications-off' {
            $marker=Join-Path $DataRoot 'notifications-enabled'
            # One Nova-owned preference file, never a recursive operation.
            if (Test-Path -LiteralPath $marker -PathType Leaf) { Remove-Item -LiteralPath $marker }
            $reply.data=@{enabled=$false}
        }
    }
    $reply.ok=$true
} catch {
    $reply.error='AutomationUnavailable'
    if ($_.Exception.Message -in @('InvalidExecutable','ScheduleOwnershipConflict','NoPublishedRelease','UnknownProvider')) { $reply.error=$_.Exception.Message }
}
$reply | ConvertTo-Json -Depth 16 -Compress
