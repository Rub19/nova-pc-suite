use std::io::{Read, Write};
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_SET_VALUE};
use winreg::RegKey;

pub const VERSION: &str = env!("NOVA_VERSION");
const MAGIC: &[u8; 8] = b"NOVAPCK1";
const TRAILER: usize = 24;
const NO_WINDOW: u32 = 0x0800_0000;
const DETACHED: u32 = 0x0000_0008;
const EXE: &str = "nova-pc-suite.exe";
const PRODUCT: &str = "Nova PC Suite";
const TASK_SCRIPT: &str = r#"$ErrorActionPreference='Stop';try{$sid=[Security.Principal.WindowsIdentity]::GetCurrent().User.Value;$name='Nova PC Suite Audit '+$sid;$task=Get-ScheduledTask -TaskName $name -ErrorAction SilentlyContinue;if($task -and $task.Description -eq 'Nova PC Suite read-only audit v1'){$actions=@($task.Actions);if($actions.Count -eq 1 -and $actions[0].Arguments -eq '--scheduled-scan'){Unregister-ScheduledTask -TaskName $name -Confirm:$false}}}catch{exit 1}"#;

#[derive(Clone)]
pub struct Target {
    pub dir: PathBuf,
    pub local: PathBuf,
    pub programs: PathBuf,
    pub desktop: Option<PathBuf>,
    pub windows: String,
    pub soft: String,
    pub test: bool,
}

impl Target {
    pub fn current() -> Self {
        let test_root = std::env::var_os("NOVA_SETUP_TEST").map(PathBuf::from);
        let test = test_root.is_some();
        let (local, programs, desktop, windows, soft) = match &test_root {
            Some(root) => (root.join("LocalAppData"), root.join("StartMenu"), Some(root.join("Desktop")), r"Software\NovaSetupTest\Windows".to_string(), r"Software\NovaSetupTest".to_string()),
            None => (env_dir("LOCALAPPDATA"), env_dir("APPDATA").join(r"Microsoft\Windows\Start Menu\Programs"), desktop_dir(), r"Software\Microsoft\Windows\CurrentVersion".to_string(), "Software".to_string()),
        };
        let mut target = Self { dir: PathBuf::new(), local, programs, desktop, windows, soft, test };
        target.dir = target.location(PRODUCT).unwrap_or_else(|| target.local.join(PRODUCT));
        target
    }

    pub fn exe(&self) -> PathBuf {
        self.dir.join(EXE)
    }

    fn uninstall_key(&self, product: &str) -> String {
        format!(r"{}\Uninstall\{product}", self.windows)
    }

    fn run_key(&self) -> String {
        format!(r"{}\Run", self.windows)
    }

    fn location(&self, product: &str) -> Option<PathBuf> {
        read_value(&self.uninstall_key(product), "InstallLocation").map(|text| PathBuf::from(text.trim().trim_matches('"'))).filter(|path| path.is_absolute())
    }

    pub fn installed_version(&self) -> Option<String> {
        read_value(&self.uninstall_key(PRODUCT), "DisplayVersion").or_else(|| self.exe().exists().then(String::new))
    }

    pub fn desktop_link(&self) -> bool {
        self.desktop.as_ref().is_some_and(|desktop| desktop.join(format!("{PRODUCT}.lnk")).exists())
    }

    pub fn autostart(&self) -> bool {
        read_value(&self.run_key(), PRODUCT).is_some()
    }
}

fn read_value(key: &str, name: &str) -> Option<String> {
    RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(key, KEY_READ).and_then(|key| key.get_value(name)).ok()
}

fn env_dir(name: &str) -> PathBuf {
    std::env::var_os(name).map(PathBuf::from).unwrap_or_else(std::env::temp_dir)
}

fn desktop_dir() -> Option<PathBuf> {
    unsafe {
        let path = windows::Win32::UI::Shell::SHGetKnownFolderPath(&windows::Win32::UI::Shell::FOLDERID_Desktop, windows::Win32::UI::Shell::KNOWN_FOLDER_FLAG(0), None).ok()?;
        let text = path.to_string().ok();
        windows::Win32::System::Com::CoTaskMemFree(Some(path.0 as _));
        text.map(PathBuf::from)
    }
}

pub fn stop_app(target: &Target) {
    if target.test {
        return;
    }
    let _ = Command::new("taskkill").args(["/IM", EXE]).creation_flags(NO_WINDOW).output();
    std::thread::sleep(std::time::Duration::from_millis(1200));
    let _ = Command::new("taskkill").args(["/IM", EXE, "/F"]).creation_flags(NO_WINDOW).output();
    std::thread::sleep(std::time::Duration::from_millis(500));
}

pub struct Payload {
    pub data: Vec<u8>,
    pub size: u64,
    pub program_len: usize,
}

pub fn split(bytes: &[u8]) -> (usize, Option<(usize, u64)>) {
    if bytes.len() < TRAILER || &bytes[bytes.len() - 8..] != MAGIC {
        return (bytes.len(), None);
    }
    let field = |at: usize| u64::from_le_bytes(bytes[at..at + 8].try_into().unwrap_or([0; 8]));
    let size = field(bytes.len() - TRAILER);
    let packed = field(bytes.len() - 16) as usize;
    let Some(program_len) = bytes.len().checked_sub(TRAILER + packed) else { return (bytes.len(), None) };
    (program_len, Some((packed, size)))
}

pub fn own_payload() -> Result<Payload, String> {
    let bytes = std::fs::read(std::env::current_exe().map_err(|error| error.to_string())?).map_err(|error| error.to_string())?;
    let (program_len, found) = split(&bytes);
    let Some((packed, size)) = found else { return Err("Cet installeur ne contient pas l'application.".into()) };
    Ok(Payload { data: bytes[program_len..program_len + packed].to_vec(), size, program_len })
}

pub fn compress(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let mut encoder = xz2::write::XzEncoder::new(Vec::new(), 9 | 0x8000_0000);
    encoder.write_all(bytes).map_err(|error| error.to_string())?;
    encoder.finish().map_err(|error| error.to_string())
}

pub fn pack(app: &Path, out: &Path) -> Result<(), String> {
    let own = std::fs::read(std::env::current_exe().map_err(|error| error.to_string())?).map_err(|error| error.to_string())?;
    let (program_len, _) = split(&own);
    let app_bytes = std::fs::read(app).map_err(|error| format!("{}: {error}", app.display()))?;
    let packed = compress(&app_bytes).map_err(|error| error.to_string())?;
    let mut file = Vec::with_capacity(program_len + packed.len() + TRAILER);
    file.extend_from_slice(&own[..program_len]);
    file.extend_from_slice(&packed);
    file.extend_from_slice(&(app_bytes.len() as u64).to_le_bytes());
    file.extend_from_slice(&(packed.len() as u64).to_le_bytes());
    file.extend_from_slice(MAGIC);
    std::fs::write(out, file).map_err(|error| error.to_string())
}

fn write_payload(payload: &Payload, target: &Path, progress: &dyn Fn(f32)) -> Result<(), String> {
    let total = payload.size as f64;
    if total == 0.0 {
        return Err("Cet installeur ne contient pas l'application.".into());
    }
    let temp = target.with_extension("exe.new");
    let mut decoder = xz2::read::XzDecoder::new(payload.data.as_slice());
    let mut file = std::fs::File::create(&temp).map_err(|error| error.to_string())?;
    let mut buffer = vec![0u8; 256 * 1024];
    let mut written = 0usize;
    loop {
        let count = decoder.read(&mut buffer).map_err(|error| error.to_string())?;
        if count == 0 {
            break;
        }
        file.write_all(&buffer[..count]).map_err(|error| error.to_string())?;
        written += count;
        progress((written as f64 / total) as f32);
    }
    file.sync_all().map_err(|error| error.to_string())?;
    drop(file);
    if written as f64 != total {
        let _ = std::fs::remove_file(&temp);
        return Err("L'application embarquée est incomplète.".into());
    }
    let mut last = String::new();
    for _ in 0..20 {
        match std::fs::rename(&temp, target) {
            Ok(()) => return Ok(()),
            Err(error) => last = error.to_string(),
        }
        std::thread::sleep(std::time::Duration::from_millis(250));
    }
    let _ = std::fs::remove_file(&temp);
    Err(format!("Nova est encore ouvert ou verrouillé : {last}"))
}

fn shortcut(link: &Path, exe: &Path) -> Result<(), String> {
    use windows::core::{Interface, HSTRING};
    use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, IPersistFile, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED};
    use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};
    if let Some(parent) = link.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let link_object: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).map_err(|error| error.to_string())?;
        link_object.SetPath(&HSTRING::from(exe.as_os_str())).map_err(|error| error.to_string())?;
        if let Some(parent) = exe.parent() {
            link_object.SetWorkingDirectory(&HSTRING::from(parent.as_os_str())).map_err(|error| error.to_string())?;
        }
        link_object.SetIconLocation(&HSTRING::from(exe.as_os_str()), 0).map_err(|error| error.to_string())?;
        link_object.SetDescription(&HSTRING::from(PRODUCT)).map_err(|error| error.to_string())?;
        let file: IPersistFile = link_object.cast().map_err(|error| error.to_string())?;
        file.Save(&HSTRING::from(link.as_os_str()), true).map_err(|error| error.to_string())
    }
}

pub struct Choices {
    pub desktop: bool,
    pub autostart: Option<bool>,
}

pub fn install(target: &Target, choices: &Choices, step: &dyn Fn(&str, f32)) -> Result<(), String> {
    step("stop", 0.02);
    stop_app(target);
    std::fs::create_dir_all(&target.dir).map_err(|error| error.to_string())?;
    let payload = own_payload()?;
    step("copy", 0.05);
    write_payload(&payload, &target.exe(), &|part| step("copy", 0.05 + part * 0.75))?;
    step("register", 0.82);
    let own = std::env::current_exe().map_err(|error| error.to_string())?;
    let uninstaller = target.dir.join("uninstall.exe");
    if own != uninstaller {
        let program = std::fs::read(&own).map_err(|error| error.to_string())?;
        let _ = std::fs::remove_file(&uninstaller);
        std::fs::write(&uninstaller, &program[..payload.program_len]).map_err(|error| error.to_string())?;
    }
    let root = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = root.create_subkey(target.uninstall_key(PRODUCT)).map_err(|error| error.to_string())?;
    let exe = target.exe().display().to_string();
    let size_kb = (payload.size / 1024) as u32;
    for (name, value) in [
        ("DisplayName", PRODUCT.to_string()),
        ("DisplayVersion", VERSION.to_string()),
        ("Publisher", "Rub19".to_string()),
        ("MainBinaryName", EXE.to_string()),
        ("DisplayIcon", format!("\"{exe}\"")),
        ("InstallLocation", target.dir.display().to_string()),
        ("UninstallString", format!("\"{}\"", uninstaller.display())),
        ("QuietUninstallString", format!("\"{}\" /S", uninstaller.display())),
    ] {
        key.set_value(name, &value).map_err(|error| error.to_string())?;
    }
    key.set_value("NoModify", &1u32).map_err(|error| error.to_string())?;
    key.set_value("NoRepair", &1u32).map_err(|error| error.to_string())?;
    key.set_value("EstimatedSize", &size_kb).map_err(|error| error.to_string())?;
    step("shortcut", 0.88);
    shortcut(&target.programs.join(format!("{PRODUCT}.lnk")), &target.exe())?;
    if let Some(desktop) = &target.desktop {
        let link = desktop.join(format!("{PRODUCT}.lnk"));
        if choices.desktop {
            shortcut(&link, &target.exe())?;
        } else {
            let _ = std::fs::remove_file(&link);
        }
    }
    if let Some(enabled) = choices.autostart {
        let (run, _) = root.create_subkey(target.run_key()).map_err(|error| error.to_string())?;
        if enabled {
            run.set_value(PRODUCT, &format!("\"{exe}\"")).map_err(|error| error.to_string())?;
        } else {
            let _ = run.delete_value(PRODUCT);
        }
    }
    step("legacy", 0.94);
    remove_legacy(target);
    step("done", 1.0);
    Ok(())
}

pub fn remove_legacy(target: &Target) {
    let folder = target.programs.join(PRODUCT);
    let _ = std::fs::remove_file(folder.join(format!("{PRODUCT}.lnk")));
    let _ = std::fs::remove_dir(&folder);
    let _ = std::fs::remove_file(target.dir.join("resources").join("icon.ico"));
    let _ = std::fs::remove_dir(target.dir.join("resources"));
    let _ = RegKey::predef(HKEY_CURRENT_USER).delete_subkey_all(format!(r"{}\github\{PRODUCT}", target.soft));
}

fn remove_task(target: &Target) {
    if target.test {
        return;
    }
    let _ = Command::new("powershell").args(["-NoProfile", "-NonInteractive", "-Command", TASK_SCRIPT]).creation_flags(NO_WINDOW).output();
}

pub fn uninstall(target: &Target, step: &dyn Fn(&str, f32)) -> Result<(), String> {
    step("stop", 0.1);
    stop_app(target);
    step("remove", 0.4);
    let exe = target.exe();
    if exe.exists() {
        std::fs::remove_file(&exe).map_err(|error| format!("Nova est encore ouvert : {error}"))?;
    }
    let _ = std::fs::remove_file(target.programs.join(format!("{PRODUCT}.lnk")));
    if let Some(desktop) = &target.desktop {
        let _ = std::fs::remove_file(desktop.join(format!("{PRODUCT}.lnk")));
    }
    remove_task(target);
    remove_legacy(target);
    step("register", 0.7);
    let root = RegKey::predef(HKEY_CURRENT_USER);
    let _ = root.delete_subkey_all(target.uninstall_key(PRODUCT));
    if let Ok(run) = root.open_subkey_with_flags(target.run_key(), KEY_SET_VALUE) {
        let _ = run.delete_value(PRODUCT);
    }
    step("done", 1.0);
    Ok(())
}

pub fn remove_self_later(target: &Target) {
    let uninstaller = target.dir.join("uninstall.exe");
    let script = format!("ping -n 3 127.0.0.1 >nul & del /f /q \"{}\" & rmdir \"{}\"", uninstaller.display(), target.dir.display());
    let _ = Command::new("cmd").arg("/c").raw_arg(&script).creation_flags(NO_WINDOW | DETACHED).spawn();
}

pub fn launch(target: &Target, args: &[String]) {
    let _ = Command::new(target.exe()).args(args).current_dir(&target.dir).creation_flags(DETACHED).spawn();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packed_files_round_trip() {
        let program = b"MZprogram".to_vec();
        assert_eq!(split(&program), (program.len(), None));
        let app = b"application".repeat(1000);
        let packed = compress(&app).expect("compression");
        let mut file = program.clone();
        file.extend_from_slice(&packed);
        file.extend_from_slice(&(app.len() as u64).to_le_bytes());
        file.extend_from_slice(&(packed.len() as u64).to_le_bytes());
        file.extend_from_slice(MAGIC);
        let (program_len, found) = split(&file);
        assert_eq!(program_len, program.len());
        assert_eq!(found, Some((packed.len(), app.len() as u64)));
        let dir = std::env::temp_dir().join(format!("nova-setup-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("dossier");
        let target = dir.join(EXE);
        let seen = std::cell::Cell::new(0.0f32);
        write_payload(&Payload { data: packed, size: app.len() as u64, program_len }, &target, &|part| seen.set(part)).expect("copie");
        assert_eq!(std::fs::read(&target).expect("lecture"), app);
        assert!((seen.get() - 1.0).abs() < 1e-6);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
