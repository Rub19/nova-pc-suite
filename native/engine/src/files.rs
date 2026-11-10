use serde_json::{json, Value};
use std::collections::HashMap;
use std::hash::Hasher;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

const KINDS: [(&str, &[&str]); 6] = [
    ("Vidéos", &["mp4", "mkv", "avi", "mov", "webm", "wmv", "m4v"]),
    ("Images", &["jpg", "jpeg", "png", "gif", "webp", "heic", "bmp", "raw", "cr2", "nef"]),
    ("Musique", &["mp3", "flac", "wav", "m4a", "ogg", "aac"]),
    ("Archives", &["zip", "rar", "7z", "tar", "gz", "iso"]),
    ("Installeurs", &["exe", "msi", "msix", "appx", "msixbundle"]),
    ("Documents", &["pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx", "odt", "ods", "txt", "csv"]),
];
const OTHER: &str = "Autres";
const OLD_DAYS: u64 = 90;
const BIG: u64 = 100 * 1024 * 1024;
const MIN_DUPLICATE: u64 = 1024 * 1024;
const HASH_BUDGET: u64 = 8 * 1024 * 1024 * 1024;

pub struct Entry {
    pub path: PathBuf,
    pub bytes: u64,
    pub age_days: u64,
}

pub fn kind_of(path: &Path) -> &'static str {
    let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    KINDS.iter().find(|(_, list)| list.contains(&ext.as_str())).map_or(OTHER, |(label, _)| label)
}

fn collect(dir: &Path, depth: usize, now: SystemTime, out: &mut Vec<Entry>) {
    let Ok(read) = std::fs::read_dir(dir) else { return };
    for entry in read.flatten() {
        let Ok(kind) = entry.file_type() else { continue };
        if kind.is_symlink() {
            continue;
        }
        if kind.is_dir() {
            if depth > 0 {
                collect(&entry.path(), depth - 1, now, out);
            }
            continue;
        }
        let Ok(meta) = entry.metadata() else { continue };
        let age = meta.modified().ok().and_then(|m| now.duration_since(m).ok()).unwrap_or(Duration::ZERO);
        out.push(Entry { path: entry.path(), bytes: meta.len(), age_days: age.as_secs() / 86_400 });
    }
}

pub fn by_kind(entries: &[Entry]) -> Vec<(String, u64, u64)> {
    let mut totals: HashMap<&'static str, (u64, u64)> = HashMap::new();
    for entry in entries {
        let slot = totals.entry(kind_of(&entry.path)).or_default();
        slot.0 += entry.bytes;
        slot.1 += 1;
    }
    let mut list: Vec<(String, u64, u64)> = totals.into_iter().filter(|(_, (bytes, _))| *bytes > 0).map(|(label, (bytes, files))| (label.to_string(), bytes, files)).collect();
    list.sort_by(|a, b| b.1.cmp(&a.1));
    list
}

pub fn old_and_big(entries: &[Entry]) -> Vec<&Entry> {
    let mut list: Vec<&Entry> = entries.iter().filter(|e| e.bytes >= BIG && e.age_days >= OLD_DAYS).collect();
    list.sort_by(|a, b| b.bytes.cmp(&a.bytes));
    list.truncate(15);
    list
}

fn fingerprint(path: &Path) -> Option<u64> {
    let mut file = std::fs::File::open(path).ok()?;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    let mut buffer = vec![0u8; 1 << 20];
    loop {
        let read = file.read(&mut buffer).ok()?;
        if read == 0 {
            break;
        }
        hasher.write(&buffer[..read]);
    }
    Some(hasher.finish())
}

pub fn duplicates(entries: &[Entry], same: impl Fn(&Path) -> Option<u64>) -> Vec<(u64, Vec<PathBuf>)> {
    let mut by_size: HashMap<u64, Vec<&Entry>> = HashMap::new();
    for entry in entries.iter().filter(|e| e.bytes >= MIN_DUPLICATE) {
        by_size.entry(entry.bytes).or_default().push(entry);
    }
    let mut sizes: Vec<(u64, Vec<&Entry>)> = by_size.into_iter().filter(|(_, list)| list.len() > 1).collect();
    sizes.sort_by(|a, b| b.0.cmp(&a.0));
    let mut budget = HASH_BUDGET;
    let mut groups = Vec::new();
    for (bytes, list) in sizes {
        let cost = bytes * list.len() as u64;
        if cost > budget {
            continue;
        }
        budget -= cost;
        let mut by_hash: HashMap<u64, Vec<PathBuf>> = HashMap::new();
        for entry in list {
            if let Some(hash) = same(&entry.path) {
                by_hash.entry(hash).or_default().push(entry.path.clone());
            }
        }
        groups.extend(by_hash.into_values().filter(|paths| paths.len() > 1).map(|mut paths| {
            paths.sort();
            (bytes, paths)
        }));
    }
    groups.sort_by(|a, b| (b.0 * b.1.len() as u64).cmp(&(a.0 * a.1.len() as u64)));
    groups.truncate(15);
    groups
}

fn known(id: &windows::core::GUID) -> Option<PathBuf> {
    use windows::Win32::System::Com::CoTaskMemFree;
    use windows::Win32::UI::Shell::{SHGetKnownFolderPath, KNOWN_FOLDER_FLAG};
    unsafe {
        let raw = SHGetKnownFolderPath(id, KNOWN_FOLDER_FLAG(0), None).ok()?;
        let path = raw.to_string().ok().map(PathBuf::from);
        CoTaskMemFree(Some(raw.0 as *const _));
        path
    }
}

pub fn analyse() -> Result<Value, String> {
    use windows::Win32::UI::Shell::{FOLDERID_Desktop, FOLDERID_Documents, FOLDERID_Downloads, FOLDERID_Music, FOLDERID_Pictures, FOLDERID_Videos};
    let downloads = known(&FOLDERID_Downloads).ok_or("Dossier Téléchargements introuvable.")?;
    let now = SystemTime::now();
    let mut personal = Vec::new();
    for id in [&FOLDERID_Desktop, &FOLDERID_Documents, &FOLDERID_Pictures, &FOLDERID_Videos, &FOLDERID_Music] {
        if let Some(dir) = known(id).filter(|dir| !dir.starts_with(&downloads)) {
            collect(&dir, 6, now, &mut personal);
        }
    }
    let mut loaded = Vec::new();
    collect(&downloads, 3, now, &mut loaded);
    let kinds: Vec<Value> = by_kind(&personal.iter().chain(loaded.iter()).map(|e| Entry { path: e.path.clone(), bytes: e.bytes, age_days: e.age_days }).collect::<Vec<_>>())
        .into_iter()
        .map(|(label, bytes, files)| json!({"label": label, "bytes": bytes, "files": files}))
        .collect();
    let old: Vec<Value> = old_and_big(&loaded).into_iter().map(|e| json!({"name": e.path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(), "path": e.path.to_string_lossy(), "bytes": e.bytes, "days": e.age_days})).collect();
    let twins: Vec<Value> = duplicates(&loaded, fingerprint).into_iter().map(|(bytes, paths)| json!({"name": paths[0].file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(), "bytes": bytes, "paths": paths.iter().map(|p| p.to_string_lossy().to_string()).collect::<Vec<_>>()})).collect();
    Ok(json!({"downloads": downloads.to_string_lossy(), "kinds": kinds, "old": old, "duplicates": twins}))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(path: &str, bytes: u64, age_days: u64) -> Entry {
        Entry { path: PathBuf::from(path), bytes, age_days }
    }

    #[test]
    fn files_are_sorted_by_kind_age_and_content() {
        assert_eq!(kind_of(Path::new("film.MKV")), "Vidéos");
        assert_eq!(kind_of(Path::new("setup.exe")), "Installeurs");
        assert_eq!(kind_of(Path::new("sans-extension")), "Autres");
        let list = vec![entry("a.mp4", 500, 1), entry("b.mp4", 300, 1), entry("c.zip", 900, 1)];
        assert_eq!(by_kind(&list), vec![("Archives".to_string(), 900, 1), ("Vidéos".to_string(), 800, 2)]);
        let big = 200 * 1024 * 1024;
        let old = vec![entry("vieux.iso", big, 200), entry("recent.iso", big, 10), entry("petit.zip", 10, 400)];
        assert_eq!(old_and_big(&old).iter().map(|e| e.path.to_string_lossy().to_string()).collect::<Vec<_>>(), vec!["vieux.iso"]);
        let mega = 2 * 1024 * 1024;
        let twins = vec![entry("x.zip", mega, 1), entry("x (1).zip", mega, 1), entry("y.zip", mega, 1), entry("petit1", 10, 1), entry("petit2", 10, 1)];
        let found = duplicates(&twins, |path| Some(if path.to_string_lossy().starts_with('x') { 1 } else { 2 }));
        assert_eq!(found, vec![(mega, vec![PathBuf::from("x (1).zip"), PathBuf::from("x.zip")])]);
    }

    #[test]
    #[ignore]
    fn real_folders_are_analysed() {
        let started = std::time::Instant::now();
        let data = analyse().unwrap();
        let kinds: Vec<String> = data["kinds"].as_array().unwrap().iter().map(|k| format!("{} {} Mo", k["label"].as_str().unwrap(), k["bytes"].as_u64().unwrap() / 1_048_576)).collect();
        println!("{:?} | vieux gros : {} | doublons : {} | {:.1} s", kinds, data["old"].as_array().unwrap().len(), data["duplicates"].as_array().unwrap().len(), started.elapsed().as_secs_f32());
        assert!(!kinds.is_empty());
    }

    #[test]
    fn identical_files_have_the_same_fingerprint() {
        let dir = std::env::temp_dir().join(format!("nova-files-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a"), b"meme contenu").unwrap();
        std::fs::write(dir.join("b"), b"meme contenu").unwrap();
        std::fs::write(dir.join("c"), b"autre contenu").unwrap();
        assert_eq!(fingerprint(&dir.join("a")), fingerprint(&dir.join("b")));
        assert_ne!(fingerprint(&dir.join("a")), fingerprint(&dir.join("c")));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
