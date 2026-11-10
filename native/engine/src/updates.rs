use crate::win::{self, number, query, text};
use serde_json::{json, Value};
use std::os::windows::process::CommandExt;
use windows::{
    core::{Interface, BSTR},
    Win32::System::{
        Com::{CoCreateInstance, CLSCTX_ALL},
        Ole::VarR8FromDec,
        UpdateAgent::{IUpdate, IUpdateSession, IWindowsDriverUpdate, UpdateSession},
    },
};

struct Vendor {
    id: &'static str,
    name: &'static str,
    url: &'static str,
    words: &'static [&'static str],
    within: &'static [&'static str],
}

const OEM: [Vendor; 9] = [
    Vendor { id: "nvidia", name: "NVIDIA", url: "https://www.nvidia.com/fr-fr/software/nvidia-app/", words: &[], within: &["nvidia"] },
    Vendor { id: "amd", name: "AMD", url: "https://www.amd.com/fr/support/download/drivers.html", words: &["amd"], within: &["advanced micro devices", "authenticamd", "radeon", "ryzen"] },
    Vendor { id: "intel", name: "Intel", url: "https://www.intel.fr/content/www/fr/fr/support/detect.html", words: &[], within: &["intel"] },
    Vendor { id: "gigabyte", name: "GIGABYTE", url: "https://www.gigabyte.com/fr/Support", words: &[], within: &["gigabyte", "giga-byte"] },
    Vendor { id: "asus", name: "ASUS", url: "https://www.asus.com/fr/support/download-center/", words: &[], within: &["asus"] },
    Vendor { id: "msi", name: "MSI", url: "https://fr.msi.com/support/download", words: &["msi"], within: &["micro-star"] },
    Vendor { id: "dell", name: "Dell", url: "https://www.dell.com/support/home/fr-fr", words: &[], within: &["dell"] },
    Vendor { id: "lenovo", name: "Lenovo", url: "https://pcsupport.lenovo.com/fr/fr", words: &[], within: &["lenovo"] },
    Vendor { id: "hp", name: "HP", url: "https://support.hp.com/fr-fr/drivers", words: &["hp"], within: &["hewlett"] },
];

fn has_word(haystack: &str, word: &str) -> bool {
    haystack
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .any(|part| part == word)
}

fn vendors_in(seen: &str) -> Vec<&'static Vendor> {
    let lower = seen.to_lowercase();
    OEM.iter()
        .filter(|v| {
            v.within.iter().any(|w| lower.contains(w)) || v.words.iter().any(|w| has_word(&lower, w))
        })
        .collect()
}

pub fn devices() -> Result<Value, String> {
    let cim = win::wmi_at("ROOT\\CIMV2");
    if cim.is_none() {
        return Err("Windows n'a pas répondu à la lecture des pilotes (WMI).".into());
    }
    let drivers: Vec<Value> = query(&cim, "SELECT DeviceName, Manufacturer, DriverVersion, DriverDate, DeviceClass, IsSigned FROM Win32_PnPSignedDriver")
        .iter()
        .filter(|d| !text(d, "DeviceName").is_empty())
        .map(|d| {
            json!({
                "name": text(d, "DeviceName"),
                "manufacturer": text(d, "Manufacturer"),
                "version": text(d, "DriverVersion"),
                "deviceClass": text(d, "DeviceClass"),
                "signed": win::flag(d, "IsSigned"),
                "date": win::wmi_date(&text(d, "DriverDate")),
            })
        })
        .collect();
    let problems: Vec<Value> = query(&cim, "SELECT Name, Manufacturer, PNPClass, ConfigManagerErrorCode FROM Win32_PnPEntity WHERE ConfigManagerErrorCode <> 0 AND ConfigManagerErrorCode <> 22")
        .iter()
        .map(|p| {
            json!({
                "name": text(p, "Name"),
                "manufacturer": text(p, "Manufacturer"),
                "deviceClass": text(p, "PNPClass"),
                "code": number(p, "ConfigManagerErrorCode").unwrap_or(0.0),
            })
        })
        .collect();
    let mut seen = Vec::new();
    for (class, field) in [
        ("Win32_ComputerSystem", "Manufacturer"),
        ("Win32_BaseBoard", "Manufacturer"),
        ("Win32_Processor", "Manufacturer"),
        ("Win32_VideoController", "Name"),
    ] {
        for row in query(&cim, &format!("SELECT {field} FROM {class}")) {
            seen.push(text(&row, field));
        }
    }
    let vendors: Vec<Value> = vendors_in(&seen.join(" "))
        .into_iter()
        .map(|v| json!({"id": v.id, "name": v.name}))
        .collect();
    Ok(json!({"drivers": drivers, "problems": problems, "vendors": vendors}))
}

pub fn open_vendor(id: &str) -> Result<Value, String> {
    let vendor = OEM.iter().find(|v| v.id == id).ok_or("Fabricant inconnu")?;
    win::shell_open(vendor.url)?;
    Ok(json!({"opened": true}))
}

fn session() -> windows::core::Result<IUpdateSession> {
    win::com();
    unsafe {
        let session: IUpdateSession = CoCreateInstance(&UpdateSession, None, CLSCTX_ALL)?;
        session.SetClientApplicationID(&BSTR::from("Nova PC Suite"))?;
        Ok(session)
    }
}

fn convert(update: &IUpdate) -> windows::core::Result<Value> {
    unsafe {
        let size = update
            .MaxDownloadSize()
            .ok()
            .and_then(|d| VarR8FromDec(&d).ok())
            .unwrap_or(0.0);
        let kb: Vec<String> = {
            let ids = update.KBArticleIDs()?;
            (0..ids.Count()?)
                .filter_map(|i| ids.get_Item(i).ok().map(|k| format!("KB{k}")))
                .collect()
        };
        let accepted = update.EulaAccepted()?.as_bool();
        let driver = update.cast::<IWindowsDriverUpdate>().ok();
        let read = |f: &dyn Fn(&IWindowsDriverUpdate) -> windows::core::Result<BSTR>| {
            driver.as_ref().and_then(|d| f(d).ok()).map(|s| s.to_string()).unwrap_or_default()
        };
        Ok(json!({
            "id": update.Identity()?.UpdateID()?.to_string(),
            "title": update.Title()?.to_string(),
            "description": update.Description().map(|s| s.to_string()).unwrap_or_default(),
            "kb": kb,
            "sizeBytes": size,
            "severity": update.MsrcSeverity().map(|s| s.to_string()).unwrap_or_default(),
            "eulaAccepted": accepted,
            "eula": if accepted { Value::Null } else { json!(update.EulaText().map(|s| s.to_string()).unwrap_or_default()) },
            "rebootLikely": update.InstallationBehavior().and_then(|b| b.RebootBehavior()).map(|r| r.0 != 0).unwrap_or(false),
            "driverClass": read(&|d| d.DriverClass()),
            "driverProvider": read(&|d| d.DriverProvider()),
            "driverModel": read(&|d| d.DriverModel()),
            "driverDate": driver.as_ref().and_then(|d| d.DriverVerDate().ok()).map(win::ymd_from_ole).unwrap_or(Value::Null),
        }))
    }
}

pub fn wu_search(kind: &str) -> Result<Value, String> {
    let kind = if kind == "Driver" { "Driver" } else { "Software" };
    let failed = |_| "Windows Update n'a pas terminé la recherche. Vérifiez la connexion et réessayez.".to_string();
    let session = session().map_err(failed)?;
    unsafe {
        let searcher = session.CreateUpdateSearcher().map_err(failed)?;
        let criteria = format!("IsInstalled=0 and IsHidden=0 and Type='{kind}'");
        let result = searcher.Search(&BSTR::from(criteria)).map_err(failed)?;
        if result.ResultCode().map_err(failed)?.0 != 2 {
            return Err("Windows Update n'a pas terminé la recherche. Vérifiez la connexion et réessayez.".into());
        }
        let list = result.Updates().map_err(failed)?;
        let mut items = Vec::new();
        for i in 0..list.Count().map_err(failed)? {
            let update = list.get_Item(i).map_err(failed)?;
            items.push(convert(&update).map_err(failed)?);
        }
        Ok(json!({
            "items": items,
            "searchedAt": chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        }))
    }
}

pub fn wu_history() -> Result<Value, String> {
    let failed = |_| "Windows Update n'a pas répondu à la lecture de l'historique.".to_string();
    let session = session().map_err(failed)?;
    unsafe {
        let searcher = session.CreateUpdateSearcher().map_err(failed)?;
        let total = searcher.GetTotalHistoryCount().map_err(failed)?;
        let mut items = Vec::new();
        if total > 0 {
            let history = searcher.QueryHistory(0, total.min(15)).map_err(failed)?;
            for i in 0..history.Count().map_err(failed)? {
                let entry = history.get_Item(i).map_err(failed)?;
                let title = entry.Title().map(|t| t.to_string()).unwrap_or_default();
                if title.is_empty() {
                    continue;
                }
                items.push(json!({
                    "title": title,
                    "date": entry.Date().ok().and_then(win::iso_from_ole),
                    "ok": matches!(entry.ResultCode().map(|c| c.0), Ok(2 | 3)),
                }));
            }
        }
        Ok(json!({"items": items}))
    }
}

fn read_uninstall(hive: &windows_registry::Key, path: &str, tag: &str, list: &mut Vec<Value>) {
    let Ok(root) = hive.open(path) else { return };
    let Ok(keys) = root.keys() else { return };
    for name in keys {
        let Ok(app) = root.open(&name) else { continue };
        let display = app.get_string("DisplayName").unwrap_or_default();
        let hidden = app.get_u32("SystemComponent").unwrap_or(0) == 1
            || app.get_string("ParentKeyName").is_ok()
            || app.get_string("ReleaseType").is_ok_and(|r| r.contains("Update") || r.contains("Hotfix"));
        if display.trim().is_empty() || hidden {
            continue;
        }
        let date = app.get_string("InstallDate").unwrap_or_default();
        list.push(json!({
            "name": display.trim(),
            "version": app.get_string("DisplayVersion").unwrap_or_default(),
            "publisher": app.get_string("Publisher").unwrap_or_default(),
            "bytes": app.get_u32("EstimatedSize").unwrap_or(0) as f64 * 1024.0,
            "date": win::wmi_date(&date),
            "id": format!("{tag}|{name}"),
            "removable": app.get_string("UninstallString").is_ok_and(|command| !command.trim().is_empty()),
        }));
    }
}

pub fn dedupe_installed(list: Vec<Value>) -> Vec<Value> {
    let mut seen = std::collections::HashSet::new();
    let mut unique: Vec<Value> = list
        .into_iter()
        .filter(|a| seen.insert(format!("{}|{}", a["name"].as_str().unwrap_or("").to_lowercase(), a["version"])))
        .collect();
    unique.sort_by_key(|a| a["name"].as_str().unwrap_or("").to_lowercase());
    unique
}

pub fn installed() -> Result<Value, String> {
    const KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall";
    const KEY32: &str = "Software\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\Uninstall";
    let mut list = Vec::new();
    read_uninstall(windows_registry::LOCAL_MACHINE, KEY, "hklm", &mut list);
    read_uninstall(windows_registry::LOCAL_MACHINE, KEY32, "hklm32", &mut list);
    read_uninstall(windows_registry::CURRENT_USER, KEY, "hkcu", &mut list);
    Ok(json!({"items": dedupe_installed(list)}))
}

pub fn split_command(command: &str) -> Option<(String, String)> {
    let command = command.trim();
    if let Some(rest) = command.strip_prefix('"') {
        let end = rest.find('"')?;
        return Some((rest[..end].to_string(), rest[end + 1..].trim().to_string()));
    }
    let lower = command.to_lowercase();
    let cut = lower.find(".exe").map(|index| index + 4).unwrap_or(command.len());
    let (exe, args) = command.split_at(cut);
    (!exe.trim().is_empty()).then(|| (exe.trim().to_string(), args.trim().to_string()))
}

pub fn uninstall(id: &str) -> Result<Value, String> {
    use std::os::windows::process::CommandExt;
    let (tag, name) = id.split_once('|').ok_or("Logiciel inconnu.")?;
    if name.is_empty() || name.contains(['\\', '/']) {
        return Err("Logiciel inconnu.".into());
    }
    let (hive, path) = match tag {
        "hklm" => (windows_registry::LOCAL_MACHINE, "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall"),
        "hklm32" => (windows_registry::LOCAL_MACHINE, "Software\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\Uninstall"),
        "hkcu" => (windows_registry::CURRENT_USER, "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall"),
        _ => return Err("Logiciel inconnu.".into()),
    };
    let app = hive.open(path).and_then(|root| root.open(name)).map_err(|_| "Ce logiciel n'est plus installé.")?;
    let display = app.get_string("DisplayName").unwrap_or_else(|_| name.to_string());
    let command = app.get_string("UninstallString").map_err(|_| "Ce logiciel ne propose pas de désinstallation.")?;
    let (exe, args) = split_command(&command).ok_or("Commande de désinstallation illisible.")?;
    std::process::Command::new(&exe).raw_arg(&args).spawn().map_err(|error| format!("Le désinstalleur n'a pas pu démarrer : {error}"))?;
    Ok(json!({"summary": format!("Désinstalleur de {} ouvert : suivez ses étapes.", display.trim()), "measures": []}))
}

pub fn parse_winget(output: &str) -> Vec<Value> {
    let cleaned = output.replace('\r', "\n");
    let lines: Vec<Vec<char>> = cleaned
        .split('\n')
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.chars().collect())
        .collect();
    let Some(sep) = lines
        .iter()
        .position(|l| l.len() >= 10 && l[..10].iter().all(|c| *c == '-'))
    else {
        return Vec::new();
    };
    if sep < 1 {
        return Vec::new();
    }
    let head = &lines[sep - 1];
    let id_col = (1..head.len().saturating_sub(1))
        .find(|&i| {
            head[i - 1].is_whitespace()
                && head[i].eq_ignore_ascii_case(&'i')
                && head[i + 1].eq_ignore_ascii_case(&'d')
                && head.get(i + 2).map(|c| !c.is_alphanumeric()).unwrap_or(true)
        })
        .unwrap_or(0);
    let mut cols = Vec::new();
    for i in id_col..head.len() {
        if !head[i].is_whitespace() && (i == id_col || head[i - 1].is_whitespace()) {
            cols.push(i);
        }
    }
    if cols.len() < 3 {
        return Vec::new();
    }
    let cut = |line: &[char], from: usize, to: usize| -> String {
        line[from.min(line.len())..to.min(line.len())]
            .iter()
            .collect::<String>()
            .trim()
            .to_string()
    };
    let mut items = Vec::new();
    for line in &lines[sep + 1..] {
        let first_is_count = {
            let text: String = line.iter().collect();
            let mut parts = text.split_whitespace();
            parts.next().map(|p| p.bytes().all(|b| b.is_ascii_digit())).unwrap_or(false)
                && line.first().map(char::is_ascii_digit).unwrap_or(false)
                && parts.next().is_some()
        };
        if line.len() <= cols[2] || first_is_count {
            continue;
        }
        let id = cut(line, cols[0], cols[1]);
        let mut bytes = id.bytes();
        let valid = id.len() >= 2
            && bytes.next().map(|b| b.is_ascii_alphanumeric()).unwrap_or(false)
            && bytes.all(|b| b.is_ascii_alphanumeric() || b".-_+".contains(&b));
        if !valid {
            continue;
        }
        let available = if cols.len() > 3 && line.len() > cols[3] {
            cut(line, cols[2], cols[3])
        } else {
            cut(line, cols[2], line.len())
        };
        items.push(json!({
            "name": cut(line, 0, cols[0]),
            "id": id,
            "version": cut(line, cols[1], cols[2]),
            "available": available,
        }));
    }
    items
}

pub fn winget_list() -> Result<Value, String> {
    let base = std::env::var("LOCALAPPDATA").map_err(|_| "WinGet n'est pas installé sur ce PC.")?;
    let exe = format!("{base}\\Microsoft\\WindowsApps\\winget.exe");
    if std::fs::symlink_metadata(&exe).is_err() {
        return Err("WinGet n'est pas installé sur ce PC.".into());
    }
    let output = std::process::Command::new(&exe)
        .args(["upgrade", "--accept-source-agreements", "--disable-interactivity"])
        .creation_flags(0x08000000)
        .stdin(std::process::Stdio::null())
        .output()
        .map_err(|_| "Windows a refusé de lancer WinGet.")?;
    Ok(json!({"items": parse_winget(&String::from_utf8_lossy(&output.stdout))}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_real_installed_list_has_names() {
        let data = installed().unwrap();
        let items = data["items"].as_array().unwrap();
        assert!(!items.is_empty());
        assert!(items.iter().all(|i| !i["name"].as_str().unwrap().is_empty()));
    }

    #[test]
    fn uninstall_commands_are_split_like_windows() {
        assert_eq!(split_command(r#""C:\Program Files\App\unins000.exe" /SILENT"#), Some((r"C:\Program Files\App\unins000.exe".to_string(), "/SILENT".to_string())));
        assert_eq!(split_command("MsiExec.exe /X{1234-5678}"), Some(("MsiExec.exe".to_string(), "/X{1234-5678}".to_string())));
        assert_eq!(split_command(r"C:\Tools\remove.exe"), Some((r"C:\Tools\remove.exe".to_string(), String::new())));
        assert_eq!(split_command("   "), None);
        assert!(uninstall("hklm|..\\Windows").is_err());
        assert!(uninstall("autre|x").is_err());
    }

    #[test]
    fn installed_software_is_deduplicated_and_sorted() {
        let list = vec![json!({"name": "b", "version": "1"}), json!({"name": "A", "version": "2"}), json!({"name": "a", "version": "2"})];
        let done = dedupe_installed(list);
        assert_eq!(done.len(), 2);
        assert_eq!(done[0]["name"], "A");
    }

    #[test]
    fn winget_table_is_read_by_columns() {
        let sample = "   - \r   \\ \r\nNom                 ID                 Version   Disponible Source\n------------------------------------------------------------------\nGit                 Git.Git            2.40.0    2.47.0     winget\nÉditeur très long…  Vendor.Tool        1.0       1.1        winget\nCassé               ?bad id            1.0       1.1        winget\n2 mises à niveau disponibles.\n";
        let items = parse_winget(sample);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0]["id"], "Git.Git");
        assert_eq!(items[0]["version"], "2.40.0");
        assert_eq!(items[0]["available"], "2.47.0");
        assert_eq!(items[1]["name"], "Éditeur très long…");
    }

    #[test]
    fn winget_without_table_means_nothing_to_report() {
        assert!(parse_winget("Aucune mise à jour disponible.").is_empty());
    }

    #[test]
    fn vendors_need_a_real_word_match() {
        let found: Vec<&str> = vendors_in("Micro-Star International AuthenticAMD NVIDIA GeForce RTX").iter().map(|v| v.id).collect();
        assert_eq!(found, ["nvidia", "amd", "msi"]);
        assert!(vendors_in("Chipset Champ").iter().all(|v| v.id != "hp"));
    }
}
