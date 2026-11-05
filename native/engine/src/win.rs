use serde_json::Value;
use std::{collections::HashMap, os::windows::ffi::OsStrExt};
use windows::{
    core::{BOOL, HSTRING, PCWSTR},
    Win32::{
        Foundation::{CloseHandle, HWND, LPARAM},
        System::{
            Com::{CoInitializeEx, COINIT_MULTITHREADED},
            EventLog::{
                EvtClose, EvtNext, EvtQuery, EvtQueryChannelPath, EvtQueryReverseDirection,
                EvtRender, EvtRenderEventXml, EVT_HANDLE,
            },
            Threading::{
                OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
                PROCESS_QUERY_LIMITED_INFORMATION,
            },
        },
        UI::{
            Shell::ShellExecuteW,
            WindowsAndMessaging::{
                EnumWindows, GetWindow, GetWindowThreadProcessId, IsWindowVisible, GW_OWNER,
                SW_SHOWNORMAL,
            },
        },
    },
};
pub use wmi::Variant;
use wmi::WMIConnection;

pub type Row = HashMap<String, Variant>;

pub fn com() {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
}

pub fn wide(text: &str) -> Vec<u16> {
    std::ffi::OsStr::new(text)
        .encode_wide()
        .chain(Some(0))
        .collect()
}

pub fn from_wide(buffer: &[u16]) -> String {
    let end = buffer.iter().position(|c| *c == 0).unwrap_or(buffer.len());
    String::from_utf16_lossy(&buffer[..end])
}

pub fn sys32(name: &str) -> String {
    let root = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".into());
    format!("{root}\\System32\\{name}")
}

pub fn shell_open(target: &str) -> Result<(), String> {
    let done = unsafe {
        ShellExecuteW(
            Some(HWND::default()),
            &HSTRING::from("open"),
            &HSTRING::from(target),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    if done.0 as isize > 32 {
        Ok(())
    } else {
        Err("Windows n'a pas pu ouvrir cet élément.".into())
    }
}

pub fn wmi_at(namespace: &str) -> Option<WMIConnection> {
    com();
    WMIConnection::with_namespace_path(namespace).ok()
}

pub fn query(connection: &Option<WMIConnection>, wql: &str) -> Vec<Row> {
    connection
        .as_ref()
        .and_then(|c| c.raw_query::<Row>(wql).ok())
        .unwrap_or_default()
}

pub fn text(row: &Row, key: &str) -> String {
    match row.get(key) {
        Some(Variant::String(value)) => value.clone(),
        _ => String::new(),
    }
}

pub fn number(row: &Row, key: &str) -> Option<f64> {
    match row.get(key)? {
        Variant::String(v) => v.trim().parse().ok(),
        Variant::I1(v) => Some(*v as f64),
        Variant::I2(v) => Some(*v as f64),
        Variant::I4(v) => Some(*v as f64),
        Variant::I8(v) => Some(*v as f64),
        Variant::R4(v) => Some(*v as f64),
        Variant::R8(v) => Some(*v),
        Variant::UI1(v) => Some(*v as f64),
        Variant::UI2(v) => Some(*v as f64),
        Variant::UI4(v) => Some(*v as f64),
        Variant::UI8(v) => Some(*v as f64),
        Variant::Bool(v) => Some(*v as u8 as f64),
        _ => None,
    }
}

pub fn flag(row: &Row, key: &str) -> bool {
    matches!(row.get(key), Some(Variant::Bool(true)))
}

pub fn wmi_date(raw: &str) -> Value {
    if raw.len() >= 8 && raw[..8].bytes().all(|b| b.is_ascii_digit()) {
        Value::String(format!("{}-{}-{}", &raw[..4], &raw[4..6], &raw[6..8]))
    } else {
        Value::Null
    }
}

pub fn iso_from_ole(days: f64) -> Option<String> {
    if days <= 0.0 {
        return None;
    }
    let epoch = chrono::DateTime::from_timestamp(-2_209_161_600, 0)?;
    let moment = epoch + chrono::Duration::milliseconds((days * 86_400_000.0).round() as i64);
    Some(moment.to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
}

pub fn ymd_from_ole(days: f64) -> Value {
    iso_from_ole(days)
        .and_then(|iso| iso.get(..10).map(str::to_string))
        .map(Value::String)
        .unwrap_or(Value::Null)
}

pub struct EventXml(pub String);

impl EventXml {
    pub fn time(&self) -> String {
        let source = &self.0;
        source
            .find("SystemTime=")
            .and_then(|i| {
                let rest = &source[i + 12..];
                rest.find(['\'', '"']).map(|end| rest[..end].to_string())
            })
            .and_then(|raw| chrono::DateTime::parse_from_rfc3339(&raw).ok())
            .map(|d| {
                d.with_timezone(&chrono::Utc)
                    .to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
            })
            .unwrap_or_default()
    }

    pub fn data(&self) -> Vec<(String, String)> {
        let source = match self.0.find("<EventData") {
            Some(i) => &self.0[i..],
            None => return Vec::new(),
        };
        let mut found = Vec::new();
        let mut rest = source;
        while let Some(start) = rest.find("<Data") {
            rest = &rest[start + 5..];
            let Some(close) = rest.find('>') else { break };
            let head = &rest[..close];
            let name = head
                .find("Name=")
                .and_then(|i| {
                    let quoted = &head[i + 6..];
                    quoted.find(['\'', '"']).map(|end| quoted[..end].to_string())
                })
                .unwrap_or_default();
            if head.ends_with('/') {
                found.push((name, String::new()));
                rest = &rest[close + 1..];
                continue;
            }
            rest = &rest[close + 1..];
            let Some(end) = rest.find("</Data>") else { break };
            found.push((name, unescape(&rest[..end])));
            rest = &rest[end + 7..];
        }
        found
    }

    pub fn named(&self, key: &str) -> Option<String> {
        self.data().into_iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }
}

fn unescape(text: &str) -> String {
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

pub fn events(channel: &str, xpath: &str, max: usize) -> Result<Vec<EventXml>, u32> {
    let flags = EvtQueryChannelPath.0 as u32 | EvtQueryReverseDirection.0 as u32;
    let results = unsafe {
        EvtQuery(None, &HSTRING::from(channel), &HSTRING::from(xpath), flags)
    }
    .map_err(|e| (e.code().0 as u32) & 0xFFFF)?;
    let mut found = Vec::new();
    loop {
        let mut handles = [0isize; 64];
        let mut returned = 0u32;
        let next = unsafe {
            EvtNext(results, &mut handles, 2000, 0, &mut returned)
        };
        if let Err(error) = next {
            let code = (error.code().0 as u32) & 0xFFFF;
            unsafe {
                let _ = EvtClose(results);
            }
            if code == 259 || code == 15007 {
                return Ok(found);
            }
            return Err(code);
        }
        for handle in &handles[..returned as usize] {
            let event = EVT_HANDLE(*handle);
            if found.len() < max {
                if let Some(xml) = render(event) {
                    found.push(EventXml(xml));
                }
            }
            unsafe {
                let _ = EvtClose(event);
            }
        }
        if found.len() >= max {
            unsafe {
                let _ = EvtClose(results);
            }
            return Ok(found);
        }
    }
}

fn render(event: EVT_HANDLE) -> Option<String> {
    let mut used = 0u32;
    let mut properties = 0u32;
    let _ = unsafe {
        EvtRender(None, event, EvtRenderEventXml.0 as u32, 0, None, &mut used, &mut properties)
    };
    if used == 0 {
        return None;
    }
    let mut buffer = vec![0u16; used as usize / 2 + 1];
    unsafe {
        EvtRender(
            None,
            event,
            EvtRenderEventXml.0 as u32,
            used,
            Some(buffer.as_mut_ptr().cast()),
            &mut used,
            &mut properties,
        )
    }
    .ok()?;
    Some(from_wide(&buffer))
}

#[link(name = "ntdll")]
extern "system" {
    fn NtQuerySystemInformation(class: u32, info: *mut u8, length: u32, returned: *mut u32) -> i32;
}

#[derive(Clone)]
pub struct Process {
    pub pid: u32,
    pub name: String,
    pub cpu_ticks: u64,
    pub working_set: u64,
}

fn read_u64(bytes: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(bytes[at..at + 8].try_into().unwrap_or([0; 8]))
}

pub fn processes() -> Vec<Process> {
    let mut buffer = vec![0u8; 1 << 20];
    loop {
        let mut needed = 0u32;
        let status = unsafe {
            NtQuerySystemInformation(5, buffer.as_mut_ptr(), buffer.len() as u32, &mut needed)
        };
        if status == 0 {
            break;
        }
        if status as u32 != 0xC0000004 || buffer.len() > 1 << 28 {
            return Vec::new();
        }
        buffer.resize(buffer.len().max(needed as usize) * 2, 0);
    }
    let mut list = Vec::new();
    let mut offset = 0usize;
    loop {
        let record = &buffer[offset..];
        if record.len() < 152 {
            break;
        }
        let next = u32::from_le_bytes(record[0..4].try_into().unwrap_or([0; 4])) as usize;
        let length = u16::from_le_bytes(record[56..58].try_into().unwrap_or([0; 2])) as usize;
        let pointer = read_u64(record, 64) as *const u16;
        let name = if pointer.is_null() || length == 0 {
            String::new()
        } else {
            String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(pointer, length / 2) })
        };
        list.push(Process {
            pid: read_u64(record, 80) as u32,
            name,
            cpu_ticks: read_u64(record, 40) + read_u64(record, 48),
            working_set: read_u64(record, 144),
        });
        if next == 0 {
            break;
        }
        offset += next;
    }
    list
}

pub fn process_path(pid: u32) -> Option<String> {
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buffer = [0u16; 1024];
        let mut size = buffer.len() as u32;
        let done = QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_WIN32,
            windows::core::PWSTR(buffer.as_mut_ptr()),
            &mut size,
        );
        let _ = CloseHandle(handle);
        done.ok()?;
        Some(String::from_utf16_lossy(&buffer[..size as usize]))
    }
}

unsafe extern "system" fn collect_window(window: HWND, list: LPARAM) -> BOOL {
    let pids = &mut *(list.0 as *mut Vec<u32>);
    let owned = GetWindow(window, GW_OWNER).map(|o| !o.is_invalid()).unwrap_or(false);
    if IsWindowVisible(window).as_bool() && !owned {
        let mut pid = 0u32;
        GetWindowThreadProcessId(window, Some(&mut pid));
        pids.push(pid);
    }
    BOOL(1)
}

pub fn pids_with_window() -> Vec<u32> {
    let mut pids: Vec<u32> = Vec::new();
    unsafe {
        let _ = EnumWindows(Some(collect_window), LPARAM(&mut pids as *mut _ as isize));
    }
    pids
}
