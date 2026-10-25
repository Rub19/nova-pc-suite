[CmdletBinding()]
param([ValidateSet('hardware-info')][string]$Action='hardware-info',[string]$PayloadBase64='',[string]$DataRoot)
$ErrorActionPreference='Stop'
$ProgressPreference='SilentlyContinue'
$script:hardwareIssues=@()
function Read-HardwareClass([string]$Class,[string[]]$Properties) {
 try { return @(Get-CimInstance -ClassName $Class -Property $Properties -ErrorAction Stop | Select-Object -First 128) }
 catch { $script:hardwareIssues+=$Class; return @() }
}
function Hardware-Date($Value) {
 if($Value -is [datetime]){return $Value.ToUniversalTime().ToString('o')};return $null
}
function Get-HardwareInfo {
 $script:hardwareIssues=@()
 $computer=@(Read-HardwareClass 'Win32_ComputerSystem' @('Manufacturer','Model','SystemType','TotalPhysicalMemory','NumberOfLogicalProcessors','HypervisorPresent'))|Select-Object -First 1
 $os=@(Read-HardwareClass 'Win32_OperatingSystem' @('Caption','Version','BuildNumber','OSArchitecture','InstallDate','LastBootUpTime','TotalVisibleMemorySize'))|Select-Object -First 1
 $cpu=@(Read-HardwareClass 'Win32_Processor' @('Name','Manufacturer','SocketDesignation','NumberOfCores','NumberOfLogicalProcessors','MaxClockSpeed','L2CacheSize','L3CacheSize','VirtualizationFirmwareEnabled')|ForEach-Object{
  @{name=$_.Name;manufacturer=$_.Manufacturer;socket=$_.SocketDesignation;cores=$_.NumberOfCores;threads=$_.NumberOfLogicalProcessors;maxClockMHz=$_.MaxClockSpeed;l2CacheKiB=$_.L2CacheSize;l3CacheKiB=$_.L3CacheSize;virtualizationFirmware=$_.VirtualizationFirmwareEnabled}
 })
 $memory=@(Read-HardwareClass 'Win32_PhysicalMemory' @('Manufacturer','PartNumber','Capacity','DeviceLocator','BankLabel','ConfiguredClockSpeed','SMBIOSMemoryType')|ForEach-Object{
  @{manufacturer=$_.Manufacturer;partNumber=$_.PartNumber;capacityBytes=$_.Capacity;slot=$_.DeviceLocator;bank=$_.BankLabel;configuredClockMHz=$_.ConfiguredClockSpeed;smbiosType=$_.SMBIOSMemoryType}
 })
 $board=@(Read-HardwareClass 'Win32_BaseBoard' @('Manufacturer','Product','Version')|ForEach-Object{@{manufacturer=$_.Manufacturer;product=$_.Product;version=$_.Version}})
 $bios=@(Read-HardwareClass 'Win32_BIOS' @('Manufacturer','SMBIOSBIOSVersion','ReleaseDate')|ForEach-Object{@{manufacturer=$_.Manufacturer;version=$_.SMBIOSBIOSVersion;dateUtc=(Hardware-Date $_.ReleaseDate)}})
 $gpu=@(Read-HardwareClass 'Win32_VideoController' @('Name','VideoProcessor','DriverVersion','DriverDate','CurrentHorizontalResolution','CurrentVerticalResolution','CurrentRefreshRate','Status')|ForEach-Object{
  @{name=$_.Name;processor=$_.VideoProcessor;driverVersion=$_.DriverVersion;driverDateUtc=(Hardware-Date $_.DriverDate);width=$_.CurrentHorizontalResolution;height=$_.CurrentVerticalResolution;refreshHz=$(if($_.CurrentRefreshRate -gt 1){$_.CurrentRefreshRate}else{$null});status=$_.Status}
 })
 $disks=@(Read-HardwareClass 'Win32_DiskDrive' @('Model','InterfaceType','MediaType','Size','FirmwareRevision','Status')|ForEach-Object{@{model=$_.Model;interface=$_.InterfaceType;media=$_.MediaType;sizeBytes=$_.Size;firmware=$_.FirmwareRevision;status=$_.Status}})
 $volumes=@(Read-HardwareClass 'Win32_LogicalDisk' @('DeviceID','DriveType','FileSystem','Size','FreeSpace')|Where-Object{$_.DriveType -eq 3}|ForEach-Object{@{drive=$_.DeviceID;fileSystem=$_.FileSystem;sizeBytes=$_.Size;freeBytes=$_.FreeSpace}})
 $network=@(Read-HardwareClass 'Win32_NetworkAdapter' @('Name','Manufacturer','PhysicalAdapter','NetEnabled','Speed')|Where-Object{$_.PhysicalAdapter}|ForEach-Object{@{name=$_.Name;manufacturer=$_.Manufacturer;enabled=$_.NetEnabled;speedBitsPerSecond=$_.Speed}})
 $audio=@(Read-HardwareClass 'Win32_SoundDevice' @('Name','Manufacturer','Status')|ForEach-Object{@{name=$_.Name;manufacturer=$_.Manufacturer;status=$_.Status}})
 $batteries=@(Read-HardwareClass 'Win32_Battery' @('Name','EstimatedChargeRemaining','BatteryStatus')|ForEach-Object{@{name=$_.Name;chargePercent=$_.EstimatedChargeRemaining;statusCode=$_.BatteryStatus}})
 $usable=$null;if($null -ne $os.TotalVisibleMemorySize){$usable=[double]$os.TotalVisibleMemorySize*1024}
 return @{timestampUtc=[datetime]::UtcNow.ToString('o');computer=@{manufacturer=$computer.Manufacturer;model=$computer.Model;systemType=$computer.SystemType;installedMemoryBytes=$computer.TotalPhysicalMemory;logicalProcessors=$computer.NumberOfLogicalProcessors;hypervisorPresent=$computer.HypervisorPresent};windows=@{name=$os.Caption;version=$os.Version;build=$os.BuildNumber;architecture=$os.OSArchitecture;installedUtc=(Hardware-Date $os.InstallDate);lastBootUtc=(Hardware-Date $os.LastBootUpTime);usableMemoryBytes=$usable};cpu=$cpu;memory=$memory;motherboard=$board;bios=$bios;gpu=$gpu;disks=$disks;volumes=$volumes;network=$network;audio=$audio;batteries=$batteries;unavailable=@($script:hardwareIssues);limitations=@('Informations déclarées par Windows/SMBIOS ; certaines valeurs peuvent être absentes ou inexactes.',"La VRAM dédiée n'est pas mesurée : AdapterRAM WMI peut tronquer les capacités supérieures à 4 Gio.","Les capteurs de température et les tensions ne sont pas interrogés. La présence d'un hyperviseur ne prouve pas qu'un réglage de virtualisation est modifiable.")}
}
if($MyInvocation.InvocationName -ne '.'){
 [Console]::OutputEncoding=New-Object Text.UTF8Encoding($false)
 try{
  if($PayloadBase64){$json=[Text.Encoding]::UTF8.GetString([Convert]::FromBase64String($PayloadBase64));if($json -notmatch '^\s*\{\s*\}\s*$'){throw 'PayloadNotSupported'}}
  @{ok=$true;data=(Get-HardwareInfo)}|ConvertTo-Json -Depth 8 -Compress
 }catch{@{ok=$false;data=$null;error='HardwareInventoryUnavailable'}|ConvertTo-Json -Compress}
}
