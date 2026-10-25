# Deterministic tests: never invoke the backend file or a real mutation.
$ErrorActionPreference = 'Stop'
$backend = Join-Path $PSScriptRoot '../src-tauri/scripts/system.ps1'
$tokens = $null
$errors = $null
$ast = [System.Management.Automation.Language.Parser]::ParseFile($backend, [ref]$tokens, [ref]$errors)
if ($errors.Count) { throw 'Backend syntax is invalid.' }

function Assert-SafeCommands($Node, [string[]]$Allowed) {
    $commands = @($Node.FindAll({ param($n) $n -is [System.Management.Automation.Language.CommandAst] }, $true))
    foreach ($command in $commands) {
        if ($command.GetCommandName() -notin $Allowed -or
            $command.InvocationOperator -ne [System.Management.Automation.Language.TokenKind]::Unknown) {
            throw 'Unsafe or unexpected command in extracted test block; refusing execution.'
        }
    }
    # Reject member calls too: a future edit must not introduce COM/native dispatch.
    if (@($Node.FindAll({ param($n) $n -is [System.Management.Automation.Language.InvokeMemberExpressionAst] }, $true)).Count) {
        throw 'Member invocation in extracted test block; refusing execution.'
    }
}

$mainTry = @($ast.EndBlock.Statements | Where-Object { $_ -is [System.Management.Automation.Language.TryStatementAst] })
if ($mainTry.Count -ne 1) { throw 'Expected one backend dispatch try block.' }
$dispatch = @($mainTry[0].Body.Statements | Where-Object { $_ -is [System.Management.Automation.Language.SwitchStatementAst] })
if ($dispatch.Count -ne 1) { throw 'Expected one action switch.' }
$prefix = @($mainTry[0].Body.Statements | Where-Object { $_.Extent.StartOffset -lt $dispatch[0].Extent.StartOffset })
if ($prefix.Count -ne 3) { throw 'Validation/admin-gate structure changed; review tests before executing.' }
foreach ($statement in $prefix[0..1]) { Assert-SafeCommands $statement @('Test-Administrator') }
# Inspect, but never execute, the real mutex constructor. Only the exact known
# lock operations may enter the mock harness.
$adminNode = $prefix[2]
if ($adminNode.Clauses[0].Item1.Extent.Text -ne '$mutations -contains $Action') { throw 'Mutation guard changed.' }
$adminStatements = @($adminNode.Clauses[0].Item2.Statements)
Assert-SafeCommands $adminStatements[0] @('Test-Administrator')
$mutexCommands = @($adminNode.FindAll({ param($n) $n -is [System.Management.Automation.Language.CommandAst] }, $true))
foreach ($command in $mutexCommands) {
    if ($command.Extent.Text -notin @('Test-Administrator', "New-Object System.Threading.Mutex(`$false, 'Global\NovaPCSuite.Maintenance')")) {
        throw 'Unexpected command in mutex guard; refusing execution.'
    }
}
$lockCalls = @($adminNode.FindAll({ param($n) $n -is [System.Management.Automation.Language.InvokeMemberExpressionAst] }, $true))
if ($lockCalls.Count -ne 1 -or $lockCalls[0].Extent.Text -ne '$maintenanceMutex.WaitOne(0)') { throw 'Unexpected mutex acquisition.' }
$finallyNode = $mainTry[0].Finally
if ($null -eq $finallyNode) { throw 'Mutex cleanup must be in outer finally.' }
$cleanupCalls = @($finallyNode.FindAll({ param($n) $n -is [System.Management.Automation.Language.InvokeMemberExpressionAst] }, $true))
if ($cleanupCalls.Count -ne 2 -or $cleanupCalls[0].Extent.Text -ne '$maintenanceMutex.ReleaseMutex()' -or
    $cleanupCalls[1].Extent.Text -ne '$maintenanceMutex.Dispose()' -or
    @($finallyNode.FindAll({ param($n) $n -is [System.Management.Automation.Language.CommandAst] }, $true)).Count) {
    throw 'Unexpected mutex cleanup; refusing execution.'
}
$allReleases = @($ast.FindAll({ param($n)
    $n -is [System.Management.Automation.Language.InvokeMemberExpressionAst] -and $n.Member.Value -in @('ReleaseMutex','Dispose') -and
    $n.Expression.Extent.Text -eq '$maintenanceMutex'
}, $true))
if ($allReleases.Count -ne 2 -or $adminNode.Extent.EndOffset -ge $dispatch[0].Extent.StartOffset -or
    $dispatch[0].Extent.EndOffset -ge $finallyNode.Extent.StartOffset) { throw 'Lock must surround dispatch with cleanup only in finally.' }
$finallyText = $finallyNode.Extent.Text
$cleanup = [scriptblock]::Create($finallyText.Substring(1, $finallyText.Length - 2))
$gate = [scriptblock]::Create(($prefix | ForEach-Object { $_.Extent.Text }) -join "`n")

function Assert-GateFailure([string]$Action, [string[]]$ApprovedIds, [string]$Expected, [bool]$ExpectAdminCheck) {
    $script:adminChecks = 0
    function Test-Administrator { $script:adminChecks++; return $false }
    function New-Object { throw 'Nonadmin must never construct a mutex.' }
    # This contains no dispatch switch, COM creation, or mutation commands.
    $caught = $null
    try { & $gate } catch { $caught = $_.Exception.Message }
    if ($caught -ne $Expected) { throw "Gate assertion failed: expected $Expected" }
    if (($script:adminChecks -gt 0) -ne $ExpectAdminCheck) { throw 'Validation/admin ordering assertion failed.' }
}

$validId = '00000000-0000-0000-0000-000000000001'
Assert-GateFailure 'install-drivers' @() 'ApprovedIdsRequired' $false
Assert-GateFailure 'install-drivers' @('not-a-uuid') 'InvalidApprovedId' $false
Assert-GateFailure 'install-drivers' @('') 'InvalidApprovedId' $false
Assert-GateFailure 'install-drivers' @($validId, 'invalid') 'InvalidApprovedId' $false
Assert-GateFailure 'install-drivers' (@($validId) * 201) 'TooManyApprovedIds' $false
Write-Output 'Approved IDs required, bounded, and validated before admin/COM search: PASS'

foreach ($action in @('install-drivers','restore','repair','trim','energy-balanced','energy-performance')) {
    Assert-GateFailure $action @($validId) 'AdministratorRequired' $true
}
Write-Output 'All mutation actions reject mocked nonadmin before dispatch: PASS'

function Assert-MutexLifetime([bool]$Acquired, [bool]$DispatchThrows) {
    $fake = [pscustomobject]@{ Acquire = $Acquired; Held = $false; Waits = 0; Releases = 0; Disposals = 0 }
    $fake | Add-Member ScriptMethod WaitOne {
        param($Timeout)
        if ($Timeout -ne 0) { throw 'Mutex wait must be nonblocking.' }
        $this.Waits++; $this.Held = $this.Acquire; return $this.Acquire
    }
    $fake | Add-Member ScriptMethod ReleaseMutex {
        if (-not $this.Held) { throw 'Releasing an unowned mutex.' }
        $this.Releases++; $this.Held = $false
    }
    $fake | Add-Member ScriptMethod Dispose { $this.Disposals++ }
    function Test-Administrator { return $true }
    # Shadows constructor entirely: no System.Threading.Mutex is created.
    function New-Object { return $fake }
    $Action = 'restore'
    $ApprovedIds = @()
    $maintenanceMutex = $null
    $ownsMaintenanceMutex = $false
    $dispatched = $false
    $caught = $null
    try {
        . $gate
        if (-not $fake.Held -or $fake.Releases -ne 0 -or $fake.Disposals -ne 0) { throw 'Lock not held at simulated dispatch.' }
        $dispatched = $true
        if ($DispatchThrows) { throw 'SyntheticDispatchFailure' }
    } catch { $caught = $_.Exception.Message }
    finally { . $cleanup }
    if ($fake.Waits -ne 1 -or $fake.Disposals -ne 1 -or $fake.Held) { throw 'Mutex lifecycle failed.' }
    if ($Acquired) {
        if (-not $dispatched -or $fake.Releases -ne 1) { throw 'Acquired lock must survive dispatch and release in finally.' }
        if ($DispatchThrows -and $caught -ne 'SyntheticDispatchFailure') { throw 'Exception cleanup failed.' }
        if (-not $DispatchThrows -and $null -ne $caught) { throw 'Successful cleanup failed.' }
    } elseif ($dispatched -or $fake.Releases -ne 0 -or $caught -ne 'MaintenanceAlreadyRunning') {
        throw 'Busy mutex must block dispatch without releasing another owner.'
    }
}
Assert-MutexLifetime $true $false
Assert-MutexLifetime $true $true
Assert-MutexLifetime $false $false
Write-Output 'Global lock surrounds dispatch; finally releases owned lock on success/error, busy lock blocks dispatch (mocked): PASS'

$restoreClauses = @($dispatch[0].Clauses | Where-Object { $_.Item1.Value -eq 'restore' })
if ($restoreClauses.Count -ne 1) { throw 'Expected one restore branch.' }
$restoreNode = $restoreClauses[0].Item2
Assert-SafeCommands $restoreNode @('Get-ComputerRestorePoint','Checkpoint-Computer','Where-Object','Sort-Object')
$restoreText = $restoreNode.Extent.Text
if (-not $restoreText.StartsWith('{') -or -not $restoreText.EndsWith('}')) { throw 'Unexpected restore block extent.' }
$restoreBlock = [scriptblock]::Create($restoreText.Substring(1, $restoreText.Length - 2))

function Assert-Restore([object[]]$Before, [object[]]$After, [bool]$ShouldSucceed) {
    $script:restoreReads = 0
    $script:checkpointCalls = 0
    $script:beforePoints = $Before
    $script:afterPoints = $After
    function Get-ComputerRestorePoint {
        [CmdletBinding()] param()
        $script:restoreReads++
        if ($script:restoreReads -eq 1) { return $script:beforePoints }
        if ($script:restoreReads -eq 2) { return $script:afterPoints }
        throw 'Unexpected restore inventory call.'
    }
    function Checkpoint-Computer {
        [CmdletBinding()] param([string]$Description, [string]$RestorePointType)
        if ($script:restoreReads -ne 1 -or $Description -ne 'Nova PC Suite restore point' -or $RestorePointType -ne 'MODIFY_SETTINGS') {
            throw 'Checkpoint ordering or arguments changed.'
        }
        $script:checkpointCalls++
    }
    $data = $null
    $caught = $null
    try { . $restoreBlock } catch { $caught = $_.Exception.Message }
    if ($script:restoreReads -ne 2 -or $script:checkpointCalls -ne 1) { throw 'Restore must verify inventory after mocked checkpoint.' }
    if ($ShouldSucceed) {
        if ($null -ne $caught -or -not $data.created) { throw 'Verified restore should succeed.' }
    } elseif ($caught -ne 'RestorePointNotCreated') { throw 'Unverified restore must fail.' }
}

$matching = 'Nova PC Suite restore point'
$old = [pscustomobject]@{ SequenceNumber = 10; Description = $matching }
$older = [pscustomobject]@{ SequenceNumber = 5; Description = 'Other' }
$new = [pscustomobject]@{ SequenceNumber = 11; Description = $matching }
$wrongDescription = [pscustomobject]@{ SequenceNumber = 12; Description = 'Other' }
Assert-Restore @($old, $older) @($old, $older) $false
Assert-Restore @($old) @($wrongDescription) $false
Assert-Restore @($old) @() $false
Assert-Restore @($old, $older) @($old, $new) $true
Assert-Restore @() @($new) $true
Write-Output 'Restore requires a newer sequence and matching description: PASS'

# Execute only the two EULA guards, against entirely synthetic update objects.
$install = @($dispatch[0].Clauses | Where-Object { $_.Item1.Value -eq 'install-drivers' })[0].Item2
$eulaGuards = @($install.Statements | Where-Object {
    $_ -is [System.Management.Automation.Language.IfStatementAst] -and
    ($_.Clauses[0].Item1.Extent.Text -eq '$unaccepted -and -not $AcceptEulas' -or
     $_.Clauses[0].Item1.Extent.Text -eq '$AcceptEulas')
})
if ($eulaGuards.Count -ne 2) { throw 'Expected refusal guard followed by explicit EULA consent guard.' }
foreach ($node in $eulaGuards) {
    if (@($node.FindAll({ param($n) $n -is [System.Management.Automation.Language.CommandAst] }, $true)).Count) {
        throw 'Unexpected command in EULA guards; refusing execution.'
    }
    foreach ($call in @($node.FindAll({ param($n) $n -is [System.Management.Automation.Language.InvokeMemberExpressionAst] }, $true))) {
        if ($call.Extent.Text -notin @('$updates.Item($i)', '$updates.Item($i).AcceptEula()')) {
            throw 'Unexpected member invocation in EULA guards; refusing execution.'
        }
    }
}
$eulaBlock = [scriptblock]::Create(($eulaGuards | ForEach-Object { $_.Extent.Text }) -join "`n")
# Ensure the guards run after approved-only selection and before download setup.
$installText = $install.Extent.Text
if ($installText.IndexOf('$updates.Add($match)') -gt $installText.IndexOf($eulaGuards[0].Extent.Text) -or
    $installText.IndexOf($eulaGuards[1].Extent.Text) -gt $installText.IndexOf('$downloader =')) {
    throw 'EULA guards must follow approved selection and precede downloads.'
}
$acceptCalls = @($ast.FindAll({ param($n)
    $n -is [System.Management.Automation.Language.InvokeMemberExpressionAst] -and $n.Member.Value -eq 'AcceptEula'
}, $true))
if ($acceptCalls.Count -ne 1 -or $acceptCalls[0].Extent.StartOffset -lt $eulaGuards[1].Extent.StartOffset -or
    $acceptCalls[0].Extent.EndOffset -gt $eulaGuards[1].Extent.EndOffset) { throw 'AcceptEula must exist only within explicit consent guard.' }

function New-FakeUpdate([bool]$Accepted) {
    $fake = [pscustomobject]@{ EulaAccepted = $Accepted; AcceptCalls = 0 }
    $fake | Add-Member ScriptMethod AcceptEula { $this.AcceptCalls++; $this.EulaAccepted = $true }
    return $fake
}
function Assert-EulaConsent([bool]$AcceptEulas, [bool]$InitiallyAccepted) {
    $approved = New-FakeUpdate $InitiallyAccepted
    $alreadyAccepted = New-FakeUpdate $true
    $unapproved = New-FakeUpdate $false
    $updates = [pscustomobject]@{ Count = 2; Items = @($approved, $alreadyAccepted) }
    $updates | Add-Member ScriptMethod Item { param($Index) return $this.Items[$Index] }
    $unaccepted = -not $InitiallyAccepted
    $caught = $null
    try { & $eulaBlock } catch { $caught = $_.Exception.Message }
    if ($unaccepted -and -not $AcceptEulas) {
        if ($caught -ne 'UnacceptedDriverEula') { throw 'Missing explicit consent must refuse unaccepted EULAs.' }
    } elseif ($null -ne $caught) { throw 'Unexpected EULA guard failure.' }
    $expected = 0
    if ($AcceptEulas -and -not $InitiallyAccepted) { $expected = 1 }
    if ($approved.AcceptCalls -ne $expected -or $alreadyAccepted.AcceptCalls -ne 0 -or $unapproved.AcceptCalls -ne 0) {
        throw 'Acceptance must affect only approved, unaccepted updates with explicit consent.'
    }
}
Assert-EulaConsent $false $false
Assert-EulaConsent $true $false
Assert-EulaConsent $false $true
Assert-EulaConsent $true $true
Write-Output 'EULA acceptance requires explicit consent and affects only approved unaccepted updates: PASS'
$converter = @($ast.FindAll({ param($n)
    $n -is [System.Management.Automation.Language.FunctionDefinitionAst] -and $n.Name -eq 'Convert-Update'
}, $true))
if ($converter.Count -ne 1) { throw 'Expected one update converter.' }
Assert-SafeCommands $converter[0] @()
$converterDefinition = [scriptblock]::Create($converter[0].Extent.Text)
& {
    . $converterDefinition
    $fake = [pscustomobject]@{
        Identity = [pscustomobject]@{ UpdateID = $validId }
        Title = 'Synthetic test update'; EulaAccepted = $false
        EulaText = "Full synthetic terms`nSecond paragraph"; IsDownloaded = $false; RebootRequired = $false
    }
    $converted = Convert-Update $fake
    if ($converted.eulaAccepted -or $converted.eulaText -cne $fake.EulaText) { throw 'Unaccepted update must expose full EULA text.' }
    $fake.EulaAccepted = $true
    $fake.PSObject.Properties.Remove('EulaText')
    $fake | Add-Member ScriptProperty EulaText { throw 'Accepted EULA text must not be accessed.' }
    $converted = Convert-Update $fake
    if (-not $converted.eulaAccepted -or $null -ne $converted.eulaText) { throw 'Accepted update must omit EULA content.' }
}
Write-Output 'Full EULA text exposed only for unaccepted updates: PASS'
Write-Output 'Safety tests passed; no backend dispatch, COM search, or actual mutation executed.'
