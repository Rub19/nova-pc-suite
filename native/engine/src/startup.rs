use crate::win::{self, from_wide, wide};
use serde_json::{json, Value};
use std::{collections::HashMap, path::Path};
use windows::{
    core::{Interface, HSTRING, PCWSTR},
    Win32::{
        Storage::FileSystem::{GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW},
        System::{
            Com::{CoCreateInstance, IPersistFile, CLSCTX_ALL, STGM_READ},
            Environment::ExpandEnvironmentStringsW,
            TaskScheduler::{
                IExecAction, ITaskFolder, ITaskService, TaskScheduler, TASK_ACTION_EXEC,
                TASK_ENUM_HIDDEN, TASK_STATE_DISABLED, TASK_TRIGGER_LOGON,
            },
            Variant::VARIANT,
        },
        UI::Shell::{IShellLinkW, ShellLink},
    },
};
use windows_registry::{Key, CURRENT_USER, LOCAL_MACHINE};

const RUN: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
const APPROVED: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\StartupApproved";
const GUARDED: [&str; 18] = [
    "defender", "securityhealth", "antivirus", "antimalware", "anti-malware", "anti malware",
    "smartscreen", "crowdstrike", "sentinel", "sophos", "kaspersky", "bitdefender", "avast",
    "avg", "mcafee", "norton", "symantec", "eset",
];
const GUARDED_MORE: [&str; 3] = ["malwarebytes", "webroot", "f-secure"];

fn guarded(text: &str) -> bool {
    let lower = text.to_lowercase();
    GUARDED.iter().chain(GUARDED_MORE.iter()).any(|w| lower.contains(w))
        || lower
            .find("trend")
            .map(|i| {
                let rest: Vec<char> = lower[i + 5..].chars().collect();
                rest.starts_with(&['m', 'i', 'c', 'r', 'o'])
                    || (rest.len() >= 6 && rest[1..6] == ['m', 'i', 'c', 'r', 'o'])
            })
            .unwrap_or(false)
}

fn expand(command: &str) -> String {
    let source = wide(command);
    let mut buffer = vec![0u16; 8192];
    let size = unsafe {
        ExpandEnvironmentStringsW(PCWSTR(source.as_ptr()), Some(&mut buffer))
    };
    if size == 0 {
        command.to_string()
    } else {
        from_wide(&buffer)
    }
}

pub fn exe_path(command: &str) -> Option<String> {
    if command.is_empty() {
        return None;
    }
    let expanded = expand(command);
    let c = expanded.trim();
    if let Some(rest) = c.strip_prefix('"') {
        if let Some(end) = rest.find('"') {
            if end > 0 {
                return Some(rest[..end].to_string());
            }
        }
    }
    let lower = c.to_ascii_lowercase();
    let mut best: Option<usize> = None;
    for ext in [".exe", ".cmd", ".bat", ".com"] {
        let mut from = 0;
        while let Some(i) = lower[from..].find(ext) {
            let end = from + i + 4;
            if end == c.len() || c[end..].starts_with(char::is_whitespace) {
                best = Some(best.map_or(end, |b: usize| b.min(end)));
                break;
            }
            from = end;
        }
    }
    if let Some(end) = best {
        return Some(c[..end].to_string());
    }
    c.split_whitespace().next().map(str::to_string)
}

fn company_name(path: &str) -> Option<String> {
    let name = HSTRING::from(path);
    unsafe {
        let size = GetFileVersionInfoSizeW(&name, None);
        if size == 0 {
            return None;
        }
        let mut data = vec![0u8; size as usize];
        GetFileVersionInfoW(&name, None, size, data.as_mut_ptr().cast()).ok()?;
        let mut pointer = std::ptr::null_mut();
        let mut length = 0u32;
        let found = VerQueryValueW(
            data.as_ptr().cast(),
            &HSTRING::from("\\VarFileInfo\\Translation"),
            &mut pointer,
            &mut length,
        );
        let mut pages = Vec::new();
        if found.as_bool() && length >= 4 {
            let words = std::slice::from_raw_parts(pointer as *const u16, 2);
            pages.push(format!("{:04x}{:04x}", words[0], words[1]));
        }
        pages.extend(["040904b0".to_string(), "040904e4".to_string(), "04090000".to_string()]);
        for page in pages {
            let query = HSTRING::from(format!("\\StringFileInfo\\{page}\\CompanyName"));
            let mut value = std::ptr::null_mut();
            let mut chars = 0u32;
            if VerQueryValueW(data.as_ptr().cast(), &query, &mut value, &mut chars).as_bool() && chars > 0 {
                let text = from_wide(std::slice::from_raw_parts(value as *const u16, chars as usize));
                let text = text.trim();
                if !text.is_empty() {
                    return Some(text.to_string());
                }
            }
        }
        None
    }
}

fn shortcut_target(path: &str) -> Option<String> {
    win::com();
    unsafe {
        let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_ALL).ok()?;
        link.cast::<IPersistFile>().ok()?.Load(&HSTRING::from(path), STGM_READ).ok()?;
        let mut buffer = [0u16; 1024];
        link.GetPath(&mut buffer, std::ptr::null_mut(), 0).ok()?;
        Some(from_wide(&buffer))
    }
}

fn describe(exe: &Option<String>, ram: &HashMap<String, f64>) -> (Option<String>, bool, f64) {
    let Some(path) = exe else { return (None, false, 0.0) };
    let found = Path::new(path).is_file();
    let publisher = found.then(|| company_name(path)).flatten();
    let used = ram.get(&path.to_lowercase()).copied().unwrap_or(0.0);
    (publisher, found, used)
}

fn entry(
    source: &str,
    label: &str,
    name: String,
    exe: Option<String>,
    enabled: bool,
    writable: bool,
    ram: &HashMap<String, f64>,
) -> Value {
    let (publisher, exists, used) = describe(&exe, ram);
    let protected = guarded(&format!("{name} {}", exe.as_deref().unwrap_or("")));
    json!({
        "source": source,
        "sourceLabel": label,
        "name": name,
        "enabled": enabled,
        "path": exe,
        "publisher": publisher,
        "exists": exists,
        "ramBytes": used,
        "running": used > 0.0,
        "protected": protected,
        "manageable": !protected && writable,
    })
}

fn approved(hive: &Key, approved: &str, name: &str) -> bool {
    hive.open(format!("{APPROVED}\\{approved}"))
        .and_then(|k| k.get_value(name))
        .map(|v| v.first().map(|b| b & 1 == 0).unwrap_or(true))
        .unwrap_or(true)
}

fn from_registry(
    hive: &Key,
    source: &str,
    key: &str,
    approved_key: &str,
    label: &str,
    writable: bool,
    ram: &HashMap<String, f64>,
    items: &mut Vec<Value>,
) {
    let Ok(run) = hive.open(key) else { return };
    let Ok(values) = run.values() else { return };
    for (name, _) in values {
        if name.is_empty() {
            continue;
        }
        let command = run.get_string(&name).unwrap_or_default();
        let enabled = approved(hive, approved_key, &name);
        items.push(entry(source, label, name, exe_path(&command), enabled, writable, ram));
    }
}

fn from_folder(
    hive: &Key,
    source: &str,
    folder: &Path,
    label: &str,
    writable: bool,
    ram: &HashMap<String, f64>,
    items: &mut Vec<Value>,
) {
    let Ok(entries) = std::fs::read_dir(folder) else { return };
    for file in entries.flatten() {
        let is_file = file.file_type().map(|t| t.is_file()).unwrap_or(false);
        let name = file.file_name().to_string_lossy().to_string();
        if !is_file || name.eq_ignore_ascii_case("desktop.ini") {
            continue;
        }
        let full = file.path().to_string_lossy().to_string();
        let exe = if name.to_lowercase().ends_with(".lnk") {
            shortcut_target(&full)
        } else {
            Some(full)
        };
        let enabled = approved(hive, "StartupFolder", &name);
        items.push(entry(source, label, name, exe, enabled, writable, ram));
    }
}

fn walk_tasks(
    folder: &ITaskFolder,
    admin: bool,
    ram: &HashMap<String, f64>,
    items: &mut Vec<Value>,
) -> windows::core::Result<()> {
    unsafe {
        let path = folder.Path()?.to_string();
        if path.to_lowercase().starts_with("\\microsoft") {
            return Ok(());
        }
        let tasks = folder.GetTasks(TASK_ENUM_HIDDEN.0)?;
        for i in 1..=tasks.Count()? {
            let task = tasks.get_Item(&VARIANT::from(i))?;
            let definition = task.Definition()?;
            let triggers = definition.Triggers()?;
            let mut logon = false;
            let mut trigger_count = 0;
            triggers.Count(&mut trigger_count)?;
            for t in 1..=trigger_count {
                let mut kind = Default::default();
                triggers.get_Item(t)?.Type(&mut kind)?;
                logon |= kind == TASK_TRIGGER_LOGON;
            }
            if !logon {
                continue;
            }
            let actions = definition.Actions()?;
            let mut exe = None;
            let mut action_count = 0;
            actions.Count(&mut action_count)?;
            if action_count > 0 {
                let first = actions.get_Item(1)?;
                let mut kind = Default::default();
                first.Type(&mut kind)?;
                if kind == TASK_ACTION_EXEC {
                    let mut raw = windows::core::BSTR::new();
                    first.cast::<IExecAction>()?.Path(&mut raw)?;
                    exe = exe_path(&raw.to_string());
                }
            }
            let state = task.State()?;
            let full = format!("{}{}{}", path, if path.ends_with('\\') { "" } else { "\\" }, task.Name()?);
            items.push(entry(
                "task",
                "Tâche planifiée à l'ouverture de session",
                full,
                exe,
                state != TASK_STATE_DISABLED,
                admin,
                ram,
            ));
        }
        let folders = folder.GetFolders(0)?;
        for i in 1..=folders.Count()? {
            let _ = walk_tasks(&folders.get_Item(&VARIANT::from(i))?, admin, ram, items);
        }
        Ok(())
    }
}

fn scheduled(admin: bool, ram: &HashMap<String, f64>, items: &mut Vec<Value>) {
    win::com();
    let mut read = || -> windows::core::Result<()> {
        unsafe {
            let service: ITaskService = CoCreateInstance(&TaskScheduler, None, CLSCTX_ALL)?;
            service.Connect(&VARIANT::default(), &VARIANT::default(), &VARIANT::default(), &VARIANT::default())?;
            walk_tasks(&service.GetFolder(&windows::core::BSTR::from("\\"))?, admin, ram, items)
        }
    };
    let _ = read();
}

pub fn list() -> Result<Value, String> {
    let admin = crate::elevation::is_admin();
    let mut ram: HashMap<String, f64> = HashMap::new();
    for process in win::processes() {
        if let Some(path) = win::process_path(process.pid) {
            *ram.entry(path.to_lowercase()).or_default() += process.working_set as f64;
        }
    }
    let mut items = Vec::new();
    from_registry(CURRENT_USER, "hkcu-run", RUN, "Run", "Registre, cet utilisateur", true, &ram, &mut items);
    from_registry(LOCAL_MACHINE, "hklm-run", RUN, "Run", "Registre, tous les utilisateurs", admin, &ram, &mut items);
    from_registry(
        LOCAL_MACHINE,
        "hklm-run32",
        "Software\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\Run",
        "Run32",
        "Registre 32 bits, tous les utilisateurs",
        admin,
        &ram,
        &mut items,
    );
    let startup = "Microsoft\\Windows\\Start Menu\\Programs\\Startup";
    if let Some(base) = std::env::var_os("APPDATA") {
        from_folder(CURRENT_USER, "user-folder", &Path::new(&base).join(startup), "Dossier Démarrage, cet utilisateur", true, &ram, &mut items);
    }
    if let Some(base) = std::env::var_os("ProgramData") {
        from_folder(LOCAL_MACHINE, "common-folder", &Path::new(&base).join(startup), "Dossier Démarrage, tous les utilisateurs", admin, &ram, &mut items);
    }
    scheduled(admin, &ram, &mut items);
    items.sort_by(|a, b| {
        let rank = |v: &Value| !v["enabled"].as_bool().unwrap_or(false);
        rank(a)
            .cmp(&rank(b))
            .then(
                b["ramBytes"].as_f64().unwrap_or(0.0).total_cmp(&a["ramBytes"].as_f64().unwrap_or(0.0)),
            )
            .then_with(|| {
                a["name"].as_str().unwrap_or("").to_lowercase().cmp(&b["name"].as_str().unwrap_or("").to_lowercase())
            })
    });
    Ok(json!({"items": items, "isAdmin": admin}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands_give_the_program_path() {
        assert_eq!(exe_path("\"C:\\Program Files\\App\\a.exe\" --min").unwrap(), "C:\\Program Files\\App\\a.exe");
        assert_eq!(exe_path("C:\\Program Files\\App\\a.exe --min").unwrap(), "C:\\Program Files\\App\\a.exe");
        assert_eq!(exe_path("C:\\tools\\run.cmd").unwrap(), "C:\\tools\\run.cmd");
        assert_eq!(exe_path("rundll32 x.dll").unwrap(), "rundll32");
        assert!(exe_path("").is_none());
    }

    #[test]
    fn security_software_is_protected() {
        for name in ["SecurityHealth", "Windows Defender", "Trend Micro", "trendmicro", "Malwarebytes Tray"] {
            assert!(guarded(name), "{name}");
        }
        assert!(!guarded("Discord"));
    }
}
