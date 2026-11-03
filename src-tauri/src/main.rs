#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::{json, Value};
#[cfg(windows)]
use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    process::{Command, Stdio},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tauri::Manager;

mod cleanup;
mod elevation;
mod vitals;
mod winpaths;

struct OperationLock(Arc<Mutex<()>>);

fn hold_data_root(path: &std::path::Path) -> Result<Vec<std::fs::File>, String> {
    if !path.is_absolute() {
        return Err("Dossier Nova invalide".into());
    }
    let mut held = Vec::new();
    for component in path.ancestors().collect::<Vec<_>>().into_iter().rev() {
        if !component.exists() {
            fs::create_dir(component).map_err(|_| "Création du dossier Nova impossible")?;
        }
        let mut options = OpenOptions::new();
        options.read(true);
        #[cfg(windows)]
        options.share_mode(3).custom_flags(0x02000000 | 0x00200000);
        let file = options
            .open(component)
            .map_err(|_| "Protection du dossier Nova impossible")?;
        let metadata = file.metadata().map_err(|_| "Dossier Nova inaccessible")?;
        #[cfg(windows)]
        if metadata.file_attributes() & 0x400 != 0 {
            return Err("Nova refuse un dossier redirigé par un lien ou une jonction.".into());
        }
        if !metadata.is_dir() {
            return Err("Dossier Nova invalide".into());
        }
        held.push(file);
    }
    Ok(held)
}

fn policy(module: &str, action: &str) -> Option<bool> {
    match (module, action) {
        ("diag", "scan" | "boots") => Some(false),
        ("updates", "devices" | "wu-search" | "wu-history" | "winget-list" | "open-vendor") => {
            Some(false)
        }
        ("updates", "wu-install" | "winget-upgrade" | "reboot" | "reboot-cancel") => Some(true),
        ("cleanup", "scan") => Some(false),
        ("cleanup", "clean") => Some(true),
        ("startup", "list") => Some(false),
        ("startup", "set") => Some(true),
        ("perf", "overview" | "processes" | "network-test") => Some(false),
        (
            "perf",
            "process-close" | "power-apply" | "volume-optimize" | "hibernate-set" | "dns-flush",
        ) => Some(true),
        (
            "repair",
            "restore-list" | "open-restore" | "dism-check" | "sfc-check" | "chkdsk-scan",
        ) => Some(false),
        (
            "repair",
            "restore-create" | "protection-enable" | "protection-disable" | "dism-repair"
            | "sfc-repair" | "store-cleanup",
        ) => Some(true),
        _ => None,
    }
}

fn script_source(module: &str) -> Option<&'static str> {
    match module {
        "diag" => Some(include_str!("../scripts/diag.ps1")),
        "updates" => Some(include_str!("../scripts/updates.ps1")),
        "startup" => Some(include_str!("../scripts/startup.ps1")),
        "perf" => Some(include_str!("../scripts/perf.ps1")),
        "repair" => Some(include_str!("../scripts/repair.ps1")),
        _ => None,
    }
}

fn sandbox_enabled() -> bool {
    #[cfg(debug_assertions)]
    {
        std::env::var_os("NOVA_SANDBOX").is_some()
    }
    #[cfg(not(debug_assertions))]
    {
        false
    }
}

fn build_script(module: &str, action: &str, payload: &Value) -> Result<String, String> {
    let source = script_source(module).ok_or("Module inconnu")?;
    let footer = format!(
        "try {{ $d = Invoke-Module '{action}'; @{{ ok = $true; data = $d }} | ConvertTo-Json -Depth 10 -Compress }} catch {{ @{{ ok = $false; error = [string]$_.Exception.Message }} | ConvertTo-Json -Compress }}"
    );
    Ok(format!(
        "$PayloadBase64 = '{}'\n$Sandbox = ${}\n{}\n{}\n{}",
        STANDARD.encode(payload.to_string().as_bytes()),
        sandbox_enabled(),
        include_str!("../scripts/common.ps1"),
        source,
        footer
    ))
}

fn encoded(script: &str) -> String {
    STANDARD.encode(
        script
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<u8>>(),
    )
}

fn windows_response(bytes: &[u8], stderr: &[u8], success: bool) -> Result<Value, String> {
    let parsed = std::str::from_utf8(bytes).ok().and_then(|text| {
        serde_json::from_str::<Value>(text.trim().trim_start_matches('\u{feff}')).ok()
    });
    if let Some(value) = parsed {
        if value.get("ok").and_then(Value::as_bool).is_some() && success {
            return Ok(value);
        }
    }
    let diagnostic = String::from_utf8_lossy(stderr);
    if ["ParserError", "ParseException", "UnexpectedToken"]
        .iter()
        .any(|t| diagnostic.contains(t))
    {
        return Err("Erreur interne dans un script de Nova (WindowsScriptSyntax).".into());
    }
    if !success {
        return Err("La commande Windows a échoué avant de répondre (WindowsProcessFailed).".into());
    }
    Err("Windows a renvoyé une réponse illisible (WindowsResponseInvalid).".into())
}

fn powershell(script: &str, seconds: u64) -> Result<Value, String> {
    let win = std::env::var_os("SystemRoot").ok_or("Windows est requis.")?;
    let exe = std::path::PathBuf::from(win).join("System32/WindowsPowerShell/v1.0/powershell.exe");
    let mut cmd = Command::new(exe);
    let bootstrap = "[Console]::InputEncoding = New-Object System.Text.UTF8Encoding($false); [Console]::OutputEncoding = New-Object System.Text.UTF8Encoding($false); $source = [Console]::In.ReadToEnd(); & ([ScriptBlock]::Create($source))";
    cmd.args([
        "-NoLogo",
        "-NoProfile",
        "-NonInteractive",
        "-ExecutionPolicy",
        "Bypass",
        "-EncodedCommand",
        &encoded(bootstrap),
    ])
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped());
    #[cfg(windows)]
    cmd.creation_flags(0x08000000);
    let mut child = cmd
        .spawn()
        .map_err(|_| "Impossible de démarrer PowerShell.")?;
    let mut input = child.stdin.take().ok_or("Entrée indisponible")?;
    let source = script.to_owned();
    let writer = thread::spawn(move || input.write_all(source.as_bytes()));
    let mut out = child.stdout.take().ok_or("Sortie indisponible")?;
    let mut err = child.stderr.take().ok_or("Sortie indisponible")?;
    let stdout = thread::spawn(move || {
        let mut b = Vec::new();
        let _ = out.read_to_end(&mut b);
        b
    });
    let stderr = thread::spawn(move || {
        let mut b = Vec::new();
        let _ = err.read_to_end(&mut b);
        b
    });
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|_| "État du processus indisponible")?
        {
            break status;
        }
        if seconds > 0 && start.elapsed() > Duration::from_secs(seconds) {
            let _ = child.kill();
            let _ = child.wait();
            return Err("Windows met trop de temps à répondre. Réessayez dans un instant.".into());
        }
        thread::sleep(Duration::from_millis(50));
    };
    let bytes = stdout.join().map_err(|_| "Lecture interrompue")?;
    let _ = writer.join();
    let diagnostic = stderr.join().map_err(|_| "Lecture interrompue")?;
    windows_response(&bytes, &diagnostic, status.success())
}

fn timeout_for(module: &str, action: &str, modifies: bool) -> u64 {
    if modifies || matches!(action, "dism-check" | "sfc-check" | "chkdsk-scan") {
        0
    } else if action == "wu-search" {
        900
    } else if module == "updates" {
        240
    } else {
        120
    }
}

fn execute(module: &str, action: &str, payload: &Value) -> Result<Value, String> {
    if module == "cleanup" {
        return match action {
            "scan" => cleanup::scan(),
            "clean" => cleanup::clean(
                serde_json::from_value(payload["ids"].clone()).map_err(|_| "Sélection invalide")?,
            ),
            _ => Err("Action inconnue".into()),
        };
    }
    if (module, action) == ("perf", "power-apply") {
        let before = vitals::cpu_bench();
        let mut result = powershell(&build_script(module, action, payload)?, 60)?;
        if result["ok"] == true {
            if let Some(measures) = result["data"]["measures"].as_array_mut() {
                measures.push(json!({"label": "Calcul processeur, en millions d'opérations par seconde", "before": before.round(), "after": vitals::cpu_bench().round(), "unit": "number"}));
            }
        }
        return Ok(result);
    }
    powershell(
        &build_script(module, action, payload)?,
        timeout_for(module, action, policy(module, action) == Some(true)),
    )
}

fn journal_path(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    Ok(app
        .path()
        .app_local_data_dir()
        .map_err(|_| "Stockage Nova indisponible")?
        .join("journal.jsonl"))
}

fn journal_entry(module: &str, action: &str, result: &Result<Value, String>) -> Value {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let mut entry = json!({"timestamp": now, "module": module, "action": action});
    match result {
        Ok(value) if value["ok"] == true => {
            let data = &value["data"];
            entry["ok"] = json!(true);
            for key in [
                "summary",
                "measures",
                "undo",
                "results",
                "changedDrivers",
                "restorePoint",
                "rebootRequired",
                "verdict",
            ] {
                if !data[key].is_null() {
                    entry[key] = data[key].clone();
                }
            }
        }
        Ok(value) => {
            entry["ok"] = json!(false);
            entry["summary"] = value["error"].clone();
        }
        Err(error) => {
            entry["ok"] = json!(false);
            entry["summary"] = json!(error);
        }
    }
    entry
}

#[tauri::command]
async fn run(
    app: tauri::AppHandle,
    state: tauri::State<'_, OperationLock>,
    module: String,
    action: String,
    payload: Option<Value>,
) -> Result<Value, String> {
    let modifies = policy(&module, &action).ok_or("Action inconnue")?;
    let payload = payload.unwrap_or_else(|| json!({}));
    if !payload.is_object() || payload.to_string().len() > 64_000 {
        return Err("Paramètres invalides".into());
    }
    let lock = state.0.clone();
    tauri::async_runtime::spawn_blocking(move || {
        if !modifies {
            return execute(&module, &action, &payload);
        }
        let _guard = lock
            .try_lock()
            .map_err(|_| "Une autre opération de Nova est en cours. Attendez sa fin.")?;
        let result = execute(&module, &action, &payload);
        if let Ok(path) = journal_path(&app) {
            let line = format!("{}\n", journal_entry(&module, &action, &result));
            let _ = winpaths::write_owned(&path, line.as_bytes(), true);
        }
        result
    })
    .await
    .map_err(|_| "Opération interrompue".to_string())?
}

#[tauri::command]
fn journal(app: tauri::AppHandle) -> Vec<Value> {
    journal_path(&app)
        .ok()
        .and_then(|path| fs::read_to_string(path).ok())
        .map(|text| {
            text.lines()
                .rev()
                .take(200)
                .filter_map(|l| serde_json::from_str(l).ok())
                .collect()
        })
        .unwrap_or_default()
}

#[tauri::command]
async fn vitals_sample() -> Value {
    tauri::async_runtime::spawn_blocking(vitals::sample)
        .await
        .unwrap_or(Value::Null)
}

#[tauri::command]
fn is_admin() -> bool {
    elevation::is_admin()
}

#[tauri::command]
fn startup_page() -> String {
    elevation::initial_page()
}

#[tauri::command]
fn app_ready(app: tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
    }
}

#[tauri::command]
fn open_windows(target: String) -> Result<(), String> {
    let uri = match target.as_str() {
        "windows-update" => "ms-settings:windowsupdate",
        "security" => "windowsdefender:",
        "device-manager" => "devmgmt.msc",
        _ => return Err("Page Windows inconnue".into()),
    };
    let script = format!(
        "try {{ Start-Process '{uri}'; @{{ok=$true}} | ConvertTo-Json }} catch {{ @{{ok=$false}} | ConvertTo-Json }}"
    );
    powershell(&script, 15).map(|_| ())
}

#[tauri::command]
async fn elevate(app: tauri::AppHandle, page: String) -> Result<Value, String> {
    if !elevation::valid_page(&page) {
        return Err("Écran invalide".into());
    }
    tauri::async_runtime::spawn_blocking(move || {
        if elevation::is_admin() {
            return Ok(json!({"ok": true, "alreadyAdmin": true}));
        }
        let handoff = elevation::Handoff::new()?;
        let exe = std::env::current_exe().map_err(|_| "Exécutable introuvable")?;
        let path = exe.to_string_lossy().replace('\'', "''");
        let window = app
            .get_webview_window("main")
            .ok_or("Fenêtre Nova indisponible")?;
        window
            .hide()
            .map_err(|_| "Transition de fenêtre impossible")?;
        let launched = powershell(&format!("try {{ Start-Process -FilePath '{path}' -ArgumentList @('--elevated-handoff','{}','--start-page','{page}') -Verb RunAs -ErrorAction Stop; @{{ok=$true}}|ConvertTo-Json }} catch {{ @{{ok=$false}}|ConvertTo-Json }}", handoff.token), 0);
        if launched
            .as_ref()
            .map(|v| v["ok"] == true)
            .unwrap_or(false)
            && handoff.wait(60000)
        {
            app.exit(0);
            Ok(json!({"ok": true}))
        } else {
            let _ = window.show();
            let _ = window.set_focus();
            Ok(json!({"ok": false, "error": "Passage administrateur annulé. Cette fenêtre reste ouverte."}))
        }
    })
    .await
    .map_err(|_| "Transition interrompue".to_string())?
}

fn main() {
    if std::env::args().any(|a| a == "--scheduled-scan") {
        return;
    }
    let replacement = match elevation::requested_token() {
        Ok(token) => token.is_some() && elevation::is_admin(),
        Err(_) => return,
    };
    let _instance = match elevation::Instance::acquire(replacement) {
        Ok(Some(instance)) => instance,
        _ => return,
    };
    tauri::Builder::default()
        .setup(|app| {
            if let Some(token) = elevation::requested_token()? {
                elevation::complete(&token)?;
            }
            #[cfg(debug_assertions)]
            if std::env::var_os("NOVA_OFFSCREEN").is_some() {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.set_skip_taskbar(true);
                    let _ = window.set_position(tauri::PhysicalPosition::new(-6000, -6000));
                }
            }
            let handle = app.handle().clone();
            thread::spawn(move || {
                thread::sleep(Duration::from_secs(5));
                if let Some(window) = handle.get_webview_window("main") {
                    let _ = window.show();
                }
            });
            Ok(())
        })
        .manage(OperationLock(Arc::new(Mutex::new(()))))
        .invoke_handler(tauri::generate_handler![
            run,
            journal,
            vitals_sample,
            is_admin,
            startup_page,
            app_ready,
            open_windows,
            elevate
        ])
        .run(tauri::generate_context!())
        .expect("Impossible de démarrer Nova");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_actions_are_refused() {
        for (module, action) in [
            ("cleanup", "../../Windows"),
            ("perf", "kill"),
            ("repair", "format"),
            ("updates", "cmd.exe"),
            ("", ""),
        ] {
            assert_eq!(policy(module, action), None);
        }
    }

    #[test]
    fn changes_are_journaled_and_reads_are_not() {
        for (module, action) in [
            ("cleanup", "clean"),
            ("startup", "set"),
            ("updates", "wu-install"),
            ("perf", "power-apply"),
            ("repair", "dism-repair"),
        ] {
            assert_eq!(policy(module, action), Some(true));
        }
        for (module, action) in [
            ("cleanup", "scan"),
            ("startup", "list"),
            ("updates", "wu-search"),
            ("diag", "scan"),
        ] {
            assert_eq!(policy(module, action), Some(false));
        }
    }

    #[test]
    fn every_script_parses_as_utf8() {
        for (name, source) in [
            ("common", include_str!("../scripts/common.ps1")),
            ("diag", include_str!("../scripts/diag.ps1")),
            ("updates", include_str!("../scripts/updates.ps1")),
            ("startup", include_str!("../scripts/startup.ps1")),
            ("perf", include_str!("../scripts/perf.ps1")),
            ("repair", include_str!("../scripts/repair.ps1")),
        ] {
            let code = STANDARD.encode(source.as_bytes());
            let script = format!("$code=[Text.Encoding]::UTF8.GetString([Convert]::FromBase64String('{code}'));$t=$null;$e=$null;[void][Management.Automation.Language.Parser]::ParseInput($code,[ref]$t,[ref]$e);@{{ok=($e.Count -eq 0);data=@($e|%{{$_.Message}})}}|ConvertTo-Json -Compress");
            let value = powershell(&script, 30).unwrap();
            assert_eq!(value["ok"], true, "{name}: {}", value["data"]);
        }
    }

    #[test]
    fn scan_runs_through_the_real_transport() {
        let value = powershell(&build_script("diag", "scan", &json!({})).unwrap(), 120).unwrap();
        assert_eq!(value["ok"], true, "{value}");
        assert!(value["data"]["memory"]["totalBytes"].as_f64().unwrap() > 0.0);
        assert!(value["data"]["volumes"].as_array().is_some());
    }

    #[test]
    fn script_errors_come_back_as_readable_messages() {
        let value = powershell(&build_script("perf", "bogus", &json!({})).unwrap(), 30).unwrap();
        assert_eq!(value["ok"], false);
        assert_eq!(value["error"], "Action inconnue");
    }

    #[test]
    fn journal_keeps_measures_and_undo_only_from_success() {
        let ok = Ok(
            json!({"ok": true, "data": {"summary": "fait", "measures": [1], "undo": {"module": "startup"}, "secret": 1}}),
        );
        let entry = journal_entry("startup", "set", &ok);
        assert_eq!(entry["ok"], true);
        assert_eq!(entry["undo"]["module"], "startup");
        assert!(entry["secret"].is_null());
        let failed = journal_entry("startup", "set", &Ok(json!({"ok": false, "error": "refus"})));
        assert_eq!(failed["ok"], false);
        assert_eq!(failed["summary"], "refus");
        assert!(failed["undo"].is_null());
    }
}
