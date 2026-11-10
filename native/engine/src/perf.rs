use crate::diag::{media_name, STORAGE};
use crate::win::{self, events, from_wide, number, query, text};
use serde_json::{json, Value};
use std::{collections::HashMap, os::windows::process::CommandExt, time::Instant};
use windows::{
    core::{GUID, HSTRING},
    Win32::{
        Foundation::{HLOCAL, LocalFree, ERROR_SUCCESS},
        NetworkManagement::{
            Dns::{
                DnsFree, DnsFreeRecordList, DnsQuery_W, DNS_QUERY_BYPASS_CACHE,
                DNS_QUERY_NO_HOSTS_FILE, DNS_TYPE_A,
            },
            IpHelper::{GetBestRoute, IcmpCloseHandle, IcmpCreateFile, IcmpSendEcho, MIB_IPFORWARDROW},
        },
        System::Power::{
            PowerEnumerate, PowerGetActiveScheme, PowerReadFriendlyName, ACCESS_SCHEME,
        },
    },
};

const NO_CLOSE: [&str; 26] = [
    "system", "idle", "registry", "smss", "csrss", "wininit", "winlogon", "services", "lsass",
    "svchost", "fontdrvhost", "dwm", "explorer", "sihost", "ctfmon", "memory compression",
    "nova-pc-suite", "msmpeng", "searchhost", "startmenuexperiencehost", "shellexperiencehost",
    "textinputhost", "runtimebroker", "taskhostw", "dllhost", "conhost",
];

fn may_close(name: &str) -> bool {
    let lower = name.to_lowercase();
    !NO_CLOSE.contains(&lower.as_str())
        && lower != "applicationframehost"
        && lower != "nova-native"
        && !lower.starts_with("securityhealth")
}

fn guid_text(g: &GUID) -> String {
    format!("{g:?}").to_lowercase()
}

fn plans() -> Vec<Value> {
    let mut active = std::ptr::null_mut();
    let current = unsafe {
        if PowerGetActiveScheme(None, &mut active) == ERROR_SUCCESS {
            let guid = *active;
            let _ = LocalFree(Some(HLOCAL(active.cast())));
            Some(guid)
        } else {
            None
        }
    };
    let mut list = Vec::new();
    let mut index = 0;
    loop {
        let mut guid = GUID::zeroed();
        let mut size = std::mem::size_of::<GUID>() as u32;
        let status = unsafe {
            PowerEnumerate(
                None,
                None,
                None,
                ACCESS_SCHEME,
                index,
                Some((&mut guid as *mut GUID).cast()),
                &mut size,
            )
        };
        if status != ERROR_SUCCESS {
            break;
        }
        index += 1;
        let mut bytes = 0u32;
        unsafe {
            let _ = PowerReadFriendlyName(None, Some(&guid), None, None, None, &mut bytes);
        }
        let mut buffer = vec![0u16; bytes as usize / 2 + 1];
        let read = unsafe {
            PowerReadFriendlyName(
                None,
                Some(&guid),
                None,
                None,
                Some(buffer.as_mut_ptr().cast()),
                &mut bytes,
            )
        };
        if read != ERROR_SUCCESS {
            continue;
        }
        list.push(json!({
            "guid": guid_text(&guid),
            "name": from_wide(&buffer),
            "active": current == Some(guid),
        }));
    }
    list
}

pub fn letter_in_parentheses(text: &str) -> Option<char> {
    let bytes = text.as_bytes();
    bytes.windows(4).find_map(|w| {
        (w[0] == b'(' && w[1].is_ascii_alphabetic() && w[2] == b':' && w[3] == b')')
            .then(|| (w[1] as char).to_ascii_uppercase())
    })
}

fn drive_letter(row: &win::Row) -> Option<char> {
    let from_text = text(row, "DriveLetter").chars().next();
    from_text
        .or_else(|| number(row, "DriveLetter").and_then(|code| char::from_u32(code as u32)))
        .filter(char::is_ascii_alphabetic)
}

fn volumes() -> Vec<Value> {
    let storage = win::wmi_at(STORAGE);
    let media: HashMap<String, &str> = query(&storage, "SELECT DeviceId, MediaType FROM MSFT_PhysicalDisk")
        .iter()
        .map(|d| (text(d, "DeviceId"), media_name(number(d, "MediaType"))))
        .collect();
    let disk_of: HashMap<char, String> = query(&storage, "SELECT DriveLetter, DiskNumber FROM MSFT_Partition")
        .iter()
        .filter_map(|p| {
            let letter = drive_letter(p)?;
            Some((letter.to_ascii_uppercase(), format!("{}", number(p, "DiskNumber")? as u32)))
        })
        .collect();
    let mut last: HashMap<char, String> = HashMap::new();
    let xpath = "*[System[Provider[@Name='Microsoft-Windows-Defrag'] and (EventID=258)]]";
    for event in events("Application", xpath, 40).unwrap_or_default() {
        for (_, value) in event.data() {
            if let Some(letter) = letter_in_parentheses(&value) {
                last.entry(letter).or_insert_with(|| event.time());
            }
        }
    }
    let system = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".into());
    query(&storage, "SELECT DriveLetter, DriveType, FileSystem, FileSystemLabel, Size, SizeRemaining FROM MSFT_Volume")
        .iter()
        .filter_map(|v| {
            let letter = drive_letter(v)?;
            let fixed = number(v, "DriveType") == Some(3.0);
            if !fixed || !matches!(text(v, "FileSystem").as_str(), "NTFS" | "ReFS") {
                return None;
            }
            let upper = letter.to_ascii_uppercase();
            let kind = disk_of.get(&upper).and_then(|d| media.get(d)).copied();
            Some(json!({
                "letter": letter.to_string(),
                "label": text(v, "FileSystemLabel"),
                "sizeBytes": number(v, "Size").unwrap_or(0.0),
                "freeBytes": number(v, "SizeRemaining").unwrap_or(0.0),
                "media": kind,
                "lastOptimized": last.get(&upper),
                "system": format!("{letter}:").eq_ignore_ascii_case(&system),
            }))
        })
        .collect()
}

fn hibernation() -> Value {
    let enabled = windows_registry::LOCAL_MACHINE
        .open("SYSTEM\\CurrentControlSet\\Control\\Power")
        .and_then(|k| k.get_u32("HibernateEnabled"))
        .map(|v| v != 0)
        .unwrap_or(false);
    let drive = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".into());
    let size = std::fs::metadata(format!("{drive}\\hiberfil.sys"))
        .map(|m| m.len() as f64)
        .unwrap_or(0.0);
    json!({"enabled": enabled, "fileBytes": size})
}

pub fn parse_nvidia(line: &str) -> Option<Value> {
    let fields: Vec<&str> = line.split(',').map(str::trim).collect();
    if fields.len() < 7 {
        return None;
    }
    let num = |i: usize| fields[i].parse::<f64>().ok();
    Some(json!({
        "name": fields[0],
        "temperature": num(1)?,
        "usePercent": num(2)?,
        "memoryUsedMiB": num(3)?,
        "memoryTotalMiB": num(4)?,
        "powerW": num(5),
        "driver": fields[6],
    }))
}

fn gpu() -> Value {
    let exe = win::sys32("nvidia-smi.exe");
    if !std::path::Path::new(&exe).exists() {
        return Value::Null;
    }
    std::process::Command::new(exe)
        .args([
            "--query-gpu=name,temperature.gpu,utilization.gpu,memory.used,memory.total,power.draw,driver_version",
            "--format=csv,noheader,nounits",
        ])
        .creation_flags(0x08000000)
        .output()
        .ok()
        .and_then(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .next()
                .and_then(parse_nvidia)
        })
        .unwrap_or(Value::Null)
}

pub fn overview() -> Result<Value, String> {
    Ok(json!({
        "plans": plans(),
        "volumes": volumes(),
        "hibernation": hibernation(),
        "gpu": gpu(),
        "isAdmin": crate::elevation::is_admin(),
    }))
}

pub fn processes() -> Result<Value, String> {
    let first: HashMap<u32, u64> = win::processes().iter().map(|p| (p.pid, p.cpu_ticks)).collect();
    let started = Instant::now();
    std::thread::sleep(std::time::Duration::from_millis(1000));
    let second = win::processes();
    let elapsed = started.elapsed().as_secs_f64();
    let windows = win::pids_with_window();
    let logical = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1) as f64;
    struct Group {
        name: String,
        count: u32,
        cpu: f64,
        ram: f64,
        closable: bool,
    }
    let mut groups: HashMap<String, Group> = HashMap::new();
    for p in second {
        if p.pid == 0 || p.pid == 4 {
            continue;
        }
        let name = p.name.strip_suffix(".exe").or_else(|| p.name.strip_suffix(".EXE")).unwrap_or(&p.name).to_string();
        let cpu = first
            .get(&p.pid)
            .map(|before| p.cpu_ticks.saturating_sub(*before) as f64 / 10_000_000.0 / elapsed * 100.0 / logical)
            .unwrap_or(0.0);
        let group = groups.entry(name.to_lowercase()).or_insert(Group {
            name: name.clone(),
            count: 0,
            cpu: 0.0,
            ram: 0.0,
            closable: false,
        });
        group.count += 1;
        group.cpu += cpu;
        group.ram += p.working_set as f64;
        if windows.contains(&p.pid) && may_close(&name) {
            group.closable = true;
        }
    }
    let mut ranked: Vec<Group> = groups.into_values().collect();
    ranked.sort_by(|a, b| b.ram.total_cmp(&a.ram));
    let items: Vec<Value> = ranked
        .iter()
        .enumerate()
        .filter(|(i, g)| *i < 25 || g.closable)
        .map(|(_, g)| {
            json!({
                "name": g.name,
                "count": g.count,
                "cpuPercent": (g.cpu * 10.0).round() / 10.0,
                "ramBytes": g.ram,
                "closable": g.closable,
            })
        })
        .collect();
    Ok(json!({"items": items}))
}

pub fn link_speed(bits: f64) -> String {
    if bits >= 1e9 {
        format!("{} Gb/s", crate::report::fr_number(bits / 1e9, 1))
    } else if bits > 0.0 {
        format!("{} Mb/s", crate::report::fr_number(bits / 1e6, 0))
    } else {
        String::new()
    }
}

fn strings(row: &win::Row, key: &str) -> Vec<String> {
    match row.get(key) {
        Some(win::Variant::Array(list)) => list
            .iter()
            .filter_map(|v| match v {
                win::Variant::String(s) => Some(s.clone()),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

pub fn adapters() -> Result<Value, String> {
    let cim = win::wmi_at("ROOT\\CIMV2");
    let links = query(&cim, "SELECT InterfaceIndex, Speed FROM Win32_NetworkAdapter WHERE NetEnabled = TRUE");
    let items: Vec<Value> = query(&cim, "SELECT Description, InterfaceIndex, IPAddress, DefaultIPGateway, MACAddress FROM Win32_NetworkAdapterConfiguration WHERE IPEnabled = TRUE")
        .iter()
        .map(|a| {
            let index = number(a, "InterfaceIndex");
            let speed = links.iter().find(|l| number(l, "InterfaceIndex") == index).and_then(|l| number(l, "Speed")).unwrap_or(0.0);
            json!({
                "name": text(a, "Description"),
                "addresses": strings(a, "IPAddress"),
                "gateway": strings(a, "DefaultIPGateway").into_iter().next(),
                "mac": text(a, "MACAddress"),
                "speed": link_speed(speed),
            })
        })
        .collect();
    Ok(json!({"items": items}))
}

fn ping(target: std::net::Ipv4Addr, role: &str) -> Value {
    let address = u32::from_ne_bytes(target.octets());
    let mut times = Vec::new();
    let mut lost = 0;
    unsafe {
        match IcmpCreateFile() {
            Ok(handle) => {
                let payload = [0x4eu8; 32];
                for _ in 0..5 {
                    let mut reply = [0u8; 256];
                    let answers = IcmpSendEcho(
                        handle,
                        address,
                        payload.as_ptr().cast(),
                        payload.len() as u16,
                        None,
                        reply.as_mut_ptr().cast(),
                        reply.len() as u32,
                        1000,
                    );
                    let status = u32::from_le_bytes([reply[4], reply[5], reply[6], reply[7]]);
                    if answers > 0 && status == 0 {
                        times.push(u32::from_le_bytes([reply[8], reply[9], reply[10], reply[11]]) as f64);
                    } else {
                        lost += 1;
                    }
                }
                let _ = IcmpCloseHandle(handle);
            }
            Err(_) => lost = 5,
        }
    }
    let average = (!times.is_empty()).then(|| {
        (times.iter().sum::<f64>() / times.len() as f64 * 10.0).round() / 10.0
    });
    json!({
        "target": target.to_string(),
        "role": role,
        "lossPercent": lost * 20,
        "averageMs": average,
        "worstMs": times.iter().copied().reduce(f64::max),
    })
}

fn gateway() -> Option<std::net::Ipv4Addr> {
    let mut row = MIB_IPFORWARDROW::default();
    let destination = u32::from_ne_bytes([1, 1, 1, 1]);
    if unsafe { GetBestRoute(destination, Some(0), &mut row) } != 0 {
        return None;
    }
    let hop = std::net::Ipv4Addr::from(row.dwForwardNextHop.to_ne_bytes());
    (!hop.is_unspecified()).then_some(hop)
}

pub fn disk_speed() -> Result<Value, String> {
    use std::io::{Read, Write};
    use std::os::windows::fs::OpenOptionsExt;
    const SIZE: usize = 256 * 1024 * 1024;
    const BLOCK: usize = 4 * 1024 * 1024;
    const NO_BUFFERING: u32 = 0x2000_0000;
    const WRITE_THROUGH: u32 = 0x8000_0000;
    let path = std::env::temp_dir().join(format!("nova-vitesse-{}.tmp", std::process::id()));
    let mut raw = vec![0u8; BLOCK + 4096];
    let offset = raw.as_ptr().align_offset(4096);
    let block = &mut raw[offset..offset + BLOCK];
    for (index, byte) in block.iter_mut().enumerate() {
        *byte = (index % 251) as u8;
    }
    let mut run = || -> std::io::Result<(f64, f64)> {
        let started = std::time::Instant::now();
        {
            let mut file = std::fs::OpenOptions::new().write(true).create(true).truncate(true).custom_flags(NO_BUFFERING | WRITE_THROUGH).open(&path)?;
            for _ in 0..SIZE / BLOCK {
                file.write_all(block)?;
            }
        }
        let write = started.elapsed().as_secs_f64();
        let started = std::time::Instant::now();
        {
            let mut file = std::fs::OpenOptions::new().read(true).custom_flags(NO_BUFFERING).open(&path)?;
            for _ in 0..SIZE / BLOCK {
                file.read_exact(block)?;
            }
        }
        Ok((write, started.elapsed().as_secs_f64()))
    };
    let result = run();
    let _ = std::fs::remove_file(&path);
    let (write, read) = result.map_err(|error| format!("Test impossible : {error}"))?;
    let mbs = |seconds: f64| (SIZE as f64 / 1_048_576.0 / seconds.max(1e-6)).round();
    let drive = path.components().next().map(|c| c.as_os_str().to_string_lossy().to_string()).unwrap_or_default();
    Ok(json!({"drive": drive, "writeMBs": mbs(write), "readMBs": mbs(read), "sizeMB": SIZE / 1_048_576}))
}

pub fn network_test() -> Result<Value, String> {
    let mut probes = Vec::new();
    if let Some(hop) = gateway() {
        probes.push(ping(hop, "Box ou routeur"));
    }
    probes.push(ping(std::net::Ipv4Addr::new(1, 1, 1, 1), "Internet"));
    let watch = Instant::now();
    let mut records = std::ptr::null_mut();
    let status = unsafe {
        DnsQuery_W(
            &HSTRING::from("example.com"),
            DNS_TYPE_A,
            DNS_QUERY_BYPASS_CACHE | DNS_QUERY_NO_HOSTS_FILE,
            None,
            &mut records,
            None,
        )
    };
    let ok = status.0 == 0 && !records.is_null();
    let ms = watch.elapsed().as_millis() as u64;
    if !records.is_null() {
        unsafe { DnsFree(Some(records.cast()), DnsFreeRecordList) };
    }
    Ok(json!({"probes": probes, "dnsOk": ok, "dnsMs": ms}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_disk_speed_test_cleans_up_after_itself() {
        let data = disk_speed().unwrap();
        assert!(data["writeMBs"].as_f64().unwrap() > 0.0 && data["readMBs"].as_f64().unwrap() > 0.0);
        let leftover = std::fs::read_dir(std::env::temp_dir()).unwrap().flatten().any(|e| e.file_name().to_string_lossy() == format!("nova-vitesse-{}.tmp", std::process::id()));
        assert!(!leftover);
        println!("{data}");
    }

    #[test]
    fn real_adapters_are_listed_with_addresses() {
        let data = adapters().unwrap();
        let items = data["items"].as_array().unwrap();
        assert!(!items.is_empty());
        assert!(items.iter().any(|a| !a["addresses"].as_array().unwrap().is_empty()));
    }

    #[test]
    fn link_speeds_read_in_french() {
        assert_eq!(link_speed(2.5e9), "2,5 Gb/s");
        assert_eq!(link_speed(100e6), "100 Mb/s");
        assert_eq!(link_speed(0.0), "");
    }

    #[test]
    fn windows_processes_stay_closed() {
        assert!(!may_close("explorer"));
        assert!(!may_close("SecurityHealthSystray"));
        assert!(!may_close("nova-native"));
        assert!(may_close("Discord"));
    }

    #[test]
    fn nvidia_line_is_read_field_by_field() {
        let gpu = parse_nvidia("NVIDIA GeForce RTX 5070, 45, 7, 1024, 12288, [N/A], 581.57").unwrap();
        assert_eq!(gpu["memoryTotalMiB"], 12288.0);
        assert!(gpu["powerW"].is_null());
        assert_eq!(gpu["driver"], "581.57");
        assert!(parse_nvidia("erreur").is_none());
    }

    #[test]
    fn defrag_messages_give_the_letter() {
        assert_eq!(letter_in_parentheses("Windows (c:)"), Some('C'));
        assert_eq!(letter_in_parentheses("retrim"), None);
    }
}
