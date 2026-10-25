$ErrorActionPreference='Stop'
try {
    $sid=[Security.Principal.WindowsIdentity]::GetCurrent().User.Value
    $name='Nova PC Suite Audit '+$sid
    $task=Get-ScheduledTask -TaskName $name -ErrorAction SilentlyContinue
    if($task -and $task.Description -eq 'Nova PC Suite read-only audit v1') {
        $actions=@($task.Actions)
        if($actions.Count -eq 1 -and $actions[0].Arguments -eq '--scheduled-scan') {
            Unregister-ScheduledTask -TaskName $name -Confirm:$false
        }
    }
} catch { exit 1 }
