# Deterministic Windows boundary tests; no real maintenance command is executed.
$ErrorActionPreference='Stop'
$module=Join-Path $PSScriptRoot '../src-tauri/scripts/optimize.ps1'
$tokens=$null; $errors=$null
$ast=[Management.Automation.Language.Parser]::ParseFile($module,[ref]$tokens,[ref]$errors)
if ($errors.Count) { throw 'Optimize syntax errors' }
. $module
$script:checks=0
function Assert($Condition,[string]$Message) { if (-not $Condition) { throw $Message }; $script:checks++ }
$script:admin=$false; $script:calls=@(); $script:dns=0; $script:fail=$false; $script:available=$true
function Test-OptimizeAdministrator { return $script:admin }
function Invoke-OptimizeNative([string]$Name,[string[]]$Arguments) {
 $script:calls+=,@{name=$Name;arguments=$Arguments}
 if ($script:fail) { throw 'NativeCommandFailed' }
 $lines=if ($script:available) { @('a1841308-3541-4fab-bc81-f71556f20b4a') } else { @() }
 return @{exitCode=0;rebootRequired=$false;lines=$lines}
}
function Clear-DnsClientCache { param($ErrorAction); $script:dns++ }
function Get-CimInstance { param($ClassName,$Filter); throw 'unavailable' }
function Get-PhysicalDisk { param($ErrorAction); throw 'unavailable' }
function Get-NetAdapter { param($ErrorAction); throw 'unavailable' }
function Get-WinEvent { param($FilterHashtable,$MaxEvents,$ErrorAction); throw 'unavailable' }
function Get-Process { @([pscustomobject]@{},[pscustomobject]@{}) }
$root=Join-Path (Split-Path $PSScriptRoot -Parent) ('work\optimize-tests-'+[guid]::NewGuid().ToString('N'))
Assert (-not (Test-Path -LiteralPath $root)) 'Scratch root must be unique'
$protected=@('dism-analyze','dism-cleanup','system-check','disk-scan','dns-flush','energy-saver','hibernate-on','hibernate-off','power-report','storage-analyze','storage-optimize','power-requests','wake-sources')
foreach ($action in $protected) {
 $r=Invoke-OptimizeAction $action '' $root
 Assert ($r.error -eq 'AdministratorRequired' -and -not $r.ok) "Missing admin guard: $action"
}
Assert ($script:calls.Count -eq 0 -and $script:dns -eq 0) 'Admin rejection dispatched work'
$r=Invoke-OptimizeAction 'dns-flush' 'eyJleHRyYSI6dHJ1ZX0=' $root
Assert ($r.error -eq 'PayloadNotSupported') 'Payload must be rejected'
$r=Invoke-OptimizeAction 'dns-flush' 'e30=' $root
Assert ($r.error -eq 'AdministratorRequired') 'Empty JSON object must reach the admin guard'
$r=Invoke-OptimizeAction 'enable-schedule' '' $root
Assert ($r.error -eq 'InvalidAction') 'Schedule must not dispatch'
$r=Invoke-OptimizeAction 'opti-audit' '' $root
Assert ($r.ok -and $null -eq $r.data.physicalDisks -and $null -eq $r.data.networkAdapters) 'Unavailable hardware must be graceful'
Assert ($null -eq $r.data.sample.bootDurationMs -and $null -eq $r.data.sample.cpuLoadPercent) 'Unavailable sample must be null'
Assert ($r.data.sample.processCount -eq 2) 'Process count mismatch'
Assert ($r.data.activePowerScheme -eq 'a1841308-3541-4fab-bc81-f71556f20b4a') 'Power GUID missing'
Assert (-not (Test-Path -LiteralPath $root)) 'Read-only audit wrote files'
function Get-PhysicalDisk { param($ErrorAction); [pscustomobject]@{MediaType='SSD';HealthStatus='Healthy';OperationalStatus='OK';Size=1000;SerialNumber='SECRET-SERIAL';FriendlyName='PRIVATE'} }
function Get-StorageReliabilityCounter { param([Parameter(ValueFromPipeline=$true)]$Disk); process { [pscustomobject]@{Temperature=35;Wear=2;SerialNumber='SECRET-SERIAL'} } }
function Get-NetAdapter { param($ErrorAction); [pscustomobject]@{ifIndex=1;Status='Up';LinkSpeed='1 Gbps';MacAddress='SECRET-MAC';Name='PRIVATE'} }
function Get-WinEvent {
 param($FilterHashtable,$MaxEvents,$ErrorAction)
 $event=New-Object PSObject
 $event | Add-Member ScriptMethod ToXml { '<Event><EventData><Data Name="BootTime">12345</Data></EventData></Event>' }
 return $event
}
$r=Invoke-OptimizeAction 'opti-audit' '' $root
Assert ($r.ok -and $r.data.sample.bootDurationMs -eq 12345 -and $r.data.storageReliability[0].Temperature -eq 35) 'Available boot/reliability not collected'
$auditJson=$r | ConvertTo-Json -Depth 8 -Compress
Assert ($auditJson -notmatch 'SECRET|PRIVATE|SerialNumber|MacAddress') 'Audit leaked identifying fields'
$script:admin=$true
foreach ($action in @('dism-analyze','dism-cleanup','system-check','disk-scan','dns-flush','energy-saver','hibernate-on','hibernate-off','power-report','battery-report')) {
 $r=Invoke-OptimizeAction $action '' $root
 Assert $r.ok "Mock action failed: $action ($($r.error))"
 if ($action -eq 'hibernate-off') { Assert ($r.data.explanation -match 'Fast Startup' -and $r.data.reversible) 'Hibernation explanation missing' }
 if ($action -eq 'power-report') { Assert ($r.data.reportPath -eq (Join-Path $root 'reports\energy.html')) 'Energy path escaped root' }
 if ($action -eq 'battery-report') { Assert ($r.data.reportPath -eq (Join-Path $root 'reports\battery.html')) 'Battery path escaped root' }
}
Assert ($script:dns -eq 1) 'DNS mock not dispatched'
$commands=@($script:calls | ForEach-Object { $_.name+' '+($_.arguments -join ' ') })
Assert (@($commands | Where-Object { $_ -match '/ResetBase|/RestoreHealth|/scannow|/f\b|/r\b' }).Count -eq 0) 'Unexpected repair flag'
foreach ($expected in @('dism.exe /Online /Cleanup-Image /AnalyzeComponentStore','dism.exe /Online /Cleanup-Image /StartComponentCleanup','dism.exe /Online /Cleanup-Image /CheckHealth','sfc.exe /verifyonly',('chkdsk.exe '+$env:SystemDrive+' /scan'),'powercfg.exe /hibernate off','powercfg.exe /hibernate on')) {
 Assert ($commands -contains $expected) "Missing fixed command $expected"
}
Assert (@($commands | Where-Object { $_ -like 'powercfg.exe /energy /duration 15 /output *' }).Count -eq 1) 'Energy command incorrect'
$script:available=$false; $before=$script:calls.Count
$r=Invoke-OptimizeAction 'energy-saver' '' $root
Assert ($r.error -eq 'PowerSchemeUnavailable' -and $script:calls.Count -eq $before+1) 'Unavailable power scheme was activated'
$script:fail=$true
$r=Invoke-OptimizeAction 'dism-cleanup' '' $root
Assert ($r.error -eq 'NativeCommandFailed' -and $null -eq $r.data) 'Native error not contained'
$script:fail=$false
foreach ($badRoot in @('', 'relative','C:\','C:\safe\..\escape','\\server\share','C:relative')) {
 $r=Invoke-OptimizeAction 'snapshot-baseline' '' $badRoot
 Assert ($r.error -eq 'InvalidDataRoot') "Accepted unsafe root $badRoot"
}
$r=Invoke-OptimizeAction 'snapshot-compare' '' $root
Assert ($r.error -eq 'BaselineMissing') 'Missing baseline not handled'
function Get-OptimizeSample { return [ordered]@{timestampUtc='2026-10-25T12:00:00Z';freeMemoryBytes=100;cpuLoadPercent=20;processCount=10;bootDurationMs=$null;freeDiskBytes=200} }
$r=Invoke-OptimizeAction 'snapshot-baseline' '' $root
Assert ($r.ok -and $r.data.baseline.timestampUtc -eq '2026-10-25T12:00:00Z') 'Baseline not persisted'
function Get-OptimizeSample { return [ordered]@{timestampUtc='2026-10-25T13:00:00Z';freeMemoryBytes=120;cpuLoadPercent=15;processCount=12;bootDurationMs=$null;freeDiskBytes=195} }
$r=Invoke-OptimizeAction 'snapshot-compare' '' $root
Assert ($r.ok -and $r.data.delta.freeMemoryBytes -eq 20 -and $r.data.delta.cpuLoadPercent -eq -5 -and $r.data.delta.freeDiskBytes -eq -5) 'Numeric deltas incorrect'
Assert ($null -eq $r.data.delta.bootDurationMs -and $r.data.caveat -match 'do not prove' -and $r.data.baselineTimestampUtc -eq '2026-10-25T12:00:00Z') 'Comparison caveat/timestamp missing'
[IO.File]::WriteAllText((Join-Path $root 'maintenance\baseline.json'),'{"timestampUtc":"bad"}')
$r=Invoke-OptimizeAction 'snapshot-compare' '' $root
Assert ($r.error -eq 'InvalidBaseline') 'Corrupt baseline accepted'
# A junction under the trusted root must not redirect writes outside it.
$junctionRoot=Join-Path $root 'junction-root'
$outside=Join-Path $root 'outside'
[void][IO.Directory]::CreateDirectory($junctionRoot)
[void][IO.Directory]::CreateDirectory($outside)
[void](New-Item -ItemType Junction -Path (Join-Path $junctionRoot 'maintenance') -Target $outside)
$r=Invoke-OptimizeAction 'snapshot-baseline' '' $junctionRoot
Assert ($r.error -eq 'UnsafeDataPath' -and -not (Test-Path -LiteralPath (Join-Path $outside 'baseline.json'))) 'Junction escaped trusted root'
# Separate thread owns mutex; same-thread mutex acquisition would be reentrant.
$ready=New-Object Threading.ManualResetEvent($false)
$release=New-Object Threading.ManualResetEvent($false)
$worker=[powershell]::Create()
[void]$worker.AddScript({param($Ready,$Release) $m=New-Object Threading.Mutex($false,'Global\NovaPCSuite.Maintenance'); try { [void]$m.WaitOne(); [void]$Ready.Set(); [void]$Release.WaitOne(10000) } finally { $m.ReleaseMutex(); $m.Dispose() }}).AddArgument($ready).AddArgument($release)
$pending=$worker.BeginInvoke()
try {
 Assert ($ready.WaitOne(5000)) 'Mutex fixture failed'
 $before=$script:dns; $r=Invoke-OptimizeAction 'dns-flush' '' $root
 Assert ($r.error -eq 'MaintenanceBusy' -and $script:dns -eq $before) 'Busy mutex dispatched mutation'
} finally { [void]$release.Set(); $worker.EndInvoke($pending); $worker.Dispose(); $ready.Dispose(); $release.Dispose() }
$r=Invoke-OptimizeAction 'dns-flush' '' $root
Assert $r.ok 'Mutex not released after failure'
$json=$r | ConvertTo-Json -Depth 8 -Compress | ConvertFrom-Json
Assert ($json.ok -and $null -eq $json.error -and $json.data.flushed) 'Envelope invalid'
# New storage actions use mocks only, including the volume query.
$script:volumeHealth='Healthy';$script:storageCalls=@()
function Get-Volume {param($DriveLetter,$ErrorAction);[pscustomobject]@{FileSystem='NTFS';DriveType='Fixed';HealthStatus=$script:volumeHealth}}
function Optimize-Volume {
 [CmdletBinding()]param([char[]]$DriveLetter,[switch]$Analyze)
 $script:storageCalls+=,@{letter=[string]$DriveLetter[0];analyze=[bool]$Analyze;parameters=@($PSBoundParameters.Keys)}
 Write-Verbose 'Fixture: volume analysis / operation completed'
}
$r=Invoke-OptimizeAction 'storage-analyze' 'e30=' $root
Assert ($r.ok -and $r.data.readOnly -and $r.data.analysisOnly -and $r.data.output -match 'Fixture') 'Storage analysis not read-only or missing diagnostics'
$r=Invoke-OptimizeAction 'storage-optimize' 'e30=' $root
Assert ($r.ok -and -not $r.data.readOnly -and -not $r.data.analysisOnly -and $r.data.completed) 'Storage optimization dispatch failed'
Assert ($script:storageCalls.Count -eq 2 -and $script:storageCalls[0].analyze -and -not $script:storageCalls[1].analyze) 'Analysis and optimization flags confused'
Assert ($script:storageCalls[1].letter -eq $env:SystemDrive[0].ToString() -and $script:storageCalls[1].parameters -notcontains 'Defrag' -and $script:storageCalls[1].parameters -notcontains 'ReTrim' -and $script:storageCalls[1].parameters -notcontains 'NormalPriority') 'Windows media choice or low priority overridden'
$script:volumeHealth='Warning'
$r=Invoke-OptimizeAction 'storage-optimize' '' $root
Assert ($r.error -eq 'StorageVolumeUnsupported' -and $script:storageCalls.Count -eq 2) 'Unhealthy storage must not be optimized'
$script:volumeHealth='Healthy'
$r=Invoke-OptimizeAction 'storage-optimize' 'eyJEcml2ZUxldHRlciI6IkQifQ==' $root
Assert ($r.error -eq 'PayloadNotSupported' -and $script:storageCalls.Count -eq 2) 'Arbitrary target accepted'
foreach($action in @('power-requests','sleep-states','wake-sources')){
 $r=Invoke-OptimizeAction $action '' $root
 Assert ($r.ok -and $r.data.readOnly -and $r.data.output.Length -gt 0) "Missing power diagnostics $action"
}
$commands=@($script:calls|ForEach-Object{$_.name+' '+($_.arguments -join ' ')})
foreach($expected in @('powercfg.exe /requests','powercfg.exe /availablesleepstates','powercfg.exe /lastwake','powercfg.exe /waketimers')){Assert ($commands -contains $expected) "Missing fixed diagnostic $expected"}
function Get-WinEvent {
 param($FilterHashtable,$MaxEvents,$ErrorAction)
 Assert ($MaxEvents -eq 30 -and $FilterHashtable.Id.Count -eq 3 -and $FilterHashtable.StartTime -is [datetime]) 'Boot query must be bounded'
 $event=[pscustomobject]@{Id=101;TimeCreated=[datetime]'2026-10-25T12:00:00Z'}
 $event|Add-Member ScriptMethod ToXml {'<Event><EventData><Data Name="Name">Fixture.exe</Data><Data Name="TotalTime">1234</Data><Data Name="DegradationTime">567</Data><Data Name="CommandLine">SECRET-ARGS</Data></EventData></Event>'}
 $event
}
$r=Invoke-OptimizeAction 'boot-slowdowns' '' $root
Assert ($r.ok -and $r.data.count -eq 1 -and $r.data.items[0].durationMs -eq 1234 -and $r.data.items[0].degradationMs -eq 567) 'Boot event extraction failed'
Assert (($r|ConvertTo-Json -Depth 8) -notmatch 'SECRET-ARGS|CommandLine') 'Boot data includes extra event fields'
function Get-WinEvent {param($FilterHashtable,$MaxEvents,$ErrorAction);throw 'Access denied fixture'}
$r=Invoke-OptimizeAction 'boot-slowdowns' '' $root
Assert (-not $r.ok -and $r.error -eq 'BootDiagnosticsUnavailable') 'Access failure must not look like a healthy empty journal'
function Get-WinEvent {param($FilterHashtable,$MaxEvents,$ErrorAction);Write-Error 'No events fixture' -ErrorId NoMatchingEventsFound -ErrorAction Stop}
$r=Invoke-OptimizeAction 'boot-slowdowns' '' $root
Assert ($r.ok -and $r.data.count -eq 0 -and $r.data.available) 'Empty journal should be distinguished from access failure'
Write-Output "PASS: $script:checks checks; mocked OS commands only. Scratch fixtures: $root"
