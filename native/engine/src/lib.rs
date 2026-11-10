use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::{json, Value};
use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
use std::os::windows::process::CommandExt;
use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::Mutex,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

pub mod cleanup;
pub mod diag;
pub mod elevation;
pub mod files;
pub mod perf;
pub mod report;
pub mod selfupdate;
pub mod startup;
pub mod updates;
pub mod vitals;
pub mod win;
pub mod winpaths;

static OPERATION: Mutex<()> = Mutex::new(());

pub(crate) fn hold_data_root(path: &Path) -> Result<Vec<fs::File>, String> {
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
        options.share_mode(3).custom_flags(0x02000000 | 0x00200000);
        let file = options
            .open(component)
            .map_err(|_| "Protection du dossier Nova impossible")?;
        let metadata = file.metadata().map_err(|_| "Dossier Nova inaccessible")?;
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

pub fn policy(module: &str, action: &str) -> Option<bool> {
    match (module, action) {
        ("diag", "scan" | "boots") => Some(false),
        ("updates", "devices" | "wu-search" | "wu-history" | "winget-list" | "open-vendor" | "installed") => {
            Some(false)
        }
        ("updates", "wu-install" | "winget-upgrade" | "reboot" | "reboot-cancel") => Some(true),
        ("cleanup", "scan" | "largest" | "files") => Some(false),
        ("cleanup", "clean") => Some(true),
        ("startup", "list") => Some(false),
        ("startup", "set") => Some(true),
        ("perf", "overview" | "processes" | "network-test" | "adapters") => Some(false),
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

pub fn powershell(script: &str, seconds: u64) -> Result<Value, String> {
    let win = std::env::var_os("SystemRoot").ok_or("Windows est requis.")?;
    let exe = PathBuf::from(win).join("System32/WindowsPowerShell/v1.0/powershell.exe");
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

fn native(module: &str, action: &str, payload: &Value) -> Option<Result<Value, String>> {
    if sandbox_enabled() && module == "startup" {
        return None;
    }
    Some(match (module, action) {
        ("diag", "scan") => diag::scan(),
        ("diag", "boots") => Ok(diag::boots(8)),
        ("updates", "devices") => updates::devices(),
        ("updates", "open-vendor") => updates::open_vendor(payload["id"].as_str().unwrap_or("")),
        ("updates", "wu-search") => updates::wu_search(payload["kind"].as_str().unwrap_or("")),
        ("updates", "wu-history") => updates::wu_history(),
        ("updates", "winget-list") => updates::winget_list(),
        ("updates", "installed") => updates::installed(),
        ("startup", "list") => startup::list(),
        ("perf", "overview") => perf::overview(),
        ("perf", "processes") => perf::processes(),
        ("perf", "network-test") => perf::network_test(),
        ("perf", "adapters") => perf::adapters(),
        ("repair", "open-restore") => win::shell_open(&win::sys32("rstrui.exe"))
            .map(|_| json!({"opened": true})),
        ("repair", "restore-list") if !elevation::is_admin() => {
            Ok(json!({"needsAdmin": true, "items": []}))
        }
        _ => return None,
    })
}

pub fn execute(module: &str, action: &str, payload: &Value) -> Result<Value, String> {
    if module == "cleanup" {
        return match action {
            "scan" => cleanup::scan(),
            "largest" => cleanup::largest().map(|data| json!({"ok": true, "data": data})),
            "files" => files::analyse().map(|data| json!({"ok": true, "data": data})),
            "clean" => cleanup::clean(
                serde_json::from_value(payload["ids"].clone()).map_err(|_| "Sélection invalide")?,
            ),
            _ => Err("Action inconnue".into()),
        };
    }
    if let Some(result) = native(module, action, payload) {
        return Ok(match result {
            Ok(data) => json!({"ok": true, "data": data}),
            Err(error) => json!({"ok": false, "error": error}),
        });
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

pub fn data_dir() -> Result<PathBuf, String> {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .map(|p| p.join("io.github.rub19.novapcsuite"))
        .ok_or_else(|| "Stockage Nova indisponible".to_string())
}

fn journal_path() -> Result<PathBuf, String> {
    Ok(data_dir()?.join("journal.jsonl"))
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

pub fn run(module: &str, action: &str, payload: Option<Value>) -> Result<Value, String> {
    let modifies = policy(module, action).ok_or("Action inconnue")?;
    let payload = payload.unwrap_or_else(|| json!({}));
    if !payload.is_object() || payload.to_string().len() > 64_000 {
        return Err("Paramètres invalides".into());
    }
    if !modifies {
        return execute(module, action, &payload);
    }
    let _guard = OPERATION
        .try_lock()
        .map_err(|_| "Une autre opération de Nova est en cours. Attendez sa fin.")?;
    let result = execute(module, action, &payload);
    if let Ok(path) = journal_path() {
        let line = format!("{}\n", journal_entry(module, action, &result));
        let _ = winpaths::write_owned(&path, line.as_bytes(), true);
    }
    result
}

pub fn journal() -> Vec<Value> {
    journal_path()
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

pub fn open_windows(target: &str) -> Result<(), String> {
    let uri = match target {
        "windows-update" => "ms-settings:windowsupdate",
        "security" => "windowsdefender:",
        "device-manager" => "devmgmt.msc",
        "apps" => "ms-settings:appsfeatures",
        "event-viewer" => "eventvwr.msc",
        "disk-management" => "diskmgmt.msc",
        "services" => "services.msc",
        "task-manager" => "taskmgr.exe",
        "resource-monitor" => "resmon.exe",
        "system-info" => "msinfo32.exe",
        _ => return Err("Page Windows inconnue".into()),
    };
    win::shell_open(uri)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_known_windows_tools_open() {
        assert!(open_windows("cmd.exe").is_err());
        assert!(open_windows("../../Windows/regedit.exe").is_err());
        assert!(open_windows("").is_err());
    }

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
    fn repair_verdicts_never_hide_a_problem() {
        let script = format!(
            "{}
@{{ ok = $true; v = @(Get-Verdict 'La Protection des ressources Windows a détecté des violations de l''intégrité.' 0; Get-Verdict 'Windows Resource Protection found integrity violations.' 0; Get-Verdict 'La protection n''a trouvé aucune violation d''intégrité.' 0; Get-Verdict 'No component store corruption detected.' 0; Get-Verdict 'The component store is repairable.' 0; Get-Verdict 'a trouvé des fichiers endommagés et les a réparés.' 0; Get-Verdict 'Texte inconnu' 0; Get-Verdict 'Texte inconnu' 5) }} | ConvertTo-Json -Compress",
            include_str!("../scripts/repair.ps1")
        );
        let value = powershell(&script, 30).unwrap();
        assert_eq!(
            value["v"],
            json!(["problem", "problem", "ok", "ok", "problem", "repaired", "done", "problem"])
        );
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

    fn powershell_data(module: &str, action: &str) -> Value {
        let value = powershell(&build_script(module, action, &json!({})).unwrap(), 240).unwrap();
        assert_eq!(value["ok"], true, "{value}");
        value["data"].clone()
    }

    #[test]
    fn native_scan_matches_powershell() {
        let ps = powershell_data("diag", "scan");
        let native = diag::scan().unwrap();
        for path in [
            "/isAdmin",
            "/os/name",
            "/os/release",
            "/os/build",
            "/computer/manufacturer",
            "/computer/model",
            "/computer/board",
            "/computer/bios",
            "/cpu/name",
            "/cpu/cores",
            "/cpu/threads",
            "/memory/totalBytes",
            "/memory/modules",
            "/memory/speedMHz",
            "/pendingReboot",
            "/systemDrive",
            "/deviceProblems",
            "/security/otherAntivirus",
            "/security/firewallOff",
            "/security/defender/running",
            "/security/defender/realtime",
            "/stability/unexpectedShutdowns",
            "/stability/blueScreens",
            "/stability/gpuHangs",
            "/stability/appCrashes",
            "/lastUpdate",
            "/boots/available",
        ] {
            assert!(
                same(ps.pointer(path), native.pointer(path), path),
                "{path}: ps={:?} natif={:?}",
                ps.pointer(path),
                native.pointer(path)
            );
        }
        let names = |v: &Value, key: &str, field: &str| {
            let mut list: Vec<String> = v[key]
                .as_array()
                .unwrap()
                .iter()
                .map(|i| i[field].as_str().unwrap_or("").to_string())
                .collect();
            list.sort();
            list
        };
        assert_eq!(names(&ps, "volumes", "drive"), names(&native, "volumes", "drive"));
        assert_eq!(names(&ps, "disks", "name"), names(&native, "disks", "name"));
        assert_eq!(names(&ps, "disks", "media"), names(&native, "disks", "media"));
        assert_eq!(names(&ps, "disks", "bus"), names(&native, "disks", "bus"));
        assert_eq!(names(&ps, "disks", "health"), names(&native, "disks", "health"));
        assert_eq!(names(&ps, "gpus", "name"), names(&native, "gpus", "name"));
        assert_eq!(names(&ps, "gpus", "driverDate"), names(&native, "gpus", "driverDate"));
    }

    fn same(a: Option<&Value>, b: Option<&Value>, path: &str) -> bool {
        match (a, b) {
            (Some(Value::Number(x)), Some(Value::Number(y))) => {
                let (x, y) = (x.as_f64().unwrap(), y.as_f64().unwrap());
                let slack = if path.contains("stability") { 2.0 } else { 0.5 };
                (x - y).abs() <= slack
            }
            (Some(Value::String(x)), Some(Value::String(y))) if path == "/lastUpdate" => {
                let (x, y) = (report::parse_moment(x).unwrap(), report::parse_moment(y).unwrap());
                (x - y).num_hours().abs() <= 14
            }
            (Some(Value::Null), None) | (None, Some(Value::Null)) | (None, None) => true,
            (Some(x), Some(y)) => x == y,
            _ => false,
        }
    }

    #[test]
    fn native_startup_matches_powershell() {
        let ps = powershell_data("startup", "list");
        let native = startup::list().unwrap();
        let rows = |v: &Value| {
            let mut list: Vec<String> = v["items"]
                .as_array()
                .unwrap()
                .iter()
                .map(|i| {
                    format!(
                        "{}|{}|{}|{}|{}|{}|{}",
                        i["source"],
                        i["name"],
                        i["enabled"],
                        i["exists"],
                        i["protected"],
                        i["manageable"],
                        i["publisher"]
                    )
                })
                .collect();
            list.sort();
            list
        };
        assert_eq!(rows(&ps), rows(&native));
    }

    #[test]
    fn native_devices_match_powershell() {
        let ps = powershell_data("updates", "devices");
        let native = updates::devices().unwrap();
        assert_eq!(ps["vendors"], native["vendors"]);
        assert_eq!(
            ps["drivers"].as_array().unwrap().len(),
            native["drivers"].as_array().unwrap().len()
        );
        assert_eq!(
            ps["problems"].as_array().map(Vec::len).unwrap_or(0),
            native["problems"].as_array().unwrap().len()
        );
    }

    #[test]
    fn native_overview_matches_powershell() {
        let ps = powershell_data("perf", "overview");
        let native = perf::overview().unwrap();
        let plans = |v: &Value| {
            let mut list: Vec<String> = v["plans"]
                .as_array()
                .unwrap()
                .iter()
                .map(|p| format!("{}|{}|{}", p["guid"], p["name"], p["active"]))
                .collect();
            list.sort();
            list
        };
        assert_eq!(plans(&ps), plans(&native));
        assert_eq!(ps["hibernation"]["enabled"], native["hibernation"]["enabled"]);
        let volumes = |v: &Value| {
            let mut list: Vec<String> = v["volumes"]
                .as_array()
                .unwrap()
                .iter()
                .map(|p| format!("{}|{}|{}|{}|{}", p["letter"], p["label"], p["media"], p["lastOptimized"].as_str().and_then(|d| d.get(..16)).unwrap_or(""), p["system"]))
                .collect();
            list.sort();
            list
        };
        assert_eq!(volumes(&ps), volumes(&native));
    }

    #[test]
    fn native_processes_report_real_programs() {
        let native = perf::processes().unwrap();
        let items = native["items"].as_array().unwrap();
        assert!(items.len() >= 25);
        assert!(items.iter().all(|i| i["ramBytes"].as_f64().unwrap() >= 0.0));
        assert!(items[0]["ramBytes"].as_f64() >= items[1]["ramBytes"].as_f64());
    }

    #[test]
    fn native_network_answers_like_powershell() {
        let native = perf::network_test().unwrap();
        let ps = powershell_data("perf", "network-test");
        assert_eq!(ps["dnsOk"], native["dnsOk"]);
        let targets = |v: &Value| {
            let mut list: Vec<String> = v["probes"]
                .as_array()
                .unwrap()
                .iter()
                .map(|p| p["target"].as_str().unwrap_or("").to_string())
                .collect();
            list.sort();
            list
        };
        assert_eq!(targets(&ps), targets(&native));
    }

    #[test]
    fn native_history_matches_powershell() {
        let ps = powershell_data("updates", "wu-history");
        let native = updates::wu_history().unwrap();
        let titles = |v: &Value| -> Vec<Value> {
            v["items"].as_array().cloned().unwrap_or_default().iter().map(|i| i["title"].clone()).collect()
        };
        assert_eq!(titles(&ps), titles(&native));
    }
}
