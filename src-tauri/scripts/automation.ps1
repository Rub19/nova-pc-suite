[CmdletBinding()]
param(
    [ValidateSet('schedule-status','schedule-daily','schedule-weekly','schedule-disable','check-release','open-release','open-source','notifications-on','notifications-off')][string]$Action,
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
    }
    switch ($Action) {
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
