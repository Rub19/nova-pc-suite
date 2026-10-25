# Only the child created here is eligible for a mutation in this test.
# No UI, no user application closure, no cache-wide memory operation.
$ErrorActionPreference='Stop'
$root=Split-Path $PSScriptRoot -Parent
$os=Get-CimInstance Win32_OperatingSystem
if([double]$os.FreePhysicalMemory*1024 -lt 1GB){throw 'Insufficient free memory for the isolated fixture; no mutation attempted'}
$scratch=Join-Path $root ('work/memory-fixture-'+[guid]::NewGuid().ToString('N'))
[void][IO.Directory]::CreateDirectory($scratch)
$exe=Join-Path $scratch 'NovaMemoryFixture.exe'
Add-Type -TypeDefinition @'
using System;
public static class NovaMemoryFixture {
 public static void Main() {
  byte[] pages = new byte[256 * 1024 * 1024];
  for (int i=0;i<pages.Length;i+=4096) pages[i]=1;
  Console.WriteLine("READY"); Console.Out.Flush();
  Console.ReadLine(); GC.KeepAlive(pages);
 }
}
'@ -OutputAssembly $exe -OutputType ConsoleApplication
$start=New-Object Diagnostics.ProcessStartInfo
$start.FileName=$exe;$start.UseShellExecute=$false;$start.CreateNoWindow=$true
$start.RedirectStandardInput=$true;$start.RedirectStandardOutput=$true
$child=New-Object Diagnostics.Process
$child.StartInfo=$start
try{
 [void]$child.Start()
 $retained=$child.Handle
 $ready=$child.StandardOutput.ReadLineAsync()
 if(-not $ready.Wait(15000) -or $ready.Result -ne 'READY'){throw 'Isolated child did not initialize'}
 if($child.HasExited -or $child.MainModule.FileName -ine $exe){throw 'Fixture identity mismatch'}
 $started=$child.StartTime.ToUniversalTime().Ticks.ToString()
 $payload=@{pid=$child.Id;started=$started}|ConvertTo-Json -Compress
 $encoded=[Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($payload))
 $script=Join-Path $root 'src-tauri/scripts/memory.ps1'
 $result=& $script -Action memory-trim -PayloadBase64 $encoded|ConvertFrom-Json
 if(-not $result.ok -or -not $result.data.trimRequested -or $result.data.targetPid -ne $child.Id){throw "Native trim of owned fixture failed: $($result.error)"}
 $before=@($result.data.before.items|Where-Object{$_.pid -eq $child.Id -and $_.started -eq $started})
 # After trimming the child may drop below the inventory's 100 MiB cutoff.
 $child.Refresh();$afterBytes=$child.WorkingSet64
 if($before.Count -ne 1 -or $before[0].workingSetBytes -lt 200MB){throw 'Owned fixture was not captured before trimming'}
 if($child.HasExited){throw 'Working set reduction must not stop the fixture'}
 if($afterBytes -ge $before[0].workingSetBytes){throw 'No working set reduction observed on the idle fixture'}
 # A hidden console fixture has no normal window to close: reject, never force.
 $close=& $script -Action process-close -PayloadBase64 $encoded|ConvertFrom-Json
 if($close.ok -or $child.HasExited){throw 'A windowless fixture must not be forcefully stopped'}
 Write-Output "Owned fixture PASS: native working set $($before[0].workingSetBytes) -> $afterBytes bytes; process remained alive. Only the child created by this test was targeted."
}finally{
 if($child.Id -and -not $child.HasExited){
  try{$child.StandardInput.WriteLine('exit');$child.StandardInput.Flush()}catch{}
  if(-not $child.WaitForExit(5000)){$child.Kill();$child.WaitForExit()}
 }
 $child.Dispose()
}
