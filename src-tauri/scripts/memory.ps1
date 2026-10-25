[CmdletBinding()]
param([ValidateSet('memory-snapshot','memory-trim','process-close')][string]$Action='memory-snapshot',[string]$PayloadBase64='',[string]$DataRoot)
$ErrorActionPreference='Stop'
[Console]::OutputEncoding=New-Object Text.UTF8Encoding($false)
function Memory-Sample {
    $os=Get-CimInstance Win32_OperatingSystem
    $items=@(Get-Process|Where-Object{$_.WorkingSet64 -gt 100MB}|Sort-Object WorkingSet64 -Descending|Select-Object -First 30|ForEach-Object{
        $process=$_; $started=$null; $path=$null
        try{$started=$process.StartTime.ToUniversalTime().Ticks.ToString();$path=$process.Path}catch{}
        $manageable=$false
        if($path -and $started -and $path -notlike "$env:SystemRoot\*" -and $process.ProcessName -notmatch '(?i)defender|security|antivirus|msmpeng|sense|lsass|csrss|winlogon|services|svchost|dwm|explorer|nova-pc-suite|memorycompression'){
            try{$owner=Get-CimInstance Win32_Process -Filter "ProcessId=$($process.Id)"|Invoke-CimMethod -MethodName GetOwnerSid;$manageable=$owner.Sid -eq [Security.Principal.WindowsIdentity]::GetCurrent().User.Value}catch{}
        }
        @{pid=$process.Id;name=$process.ProcessName;workingSetBytes=$process.WorkingSet64;privateBytes=$process.PrivateMemorySize64;started=$started;manageable=[bool]$manageable}
    })
    $paging=$null;try{$paging=(Get-CimInstance Win32_PerfFormattedData_PerfOS_Memory).PageReadsPersec}catch{}
    @{totalBytes=[double]$os.TotalVisibleMemorySize*1024;freeBytes=[double]$os.FreePhysicalMemory*1024;usedPercent=[math]::Round(100*(1-[double]$os.FreePhysicalMemory/[double]$os.TotalVisibleMemorySize));pageReadsPerSecond=$paging;items=$items;timestamp=[DateTimeOffset]::UtcNow.ToUnixTimeSeconds()}
}
try {
    $payload=@{};if($PayloadBase64){$payload=[Text.Encoding]::UTF8.GetString([Convert]::FromBase64String($PayloadBase64))|ConvertFrom-Json}
    if($Action -eq 'memory-snapshot'){
        if(@($payload.PSObject.Properties).Count -gt 0 -and $payload -isnot [hashtable]){throw 'InvalidMemoryTarget'}
        $data=Memory-Sample
    }else{
        if($payload.pid -isnot [int] -and $payload.pid -isnot [long]){throw 'InvalidMemoryTarget'}
        if([long]$payload.pid -le 0 -or [long]$payload.pid -gt 4294967295 -or $payload.started -notmatch '^[0-9]{1,20}$'){throw 'InvalidMemoryTarget'}
        $before=Memory-Sample
        $target=@($before.items|Where-Object{$_.pid -eq $payload.pid -and $_.started -eq $payload.started -and $_.manageable})
        if($target.Count -ne 1){throw 'MemoryTargetUnavailable'}
        if($Action -eq 'process-close'){
            $application=[Diagnostics.Process]::GetProcessById([int]$payload.pid)
            try{
                # Retaining the handle prevents PID reuse while validating and requesting closure.
                $retained=$application.Handle
                if($application.HasExited -or $application.StartTime.ToUniversalTime().Ticks.ToString() -ne [string]$payload.started){throw 'MemoryTargetUnavailable'}
                if(-not $application.CloseMainWindow()){throw 'ProcessHasNoClosableWindow'}
                $exited=$application.WaitForExit(1500)
                @{ok=$true;data=@{targetPid=$payload.pid;closeRequested=$true;exited=$exited;status=$(if($exited){'closed'}else{'awaiting-application'});limitation='La fermeture peut demander un enregistrement ou être refusée par l’application. Aucun arrêt forcé.'}}|ConvertTo-Json -Depth 5 -Compress
                return
            }finally{$application.Dispose()}
        }
        # Operate on a retained handle and check its creation time again to avoid PID reuse.
        Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class NovaMemory {
 [DllImport("kernel32.dll",SetLastError=true)] public static extern IntPtr OpenProcess(uint a,bool i,uint p);
 [DllImport("kernel32.dll")] public static extern bool CloseHandle(IntPtr h);
 [DllImport("psapi.dll",SetLastError=true)] public static extern bool EmptyWorkingSet(IntPtr h);
 [DllImport("kernel32.dll",SetLastError=true)] public static extern bool GetProcessTimes(IntPtr h,out long c,out long e,out long k,out long u);
 [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
 [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h,out uint p);
}
'@
        [uint32]$foreground=0;[void][NovaMemory]::GetWindowThreadProcessId([NovaMemory]::GetForegroundWindow(),[ref]$foreground)
        if($foreground -eq [uint32]$payload.pid){throw 'ForegroundApplicationSkipped'}
        $handle=[NovaMemory]::OpenProcess(0x100 -bor 0x400,$false,[uint32]$payload.pid)
        if($handle -eq [IntPtr]::Zero){throw 'MemoryTargetUnavailable'}
        try{
            [long]$creation=0;[long]$exit=0;[long]$kernel=0;[long]$user=0
            if(-not [NovaMemory]::GetProcessTimes($handle,[ref]$creation,[ref]$exit,[ref]$kernel,[ref]$user)){throw 'MemoryTargetUnavailable'}
            if([DateTime]::FromFileTimeUtc($creation).Ticks.ToString() -ne [string]$payload.started){throw 'MemoryTargetUnavailable'}
            if(-not [NovaMemory]::EmptyWorkingSet($handle)){throw 'MemoryTrimFailed'}
        }finally{[void][NovaMemory]::CloseHandle($handle)}
        $after=Memory-Sample
        $data=@{before=$before;after=$after;targetPid=$payload.pid;trimRequested=$true;limitation='Réduction du working set, pas de la mémoire privée allouée. Les pages peuvent revenir en RAM et provoquer des défauts de page ; ce n’est pas une correction de fuite mémoire.'}
    }
    @{ok=$true;data=$data}|ConvertTo-Json -Depth 10 -Compress
}catch{$code=$_.Exception.Message;if($code -notin @('InvalidMemoryTarget','MemoryTargetUnavailable','ForegroundApplicationSkipped','MemoryTrimFailed','ProcessHasNoClosableWindow')){$code='MemoryDiagnosticUnavailable'};@{ok=$false;data=$null;error=$code}|ConvertTo-Json -Compress}
