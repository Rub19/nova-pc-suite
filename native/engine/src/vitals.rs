use serde_json::{json, Value};
use std::{ffi::c_void, os::windows::ffi::OsStrExt, sync::Mutex, time::Instant};

#[repr(C)]
struct FormattedValue {
    status: u32,
    value: f64,
}
#[repr(C)]
#[derive(Default)]
struct MemoryStatus {
    length: u32,
    load: u32,
    total_phys: u64,
    avail_phys: u64,
    total_page: u64,
    avail_page: u64,
    total_virtual: u64,
    avail_virtual: u64,
    avail_extended: u64,
}
#[link(name = "pdh")]
extern "system" {
    fn PdhOpenQueryW(source: *const u16, user: usize, query: *mut *mut c_void) -> u32;
    fn PdhAddEnglishCounterW(query: *mut c_void, path: *const u16, user: usize, counter: *mut *mut c_void) -> u32;
    fn PdhCollectQueryData(query: *mut c_void) -> u32;
    fn PdhGetFormattedCounterValue(counter: *mut c_void, format: u32, kind: *mut u32, value: *mut FormattedValue) -> u32;
}
#[link(name = "kernel32")]
extern "system" {
    fn GlobalMemoryStatusEx(status: *mut MemoryStatus) -> i32;
}

const PDH_FMT_DOUBLE: u32 = 0x200;
const PDH_FMT_NOCAP100: u32 = 0x8000;

struct Counters {
    query: usize,
    cpu: usize,
    disk_idle: usize,
}
static COUNTERS: Mutex<Option<Counters>> = Mutex::new(None);

fn wide(text: &str) -> Vec<u16> {
    std::ffi::OsStr::new(text).encode_wide().chain(Some(0)).collect()
}

fn open() -> Option<Counters> {
    let mut query = std::ptr::null_mut();
    if unsafe { PdhOpenQueryW(std::ptr::null(), 0, &mut query) } != 0 {
        return None;
    }
    let add = |path: &str| {
        let mut counter = std::ptr::null_mut();
        (unsafe { PdhAddEnglishCounterW(query, wide(path).as_ptr(), 0, &mut counter) } == 0).then_some(counter as usize)
    };
    let counters = Counters {
        query: query as usize,
        cpu: add("\\Processor Information(_Total)\\% Processor Utility")?,
        disk_idle: add("\\PhysicalDisk(_Total)\\% Idle Time")?,
    };
    unsafe { PdhCollectQueryData(query) };
    Some(counters)
}

fn read(counter: usize) -> Option<f64> {
    let mut value = FormattedValue { status: 0, value: 0.0 };
    let mut kind = 0;
    let status = unsafe {
        PdhGetFormattedCounterValue(counter as *mut c_void, PDH_FMT_DOUBLE | PDH_FMT_NOCAP100, &mut kind, &mut value)
    };
    (status == 0).then_some(value.value.clamp(0.0, 100.0))
}

pub fn sample() -> Value {
    let mut guard = COUNTERS.lock().unwrap_or_else(|e| e.into_inner());
    if guard.is_none() {
        *guard = open();
    }
    let (cpu, disk) = match guard.as_ref() {
        Some(c) => {
            unsafe { PdhCollectQueryData(c.query as *mut c_void) };
            (read(c.cpu), read(c.disk_idle).map(|idle| 100.0 - idle))
        }
        None => (None, None),
    };
    let mut memory = MemoryStatus {
        length: std::mem::size_of::<MemoryStatus>() as u32,
        ..Default::default()
    };
    unsafe { GlobalMemoryStatusEx(&mut memory) };
    json!({
        "cpu": cpu,
        "disk": disk,
        "memoryUsedBytes": memory.total_phys.saturating_sub(memory.avail_phys),
        "memoryTotalBytes": memory.total_phys,
    })
}

pub fn cpu_bench() -> f64 {
    let start = Instant::now();
    let mut state: u64 = 0x9E3779B97F4A7C15;
    let mut rounds: u64 = 0;
    while start.elapsed().as_millis() < 700 {
        for _ in 0..50_000 {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
        }
        rounds += 1;
    }
    std::hint::black_box(state);
    rounds as f64 * 50_000.0 / start.elapsed().as_secs_f64() / 1_000_000.0
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sample_reports_real_memory_and_bounded_percentages() {
        let first = sample();
        std::thread::sleep(std::time::Duration::from_millis(300));
        let second = sample();
        assert!(first["memoryTotalBytes"].as_u64().unwrap() > 0);
        assert!(second["memoryUsedBytes"].as_u64().unwrap() <= second["memoryTotalBytes"].as_u64().unwrap());
        let cpu = second["cpu"].as_f64().unwrap();
        assert!((0.0..=100.0).contains(&cpu));
    }
    #[test]
    fn bench_returns_a_positive_rate() {
        assert!(cpu_bench() > 0.0);
    }
}
