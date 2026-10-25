$ErrorActionPreference='Stop'
$source=Join-Path $PSScriptRoot '../src-tauri/scripts/hardware.ps1'
$tokens=$null;$errors=$null
[void][Management.Automation.Language.Parser]::ParseFile($source,[ref]$tokens,[ref]$errors)
if($errors.Count){throw 'Hardware script syntax failure'}
. $source
$script:queries=@();$script:denyGpu=$false
function Get-CimInstance {
 param($ClassName,$Property,$ErrorAction)
 $script:queries+=,@{class=$ClassName;properties=$Property}
 if($ClassName -eq 'Win32_VideoController' -and $script:denyGpu){throw 'Access denied fixture'}
 switch($ClassName){
  'Win32_ComputerSystem'{[pscustomobject]@{Manufacturer='Fixture';Model='Model PC';TotalPhysicalMemory=17179869184;NumberOfLogicalProcessors=12;HypervisorPresent=$false;Name='PRIVATE-PC';UserName='PRIVATE-USER'}}
  'Win32_OperatingSystem'{[pscustomobject]@{Caption='Windows fixture';Version='10.0';BuildNumber='12345';OSArchitecture='64-bit';InstallDate=[datetime]'2025-01-01';LastBootUpTime=[datetime]'2026-10-25';TotalVisibleMemorySize=15728640;SerialNumber='SECRET-OS'}}
  'Win32_Processor'{[pscustomobject]@{Name='Fixture CPU';NumberOfCores=6;NumberOfLogicalProcessors=12;ProcessorId='SECRET-CPU'}}
  'Win32_PhysicalMemory'{[pscustomobject]@{Capacity=8589934592;ConfiguredClockSpeed=3200;SMBIOSMemoryType=26;SerialNumber='SECRET-RAM'}}
  'Win32_VideoController'{[pscustomobject]@{Name='Fixture GPU';CurrentRefreshRate=1;AdapterRAM=4294967295;PNPDeviceID='SECRET-GPU'}}
  'Win32_NetworkAdapter'{[pscustomobject]@{Name='Fixture Ethernet';PhysicalAdapter=$true;NetEnabled=$false;MACAddress='SECRET-MAC'}}
  'Win32_LogicalDisk'{[pscustomobject]@{DriveType=3;DeviceID='C:';Size=1000;FreeSpace=200}}
 }
}
function Assert($ok,$why){if(-not $ok){throw $why}}
$data=Get-HardwareInfo
Assert ($data.cpu[0].cores -eq 6 -and $data.memory[0].capacityBytes -eq 8589934592) 'CPU/RAM projections incorrect'
Assert ($data.windows.usableMemoryBytes -eq 16106127360 -and $data.computer.installedMemoryBytes -eq 17179869184) 'Usable and physical RAM must remain separate'
Assert ($null -eq $data.gpu[0].refreshHz) 'Default refresh sentinel must not become 1 Hz'
Assert ($data.network[0].enabled -eq $false -and $data.volumes[0].freeBytes -eq 200) 'False/zero-like fields lost'
$json=$data|ConvertTo-Json -Depth 8
Assert ($json -notmatch 'SECRET|PRIVATE|"AdapterRAM"\s*:|SerialNumber|ProcessorId|PNPDeviceID|MACAddress') 'Unrequested identifying fields escaped the projection'
Assert ($script:queries.Count -eq 12 -and $data.unavailable.Count -eq 0 -and $data.batteries.Count -eq 0) 'Class inventory incomplete'
foreach($query in $script:queries){Assert ($query.properties -notcontains 'SerialNumber' -and $query.properties -notcontains 'MACAddress') 'Queries must select safe properties'}
$script:denyGpu=$true
$data=Get-HardwareInfo
Assert ($data.unavailable -contains 'Win32_VideoController' -and $data.gpu.Count -eq 0 -and $data.cpu.Count -eq 1) 'Partial inventory failure must preserve remaining sections'
$r=& $source -PayloadBase64 eyJleHRyYSI6dHJ1ZX0=|ConvertFrom-Json
Assert (-not $r.ok) 'Unexpected payload accepted'
Write-Output 'Hardware tests PASS: projected fields, partial inventory, unknown refresh, private-field exclusion; CIM mocked.'
