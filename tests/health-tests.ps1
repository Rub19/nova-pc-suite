$ErrorActionPreference='Stop'
$root=Split-Path $PSScriptRoot -Parent
function Assert($ok,$why){if(-not $ok){throw $why}}
foreach($name in @('network','memory','scheduled')){
    $source=Join-Path $root "src-tauri/scripts/$name.ps1"
    $tokens=$null;$errors=$null
    [void][Management.Automation.Language.Parser]::ParseFile($source,[ref]$tokens,[ref]$errors)
    Assert ($errors.Count -eq 0) "Syntax failure $name"
}
$network=Join-Path $root 'src-tauri/scripts/network.ps1'
function Get-NetAdapter {param([switch]$IncludeHidden,$ErrorAction) [pscustomobject]@{Name='Fixture';InterfaceIndex=1;Status='Up';LinkSpeed='1 Gbps';InterfaceDescription='Fixture card'}}
function Get-NetAdapterStatistics {param($ErrorAction) [pscustomobject]@{Name='Fixture';ReceivedBytes=100;SentBytes=50;ReceivedPacketErrors=1;OutboundPacketErrors=0}}
function Get-NetIPConfiguration {param($ErrorAction) [pscustomobject]@{InterfaceAlias='Fixture';InterfaceIndex=1;IPv4Address=@{IPAddress='192.0.2.10'};IPv6Address=$null;IPv4DefaultGateway=@{NextHop='192.0.2.1'};DNSServer=@{ServerAddresses=@('192.0.2.1')}}}
function Get-ItemProperty {param($LiteralPath,$ErrorAction) @{ProxyEnable=0;AutoConfigURL=$null}}
function Get-Process { [pscustomobject]@{Id=42;ProcessName='Fixture'} }
function Get-NetTCPConnection {param($ErrorAction) [pscustomobject]@{OwningProcess=42;LocalAddress='0.0.0.0';LocalPort=1234;RemoteAddress='0.0.0.0';RemotePort=0;State='Listen'}}
$r=& $network -Action network-audit -PayloadBase64 e30=|ConvertFrom-Json
Assert ($r.ok -and $r.data.adapters.Count -eq 1 -and $r.data.findings.Count -eq 1) 'Network audit shape'
function Get-NetAdapterStatistics {param($ErrorAction) throw 'Access denied fixture'}
$r=& $network -Action network-audit -PayloadBase64 e30=|ConvertFrom-Json
Assert ($r.ok -and $null -eq $r.data.statistics -and $r.data.findings.Count -gt 0) 'Unavailable counters must preserve the remaining audit'
$r=& $network -Action network-connections -PayloadBase64 e30=|ConvertFrom-Json
Assert ($r.ok -and $r.data.items[0].process -eq 'Fixture' -and $r.data.listenersAllInterfaces.Count -eq 1) 'Connection attribution'
$bad=[Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes('{"pid":0,"started":"123"}'))
$r=& (Join-Path $root 'src-tauri/scripts/memory.ps1') -Action memory-trim -PayloadBase64 $bad|ConvertFrom-Json
Assert (-not $r.ok -and $r.error -eq 'InvalidMemoryTarget') 'Invalid PID must be rejected before any trim'
$source=[IO.File]::ReadAllText((Join-Path $root 'src-tauri/scripts/memory.ps1'))
Assert ($source.Contains('ForegroundApplicationSkipped') -and $source.Contains('GetProcessTimes') -and $source.Contains('GetOwnerSid')) 'Memory target guard missing'
$source=[IO.File]::ReadAllText($network)
Assert ($source -notmatch '(?i)Set-DnsClientServerAddress|Disable-NetAdapter|Set-NetFirewall|Invoke-Expression') 'Diagnostic changes networking'
Write-Output 'Health diagnostics tests PASS: mocks only; no network probes or RAM trim executed.'
