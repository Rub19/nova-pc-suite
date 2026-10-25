import {readFile,writeFile} from 'node:fs/promises';
const source=await readFile('scripts/uninstall-task.ps1','utf8');
const encoded=Buffer.from(source,'utf16le').toString('base64');
await writeFile('src-tauri/hooks.nsh', `; Generated from scripts/uninstall-task.ps1. No user data embedded.\n!macro NSIS_HOOK_PREUNINSTALL\n  nsExec::ExecToStack '\"$SYSDIR\\WindowsPowerShell\\v1.0\\powershell.exe\" -NoProfile -NonInteractive -EncodedCommand ${encoded}'\n  Pop $0\n  Pop $1\n!macroend\n`);
