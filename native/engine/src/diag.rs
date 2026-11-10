use crate::win::{self, events, flag, number, query, text, EventXml};
use serde_json::{json, Value};
use std::collections::HashMap;
use windows::Win32::System::{
    Com::{CoCreateInstance, CLSCTX_ALL},
    UpdateAgent::{AutomaticUpdates, IAutomaticUpdates2},
    Variant::VT_DATE,
};

const THIRTY_DAYS_MS: u64 = 2_592_000_000;
const GPU_CODES: [&str; 7] = ["117", "141", "142", "193", "1a1", "1a8", "1b8"];
pub const STORAGE: &str = "ROOT\\Microsoft\\Windows\\Storage";

fn recent(log: &str, provider: &str, id: u32) -> Vec<EventXml> {
    let xpath = format!(
        "*[System[Provider[@Name='{provider}'] and (EventID={id}) and TimeCreated[timediff(@SystemTime) <= {THIRTY_DAYS_MS}]]]"
    );
    events(log, &xpath, 500).unwrap_or_default()
}

pub fn boots(count: usize) -> Value {
    let xpath = "*[System[(EventID=100)]]";
    match events("Microsoft-Windows-Diagnostics-Performance/Operational", xpath, count) {
        Ok(found) if !found.is_empty() => {
            let items: Vec<Value> = found
                .iter()
                .map(|e| {
                    json!({
                        "date": e.time(),
                        "ms": e.named("BootTime").and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0),
                    })
                })
                .collect();
            json!({"available": true, "items": items})
        }
        Ok(_) => json!({"available": crate::elevation::is_admin(), "items": []}),
        Err(_) => json!({"available": false, "items": []}),
    }
}

fn group(values: impl Iterator<Item = String>) -> Vec<(String, usize)> {
    let mut counts: Vec<(String, usize)> = Vec::new();
    for value in values {
        match counts.iter_mut().find(|(k, _)| k.eq_ignore_ascii_case(&value)) {
            Some(entry) => entry.1 += 1,
            None => counts.push((value, 1)),
        }
    }
    counts.sort_by(|a, b| b.1.cmp(&a.1));
    counts
}

pub fn gpu_code(values: &[String]) -> Option<String> {
    let at = values.iter().position(|v| v == "LiveKernelEvent")?;
    let code = values.get(at + 3)?.to_lowercase();
    GPU_CODES.contains(&code.as_str()).then_some(code)
}

fn stability() -> Value {
    let power = recent("System", "Microsoft-Windows-Kernel-Power", 41);
    let bug = recent("System", "Microsoft-Windows-WER-SystemErrorReporting", 1001);
    let wer = recent("Application", "Windows Error Reporting", 1001);
    let mut seen: HashMap<String, String> = HashMap::new();
    let mut last: Vec<String> = Vec::new();
    for event in &wer {
        let values: Vec<String> = event.data().into_iter().map(|(_, v)| v).collect();
        let Some(code) = gpu_code(&values) else { continue };
        let time = event.time();
        let key = format!("{code}{}", time.get(..16).unwrap_or(""));
        if seen.contains_key(&key) {
            continue;
        }
        seen.insert(key, code);
        last.push(time);
    }
    let codes: Vec<Value> = group(seen.values().cloned())
        .into_iter()
        .map(|(code, count)| json!({"code": code, "count": count}))
        .collect();
    let apps = recent("Application", "Application Error", 1000);
    let top: Vec<Value> = group(
        apps.iter()
            .filter_map(|e| e.data().into_iter().next().map(|(_, v)| v)),
    )
    .into_iter()
    .take(5)
    .map(|(name, count)| json!({"name": name, "count": count}))
    .collect();
    last.extend(power.iter().chain(bug.iter()).map(EventXml::time));
    json!({
        "unexpectedShutdowns": power.len(),
        "blueScreens": bug.len(),
        "gpuHangs": seen.len(),
        "gpuCodes": codes,
        "appCrashes": apps.len(),
        "topCrashes": top,
        "lastIncident": last.into_iter().max(),
    })
}

pub fn media_name(code: Option<f64>) -> &'static str {
    match code.unwrap_or(0.0) as u32 {
        3 => "HDD",
        4 => "SSD",
        5 => "SCM",
        _ => "Unspecified",
    }
}

fn bus_name(code: Option<f64>) -> String {
    match code.unwrap_or(0.0) as u32 {
        1 => "SCSI",
        2 => "ATAPI",
        3 => "ATA",
        4 => "1394",
        5 => "SSA",
        6 => "Fibre Channel",
        7 => "USB",
        8 => "RAID",
        9 => "iSCSI",
        10 => "SAS",
        11 => "SATA",
        12 => "SD",
        13 => "MMC",
        14 => "Virtual",
        15 => "File Backed Virtual",
        16 => "Storage Spaces",
        17 => "NVMe",
        _ => "Unknown",
    }
    .to_string()
}

fn health_name(code: Option<f64>) -> &'static str {
    match code.unwrap_or(5.0) as u32 {
        0 => "Healthy",
        1 => "Warning",
        2 => "Unhealthy",
        _ => "Unknown",
    }
}

fn disks() -> Value {
    let storage = win::wmi_at(STORAGE);
    let counters = query(
        &storage,
        "SELECT DeviceId, Temperature, Wear, ReadErrorsUncorrected, WriteErrorsUncorrected FROM MSFT_StorageReliabilityCounter",
    );
    let list: Vec<Value> = query(
        &storage,
        "SELECT DeviceId, FriendlyName, MediaType, BusType, Size, HealthStatus FROM MSFT_PhysicalDisk",
    )
    .iter()
    .map(|d| {
        let id = text(d, "DeviceId");
        let counter = counters.iter().find(|c| text(c, "DeviceId") == id);
        json!({
            "name": text(d, "FriendlyName"),
            "media": media_name(number(d, "MediaType")),
            "bus": bus_name(number(d, "BusType")),
            "sizeBytes": number(d, "Size").unwrap_or(0.0),
            "health": health_name(number(d, "HealthStatus")),
            "temperature": counter.and_then(|c| number(c, "Temperature")),
            "wear": counter.and_then(|c| number(c, "Wear")),
            "errors": counter.map(|c| number(c, "ReadErrorsUncorrected").unwrap_or(0.0) + number(c, "WriteErrorsUncorrected").unwrap_or(0.0)),
        })
    })
    .collect();
    Value::Array(list)
}

fn security() -> Value {
    let defender = query(
        &win::wmi_at("ROOT\\Microsoft\\Windows\\Defender"),
        "SELECT AMServiceEnabled, RealTimeProtectionEnabled, AntivirusSignatureAge FROM MSFT_MpComputerStatus",
    )
    .first()
    .map(|s| {
        json!({
            "running": flag(s, "AMServiceEnabled"),
            "realtime": flag(s, "RealTimeProtectionEnabled"),
            "signatureAgeDays": number(s, "AntivirusSignatureAge").unwrap_or(0.0) as i64,
        })
    });
    let others: Vec<String> = query(
        &win::wmi_at("ROOT\\SecurityCenter2"),
        "SELECT displayName FROM AntiVirusProduct",
    )
    .iter()
    .map(|p| text(p, "displayName"))
    .filter(|n| !n.to_lowercase().contains("defender"))
    .collect();
    let firewall = win::wmi_at("ROOT\\StandardCimv2");
    let profiles = query(&firewall, "SELECT Name, Enabled FROM MSFT_NetFirewallProfile");
    let off: Value = if firewall.is_some() && !profiles.is_empty() {
        json!(profiles
            .iter()
            .filter(|p| number(p, "Enabled") == Some(0.0))
            .map(|p| text(p, "Name"))
            .collect::<Vec<_>>())
    } else {
        Value::Null
    };
    json!({"defender": defender, "otherAntivirus": others, "firewallOff": off})
}

fn battery() -> Value {
    let wmi = win::wmi_at("ROOT\\WMI");
    let designed = query(&wmi, "SELECT DesignedCapacity FROM BatteryStaticData").first().and_then(|b| number(b, "DesignedCapacity"));
    let full = query(&wmi, "SELECT FullChargedCapacity FROM BatteryFullChargedCapacity").first().and_then(|b| number(b, "FullChargedCapacity"));
    match (designed, full) {
        (Some(designed), Some(full)) if designed > 0.0 && full > 0.0 => json!({"designed": designed, "full": full}),
        _ => Value::Null,
    }
}

fn bitlocker() -> Value {
    let wmi = win::wmi_at("ROOT\\CIMV2\\Security\\MicrosoftVolumeEncryption");
    if wmi.is_none() {
        return Value::Null;
    }
    let list = query(&wmi, "SELECT DriveLetter, ProtectionStatus FROM Win32_EncryptableVolume");
    if list.is_empty() {
        return Value::Null;
    }
    json!(list.iter().filter(|v| !text(v, "DriveLetter").is_empty()).map(|v| json!({"drive": text(v, "DriveLetter"), "protected": number(v, "ProtectionStatus") == Some(1.0)})).collect::<Vec<_>>())
}

pub fn parse_wifi(interfaces: &str, profiles: &str) -> Value {
    let field = |name: &str| {
        interfaces.lines().find_map(|line| {
            let (key, value) = line.split_once(':')?;
            (key.trim() == name).then(|| value.trim().to_string())
        })
    };
    let saved = profiles.lines().filter(|line| line.split_once(':').is_some_and(|(key, value)| key.starts_with("    ") && !value.trim().is_empty())).count();
    let signal = field("Signal").and_then(|value| value.trim_end_matches('%').trim().parse::<u32>().ok());
    match (field("SSID"), signal) {
        (Some(ssid), Some(signal)) if !ssid.is_empty() => json!({"ssid": ssid, "signal": signal, "saved": saved}),
        _ if saved > 0 => json!({"ssid": null, "signal": null, "saved": saved}),
        _ => Value::Null,
    }
}

fn wifi() -> Value {
    use std::os::windows::process::CommandExt;
    let run = |args: &[&str]| std::process::Command::new(win::sys32("netsh.exe")).args(args).creation_flags(0x08000000).output().ok().map(|out| String::from_utf8_lossy(&out.stdout).into_owned()).unwrap_or_default();
    parse_wifi(&run(&["wlan", "show", "interfaces"]), &run(&["wlan", "show", "profiles"]))
}

fn last_update() -> Value {
    win::com();
    let read = || -> windows::core::Result<Option<String>> {
        unsafe {
            let updates: IAutomaticUpdates2 = CoCreateInstance(&AutomaticUpdates, None, CLSCTX_ALL)?;
            let value = updates.Results()?.LastInstallationSuccessDate()?;
            let inner = &value.Anonymous.Anonymous;
            if inner.vt != VT_DATE {
                return Ok(None);
            }
            Ok(win::iso_from_ole(inner.Anonymous.date))
        }
    };
    read().ok().flatten().map(Value::String).unwrap_or(Value::Null)
}

fn reg_key_exists(path: &str) -> bool {
    windows_registry::LOCAL_MACHINE.open(path).is_ok()
}

fn uptime_seconds() -> u64 {
    #[link(name = "kernel32")]
    extern "system" {
        fn GetTickCount64() -> u64;
    }
    unsafe { GetTickCount64() / 1000 }
}

pub fn scan() -> Result<Value, String> {
    let cim = win::wmi_at("ROOT\\CIMV2");
    if cim.is_none() {
        return Err("Windows n'a pas répondu à la lecture du système (WMI).".into());
    }
    let first = |wql: &str| query(&cim, wql).into_iter().next().unwrap_or_default();
    let os = first("SELECT Caption, BuildNumber, TotalVisibleMemorySize, FreePhysicalMemory FROM Win32_OperatingSystem");
    let computer = first("SELECT Manufacturer, Model FROM Win32_ComputerSystem");
    let cpu = first("SELECT Name, NumberOfCores, NumberOfLogicalProcessors FROM Win32_Processor");
    let bios = first("SELECT SMBIOSBIOSVersion FROM Win32_BIOS");
    let board = first("SELECT Manufacturer, Product FROM Win32_BaseBoard");
    let ram = query(&cim, "SELECT ConfiguredClockSpeed FROM Win32_PhysicalMemory");
    let release = windows_registry::LOCAL_MACHINE
        .open("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion")
        .and_then(|k| k.get_string("DisplayVersion"))
        .unwrap_or_default();
    let gpus: Vec<Value> = query(&cim, "SELECT Name, DriverVersion, DriverDate FROM Win32_VideoController")
        .iter()
        .map(|g| {
            json!({
                "name": text(g, "Name"),
                "driverVersion": text(g, "DriverVersion"),
                "driverDate": win::wmi_date(&text(g, "DriverDate")),
            })
        })
        .collect();
    let volumes: Vec<Value> = query(&cim, "SELECT DeviceID, VolumeName, Size, FreeSpace, FileSystem FROM Win32_LogicalDisk WHERE DriveType=3")
        .iter()
        .filter(|v| matches!(text(v, "FileSystem").as_str(), "NTFS" | "ReFS"))
        .map(|v| {
            json!({
                "drive": text(v, "DeviceID"),
                "label": text(v, "VolumeName"),
                "sizeBytes": number(v, "Size").unwrap_or(0.0),
                "freeBytes": number(v, "FreeSpace").unwrap_or(0.0),
            })
        })
        .collect();
    let problems = query(&cim, "SELECT ConfigManagerErrorCode FROM Win32_PnPEntity WHERE ConfigManagerErrorCode <> 0 AND ConfigManagerErrorCode <> 22").len();
    let pending = reg_key_exists("SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Component Based Servicing\\RebootPending")
        || reg_key_exists("SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\WindowsUpdate\\Auto Update\\RebootRequired");
    let board_name = format!("{} {}", text(&board, "Manufacturer"), text(&board, "Product"));
    Ok(json!({
        "isAdmin": crate::elevation::is_admin(),
        "timestamp": chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        "os": {"name": text(&os, "Caption"), "release": release, "build": text(&os, "BuildNumber")},
        "computer": {"manufacturer": text(&computer, "Manufacturer"), "model": text(&computer, "Model"), "board": board_name.trim(), "bios": text(&bios, "SMBIOSBIOSVersion")},
        "cpu": {"name": text(&cpu, "Name").trim(), "cores": number(&cpu, "NumberOfCores").unwrap_or(0.0) as u64, "threads": number(&cpu, "NumberOfLogicalProcessors").unwrap_or(0.0) as u64},
        "memory": {
            "totalBytes": number(&os, "TotalVisibleMemorySize").unwrap_or(0.0) * 1024.0,
            "freeBytes": number(&os, "FreePhysicalMemory").unwrap_or(0.0) * 1024.0,
            "modules": ram.len(),
            "speedMHz": ram.first().and_then(|r| number(r, "ConfiguredClockSpeed")).map(|v| v as u64),
        },
        "gpus": gpus,
        "uptimeSeconds": uptime_seconds(),
        "pendingReboot": pending,
        "systemDrive": std::env::var("SystemDrive").unwrap_or_else(|_| "C:".into()),
        "volumes": volumes,
        "disks": disks(),
        "deviceProblems": problems,
        "security": security(),
        "battery": battery(),
        "bitlocker": bitlocker(),
        "wifi": wifi(),
        "stability": stability(),
        "lastUpdate": last_update(),
        "boots": boots(1),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wifi_is_read_from_netsh_in_french_or_english() {
        let interfaces = "\n    Nom                    : Wi-Fi\n    SSID                   : Livebox-1234\n    BSSID                  : aa:bb:cc:dd:ee:ff\n    Signal                 : 87%\n";
        let profiles = "Profils sur l'interface Wi-Fi :\n\nProfils utilisateurs\n---------------------\n    Profil Tous les utilisateurs : Livebox-1234\n    Profil Tous les utilisateurs : Bureau\n";
        assert_eq!(parse_wifi(interfaces, profiles), json!({"ssid": "Livebox-1234", "signal": 87, "saved": 2}));
        assert_eq!(parse_wifi("There is no wireless interface on the system.", ""), Value::Null);
        assert_eq!(parse_wifi("", "    All User Profile     : Home\n"), json!({"ssid": null, "signal": null, "saved": 1}));
    }

    #[test]
    fn event_xml_reads_values_in_order_with_empty_ones() {
        let event = EventXml("<Event><System><TimeCreated SystemTime='2026-11-03T09:00:00.1234567Z'/></System><EventData><Data Name='BootTime'>21000</Data><Data/><Data Name='x'>a &amp; b</Data></EventData></Event>".into());
        assert_eq!(event.named("BootTime").unwrap(), "21000");
        assert_eq!(event.data().len(), 3);
        assert_eq!(event.data()[2].1, "a & b");
        assert_eq!(event.time(), "2026-11-03T09:00:00.123Z");
    }

    #[test]
    fn gpu_codes_come_from_live_kernel_events_only() {
        let values: Vec<String> = ["0", "LiveKernelEvent", "x", "y", "1A8"].iter().map(|s| s.to_string()).collect();
        assert_eq!(gpu_code(&values).unwrap(), "1a8");
        let other: Vec<String> = ["0", "LiveKernelEvent", "x", "y", "999"].iter().map(|s| s.to_string()).collect();
        assert!(gpu_code(&other).is_none());
        assert!(gpu_code(&["APPCRASH".to_string()]).is_none());
    }

    #[test]
    fn ole_dates_become_iso() {
        assert_eq!(win::iso_from_ole(46000.5).unwrap(), "2025-12-09T12:00:00.000Z");
        assert!(win::iso_from_ole(0.0).is_none());
    }
}
