# Parse the UTF-8 text that Rust embeds, not PowerShell 5.1's ANSI file view.
# Parsing only: no diagnostic or maintenance action is executed.
$ErrorActionPreference='Stop'
$root=Split-Path $PSScriptRoot -Parent
$paths=@([IO.Directory]::GetFiles((Join-Path $root 'src-tauri/scripts'),'*.ps1'))+@(Join-Path $root 'scripts/uninstall-task.ps1')
$failed=@()
foreach($path in $paths){
 $source=[IO.File]::ReadAllText($path,[Text.Encoding]::UTF8)
 $tokens=$null;$errors=$null
 [void][Management.Automation.Language.Parser]::ParseInput($source,[ref]$tokens,[ref]$errors)
 if($errors.Count){$failed+=([IO.Path]::GetFileName($path)+': '+$errors.Count+' UTF-8 parser errors')}
}
if($failed.Count){throw ($failed -join '; ')}
Write-Output "PASS: $($paths.Count) embedded UTF-8 scripts parse without execution."
