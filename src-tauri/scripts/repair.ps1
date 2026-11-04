function Get-Verdict([string]$Text, [int]$Code) {
    if ($Text -match '(?i)a réparé|successfully repaired') { return 'repaired' }
    if ($Text -match '(?i)aucune violation|did not find any integrity|aucun endommagement|no component store corruption') { return 'ok' }
    if ($Text -match '(?i)violations? de l.intégrité|integrity violations|fichiers (endommagés|corrompus)|found corrupt|repairable|réparable|corruption') { return 'problem' }
    if ($Code -ne 0) { return 'problem' }
    'done'
}

function Result-From($Run, [string]$Summary) {
    @{ summary = $Summary; exitCode = $Run.exitCode; output = @($Run.tail); verdict = (Get-Verdict $Run.text $Run.exitCode) }
}

function Invoke-Module([string]$Action) {
    switch ($Action) {
        'restore-list' {
            if (-not (Test-Admin)) { return @{ needsAdmin = $true; items = @() } }
            $items = @(Get-ComputerRestorePoint | Sort-Object SequenceNumber -Descending | Select-Object -First 15 | ForEach-Object {
                @{ sequence = [int]$_.SequenceNumber; description = [string]$_.Description; date = [Management.ManagementDateTimeConverter]::ToDateTime($_.CreationTime).ToUniversalTime().ToString('o') }
            })
            $enabled = $null
            try { $enabled = [int](Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\SystemRestore').RPSessionInterval -ne 0 } catch { }
            @{ needsAdmin = $false; items = $items; protectionOn = $enabled }
        }
        'restore-create' {
            $r = Ensure-RestorePoint 'Nova PC Suite : point manuel'
            @{ summary = $(if ($r.created) { 'Point de restauration créé' } else { 'Un point de moins de 24 h existe déjà' }); restorePoint = $r }
        }
        'protection-enable' {
            Require-Admin
            Enable-ComputerRestore -Drive "$env:SystemDrive\"
            @{ summary = "Protection du système activée sur $env:SystemDrive"; undo = @{ module = 'repair'; action = 'protection-disable'; label = 'Désactiver la protection' } }
        }
        'protection-disable' {
            Require-Admin
            Disable-ComputerRestore -Drive "$env:SystemDrive\"
            @{ summary = "Protection du système désactivée sur $env:SystemDrive" }
        }
        'open-restore' { Start-Process (Sys32 'rstrui.exe'); @{ opened = $true } }
        'dism-check' {
            Require-Admin
            Result-From (Invoke-Native 'Dism.exe' @('/Online', '/Cleanup-Image', '/ScanHealth', '/English')) 'Analyse de l''image Windows terminée'
        }
        'dism-repair' {
            $restore = Ensure-RestorePoint 'Nova PC Suite : avant réparation Windows'
            $r = Result-From (Invoke-Native 'Dism.exe' @('/Online', '/Cleanup-Image', '/RestoreHealth', '/NoRestart', '/English')) 'Réparation de l''image Windows terminée'
            $r.restorePoint = $restore
            $r
        }
        'sfc-check' {
            Require-Admin
            Result-From (Invoke-Native 'sfc.exe' @('/verifyonly') -Utf16) 'Vérification des fichiers système terminée'
        }
        'sfc-repair' {
            $restore = Ensure-RestorePoint 'Nova PC Suite : avant réparation Windows'
            $r = Result-From (Invoke-Native 'sfc.exe' @('/scannow') -Utf16) 'Réparation des fichiers système terminée'
            $r.restorePoint = $restore
            $r
        }
        'chkdsk-scan' {
            Require-Admin
            $status = [string](Repair-Volume -DriveLetter $env:SystemDrive[0] -Scan)
            @{ summary = "Analyse du disque $env:SystemDrive terminée"; output = @("Résultat de Windows : $status"); verdict = $(if ($status -eq 'NoErrorsFound') { 'ok' } else { 'problem' }) }
        }
        'store-cleanup' {
            Require-Admin
            $free = Get-FreeBytes
            $run = Invoke-Native 'Dism.exe' @('/Online', '/Cleanup-Image', '/StartComponentCleanup')
            if ($run.exitCode -ne 0) { throw "DISM a échoué (code $($run.exitCode)). $(@($run.tail) -join ' ')" }
            @{ summary = 'Anciens composants Windows supprimés'; output = @($run.tail); verdict = 'ok'; measures = @(@{ label = "Espace libre sur $env:SystemDrive"; before = $free; after = (Get-FreeBytes); unit = 'bytes' }) }
        }
        default { throw 'Action inconnue' }
    }
}
