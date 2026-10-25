$ErrorActionPreference='Stop'
$path=Join-Path $PSScriptRoot '../src-tauri/scripts/automation.ps1'
$global:novaTestActions=@();$global:novaTestExisting=$null;$global:novaTestUri=''
function Get-ScheduledTask {param($TaskName,$ErrorAction) $global:novaTestExisting}
function Get-ScheduledTaskInfo {param($TaskName) @{NextRunTime=(Get-Date);LastTaskResult=0}}
function New-ScheduledTaskAction {param($Execute,$Argument) if($Argument -ne '--scheduled-scan'){throw 'Unexpected arguments'};@{Execute=$Execute;Argument=$Argument}}
function New-ScheduledTaskTrigger {param([switch]$Daily,[switch]$Weekly,$DaysOfWeek,$At) @{daily=[bool]$Daily;weekly=[bool]$Weekly;day=$DaysOfWeek;time=$At}}
function New-ScheduledTaskPrincipal {param($UserId,$LogonType,$RunLevel) if($LogonType -ne 'Interactive' -or $RunLevel -ne 'Limited'){throw 'Unexpected privilege'};@{user=$UserId}}
function New-ScheduledTaskSettingsSet {param([switch]$StartWhenAvailable,$MultipleInstances,$ExecutionTimeLimit) @{configured=$true}}
function Register-ScheduledTask {param($TaskName,$Description,$Action,$Trigger,$Principal,$Settings,[switch]$Force) $global:novaTestActions+=@{name=$TaskName;action=$Action;trigger=$Trigger}}
function Unregister-ScheduledTask {param($TaskName,$Confirm) $global:novaTestActions+='removed'}
function Start-Process {param($FilePath) $global:novaTestUri=$FilePath}
function Invoke-RestMethod {param($Uri,$Headers,$TimeoutSec,$MaximumRedirection) @([pscustomobject]@{draft=$false;tag_name='v0.2.0-preview';published_at='2026-10-25';prerelease=$true})}
function Assert($ok,$why){if(-not $ok){throw $why}}
$root=Join-Path (Split-Path $PSScriptRoot -Parent) ('work/automation-tests-'+[guid]::NewGuid().ToString('N'))
$null=New-Item -ItemType Directory -Path $root
$exe=Join-Path $PSHOME 'powershell.exe'
foreach($action in @('schedule-daily','schedule-weekly')){$r=& $path -Action $action -DataRoot $root -Executable $exe|ConvertFrom-Json;Assert $r.ok "Schedule mock failed $action"}
Assert ($global:novaTestActions.Count -eq 2) 'Schedules not registered through mocks'
Assert ($global:novaTestActions[0].trigger.daily -and $global:novaTestActions[1].trigger.weekly) 'Incorrect triggers'
$global:novaTestExisting=@{Description='Other application'}
$r=& $path -Action schedule-disable -DataRoot $root -Executable $exe|ConvertFrom-Json
Assert (-not $r.ok -and $r.error -eq 'ScheduleOwnershipConflict') 'Foreign task touched'
$r=& $path -Action check-release -DataRoot $root -Executable $exe|ConvertFrom-Json
Assert ($r.ok -and $r.data.tag -eq 'v0.2.0-preview') 'Release format'
$payload=[Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes('{"id":"intel"}'))
$r=& $path -Action open-source -PayloadBase64 $payload -DataRoot $root -Executable $exe|ConvertFrom-Json
Assert ($r.ok -and $global:novaTestUri -like 'https://www.intel.com/*') 'Wrong official source'
$payload=[Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes('{"id":"https://attacker.invalid"}'))
$r=& $path -Action open-source -PayloadBase64 $payload -DataRoot $root -Executable $exe|ConvertFrom-Json
Assert (-not $r.ok) 'Arbitrary website accepted'
$r=& $path -Action notifications-on -DataRoot $root -Executable $exe|ConvertFrom-Json
Assert ($r.ok -and (Test-Path -LiteralPath (Join-Path $root 'notifications-enabled'))) 'Notification preference not saved'
$r=& $path -Action notifications-off -DataRoot $root -Executable $exe|ConvertFrom-Json
Assert ($r.ok -and -not (Test-Path -LiteralPath (Join-Path $root 'notifications-enabled'))) 'Notification preference not cleared'
Write-Output 'Automation tests PASS: mocked scheduler/network/browser, scratch preferences only.'
$global:novaTestExisting=@{Description='Nova PC Suite read-only audit v1';Actions=@(@{Arguments='--scheduled-scan'})}
$beforeCount=$global:novaTestActions.Count
& (Join-Path $PSScriptRoot '../scripts/uninstall-task.ps1')
Assert ($global:novaTestActions.Count -eq $beforeCount+1) 'Owned task must be removed by uninstall hook'
$global:novaTestExisting=@{Description='Another task';Actions=@(@{Arguments='--scheduled-scan'})}
$beforeCount=$global:novaTestActions.Count
& (Join-Path $PSScriptRoot '../scripts/uninstall-task.ps1')
Assert ($global:novaTestActions.Count -eq $beforeCount) 'Uninstall hook must preserve foreign task'
