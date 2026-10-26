use std::{ffi::c_void, os::windows::ffi::OsStrExt};
#[link(name = "kernel32")]
extern "system" {
    fn CreateMutexW(attributes: *const c_void, owner: i32, name: *const u16) -> *mut c_void;
    fn CreateEventW(
        attributes: *const c_void,
        manual: i32,
        initial: i32,
        name: *const u16,
    ) -> *mut c_void;
    fn OpenEventW(access: u32, inherit: i32, name: *const u16) -> *mut c_void;
    fn SetEvent(event: *mut c_void) -> i32;
    fn WaitForSingleObject(handle: *mut c_void, millis: u32) -> u32;
    fn CloseHandle(handle: *mut c_void) -> i32;
}
pub struct Instance(*mut c_void);
impl Drop for Instance {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}
impl Instance {
    pub fn acquire(replacement: bool) -> Result<Option<Self>, String> {
        Self::acquire_named("Local\\NovaPCSuite.SingleInstance", replacement)
    }
    fn acquire_named(name: &str, replacement: bool) -> Result<Option<Self>, String> {
        let wide: Vec<u16> = std::ffi::OsStr::new(name)
            .encode_wide()
            .chain(Some(0))
            .collect();
        let handle = unsafe { CreateMutexW(std::ptr::null(), 0, wide.as_ptr()) };
        if handle.is_null() {
            return Err("Une instance de Nova est déjà ouverte ou inaccessible".into());
        }
        let exists = std::io::Error::last_os_error().raw_os_error() == Some(183);
        let instance = Self(handle);
        if exists && !replacement {
            return Ok(None);
        }
        Ok(Some(instance))
    }
}
#[link(name = "shell32")]
extern "system" {
    fn IsUserAnAdmin() -> i32;
}
pub fn is_admin() -> bool {
    unsafe { IsUserAnAdmin() != 0 }
}
fn valid_token(token: &str) -> bool {
    token.len() == 32 && token.bytes().all(|b| b.is_ascii_hexdigit())
}
fn event_name(token: &str) -> Result<Vec<u16>, String> {
    if !valid_token(token) {
        return Err("Transition administrateur invalide".into());
    }
    Ok(
        std::ffi::OsStr::new(&format!("Local\\NovaPCSuite.Elevation.{token}"))
            .encode_wide()
            .chain(Some(0))
            .collect(),
    )
}
pub fn valid_page(page: &str) -> bool {
    matches!(
        page,
        "overview"
            | "about-pc"
            | "drivers"
            | "optimize"
            | "measurements"
            | "network"
            | "memory"
            | "cleanup"
            | "applications"
            | "storage"
            | "startup"
            | "processes"
            | "security"
            | "assistant"
            | "automation"
            | "driver-sources"
            | "history"
    )
}
pub fn requested_token() -> Result<Option<String>, String> {
    let args: Vec<_> = std::env::args().collect();
    if let Some(i) = args.iter().position(|arg| arg == "--elevated-handoff") {
        let token = args
            .get(i + 1)
            .filter(|t| valid_token(t))
            .ok_or("Transition invalide")?;
        return Ok(Some(token.clone()));
    }
    Ok(None)
}
pub fn initial_page() -> String {
    let args: Vec<_> = std::env::args().collect();
    args.iter()
        .position(|arg| arg == "--start-page")
        .and_then(|i| args.get(i + 1))
        .filter(|page| valid_page(page))
        .cloned()
        .unwrap_or_else(|| "overview".into())
}
pub struct Handoff {
    handle: *mut c_void,
    pub token: String,
}
impl Handoff {
    pub fn new() -> Result<Self, String> {
        let token = super::cleanup::uid()?;
        let name = event_name(&token)?;
        let handle = unsafe { CreateEventW(std::ptr::null(), 1, 0, name.as_ptr()) };
        if handle.is_null() {
            return Err("Préparation de la transition impossible".into());
        }
        if std::io::Error::last_os_error().raw_os_error() == Some(183) {
            unsafe {
                CloseHandle(handle);
            };
            return Err("Transition déjà utilisée".into());
        }
        Ok(Self { handle, token })
    }
    pub fn wait(&self, millis: u32) -> bool {
        unsafe { WaitForSingleObject(self.handle, millis) == 0 }
    }
}
impl Drop for Handoff {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.handle);
        }
    }
}
fn signal(token: &str) -> Result<(), String> {
    let name = event_name(token)?;
    let handle = unsafe { OpenEventW(2, 0, name.as_ptr()) };
    if handle.is_null() {
        return Err("La fenêtre précédente ne peut pas être remplacée".into());
    }
    let ok = unsafe { SetEvent(handle) != 0 };
    unsafe {
        CloseHandle(handle);
    }
    if ok {
        Ok(())
    } else {
        Err("Accusé de réception impossible".into())
    }
}
pub fn complete(token: &str) -> Result<(), String> {
    if !is_admin() {
        return Err("Les droits administrateur n'ont pas été accordés".into());
    }
    signal(token)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn singleton_allows_only_the_authenticated_replacement_to_overlap() {
        let name = format!(
            "Local\\NovaPCSuite.Test.{}",
            super::super::cleanup::uid().unwrap()
        );
        let first = Instance::acquire_named(&name, false).unwrap().unwrap();
        assert!(Instance::acquire_named(&name, false).unwrap().is_none());
        let replacement = Instance::acquire_named(&name, true).unwrap().unwrap();
        drop(first);
        assert!(Instance::acquire_named(&name, false).unwrap().is_none());
        drop(replacement);
        assert!(Instance::acquire_named(&name, false).unwrap().is_some());
    }
    #[test]
    fn handoff_is_scoped_to_a_fresh_token_and_requires_readiness() {
        let pending = Handoff::new().unwrap();
        assert!(!pending.wait(0));
        signal(&pending.token).unwrap();
        assert!(pending.wait(0));
        assert!(event_name("../bad").is_err());
        assert!(signal("00000000000000000000000000000000").is_err());
        assert!(!valid_page("cmd.exe"));
        assert!(valid_page("about-pc"));
    }
}
