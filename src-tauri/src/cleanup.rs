use serde_json::{json, Value};
use std::{
    env,
    ffi::c_void,
    fs,
    os::windows::{ffi::OsStrExt, fs::MetadataExt},
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

const REPARSE_POINT: u32 = 0x400;

#[repr(C, packed(8))]
struct RecycleInfo {
    size: u32,
    bytes: i64,
    items: i64,
}
#[link(name = "shell32")]
extern "system" {
    fn SHQueryRecycleBinW(root: *const u16, info: *mut RecycleInfo) -> i32;
    fn SHEmptyRecycleBinW(window: *mut c_void, root: *const u16, flags: u32) -> i32;
}
#[link(name = "kernel32")]
extern "system" {
    fn GetDiskFreeSpaceExW(root: *const u16, free: *mut u64, total: *mut u64, all: *mut u64) -> i32;
}

struct Root {
    path: PathBuf,
    prefix: Option<&'static str>,
}

struct Category {
    id: &'static str,
    label: &'static str,
    detail: &'static str,
    admin: bool,
    selected: bool,
    min_age_hours: u64,
    roots: Vec<Root>,
}

fn dir(path: PathBuf) -> Root {
    Root { path, prefix: None }
}

fn subdirs(path: &Path) -> Vec<PathBuf> {
    fs::read_dir(path)
        .map(|entries| {
            entries
                .flatten()
                .filter(|e| {
                    e.metadata()
                        .map(|m| m.is_dir() && m.file_attributes() & REPARSE_POINT == 0)
                        .unwrap_or(false)
                })
                .map(|e| e.path())
                .collect()
        })
        .unwrap_or_default()
}

fn env_path(name: &str) -> Option<PathBuf> {
    env::var_os(name).map(PathBuf::from).filter(|p| p.is_absolute())
}

fn categories() -> Vec<Category> {
    let local = env_path("LOCALAPPDATA");
    let windows = env_path("SystemRoot");
    let program_data = env_path("ProgramData");
    let on = |base: &Option<PathBuf>, rel: &str| base.as_ref().map(|b| dir(b.join(rel)));

    let mut browsers = Vec::new();
    for rel in [
        "Google\\Chrome\\User Data",
        "Microsoft\\Edge\\User Data",
        "BraveSoftware\\Brave-Browser\\User Data",
    ] {
        if let Some(base) = &local {
            for profile in subdirs(&base.join(rel)) {
                for cache in ["Cache", "Code Cache", "GPUCache"] {
                    browsers.push(dir(profile.join(cache)));
                }
            }
        }
    }
    if let Some(base) = &local {
        for profile in subdirs(&base.join("Mozilla\\Firefox\\Profiles")) {
            browsers.push(dir(profile.join("cache2")));
        }
    }

    let mut thumbs = Vec::new();
    if let Some(base) = &local {
        for prefix in ["thumbcache_", "iconcache_"] {
            thumbs.push(Root {
                path: base.join("Microsoft\\Windows\\Explorer"),
                prefix: Some(prefix),
            });
        }
    }

    vec![
        Category {
            id: "user-temp",
            label: "Fichiers temporaires de vos applications",
            detail: "Seulement ceux de plus de 24 heures. Les fichiers ouverts sont laissés en place.",
            admin: false,
            selected: true,
            min_age_hours: 24,
            roots: env_path("TEMP").map(dir).into_iter().collect(),
        },
        Category {
            id: "windows-temp",
            label: "Fichiers temporaires de Windows",
            detail: "Seulement ceux de plus de 24 heures.",
            admin: true,
            selected: true,
            min_age_hours: 24,
            roots: on(&windows, "Temp").into_iter().collect(),
        },
        Category {
            id: "windows-update",
            label: "Téléchargements de Windows Update",
            detail: "Déjà installés. Ignorés si un redémarrage de mise à jour est en attente.",
            admin: true,
            selected: true,
            min_age_hours: 0,
            roots: on(&windows, "SoftwareDistribution\\Download").into_iter().collect(),
        },
        Category {
            id: "delivery",
            label: "Cache d'optimisation de la distribution",
            detail: "Copies de mises à jour partagées entre PC. Windows les retélécharge au besoin.",
            admin: true,
            selected: true,
            min_age_hours: 0,
            roots: on(&windows, "ServiceProfiles\\NetworkService\\AppData\\Local\\Microsoft\\Windows\\DeliveryOptimization\\Cache").into_iter().collect(),
        },
        Category {
            id: "thumbnails",
            label: "Miniatures et icônes",
            detail: "Windows les recrée. Les fichiers utilisés par l'Explorateur restent en place.",
            admin: false,
            selected: true,
            min_age_hours: 0,
            roots: thumbs,
        },
        Category {
            id: "shaders",
            label: "Caches de shaders des jeux",
            detail: "Les jeux peuvent saccader le temps de les recompiler. À vider après un problème graphique.",
            admin: false,
            selected: false,
            min_age_hours: 0,
            roots: [
                on(&local, "D3DSCache"),
                on(&local, "NVIDIA\\DXCache"),
                on(&local, "NVIDIA\\GLCache"),
                on(&local, "AMD\\DxCache"),
                on(&local, "AMD\\GLCache"),
            ]
            .into_iter()
            .flatten()
            .collect(),
        },
        Category {
            id: "browsers",
            label: "Caches des navigateurs",
            detail: "Chrome, Edge, Brave et Firefox. Mots de passe, cookies et historique ne sont pas touchés. Les sites rechargeront plus lentement une fois.",
            admin: false,
            selected: false,
            min_age_hours: 0,
            roots: browsers,
        },
        Category {
            id: "crash-reports",
            label: "Rapports et vidages de plantage",
            detail: "Utiles pour comprendre un plantage. À garder tant qu'un problème n'est pas résolu.",
            admin: true,
            selected: false,
            min_age_hours: 0,
            roots: [
                on(&local, "CrashDumps"),
                on(&local, "Microsoft\\Windows\\WER"),
                on(&program_data, "Microsoft\\Windows\\WER\\ReportArchive"),
                on(&program_data, "Microsoft\\Windows\\WER\\ReportQueue"),
                on(&windows, "Minidump"),
                on(&windows, "LiveKernelReports"),
            ]
            .into_iter()
            .flatten()
            .collect(),
        },
    ]
}

fn safe_root(path: &Path) -> bool {
    path.is_absolute() && path.components().count() >= 3
}

fn matches(prefix: Option<&str>, name: &std::ffi::OsStr) -> bool {
    prefix
        .map(|p| name.to_string_lossy().to_lowercase().starts_with(p))
        .unwrap_or(true)
}

fn old_enough(meta: &fs::Metadata, min_age: Duration) -> bool {
    min_age.is_zero()
        || meta
            .modified()
            .ok()
            .and_then(|m| SystemTime::now().duration_since(m).ok())
            .map(|age| age >= min_age)
            .unwrap_or(false)
}

#[derive(Default, Debug)]
struct Tally {
    bytes: u64,
    files: u64,
    skipped: u64,
    denied: bool,
}

fn walk(path: &Path, prefix: Option<&str>, min_age: Duration, delete: bool, tally: &mut Tally) {
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(e) => {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                tally.denied = true;
            }
            return;
        }
    };
    for entry in entries.flatten() {
        let Ok(meta) = fs::symlink_metadata(entry.path()) else {
            continue;
        };
        if meta.file_attributes() & REPARSE_POINT != 0 {
            continue;
        }
        if meta.is_dir() {
            if prefix.is_none() {
                walk(&entry.path(), None, min_age, delete, tally);
                if delete {
                    let _ = fs::remove_dir(entry.path());
                }
            }
            continue;
        }
        if !matches(prefix, &entry.file_name()) || !old_enough(&meta, min_age) {
            continue;
        }
        if delete && fs::remove_file(entry.path()).is_err() {
            tally.skipped += 1;
            continue;
        }
        tally.bytes += meta.len();
        tally.files += 1;
    }
}

fn measure(category: &Category, delete: bool) -> Tally {
    let mut tally = Tally::default();
    let min_age = Duration::from_secs(category.min_age_hours * 3600);
    for root in category.roots.iter().filter(|r| safe_root(&r.path)) {
        walk(&root.path, root.prefix, min_age, delete, &mut tally);
    }
    tally
}

fn wide(text: &str) -> Vec<u16> {
    std::ffi::OsStr::new(text).encode_wide().chain(Some(0)).collect()
}

fn recycle_bin() -> (u64, u64) {
    let mut info = RecycleInfo {
        size: std::mem::size_of::<RecycleInfo>() as u32,
        bytes: 0,
        items: 0,
    };
    if unsafe { SHQueryRecycleBinW(std::ptr::null(), &mut info) } == 0 {
        (info.bytes as u64, info.items as u64)
    } else {
        (0, 0)
    }
}

fn free_bytes(drive: &str) -> u64 {
    let root = wide(drive);
    let (mut free, mut total, mut all) = (0u64, 0u64, 0u64);
    unsafe { GetDiskFreeSpaceExW(root.as_ptr(), &mut free, &mut total, &mut all) };
    free
}

fn human(bytes: u64) -> String {
    let value = bytes as f64;
    let (amount, unit) = if value >= 1073741824.0 {
        (value / 1073741824.0, "Go")
    } else if value >= 1048576.0 {
        (value / 1048576.0, "Mo")
    } else {
        (value / 1024.0, "Ko")
    };
    format!("{amount:.1} {unit}").replace('.', ",")
}

fn system_drive() -> String {
    env::var("SystemDrive").unwrap_or_else(|_| "C:".into()) + "\\"
}

fn update_pending() -> bool {
    env_path("SystemRoot")
        .map(|w| w.join("WinSxS\\pending.xml").exists())
        .unwrap_or(false)
}

pub fn scan() -> Result<Value, String> {
    let admin = super::elevation::is_admin();
    let mut items = Vec::new();
    for category in categories() {
        let needs_admin = category.admin && !admin;
        let tally = if needs_admin {
            Tally::default()
        } else {
            measure(&category, false)
        };
        let blocked = category.id == "windows-update" && update_pending();
        items.push(json!({
            "id": category.id, "label": category.label, "detail": category.detail,
            "bytes": tally.bytes, "files": tally.files, "selected": category.selected,
            "needsAdmin": needs_admin, "blocked": blocked,
        }));
    }
    let (bytes, count) = recycle_bin();
    items.push(json!({
        "id": "recycle-bin", "label": "Corbeille",
        "detail": "Vérifiez qu'elle ne contient rien à récupérer avant de la vider.",
        "bytes": bytes, "files": count, "selected": false, "needsAdmin": false, "blocked": false,
    }));
    Ok(json!({"ok": true, "data": {"items": items, "freeBytes": free_bytes(&system_drive())}}))
}

pub fn clean(ids: Vec<String>) -> Result<Value, String> {
    if ids.is_empty() || ids.len() > 16 {
        return Err("Sélection invalide".into());
    }
    let admin = super::elevation::is_admin();
    let known = categories();
    if ids
        .iter()
        .any(|id| id != "recycle-bin" && !known.iter().any(|c| c.id == id))
    {
        return Err("Catégorie inconnue".into());
    }
    let drive = system_drive();
    let before = free_bytes(&drive);
    let mut results = Vec::new();
    for category in known.iter().filter(|c| ids.iter().any(|i| i == c.id)) {
        if category.id == "windows-update" && update_pending() {
            results.push(json!({"id": category.id, "label": category.label, "bytes": 0, "files": 0, "skipped": 0, "note": "Reporté : une mise à jour attend un redémarrage."}));
            continue;
        }
        if category.admin && !admin {
            results.push(json!({"id": category.id, "label": category.label, "bytes": 0, "files": 0, "skipped": 0, "note": "Droits administrateur requis."}));
            continue;
        }
        let tally = measure(category, true);
        let note = tally
            .denied
            .then_some("Windows a refusé l'accès à une partie des fichiers.");
        results.push(json!({"id": category.id, "label": category.label, "bytes": tally.bytes, "files": tally.files, "skipped": tally.skipped, "note": note}));
    }
    if ids.iter().any(|i| i == "recycle-bin") {
        let (bytes, count) = recycle_bin();
        let cleared = bytes == 0 || unsafe { SHEmptyRecycleBinW(std::ptr::null_mut(), std::ptr::null(), 0x7) } == 0;
        results.push(json!({"id": "recycle-bin", "label": "Corbeille", "bytes": if cleared { bytes } else { 0 }, "files": if cleared { count } else { 0 }, "skipped": 0, "note": if cleared { Value::Null } else { json!("Windows a refusé de vider la corbeille.") }}));
    }
    let after = free_bytes(&drive);
    let removed: u64 = results.iter().filter_map(|r| r["bytes"].as_u64()).sum();
    let skipped: u64 = results.iter().filter_map(|r| r["skipped"].as_u64()).sum();
    Ok(json!({"ok": true, "data": {
        "summary": format!("Nettoyage terminé : {} supprimés", human(removed)),
        "removedBytes": removed, "skippedFiles": skipped, "results": results,
        "measures": [{"label": format!("Espace libre sur {}", drive.trim_end_matches('\\')), "before": before, "after": after, "unit": "bytes"}],
    }}))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> PathBuf {
        let root = env::temp_dir().join(format!("nova-clean-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("sub")).unwrap();
        root
    }

    #[test]
    fn age_filter_keeps_recent_files_and_counts_nothing() {
        let root = fixture("age");
        fs::write(root.join("new.tmp"), b"12345").unwrap();
        let mut tally = Tally::default();
        walk(&root, None, Duration::from_secs(3600), true, &mut tally);
        assert_eq!(tally.files, 0);
        assert!(root.join("new.tmp").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn delete_counts_real_bytes_and_removes_empty_folders_but_not_the_root() {
        let root = fixture("delete");
        fs::write(root.join("a.tmp"), b"12345").unwrap();
        fs::write(root.join("sub").join("b.tmp"), b"123").unwrap();
        let mut scan = Tally::default();
        walk(&root, None, Duration::ZERO, false, &mut scan);
        assert_eq!((scan.bytes, scan.files), (8, 2));
        assert!(root.join("a.tmp").exists());
        let mut tally = Tally::default();
        walk(&root, None, Duration::ZERO, true, &mut tally);
        assert_eq!((tally.bytes, tally.files), (8, 2));
        assert!(root.exists() && !root.join("sub").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn locked_files_are_skipped_not_counted() {
        use std::os::windows::fs::OpenOptionsExt;
        let root = fixture("locked");
        let path = root.join("locked.tmp");
        fs::write(&path, b"12345").unwrap();
        let held = fs::OpenOptions::new().read(true).share_mode(0).open(&path).unwrap();
        let mut tally = Tally::default();
        walk(&root, None, Duration::ZERO, true, &mut tally);
        assert_eq!((tally.bytes, tally.skipped), (0, 1));
        drop(held);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn junctions_are_never_followed() {
        let root = fixture("junction");
        let outside = fixture("outside");
        fs::write(outside.join("keep.txt"), b"keep").unwrap();
        let link = root.join("link");
        let made = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J", link.to_str().unwrap(), outside.to_str().unwrap()])
            .output()
            .unwrap();
        assert!(made.status.success());
        let mut tally = Tally::default();
        walk(&root, None, Duration::ZERO, true, &mut tally);
        assert!(outside.join("keep.txt").exists());
        let _ = fs::remove_dir(&link);
        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(outside).unwrap();
    }

    #[test]
    fn prefix_roots_only_touch_matching_files() {
        let root = fixture("prefix");
        fs::write(root.join("thumbcache_1.db"), b"x").unwrap();
        fs::write(root.join("other.db"), b"x").unwrap();
        fs::write(root.join("sub").join("thumbcache_2.db"), b"x").unwrap();
        let mut tally = Tally::default();
        walk(&root, Some("thumbcache_"), Duration::ZERO, true, &mut tally);
        assert_eq!(tally.files, 1);
        assert!(root.join("other.db").exists() && root.join("sub").join("thumbcache_2.db").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn shallow_roots_are_refused() {
        assert!(!safe_root(Path::new("C:\\")));
        assert!(!safe_root(Path::new("relative\\path")));
        assert!(safe_root(Path::new("C:\\Windows\\Temp")));
    }
}
