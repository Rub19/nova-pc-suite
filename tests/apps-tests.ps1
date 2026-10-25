$ErrorActionPreference = 'Stop'
$path = Join-Path $PSScriptRoot '../src-tauri/scripts/apps.ps1'
$tokens = $null; $errors = $null
$ast = [Management.Automation.Language.Parser]::ParseFile($path, [ref]$tokens, [ref]$errors)
if ($errors.Count) { throw 'Backend syntax errors' }
# Load definitions only: never run the backend dispatch or native host mutations.
foreach ($node in $ast.EndBlock.Statements) {
    if ($node -is [Management.Automation.Language.FunctionDefinitionAst]) {
        . ([scriptblock]::Create($node.Extent.Text))
    }
}
function Assert($Condition, $Message) { if (-not $Condition) { throw $Message } }
function Reject($Block, $Code) {
    $caught = $null
    try { & $Block } catch { $caught = $_.Exception.Message }
    Assert ($caught -eq $Code) "Expected rejection: $Code"
}
function Encode($Text) { [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($Text)) }
Reject { Read-AppsPayload 'app-upgrade' (Encode '{"id":"x;calc"}') } 'InvalidId'
Reject { Read-AppsPayload 'app-upgrade' (Encode '{"id":"--all"}') } 'InvalidId'
Reject { Read-AppsPayload 'startup-enable' (Encode '{"id":"../escape"}') } 'InvalidId'
Reject { Read-AppsPayload 'apps-list' (Encode '{"extra":true}') } 'InvalidPayload'
Reject { Read-AppsPayload 'apps-list' (Encode '[]') } 'InvalidPayload'
Reject { Read-AppsPayload 'apps-list' ('A' * 12000) } 'PayloadTooLarge'
Reject { Read-AppsPayload 'apps-list' '!!!' } 'InvalidPayload'
Reject { Read-AppsPayload 'app-upgrade' (Encode '{"id":123}') } 'InvalidId'
Reject { Read-AppsPayload 'startup-disable' (Encode '{}') } 'InvalidPayload'
Reject { Read-AppsPayload 'apps-list' (Encode 'null') } 'InvalidPayload'
Reject { Read-AppsPayload 'app-upgrade' (Encode ('{"id":"' + ('a' * 129) + '"}')) } 'InvalidId'
Assert ((Read-AppsPayload 'app-upgrade' (Encode '{"id":"Vendor.Product-1"}')).id -eq 'Vendor.Product-1') 'Valid package ID'
Assert ($null -ne (Read-AppsPayload 'apps-list' '')) 'Default empty object accepted'
Assert ((Get-StartupId 'test') -eq '9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08') 'Stable UTF8 SHA256'
Assert (Test-ProtectedStartup 'Windows Defender' 'x') 'Protection name excluded'
Assert (Test-ProtectedStartup 'innocent' 'C:\Antivirus\guard.exe') 'Protection command excluded'
Assert (-not (Test-ProtectedStartup 'Example' 'C:\Example\app.exe')) 'Ordinary entry supported'
Assert ((Convert-SafeText "a$([char]27)[31mb$([char]0)c") -eq 'abc') 'Control sequences removed'

# Fake external boundaries; registry and WinGet are never touched by tests.
$script:calls = @()
$script:mutatingCalls = @()
function Invoke-Winget([string[]]$Arguments, [switch]$Mutating) {
    $script:calls += ,$Arguments
    $script:mutatingCalls += [bool]$Mutating
    @{ available = $true; output = 'opaque localized output'; exitCode = $script:wingetExit }
}
$script:wingetExit = 1
Reject { Invoke-AppUpgrade 'Vendor.Product' } 'PackageNotInstalledOrQueryFailed'
Assert ($script:calls.Count -eq 1) 'Failed installed check must prevent upgrade'
$script:calls = @(); $script:mutatingCalls = @(); $script:wingetExit = 0
$result = Invoke-AppUpgrade 'Vendor.Product'
Assert ($script:calls.Count -eq 2) 'Query followed by individual upgrade'
Assert (($script:calls[1] -join ' ') -eq 'upgrade --id Vendor.Product --exact --silent --disable-interactivity') 'Exact individual upgrade arguments'
Assert (-not $script:mutatingCalls[0] -and $script:mutatingCalls[1]) 'Only upgrade is marked mutating and exempt from query timeout'

$script:fakeValues = @{ Example = @{ value = '%LOCALAPPDATA%\Example.exe'; kind = 'ExpandString' } }
function Open-UserRun([bool]$Writable) {
    $key = New-Object PSObject
    $key | Add-Member ScriptMethod GetValueNames { return [string[]]@($script:fakeValues.Keys) }
    $key | Add-Member ScriptMethod GetValue { param($name, $default, $options) return $script:fakeValues[$name].value }
    $key | Add-Member ScriptMethod GetValueKind { param($name) return $script:fakeValues[$name].kind }
    $key | Add-Member ScriptMethod DeleteValue { param($name, $throw) $script:fakeValues.Remove($name) }
    $key | Add-Member ScriptMethod SetValue { param($name, $value, $kind) $script:fakeValues[$name] = @{value=$value;kind=[string]$kind} }
    $key | Add-Member ScriptMethod Dispose { }
    return $key
}
$script:saved = $null
function Write-StartupSnapshot($Root, $Id, $Record) {
    if ($script:failBackup) { throw 'BackupWriteFailed' }
    if ($null -ne $script:saved) { throw 'BackupAlreadyExists' }
    $script:saved = $Record
}
function Read-StartupSnapshot($Root, $Id) { return $script:saved }
$id = Get-StartupId 'Example'
$null = Set-StartupEntry 'unused' $id $false
Assert (-not $script:fakeValues.ContainsKey('Example')) 'Disable removes exact value'
Assert ($script:saved.value -eq '%LOCALAPPDATA%\Example.exe') 'Unexpanded command preserved'
$script:fakeValues.Example = @{value='different';kind='String'}
Reject { Set-StartupEntry 'unused' $id $true } 'StartupConflict'
Assert ($script:fakeValues.Example.value -eq 'different') 'Conflict never overwritten'
$script:fakeValues.Remove('Example')
$null = Set-StartupEntry 'unused' $id $true
Assert ($script:fakeValues.Example.kind -eq 'ExpandString') 'Value kind restored'
Assert ($script:fakeValues.Example.value -eq '%LOCALAPPDATA%\Example.exe') 'Original command restored without execution'
$script:saved.name = 'Other'
Reject { Set-StartupEntry 'unused' $id $true } 'InvalidBackup'
$script:fakeValues = @{ Example = @{value='original';kind='String'} }
$script:saved = $null; $script:failBackup = $true
Reject { Set-StartupEntry 'unused' $id $false } 'BackupWriteFailed'
Assert ($script:fakeValues.Example.value -eq 'original') 'Backup failure preserves active value'
$script:failBackup = $false
$script:fakeValues = @{ Defender = @{value='protection.exe';kind='String'} }
Reject { Set-StartupEntry 'unused' (Get-StartupId 'Defender') $false } 'InvalidBackup'
Assert ($script:fakeValues.ContainsKey('Defender')) 'Protected value never removed'
Reject { Set-StartupEntry 'unused' ('0' * 64) $false } 'StartupNotFound'

function Get-CimInstance { return @([pscustomobject]@{ Name='Other source';Location='Startup';UserSID='other' }) }
function Get-BackupDirectory { return 'unused' }
function Get-ChildItem { return @() }
$script:fakeValues = @{
    Example = @{value='example.exe';kind='String'}
    Defender = @{value='protection.exe';kind='String'}
}
$startup = Get-StartupEntries ''
Assert (@($startup.items).Count -eq 3) 'Run entries and unsupported source listed'
foreach ($entry in $startup.items) {
    Assert ($entry.manageable -is [bool] -and $entry.manageable -eq $entry.supported) 'Manageable aliases supported for every startup source'
}
Assert (($startup.items | Where-Object name -eq 'Example').manageable) 'Ordinary Run entry manageable'
Assert (-not ($startup.items | Where-Object name -eq 'Defender').manageable) 'Protection entry not manageable'
Assert (-not ($startup.items | Where-Object name -eq 'Other source').manageable) 'Other source not manageable'
$script:saved = [pscustomobject]@{version=1;id=(Get-StartupId 'DisabledExample');name='DisabledExample';value='disabled.exe';kind='String'}
function Get-ChildItem { return @([pscustomobject]@{Name=($script:saved.id + '.json')}) }
$startup = Get-StartupEntries 'unused'
$disabled = $startup.items | Where-Object name -eq 'DisabledExample'
Assert ($disabled.manageable -and $disabled.supported -and -not $disabled.enabled) 'Owned disabled backup is manageable'

function Get-Item {
    return [pscustomobject]@{
        Attributes=[IO.FileAttributes]::Directory
        Parent=[pscustomobject]@{Attributes=[IO.FileAttributes]::ReparsePoint;Parent=$null}
    }
}
Reject { Assert-NoReparse 'synthetic-path' } 'UnsafeBackupPath'

# Verify the encryption primitive rejects tampering without writing any files.
Add-Type -AssemblyName System.Security
$plain = [Text.Encoding]::UTF8.GetBytes('synthetic startup snapshot')
$entropy = [Text.Encoding]::UTF8.GetBytes('Nova.Startup.v1')
$cipher = [Security.Cryptography.ProtectedData]::Protect($plain, $entropy, [Security.Cryptography.DataProtectionScope]::CurrentUser)
$decoded = [Security.Cryptography.ProtectedData]::Unprotect($cipher, $entropy, [Security.Cryptography.DataProtectionScope]::CurrentUser)
Assert ([Text.Encoding]::UTF8.GetString($decoded) -eq 'synthetic startup snapshot') 'Per-user backup roundtrip'
$cipher[$cipher.Length - 1] = $cipher[$cipher.Length - 1] -bxor 1
$rejected = $false
try { $null = [Security.Cryptography.ProtectedData]::Unprotect($cipher, $entropy, [Security.Cryptography.DataProtectionScope]::CurrentUser) } catch { $rejected = $true }
Assert $rejected 'Tampered snapshot rejected'

# Safe dispatch smoke tests: rejected payloads return one envelope before any lock/mutation.
$reply = & $path -Action app-upgrade -PayloadBase64 (Encode '{"id":"--all"}') | ConvertFrom-Json
Assert (-not $reply.ok -and $reply.error -eq 'InvalidId' -and $null -eq $reply.data) 'JSON error envelope'
$reply = & $path -Action apps-list -PayloadBase64 (Encode '{"DataRoot":"untrusted"}') | ConvertFrom-Json
Assert (-not $reply.ok -and $reply.error -eq 'InvalidPayload') 'Payload cannot supply trusted DataRoot'
$reply = & $path -Action apps-list | ConvertFrom-Json
Assert ($reply.ok -and $null -eq $reply.error) 'Read-only registry inventory smoke test'
foreach ($item in $reply.data.items) {
    Assert ((@($item.PSObject.Properties.Name | Sort-Object) -join ',') -eq 'name,publisher,version') 'Inventory exposes only display fields'
}

# Static invariants guard the process and persistence boundaries.
$source = [IO.File]::ReadAllText($path)
Assert ($source -notmatch '(?i)Win32_Product|Invoke-Expression|accept-source-agreements|accept-package-agreements') 'Forbidden APIs and automatic agreements absent'
Assert ($source.Contains('[IO.FileMode]::CreateNew')) 'Snapshots cannot be overwritten'
Assert ($source.Contains('$info.UseShellExecute = $false')) 'Native process bypasses shell'
Assert ($source.Contains('if (-not $Mutating -and $clock.ElapsedMilliseconds -gt 120000)')) 'Timeout cannot kill a mutating controller'
Assert ($source.Contains("throw 'PreviousMaintenanceInterrupted'")) 'Abandoned mutex fails explicitly'
Write-Output 'Apps backend tests passed (mocked registry and WinGet; no host mutations).'
