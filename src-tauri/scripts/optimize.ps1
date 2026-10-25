[CmdletBinding()]
param(
 [ValidateSet('opti-audit','dism-analyze','dism-cleanup','system-check','disk-scan','dns-flush','energy-saver','hibernate-on','hibernate-off','power-report','battery-report','snapshot-baseline','snapshot-compare','storage-analyze','storage-optimize','boot-slowdowns','power-requests','sleep-states','wake-sources')]
 [string]$Action='opti-audit', [string]$PayloadBase64='', [string]$DataRoot
)
# Rust must supply its trusted app-data directory. Path syntax is not authorization.
# No action accepts payload fields. No schedule or memory-diagnostic launch.
$ErrorActionPreference='Stop'
$ProgressPreference='SilentlyContinue'
$WarningPreference='SilentlyContinue'
$InformationPreference='SilentlyContinue'
function Test-OptimizeAdministrator {
 $id=[Security.Principal.WindowsIdentity]::GetCurrent()
 try { $p=New-Object Security.Principal.WindowsPrincipal($id); return $p.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator) } finally { $id.Dispose() }
}
function Test-OptimizePayload([string]$Payload) {
 if ($Payload -eq '') { return }
 if ($Payload.Length -gt 1024) { throw 'PayloadNotSupported' }
 try {
  $bytes=[Convert]::FromBase64String($Payload)
  if ($bytes.Length -gt 512) { throw 'PayloadNotSupported' }
  $utf8=New-Object Text.UTF8Encoding($false,$true)
  $json=$utf8.GetString($bytes)
  $object=ConvertFrom-Json -InputObject $json -ErrorAction Stop
  if ($json -notmatch '^\s*\{\s*\}\s*$' -or $object -isnot [pscustomobject] -or @($object.PSObject.Properties).Count -ne 0) { throw 'PayloadNotSupported' }
 } catch { throw 'PayloadNotSupported' }
}
function New-OptimizeMutex { return New-Object Threading.Mutex($false,'Global\NovaPCSuite.Maintenance') }
function Invoke-OptimizeProcess([string]$Name,[string[]]$Arguments) {
 $binary=Join-Path ([Environment]::GetFolderPath('System')) $Name
 if (-not [IO.File]::Exists($binary)) { throw 'NativeCommandUnavailable' }
 # Drain both streams to completion, retaining at most 24 KiB of UTF-16 text.
 # Never read DISM/SFC logs. Continue permits Windows PowerShell stderr records.
 $ErrorActionPreference='Continue'
 $PSNativeCommandUseErrorActionPreference=$false
 $buffer=New-Object Text.StringBuilder
 $truncated=$false
 & $binary @Arguments 2>&1 | ForEach-Object {
  $line=[string]$_+[Environment]::NewLine
  $remaining=24576-$buffer.Length
  if ($line.Length -gt $remaining) { $truncated=$true }
  if ($remaining -gt 0) { [void]$buffer.Append($line.Substring(0,[Math]::Min($line.Length,$remaining))) }
 }
 $code=$LASTEXITCODE
 return @{exitCode=$code;text=$buffer.ToString();truncated=$truncated}
}
function Invoke-OptimizeNative([string]$Name,[string[]]$Arguments) {
 if ($Name -notin @('powercfg.exe','dism.exe','sfc.exe','chkdsk.exe')) { throw 'InvalidCommand' }
 $capture=Invoke-OptimizeProcess $Name $Arguments
 $code=$capture.exitCode
 $reboot=($Name -eq 'dism.exe' -and $code -eq 3010)
 $corrected=($Name -eq 'chkdsk.exe' -and $code -eq 1)
 $success=($code -eq 0 -or $reboot -or $corrected)
 $status=if ($corrected) { 'ErrorsCorrected' } elseif ($reboot) { 'RebootRequired' } elseif ($code -eq 0) { 'Completed' } else { 'Failed' }
 $text=[string]$capture.text
 $truncated=($capture.truncated -or $text.Length -gt 24576)
 if ($text.Length -gt 24576) { $text=$text.Substring(0,24576) }
 # Retain localized diagnostic text, excluding terminal control characters.
 $text=[regex]::Replace($text,'[\x00-\x08\x0B\x0C\x0E-\x1F\x7F]','')
 $result=@{command=$Name;exitCode=$code;rebootRequired=$reboot;success=$success;status=$status;lines=@($text -split '\r?\n')}
 if ($Name -in @('dism.exe','sfc.exe','chkdsk.exe')) {
  $result.diagnostics=@{text=$text;truncated=$truncated;maxCharacters=24576;privacy='Local diagnostic output may contain paths or account identifiers. Do not upload or log it; no log files were read.'}
 }
 if (-not $success) {
  $failure=New-Object InvalidOperationException('NativeCommandFailed')
  $public=@{}; foreach ($key in $result.Keys) { if ($key -ne 'lines') { $public[$key]=$result[$key] } }
  $failure.Data['NativeResult']=$public
  throw $failure
 }
 return $result
}
function Get-OptimizePath([string]$Root,[string]$Relative) {
 if ([string]::IsNullOrWhiteSpace($Root) -or $Root -notmatch '^[A-Za-z]:[\\/]') { throw 'InvalidDataRoot' }
 $canonical=[IO.Path]::GetFullPath($Root).TrimEnd('\','/')
 if ($canonical.Length -le 3 -or $canonical -ine $Root.TrimEnd('\','/')) { throw 'InvalidDataRoot' }
 if ($Relative -notin @('reports\energy.html','reports\battery.html','maintenance\baseline.json')) { throw 'InvalidPath' }
 $target=[IO.Path]::GetFullPath((Join-Path $canonical $Relative)); $cursor=$target
 # Reject every existing reparse ancestor and existing target before writes.
 while ($cursor) {
  if (Test-Path -LiteralPath $cursor) {
   if (((Get-Item -LiteralPath $cursor -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { throw 'UnsafeDataPath' }
  }
  $parent=[IO.Path]::GetDirectoryName($cursor)
  if ($parent -eq $cursor) { break }; $cursor=$parent
 }
 return $target
}
function Get-OptimizeBoot {
 try {
  $event=Get-WinEvent -FilterHashtable @{LogName='Microsoft-Windows-Diagnostics-Performance/Operational';Id=100} -MaxEvents 1 -ErrorAction Stop
  [xml]$xml=$event.ToXml()
  $value=@($xml.Event.EventData.Data | Where-Object { $_.Name -eq 'BootTime' })
  if ($value.Count -eq 1) { return [double]$value[0].InnerText }
 } catch { }; return $null
}
function Get-OptimizeSample {
 $memory=$null; $cpu=$null; $count=$null; $free=$null
 try { $os=Get-CimInstance Win32_OperatingSystem; if ($null -ne $os.FreePhysicalMemory) { $memory=[double]$os.FreePhysicalMemory*1024 } } catch { }
 try { $average=(Get-CimInstance Win32_Processor | Measure-Object LoadPercentage -Average).Average; if ($null -ne $average) { $cpu=[double]$average } } catch { }
 try { $count=@(Get-Process).Count } catch { }
 try { $disk=Get-CimInstance Win32_LogicalDisk -Filter "DeviceID='$($env:SystemDrive)'"; if ($null -ne $disk.FreeSpace) { $free=[double]$disk.FreeSpace } } catch { }
 return [ordered]@{timestampUtc=[DateTime]::UtcNow.ToString('o');freeMemoryBytes=$memory;cpuLoadPercent=$cpu;processCount=$count;bootDurationMs=(Get-OptimizeBoot);freeDiskBytes=$free}
}
function Get-OptimizeAudit {
 $power=$null; $disks=$null; $pagefile=$null; $network=$null; $reliability=$null
 try {
  $result=Invoke-OptimizeNative 'powercfg.exe' @('/getactivescheme')
  $match=[regex]::Match(($result.lines -join ' '),'[0-9a-fA-F]{8}-(?:[0-9a-fA-F]{4}-){3}[0-9a-fA-F]{12}')
  if ($match.Success) { $power=$match.Value.ToLowerInvariant() }
 } catch { }
 try {
  $physical=@(Get-PhysicalDisk -ErrorAction Stop)
  $disks=@($physical | Select-Object MediaType,HealthStatus,OperationalStatus,Size)
  try { $reliability=@($physical | Get-StorageReliabilityCounter -ErrorAction Stop | Select-Object Temperature,Wear,ReadErrorsTotal,WriteErrorsTotal,PowerOnHours) } catch { }
 } catch { }
 try { $setting=(Get-CimInstance Win32_ComputerSystem).AutomaticManagedPagefile; if ($null -ne $setting) { $pagefile=[bool]$setting } } catch { }
 try { $network=@(Get-NetAdapter -ErrorAction Stop | Select-Object ifIndex,Status,LinkSpeed) } catch { }
 return @{activePowerScheme=$power;physicalDisks=$disks;storageReliability=$reliability;automaticPagefile=$pagefile;networkAdapters=$network;sample=(Get-OptimizeSample);unavailableMeaning='null means unavailable or access denied'}
}
function Get-OptimizeBootSlowdowns {
 try {
  $events=@(Get-WinEvent -FilterHashtable @{LogName='Microsoft-Windows-Diagnostics-Performance/Operational';Id=@(101,102,103);StartTime=(Get-Date).AddDays(-7)} -MaxEvents 30 -ErrorAction Stop)
 } catch {
  if ($_.FullyQualifiedErrorId -like 'NoMatchingEventsFound*') { return @{items=@();count=0;windowDays=7;available=$true;interpretation='Aucun événement correspondant dans les sept derniers jours ; cela ne prouve pas un démarrage optimal.'} }
  throw 'BootDiagnosticsUnavailable'
 }
 $items=@(foreach($event in $events){
  [xml]$xml=$event.ToXml();$fields=@{}
  foreach($field in @($xml.Event.EventData.Data)){ $fields[[string]$field.Name]=[string]$field.InnerText }
  $name=if($fields['FriendlyName']){$fields['FriendlyName']}else{$fields['Name']}
  if(-not $name){$name='Élément non identifié'}
  $name=$name.Substring(0,[Math]::Min(256,$name.Length))
  $duration=$null;$degradation=$null;[long]$number=0
  if([long]::TryParse($fields['TotalTime'],[ref]$number) -and $number -ge 0){$duration=$number}
  if([long]::TryParse($fields['DegradationTime'],[ref]$number) -and $number -ge 0){$degradation=$number}
  @{name=$name;eventId=[int]$event.Id;timestampUtc=$event.TimeCreated.ToUniversalTime().ToString('o');durationMs=$duration;degradationMs=$degradation}
 })
 return @{items=$items;count=$items.Count;windowDays=7;available=$true;interpretation='Événements Windows récents, pas une preuve de causalité. Examiner les applications connues dans Démarrage ; ne pas désactiver un pilote ou un service sur ce seul résultat.'}
}
function Invoke-OptimizeStorage([bool]$Analyze) {
 $systemVolume=[IO.Path]::GetPathRoot([Environment]::GetFolderPath('System')).TrimEnd('\','/')
 if($systemVolume -notmatch '^[A-Za-z]:$'){throw 'InvalidSystemVolume'}
 $letter=[char]$systemVolume[0]
 $volume=Get-Volume -DriveLetter $letter -ErrorAction Stop
 if($volume.FileSystem -ne 'NTFS' -or [string]$volume.DriveType -notin @('Fixed','3') -or [string]$volume.HealthStatus -notin @('Healthy','0')){throw 'StorageVolumeUnsupported'}
 $buffer=New-Object Text.StringBuilder;$truncated=$false
 # No -Defrag / -ReTrim override: Windows selects the operation for this medium.
 $arguments=@{DriveLetter=$letter;Verbose=$true;ErrorAction='Stop'}
 if($Analyze){$arguments.Analyze=$true}
 Optimize-Volume @arguments 4>&1 | ForEach-Object {
  $line=[string]$_+[Environment]::NewLine;$remaining=24576-$buffer.Length
  if($line.Length -gt $remaining){$truncated=$true}
  if($remaining -gt 0){[void]$buffer.Append($line.Substring(0,[Math]::Min($line.Length,$remaining)))}
 }
 return @{drive=[string]$letter+':';analysisOnly=$Analyze;completed=$true;output=$buffer.ToString();truncated=$truncated;interpretation=$(if($Analyze){'Analyse Windows uniquement. Consulter les statistiques retournées.'}else{'Opération Windows terminée. Le choix dépend du support ; certains supports ne nécessitent aucune action. Aucun gain de performances garanti.'})}
}
function Invoke-OptimizeAction([string]$SelectedAction,[string]$Payload,[string]$Root) {
 $admin=@('dism-analyze','dism-cleanup','system-check','disk-scan','dns-flush','energy-saver','hibernate-on','hibernate-off','power-report','storage-analyze','storage-optimize','power-requests','wake-sources')
 $mutations=@('dism-analyze','system-check','dism-cleanup','disk-scan','dns-flush','energy-saver','hibernate-on','hibernate-off','power-report','battery-report','snapshot-baseline','storage-analyze','storage-optimize')
 $readOnly=@('opti-audit','dism-analyze','system-check','snapshot-compare','storage-analyze','boot-slowdowns','power-requests','sleep-states','wake-sources')
 $allowed=@('opti-audit','snapshot-compare','boot-slowdowns','sleep-states')+$admin+$mutations
 $mutex=$null; $owned=$false
 try {
  if ($SelectedAction -notin $allowed) { throw 'InvalidAction' }
  Test-OptimizePayload $Payload
  if ($SelectedAction -in $admin -and -not (Test-OptimizeAdministrator)) { throw 'AdministratorRequired' }
  if ($SelectedAction -in $mutations) {
   $mutex=New-OptimizeMutex
   try { $owned=$mutex.WaitOne(0) } catch [Threading.AbandonedMutexException] { $owned=$true; throw 'PreviousMaintenanceInterrupted' }
   if (-not $owned) { throw 'MaintenanceBusy' }
  }
  $data=$null; $native=$null
  switch ($SelectedAction) {
   'opti-audit' { $data=Get-OptimizeAudit }
   'boot-slowdowns' { $data=Get-OptimizeBootSlowdowns }
   'storage-analyze' { $data=Invoke-OptimizeStorage $true }
   'storage-optimize' { $data=Invoke-OptimizeStorage $false }
   { $_ -in @('power-requests','sleep-states','wake-sources') } {
    $options=switch($_){'power-requests'{@('/requests')};'sleep-states'{@('/availablesleepstates')};'wake-sources'{@('/lastwake','/waketimers')}}
    $outputs=@(foreach($option in $options){$response=Invoke-OptimizeNative 'powercfg.exe' @($option);$option+[Environment]::NewLine+($response.lines -join [Environment]::NewLine)})
    $data=@{output=$outputs -join ([Environment]::NewLine+[Environment]::NewLine);interpretation='Diagnostic uniquement. Aucun bloqueur de veille ni réveil automatique désactivé.'}
   }
   'dism-analyze' { $native=Invoke-OptimizeNative 'dism.exe' @('/Online','/Cleanup-Image','/AnalyzeComponentStore') }
   'dism-cleanup' { $native=Invoke-OptimizeNative 'dism.exe' @('/Online','/Cleanup-Image','/StartComponentCleanup') }
   'system-check' {
    $dism=Invoke-OptimizeNative 'dism.exe' @('/Online','/Cleanup-Image','/CheckHealth')
    $sfc=Invoke-OptimizeNative 'sfc.exe' @('/verifyonly')
    $data=@{dismExitCode=$dism.exitCode;sfcExitCode=$sfc.exitCode;readOnly=$true;nativeResults=@(@{command=$dism.command;exitCode=$dism.exitCode;status=$dism.status;rebootRequired=$dism.rebootRequired;diagnostics=$dism.diagnostics},@{command=$sfc.command;exitCode=$sfc.exitCode;status=$sfc.status;rebootRequired=$sfc.rebootRequired;diagnostics=$sfc.diagnostics});interpretation='Inspect localized diagnostics for findings; completion alone does not establish health.'}
   }
   'disk-scan' {
    if ($env:SystemDrive -notmatch '^[A-Za-z]:$') { throw 'InvalidSystemVolume' }
    $native=Invoke-OptimizeNative 'chkdsk.exe' @($env:SystemDrive,'/scan')
   }
   'dns-flush' { Clear-DnsClientCache -ErrorAction Stop; $data=@{flushed=$true} }
   'energy-saver' {
    $guid='a1841308-3541-4fab-bc81-f71556f20b4a'
    $plans=Invoke-OptimizeNative 'powercfg.exe' @('/list')
    if (($plans.lines -join ' ') -notmatch $guid) { throw 'PowerSchemeUnavailable' }
    $native=Invoke-OptimizeNative 'powercfg.exe' @('/setactive',$guid)
   }
   { $_ -in @('hibernate-on','hibernate-off') } {
    $setting=if ($_ -eq 'hibernate-on') { 'on' } else { 'off' }
    $native=Invoke-OptimizeNative 'powercfg.exe' @('/hibernate',$setting)
    $data=@{enabled=($setting -eq 'on');reversible=$true;explanation='Hibernation off also disables Fast Startup. Hibernation on restores availability; Fast Startup may need enabling separately.'}
   }
   { $_ -in @('power-report','battery-report') } {
    $relative=if ($_ -eq 'power-report') { 'reports\energy.html' } else { 'reports\battery.html' }
    $path=Get-OptimizePath $Root $relative
    [void][IO.Directory]::CreateDirectory([IO.Path]::GetDirectoryName($path)); $path=Get-OptimizePath $Root $relative
    $arguments=if ($_ -eq 'power-report') { @('/energy','/duration','15','/output',$path) } else { @('/batteryreport','/output',$path) }
    $native=Invoke-OptimizeNative 'powercfg.exe' $arguments
    $data=@{reportPath=$path;privacy='Windows reports may contain identifiers; local file only, report contents not returned.'}
   }
   { $_ -in @('snapshot-baseline','snapshot-compare') } {
    $path=Get-OptimizePath $Root 'maintenance\baseline.json'; $current=Get-OptimizeSample
    if ($_ -eq 'snapshot-baseline') {
     [void][IO.Directory]::CreateDirectory([IO.Path]::GetDirectoryName($path)); $path=Get-OptimizePath $Root 'maintenance\baseline.json'
     [IO.File]::WriteAllText($path,($current | ConvertTo-Json -Compress),(New-Object Text.UTF8Encoding($false)))
     $data=@{baseline=$current}
    } else {
     if (-not [IO.File]::Exists($path)) { throw 'BaselineMissing' }
     if ((Get-Item -LiteralPath $path).Length -gt 16384) { throw 'InvalidBaseline' }
     try { $baseline=[IO.File]::ReadAllText($path) | ConvertFrom-Json; [void][DateTime]::Parse($baseline.timestampUtc) } catch { throw 'InvalidBaseline' }
     $delta=[ordered]@{}
     foreach ($key in @('freeMemoryBytes','cpuLoadPercent','processCount','bootDurationMs','freeDiskBytes')) {
      if ($null -eq $baseline.PSObject.Properties[$key]) { throw 'InvalidBaseline' }
      $before=$baseline.$key; $after=$current[$key]
      if ($null -ne $before -and ($before -is [bool] -or $before -isnot [ValueType])) { throw 'InvalidBaseline' }
      $delta[$key]=if ($null -ne $before -and $null -ne $after) { [double]$after-[double]$before } else { $null }
     }
     $data=@{baseline=$baseline;baselineTimestampUtc=$baseline.timestampUtc;current=$current;delta=$delta;caveat='Point-in-time samples vary with workload; differences do not prove performance gains.'}
    }
   }
  }
  if ($null -eq $data) { $data=@{interpretation='Inspect diagnostics for findings; exit status alone does not prove health.'} }
  if ($null -ne $native) {
   foreach ($key in @('command','exitCode','rebootRequired','status','diagnostics')) { if ($native.ContainsKey($key)) { $data[$key]=$native[$key] } }
  }
  $data.action=$SelectedAction
  $data.readOnly=($SelectedAction -in $readOnly)
  $data.serialized=($SelectedAction -in $mutations)
  return @{ok=$true;data=$data;error=$null}
 } catch {
  $code=[string]$_.Exception.Message
  if ($code -notin @('InvalidAction','PayloadNotSupported','AdministratorRequired','MaintenanceBusy','PreviousMaintenanceInterrupted','InvalidDataRoot','InvalidPath','UnsafeDataPath','InvalidCommand','NativeCommandFailed','NativeCommandUnavailable','PowerSchemeUnavailable','InvalidSystemVolume','BaselineMissing','InvalidBaseline','StorageVolumeUnsupported','BootDiagnosticsUnavailable')) { $code='MaintenanceFailed' }
  $failureData=$null
  if ($code -eq 'NativeCommandFailed' -and $null -ne $_.Exception.Data['NativeResult']) {
   $failureData=$_.Exception.Data['NativeResult']
   $failureData.action=$SelectedAction; $failureData.readOnly=($SelectedAction -in $readOnly); $failureData.serialized=($SelectedAction -in $mutations)
  }
  return @{ok=$false;data=$failureData;error=$code}
 } finally { if ($owned) { $mutex.ReleaseMutex() }; if ($null -ne $mutex) { $mutex.Dispose() } }
}
# Dot sourcing defines functions only, allowing fully mocked boundary tests.
if ($MyInvocation.InvocationName -ne '.') {
 [Console]::OutputEncoding=New-Object Text.UTF8Encoding($false)
 Invoke-OptimizeAction $Action $PayloadBase64 $DataRoot | ConvertTo-Json -Depth 8 -Compress
}
