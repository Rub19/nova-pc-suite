$ErrorActionPreference='Stop'
$os=Get-CimInstance Win32_OperatingSystem
$disks=@(Get-CimInstance Win32_LogicalDisk -Filter 'DriveType=3' | ForEach-Object { @{drive=$_.DeviceID;freeBytes=$_.FreeSpace;sizeBytes=$_.Size} })
$cpu=@(Get-CimInstance Win32_Processor | ForEach-Object { @{name=$_.Name;loadPercent=$_.LoadPercentage} })
$alerts=@()
$memoryUsed=[math]::Round(100*(1-([double]$os.FreePhysicalMemory/[double]$os.TotalVisibleMemorySize)))
if($memoryUsed -gt 85){$alerts+='MemoryPressure'}
foreach($disk in $disks){if([double]$disk.sizeBytes -gt 0 -and [double]$disk.freeBytes/[double]$disk.sizeBytes -lt .15){$alerts+='LowDiskSpace'}}
$report=@{timestamp=[DateTimeOffset]::UtcNow.ToUnixTimeSeconds();memoryUsedPercent=$memoryUsed;volumes=$disks;cpu=$cpu;alerts=$alerts;readOnly=$true}
[Console]::OutputEncoding=New-Object Text.UTF8Encoding($false)
@{ok=$true;data=$report}|ConvertTo-Json -Depth 8 -Compress
